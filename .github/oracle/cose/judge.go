package main

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/veraison/go-cose"
)

// Check is one thing the library has to agree on, and whether it did.
type Check struct {
	Name   string
	OK     bool
	Detail string
}

// Judgement is what the library makes of one message.
type Judgement struct {
	Checks  []Check
	Message *cose.Sign1Message
	Key     *Key
}

func (j *Judgement) note(name string, ok bool, detail string, args ...any) {
	c := Check{Name: name, OK: ok}
	if !ok {
		c.Detail = fmt.Sprintf(detail, args...)
	}
	j.Checks = append(j.Checks, c)
}

// Failed lists the names of the checks that failed.
func (j *Judgement) Failed() []string {
	var names []string
	for _, c := range j.Checks {
		if !c.OK {
			names = append(names, c.Name)
		}
	}
	return names
}

func (j *Judgement) failed(name string) bool {
	for _, c := range j.Checks {
		if c.Name == name && !c.OK {
			return true
		}
	}
	return false
}

// Options says which checks apply to a message.
type Options struct {
	Kind        string // "vc" or "vp"
	Expected    any    // the document the payload must be, when known
	AllowClaims bool   // a message that carries CWT claims in its protected header
}

// judgeMessage has the library read one message and say whether it agrees on each of:
//
//	cose         the bytes are a tagged COSE_Sign1 the library can read.
//	unprotected  the unprotected header is empty, which is what this crate writes.
//	claims       no CWT Claims header parameter (15) is carried, unless allowed.
//	typ, cty     application/vc+cose and application/vc for a credential,
//	             application/vp+cose and application/vp for a presentation, as exact
//	             strings, as this crate writes them.
//	kid          the protected kid names a published key.
//	alg          the protected alg is the key's own, from its kty and crv.
//	signature    the library verifies the signature under that key, as a Verifier for the
//	             key's own algorithm, so a swapped algorithm is refused.
//	payload      the payload is the document that was signed, with key order removed.
func judgeMessage(raw []byte, keys *Keys, opt Options) *Judgement {
	j := &Judgement{}
	var msg cose.Sign1Message
	if err := msg.UnmarshalCBOR(raw); err != nil {
		j.note("cose", false, "%v", err)
		return j
	}
	j.note("cose", true, "")
	j.Message = &msg

	j.note("unprotected", len(msg.Headers.Unprotected) == 0, "the unprotected header is not empty: %v", msg.Headers.Unprotected)
	_, hasClaims := msg.Headers.Protected[cose.HeaderLabelCWTClaims]
	j.note("claims", opt.AllowClaims || !hasClaims, "the protected header carries CWT claims")

	typ, _ := msg.Headers.Protected[cose.HeaderLabelType].(string)
	j.note("typ", typ == typOf[opt.Kind], "typ is %q, not %q", msg.Headers.Protected[cose.HeaderLabelType], typOf[opt.Kind])
	cty, _ := msg.Headers.Protected[cose.HeaderLabelContentType].(string)
	j.note("cty", cty == ctyOf[opt.Kind], "the content type is %q, not %q", msg.Headers.Protected[cose.HeaderLabelContentType], ctyOf[opt.Kind])

	kidBytes, _ := msg.Headers.Protected[cose.HeaderLabelKeyID].([]byte)
	key := keys.Published[string(kidBytes)]
	j.note("kid", key != nil, "kid %q names no published key", kidBytes)
	if key == nil {
		return j
	}
	j.Key = key

	// The key's own algorithm, never the header's: a message does not get to choose.
	alg, err := msg.Headers.Protected.Algorithm()
	j.note("alg", err == nil && alg == key.Alg, "header alg %v (%v), key alg %v", alg, err, key.Alg)

	verifier, err := cose.NewVerifier(key.Alg, key.Pub)
	if err != nil {
		j.note("signature", false, "no verifier: %v", err)
		return j
	}
	if err := msg.Verify(nil, verifier); err != nil {
		j.note("signature", false, "%v", err)
		return j
	}
	j.note("signature", true, "")

	if opt.Expected != nil {
		var parsed any
		err := json.Unmarshal(msg.Payload, &parsed)
		j.note("payload", err == nil && same(parsed, opt.Expected), "the payload is not the document that was signed (%v)", err)
	}
	return j
}

