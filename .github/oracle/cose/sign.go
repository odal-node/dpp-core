package main

import (
	"crypto/rand"
	"fmt"

	"github.com/veraison/go-cose"
)

// Header is what a message is signed with, so a control can sign what this crate would not.
type Header struct {
	Typ    any    // nil leaves it out
	Cty    any    // nil leaves it out
	Kid    string // the key identifier, as text; empty leaves it out
	Claims cose.CWTClaims
	// Unprotected is written into the unprotected header, which this crate leaves empty.
	Unprotected map[any]any
}

// signMessage signs payload under key with the library, and returns the tagged COSE_Sign1.
// For ECDSA, wantHigh picks the form of S: the signing is repeated until it has it, since
// the library draws a fresh nonce for every signature and does not normalise.
func signMessage(key *Key, h Header, payload []byte, wantHigh *bool) ([]byte, error) {
	signer, err := cose.NewSigner(key.Alg, key.Priv)
	if err != nil {
		return nil, err
	}
	for attempt := 0; attempt < 400; attempt++ {
		msg := cose.NewSign1Message()
		msg.Payload = payload
		if h.Typ != nil {
			msg.Headers.Protected[cose.HeaderLabelType] = h.Typ
		}
		if h.Cty != nil {
			msg.Headers.Protected[cose.HeaderLabelContentType] = h.Cty
		}
		if h.Kid != "" {
			msg.Headers.Protected[cose.HeaderLabelKeyID] = []byte(h.Kid)
		}
		for label, value := range h.Unprotected {
			msg.Headers.Unprotected[label] = value
		}
		if h.Claims != nil {
			msg.Headers.Protected[cose.HeaderLabelCWTClaims] = h.Claims
		}
		if err := msg.Sign(rand.Reader, nil, signer); err != nil {
			return nil, err
		}
		if wantHigh != nil && isEcdsa(key.Name) && isHighS(key.Name, msg.Signature) != *wantHigh {
			continue
		}
		return msg.MarshalCBOR()
	}
	return nil, fmt.Errorf("no %v signature of the wanted form in 400 attempts", wantHigh)
}

// signDocument signs a credential or a presentation as this crate does: tagged, with the
// media types of VC-JOSE-COSE 3.3.1 and 3.3.2, the kid as text bytes, and the document as
// UTF-8 JSON.
func signDocument(key *Key, kind, kid string, document any, wantHigh *bool, claims cose.CWTClaims) ([]byte, error) {
	payload, err := marshalDocument(document)
	if err != nil {
		return nil, err
	}
	return signMessage(key, Header{Typ: typOf[kind], Cty: ctyOf[kind], Kid: kid, Claims: claims}, payload, wantHigh)
}
