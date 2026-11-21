package main

import (
	"encoding/base64"
	"fmt"

	"github.com/veraison/go-cose"
)

// controls are the negative controls, which run first because a judge that cannot refuse
// proves nothing. Each message is built with the library, wrong in one way, and must fail the
// named check and no check that the fault does not touch.
func controls(r *Report, keys *Keys) {
	document := map[string]any{
		"@context": []any{"https://www.w3.org/ns/credentials/v2"},
		"type":     []any{"VerifiableCredential"},
	}
	p256 := keys.ByName["ES256"]
	p384 := keys.ByName["ES384"]
	good := func() []byte {
		raw, err := signDocument(p256, "vc", p256.ID, document, nil, nil)
		if err != nil {
			r.fail("controls: %v", err)
		}
		return raw
	}
	payload, _ := marshalDocument(document)
	sign := func(key *Key, h Header, payload []byte) []byte {
		raw, err := signMessage(key, h, payload, nil)
		if err != nil {
			r.fail("controls: %v", err)
		}
		return raw
	}
	standard := Header{Typ: typOf["vc"], Cty: ctyOf["vc"], Kid: p256.ID}

	// The control of the controls: a valid message passes every check.
	if failed := judgeMessage(good(), keys, Options{Kind: "vc", Expected: document}).Failed(); len(failed) != 0 {
		r.fail("controls: a valid message failed %v", failed)
	}

	flipped := good()
	flipped[len(flipped)-1] ^= 1

	changed := func() []byte {
		var msg cose.Sign1Message
		if err := msg.UnmarshalCBOR(good()); err != nil {
			r.fail("controls: %v", err)
			return nil
		}
		msg.Payload = append([]byte(nil), msg.Payload...)
		msg.Payload[3] ^= 1
		raw, err := msg.MarshalCBOR()
		if err != nil {
			r.fail("controls: %v", err)
		}
		return raw
	}()

	untagged := good()[1:]

	wrongKeyKid := standard
	wrongKeyKid.Kid = p384.ID // names the P-384 key, signed by the P-256 key

	badTyp, badCty, noTyp, unknownKid, claimed, withUnprotected := standard, standard, standard, standard, standard, standard
	withUnprotected.Unprotected = map[any]any{int64(99): "x"}
	badTyp.Typ = "really/bad"
	badCty.Cty = "very/bad"
	noTyp.Typ = nil
	unknownKid.Kid = "https://nobody.example/keys#1"
	claimed.Claims = cose.CWTClaims{cose.CWTClaimIssuer: "https://issuer.example"}

	for _, c := range []struct {
		label  string
		raw    []byte
		failed []string
	}{
		{"a flipped signature", flipped, []string{"signature"}},
		{"a changed payload", changed, []string{"signature"}},
		{"a typ that is not the media type", sign(p256, badTyp, payload), []string{"typ"}},
		{"a content type that is not the media type", sign(p256, badCty, payload), []string{"cty"}},
		{"no typ", sign(p256, noTyp, payload), []string{"typ"}},
		{"a kid that names no key", sign(p256, unknownKid, payload), []string{"kid"}},
		{"a kid that names another algorithm's key", sign(p256, wrongKeyKid, payload), []string{"alg", "signature"}},
		{"CWT claims in the protected header", sign(p256, claimed, payload), []string{"claims"}},
		{"a payload that is another document", sign(p256, standard, []byte(`{"other":true}`)), []string{"payload"}},
		{"bytes that are not a COSE_Sign1", []byte("not cose"), []string{"cose"}},
		{"an untagged COSE_Sign1", untagged, []string{"cose"}},
		{"a parameter in the unprotected header", sign(p256, withUnprotected, payload), []string{"unprotected"}},
	} {
		r.Run++
		got := judgeMessage(c.raw, keys, Options{Kind: "vc", Expected: document}).Failed()
		if fmt.Sprint(got) != fmt.Sprint(c.failed) {
			r.fail("control %q: failed %v, expected %v", c.label, got, c.failed)
		}
	}

	// A presentation is not a credential: the media types are the kind's.
	vp, _ := signDocument(p256, "vp", p256.ID, document, nil, nil)
	r.Run++
	if got := judgeMessage(vp, keys, Options{Kind: "vc", Expected: document}).Failed(); fmt.Sprint(got) != fmt.Sprint([]string{"typ", "cty"}) {
		r.fail("control %q: failed %v", "a presentation judged as a credential", got)
	}

	// An envelope must be padded standard base64 with no whitespace, as this crate writes it.
	raw := good()
	padded := base64.StdEncoding.EncodeToString(raw)
	for _, c := range []struct{ label, data string }{
		{"unpadded base64", base64.RawStdEncoding.EncodeToString(raw)},
		{"base64 with a line feed", padded + "\n"},
		{"base64url", base64.URLEncoding.EncodeToString(raw)},
	} {
		doc := map[string]any{"verifiableCredential": []any{map[string]any{"id": vcCosePrefix + c.data}}}
		r.Run++
		_, _, err := envelopeOf(doc, 0)
		strict := base64.StdEncoding.EncodeToString(raw) == c.data
		if err == nil && !strict {
			r.fail("control %q: the envelope was accepted", c.label)
		}
	}
	doc := map[string]any{"verifiableCredential": []any{map[string]any{"id": vcCosePrefix + padded}}}
	r.Run++
	if _, _, err := envelopeOf(doc, 0); err != nil {
		r.fail("control: a well-formed envelope was refused: %v", err)
	}
}
