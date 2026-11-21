package main

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/veraison/go-cose"
)

// The documents the library signs. They differ from this crate's in what is not meaning: the
// payload is Go's JSON of the document, with its keys sorted, and not RFC 8785 canonical form.
// One credential carries a member that is in no vocabulary, which a verifier ignores
// (VC-JOSE-COSE 3.3.3), and one carries the names of JWT claims, which are members of a COSE
// payload and not claims, so a verifier given a time and an audience must not enforce them.

type obj = map[string]any

func baseCredential() obj {
	return obj{
		"@context":          []any{"https://www.w3.org/ns/credentials/v2"},
		"id":                "urn:uuid:7b0a4c1e-3f5d-4c8e-9a41-2d6f8e0b1c33",
		"type":              []any{"VerifiableCredential"},
		"issuer":            "https://issuer.example",
		"validFrom":         "2025-01-01T00:00:00Z",
		"credentialSubject": obj{"id": "did:example:subject", "name": "Example"},
	}
}

type shape struct {
	name     string
	document obj
	claims   cose.CWTClaims
	now      *int64
	audience string
	bare     bool // names the key by the thumbprint of its JWK, which a verifier accepts
}

func shapes() []shape {
	text := baseCredential()
	text["issuer"] = obj{"id": "https://issuer.example", "name": "Zażółć gęślą jaźń"}
	text["credentialSubject"] = []any{
		obj{"id": "did:example:one", "name": "日本語 ✓ 😀", "nested": obj{"a": []any{1, obj{"b": nil}}, "c": "\u007f\"\\<>&"}},
		obj{"id": "did:example:two", "name": "Example"},
	}
	numbers := baseCredential()
	numbers["credentialSubject"] = obj{"id": "did:example:subject", "small": 0.000001, "large": 1e21, "negative": -0.5, "integer": 9007199254740991, "zero": 0}
	issuerObject := baseCredential()
	issuerObject["issuer"] = obj{"id": "https://issuer.example"}
	issuerObject["validUntil"] = "2099-01-01T00:00:00Z"
	unknown := baseCredential()
	unknown["notInAnyVocabulary"] = obj{"a": 1}
	names := baseCredential()
	names["exp"] = 1
	names["nbf"] = 4000000000
	names["aud"] = "https://nobody.example"
	names["iss"] = "https://other.example"
	cwtDoc := baseCredential()
	now := int64(1800000000)
	jwtNow := int64(1800000000)
	return []shape{
		{name: "base", document: baseCredential()},
		{name: "text", document: text},
		{name: "numbers", document: numbers},
		{name: "issuer-object", document: issuerObject},
		{name: "undeclared-member", document: unknown},
		{name: "bare-kid", document: baseCredential(), bare: true},
		{name: "jwt-names-in-the-payload", document: names, now: &jwtNow},
		{
			name:     "cwt-claims",
			document: cwtDoc,
			claims: cose.CWTClaims{
				cose.CWTClaimIssuer:         "https://issuer.example",
				cose.CWTClaimAudience:       "https://verifier.example",
				cose.CWTClaimExpirationTime: int64(4000000000),
				cose.CWTClaimNotBefore:      int64(1700000000),
				cose.CWTClaimIssuedAt:       int64(1700000000),
			},
			now:      &now,
			audience: "https://verifier.example",
		},
	}
}

type issuedEnvelope struct {
	Index    int            `json:"index"`
	Kid      string         `json:"kid"`
	Message  string         `json:"message"`
	Document map[string]any `json:"document"`
}

type issuedCase struct {
	ID        string           `json:"id"`
	Kind      string           `json:"kind"`
	Alg       string           `json:"alg"`
	Kid       string           `json:"kid"`
	Message   string           `json:"message"`
	Document  map[string]any   `json:"document"`
	Now       *int64           `json:"now,omitempty"`
	Audience  string           `json:"audience,omitempty"`
	Envelopes []issuedEnvelope `json:"envelopes,omitempty"`
}