// envelopeOf reads the COSE envelope at `index` of a presentation's verifiableCredential, as
// this crate writes it: a data: URL whose data is padded standard base64 with no whitespace,
// which the strict decoder of the library's own language enforces.
func envelopeOf(document map[string]any, index int) ([]byte, string, error) {
	list, _ := document["verifiableCredential"].([]any)
	if index >= len(list) {
		return nil, "", fmt.Errorf("no envelope %d", index)
	}
	entry, _ := list[index].(map[string]any)
	id, _ := entry["id"].(string)
	if !strings.HasPrefix(id, vcCosePrefix) {
		return nil, id, fmt.Errorf("envelope %d is not %s...", index, vcCosePrefix)
	}
	data := strings.TrimPrefix(id, vcCosePrefix)
	// Go's decoder skips carriage returns and line feeds, which this crate does not write.
	if strings.ContainsAny(data, "\r\n\t ") {
		return nil, id, fmt.Errorf("envelope %d has whitespace in its base64", index)
	}
	raw, err := base64.StdEncoding.Strict().DecodeString(data)
	return raw, id, err
}

// decodeCase reads a corpus case.
type corpusEnvelope struct {
	Index    int            `json:"index"`
	Document map[string]any `json:"document"`
}

type corpusCase struct {
	ID        string           `json:"id"`
	Group     string           `json:"group"`
	Kind      string           `json:"kind"`
	Alg       string           `json:"alg"`
	Kid       string           `json:"kid"`
	Message   string           `json:"message"`
	Document  map[string]any   `json:"document"`
	Envelopes []corpusEnvelope `json:"envelopes"`
	Now       *int64           `json:"now"`
	Audience  string           `json:"audience"`
}

// Report collects what the run found.
type Report struct {
	Failures []string
	Judged   int
	Envelope int
	Run      int // negative controls
}

func (r *Report) fail(format string, args ...any) {
	r.Failures = append(r.Failures, fmt.Sprintf(format, args...))
}

// judgeCase judges a message and every envelope a presentation carries, and records what
// failed.
func judgeCase(r *Report, keys *Keys, c corpusCase) {
	raw, err := base64.StdEncoding.Strict().DecodeString(c.Message)
	if err != nil || strings.ContainsAny(c.Message, "\r\n\t ") {
		r.fail("%s: the message is not padded base64: %v", c.ID, err)
		return
	}
	j := judgeMessage(raw, keys, Options{Kind: c.Kind, Expected: c.Document})
	r.Judged++
	for _, check := range j.Checks {
		if !check.OK {
			r.fail("%s: %s: %s", c.ID, check.Name, check.Detail)
		}
	}
	if j.Message != nil {
		if kid, _ := j.Message.Headers.Protected[cose.HeaderLabelKeyID].([]byte); string(kid) != c.Kid {
			r.fail("%s: the case lists kid %q and the message names %q", c.ID, c.Kid, kid)
		}
	}
	if c.Kind != "vp" {
		return
	}
	listed := 0
	for _, e := range c.Envelopes {
		innerRaw, _, err := envelopeOf(c.Document, e.Index)
		if err != nil {
			r.fail("%s: envelope %d: %v", c.ID, e.Index, err)
			continue
		}
		inner := judgeMessage(innerRaw, keys, Options{Kind: "vc", Expected: e.Document})
		r.Envelope++
		for _, check := range inner.Checks {
			if !check.OK {
				r.fail("%s: envelope %d: %s: %s", c.ID, e.Index, check.Name, check.Detail)
			}
		}
		listed++
	}
	if cose := coseEnvelopes(c.Document); listed != cose {
		r.fail("%s: the presentation holds %d COSE envelopes and the case lists %d", c.ID, cose, listed)
	}
}

// coseEnvelopes counts the vc+cose envelopes of a presentation. The suite's own presentation
// envelops a vc+jwt, which is not this oracle's to judge.
func coseEnvelopes(document map[string]any) int {
	list, _ := document["verifiableCredential"].([]any)
	n := 0
	for _, entry := range list {
		if e, ok := entry.(map[string]any); ok {
			if id, _ := e["id"].(string); strings.HasPrefix(id, "data:application/vc+cose") {
				n++
			}
		}
	}
	return n
}

// judgeCorpus reads the corpus the Rust side wrote and judges every case.
func judgeCorpus(r *Report, keys *Keys, path string) []corpusCase {
	raw, err := os.ReadFile(path)
	if err != nil {
		r.fail("cannot read the corpus: %v", err)
		return nil
	}
	var file struct {
		Cases []corpusCase `json:"cases"`
	}
	if err := json.Unmarshal(raw, &file); err != nil {
		r.fail("the corpus is not JSON: %v", err)
		return nil
	}
	for _, c := range file.Cases {
		judgeCase(r, keys, c)
	}
	return file.Cases
}

// floor holds the corpus to what it must contain: a loop over an empty or partial corpus
// passes, and so would a judge that never saw P-521.
func floor(r *Report, cases []corpusCase) {
	have := map[string]int{}
	for _, c := range cases {
		have[c.Group+"/"+c.Kind+"/"+c.Alg]++
		have["id/"+c.ID]++
	}
	for _, a := range algorithms {
		for _, group := range []string{"credential/vc", "presentation/vp"} {
			if have[group+"/"+a.name] == 0 {
				r.fail("floor: the corpus holds no %s signed under %s", group, a.name)
			}
		}
	}
	for _, id := range []string{"suite/row-27", "suite/row-28"} {
		if have["id/"+id] == 0 {
			r.fail("floor: the corpus holds no %s", id)
		}
	}
	if r.Envelope == 0 {
		r.fail("floor: no envelope was judged")
	}
}

// suiteInputs judges the suite's own COSE inputs under their keys. Each row has what it must
// be: the two that the suite expects to verify must verify, and each of the others must be
// refused for the one check that names its fault, which shows the library draws the line where
// this crate does.
func suiteInputs(r *Report, keys *Keys, dir string) {
	read := func(name string) []byte {
		text, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			r.fail("suite input %s: %v", name, err)
			return nil
		}
		// The suite writes standard base64 with no padding, and some files end in a newline.
		raw, err := base64.RawStdEncoding.DecodeString(strings.TrimRight(strings.TrimSpace(string(text)), "="))
		if err != nil {
			r.fail("suite input %s: %v", name, err)
		}
		return raw
	}
	for _, row := range []struct {
		row, file, kind, key string
		failing              []string // checks that must fail; empty means all must pass
	}{
		{"29", "credential-cose-minimal.txt", "vc", "ES256", nil},
		{"31", "presentation-cose-single.txt", "vp", "ES384", nil},
		{"30", "credential-cose-minimal-not-base64.txt", "vc", "ES256", []string{"cose"}},
		{"32", "credential-cose-bad-signature.txt", "vc", "EdDSA", []string{"signature"}},
		{"33", "credential-cose-bad-media-type.txt", "vc", "ES384", []string{"typ", "cty"}},
		{"34", "presentation-cose-bad-media-type.txt", "vp", "ES256", []string{"typ", "cty"}},
		{"35", "presentation-cose-bad-credential.txt", "vp", "ES256", nil},
	} {
		raw := read(row.file)
		if raw == nil {
			continue
		}
		j := judgeMessage(raw, keys, Options{Kind: row.kind})
		r.Judged++
		got := j.Failed()
		sort.Strings(got)
		want := append([]string(nil), row.failing...)
		sort.Strings(want)
		if strings.Join(got, ",") != strings.Join(want, ",") {
			r.fail("suite row %s (%s): the library failed [%s], expected [%s]: %v", row.row, row.file, strings.Join(got, ","), strings.Join(want, ","), j.Checks)
		}
		if j.Key != nil && j.Key.Name != row.key {
			r.fail("suite row %s: the kid names a %s key, expected %s", row.row, j.Key.Name, row.key)
		}
	}
	// Row 34 is signed by the P-256 key the suite maps to P-384: under the mapped key it is
	// refused for its algorithm.
	raw := read("presentation-cose-bad-media-type.txt")
	if raw != nil {
		var msg cose.Sign1Message
		if err := msg.UnmarshalCBOR(raw); err != nil {
			r.fail("suite row 34: %v", err)
		} else if verifier, err := cose.NewVerifier(keys.ByName["ES384"].Alg, keys.ByName["ES384"].Pub); err != nil {
			r.fail("suite row 34: %v", err)
		} else if err := msg.Verify(nil, verifier); err == nil {
			r.fail("suite row 34: verified under the P-384 key it is mapped to")
		}
	}
}