// issue has the library sign credentials and presentations under EdDSA, ES256, ES384 and
// ES512 and the suite's keys, for `dpp_vc::jose` to verify. It repeats each ECDSA signature
// until it has the S that alternates high and low, so the corpus holds both on purpose: this
// workspace signs low-S and verifies both, and the library does not normalise.
func issue(keys *Keys, out string) error {
	var cases []issuedCase
	var problems []string
	made := map[string]int{}
	wantHigh := func(alg, kind string) *bool {
		if !isEcdsa(alg) {
			return nil
		}
		key := alg + " " + kind
		high := made[key]%2 == 0
		made[key]++
		return &high
	}
	selfCheck := func(id string, raw []byte, kind string, document map[string]any, claims bool) {
		j := judgeMessage(raw, keys, Options{Kind: kind, Expected: document, AllowClaims: claims})
		if failed := j.Failed(); len(failed) != 0 {
			problems = append(problems, fmt.Sprintf("%s: the library cannot read its own message: %v", id, failed))
		}
	}
	encode := func(raw []byte) string { return base64.StdEncoding.EncodeToString(raw) }

	for _, a := range algorithms {
		key := keys.ByName[a.name]
		for _, s := range shapes() {
			kid := key.ID
			if s.bare {
				kid = key.Thumbprint
			}
			id := fmt.Sprintf("credential/%s/%s", a.name, s.name)
			raw, err := signDocument(key, "vc", kid, s.document, wantHigh(a.name, "vc"), s.claims)
			if err != nil {
				return fmt.Errorf("%s: %w", id, err)
			}
			selfCheck(id, raw, "vc", s.document, s.claims != nil)
			cases = append(cases, issuedCase{ID: id, Kind: "vc", Alg: a.name, Kid: kid, Message: encode(raw), Document: s.document, Now: s.now, Audience: s.audience})
		}
		// Each presentation envelopes a credential signed under one algorithm, rotating, so
		// every algorithm's credentials appear in a presentation under every other.
		for n := 0; n < 2; n++ {
			issuer := keys.ByName[algorithms[(indexOf(a.name)+n)%4].name]
			inner := baseCredential()
			inner["id"] = fmt.Sprintf("urn:uuid:5a4c1b7e-0000-4000-8000-00000000000%d", n)
			innerID := fmt.Sprintf("presentation/%s/%d/envelope", a.name, n)
			innerRaw, err := signDocument(issuer, "vc", issuer.ID, inner, wantHigh(issuer.Name, "vc"), nil)
			if err != nil {
				return fmt.Errorf("%s: %w", innerID, err)
			}
			selfCheck(innerID, innerRaw, "vc", inner, false)
			document := obj{
				"@context": []any{"https://www.w3.org/ns/credentials/v2"},
				"id":       fmt.Sprintf("urn:uuid:0d9c6b0e-5a3c-4f43-8f0e-6f2a8d1c9e5%d", n),
				"type":     []any{"VerifiablePresentation"},
				"holder":   "https://holder.example",
				"verifiableCredential": []any{obj{
					"@context": "https://www.w3.org/ns/credentials/v2",
					"type":     "EnvelopedVerifiableCredential",
					"id":       vcCosePrefix + encode(innerRaw),
				}},
			}
			id := fmt.Sprintf("presentation/%s/%d", a.name, n)
			raw, err := signDocument(key, "vp", key.ID, document, wantHigh(a.name, "vp"), nil)
			if err != nil {
				return fmt.Errorf("%s: %w", id, err)
			}
			selfCheck(id, raw, "vp", document, false)
			cases = append(cases, issuedCase{
				ID: id, Kind: "vp", Alg: a.name, Kid: key.ID, Message: encode(raw), Document: document,
				Envelopes: []issuedEnvelope{{Index: 0, Kid: issuer.ID, Message: encode(innerRaw), Document: inner}},
			})
		}
	}

	// What the file holds, counted from the signatures themselves.
	form := map[string][2]int{}
	tally := func(message string) {
		raw, _ := base64.StdEncoding.DecodeString(message)
		var msg cose.Sign1Message
		if err := msg.UnmarshalCBOR(raw); err != nil {
			return
		}
		alg, _ := msg.Headers.Protected.Algorithm()
		name := nameOf(alg)
		if !isEcdsa(name) {
			return
		}
		f := form[name]
		if isHighS(name, msg.Signature) {
			f[0]++
		} else {
			f[1]++
		}
		form[name] = f
	}
	for _, c := range cases {
		tally(c.Message)
		for _, e := range c.Envelopes {
			tally(e.Message)
		}
	}
	for _, a := range algorithms {
		if isEcdsa(a.name) && (form[a.name][0] == 0 || form[a.name][1] == 0) {
			problems = append(problems, fmt.Sprintf("%s: the file lacks a high-S or a low-S signature: %v", a.name, form[a.name]))
		}
	}

	body, err := json.Marshal(obj{
		"library": obj{"name": "github.com/veraison/go-cose", "version": libraryVersion()},
		"keys":    keys.publishedJWKs(),
		"cases":   cases,
	})
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(out), 0o755); err != nil {
		return err
	}
	if err := os.WriteFile(out, body, 0o644); err != nil {
		return err
	}
	vcs, vps := 0, 0
	for _, c := range cases {
		if c.Kind == "vc" {
			vcs++
		} else {
			vps++
		}
	}
	fmt.Printf("issued %d cases (%d credentials, %d presentations) to %s\n", len(cases), vcs, vps, out)
	for _, a := range algorithms {
		if isEcdsa(a.name) {
			fmt.Printf("%s: %d high-S, %d low-S signatures\n", a.name, form[a.name][0], form[a.name][1])
		}
	}
	if len(problems) > 0 {
		fmt.Printf("\n%d problem(s):\n", len(problems))
		for _, p := range problems {
			fmt.Println("  " + p)
		}
		return fmt.Errorf("the library could not read what it signed")
	}
	return nil
}

func indexOf(name string) int {
	for i, a := range algorithms {
		if a.name == name {
			return i
		}
	}
	return -1
}
