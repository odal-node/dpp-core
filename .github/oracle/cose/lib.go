// The independent COSE implementation, the suite's keys, and what the order of each curve
// says about an ECDSA S.
//
// The library is github.com/veraison/go-cose, with its CBOR codec github.com/fxamacker/cbor.
// It is Go, and shares no code and no author with the RustCrypto stack and the hand-written
// COSE_Sign1 layer of `dpp_crypto::cose`. Say that plainly wherever this oracle is cited.
package main

import (
	"bytes"
	"crypto"
	"crypto/ecdsa"
	"crypto/ed25519"
	"crypto/elliptic"
	_ "crypto/sha256" // go-cose hashes with whatever is linked in, and says "hash function is not available" otherwise
	_ "crypto/sha512"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"math/big"
	"os"
	"path/filepath"

	"github.com/veraison/go-cose"
)

// suiteDir is where the suite's key files are vendored, relative to the repository root. The
// binary is run from `.github/oracle/cose`, so the default is relative to that.
const defaultSuiteDir = "../../../crates/dpp-vc/src/jose/fixtures/w3c-vc-jose-cose"

// Each algorithm this crate signs under, with the suite file of its key.
var algorithms = []struct {
	name string
	file string
}{
	{"EdDSA", "vm-ed25519.json"},
	{"ES256", "vm-p256.json"},
	{"ES384", "vm-p384.json"},
	{"ES512", "vm-p521.json"},
}

// The key that signs row 27 of the suite: an Ed25519 key whose id is a DID URL.
const didFile = "vm-did-ed25519.json"

// Key is one of the suite's keys.
type Key struct {
	ID         string
	Thumbprint string
	Name       string // EdDSA, ES256, ES384 or ES512
	Alg        cose.Algorithm
	JWK        map[string]any // the public JWK, as published
	Pub        crypto.PublicKey
	Priv       crypto.Signer
}

// jwk is the part of a JWK the oracle reads.
type jwk struct {
	Kty string `json:"kty"`
	Crv string `json:"crv"`
	X   string `json:"x"`
	Y   string `json:"y"`
	D   string `json:"d"`
	Kid string `json:"kid"`
}

func b64u(text string) ([]byte, error) {
	return base64.RawURLEncoding.DecodeString(text)
}

// algorithmOf is the algorithm a JWK is for, from its kty and crv alone.
func algorithmOf(j jwk) (string, cose.Algorithm, bool) {
	switch {
	case j.Kty == "OKP" && j.Crv == "Ed25519":
		return "EdDSA", cose.AlgorithmEdDSA, true
	case j.Kty == "EC" && j.Crv == "P-256":
		return "ES256", cose.AlgorithmES256, true
	case j.Kty == "EC" && j.Crv == "P-384":
		return "ES384", cose.AlgorithmES384, true
	case j.Kty == "EC" && j.Crv == "P-521":
		return "ES512", cose.AlgorithmES512, true
	}
	return "", 0, false
}

func curveOf(crv string) elliptic.Curve {
	switch crv {
	case "P-256":
		return elliptic.P256()
	case "P-384":
		return elliptic.P384()
	default:
		return elliptic.P521()
	}
}

// publicKeyOf builds the public key of a JWK.
func publicKeyOf(j jwk) (crypto.PublicKey, error) {
	x, err := b64u(j.X)
	if err != nil {
		return nil, err
	}
	if j.Kty == "OKP" {
		if len(x) != ed25519.PublicKeySize {
			return nil, fmt.Errorf("an Ed25519 key of %d bytes", len(x))
		}
		return ed25519.PublicKey(x), nil
	}
	y, err := b64u(j.Y)
	if err != nil {
		return nil, err
	}
	return &ecdsa.PublicKey{Curve: curveOf(j.Crv), X: new(big.Int).SetBytes(x), Y: new(big.Int).SetBytes(y)}, nil
}

// privateKeyOf builds the signing key of a secret JWK.
func privateKeyOf(j jwk) (crypto.Signer, error) {
	d, err := b64u(j.D)
	if err != nil {
		return nil, err
	}
	if j.Kty == "OKP" {
		return ed25519.NewKeyFromSeed(d), nil
	}
	pub, err := publicKeyOf(j)
	if err != nil {
		return nil, err
	}
	return &ecdsa.PrivateKey{PublicKey: *pub.(*ecdsa.PublicKey), D: new(big.Int).SetBytes(d)}, nil
}

// loadKey reads a suite key file.
func loadKey(dir, file string) (*Key, error) {
	raw, err := os.ReadFile(filepath.Join(dir, file))
	if err != nil {
		return nil, err
	}
	var body struct {
		ID           string         `json:"id"`
		PublicKeyJwk map[string]any `json:"publicKeyJwk"`
		SecretKeyJwk jwk            `json:"secretKeyJwk"`
	}
	if err := json.Unmarshal(raw, &body); err != nil {
		return nil, fmt.Errorf("%s: %w", file, err)
	}
	pubRaw, _ := json.Marshal(body.PublicKeyJwk)
	var pubJwk jwk
	if err := json.Unmarshal(pubRaw, &pubJwk); err != nil {
		return nil, err
	}
	name, alg, ok := algorithmOf(pubJwk)
	if !ok {
		return nil, fmt.Errorf("%s: not a key this oracle knows", file)
	}
	pub, err := publicKeyOf(pubJwk)
	if err != nil {
		return nil, err
	}
	priv, err := privateKeyOf(body.SecretKeyJwk)
	if err != nil {
		return nil, err
	}
	return &Key{ID: body.ID, Thumbprint: pubJwk.Kid, Name: name, Alg: alg, JWK: body.PublicKeyJwk, Pub: pub, Priv: priv}, nil
}

// Keys are the suite's keys: by algorithm name, and the kid map a verifier resolves a
// message's kid in. A message of the suite's names a key by the thumbprint in its JWK's
// kid, and a message of this crate's by the id of its verification method, so each key is
// listed under both.
type Keys struct {
	ByName    map[string]*Key
	Did       *Key
	Published map[string]*Key
}

func loadKeys(dir string) (*Keys, error) {
	keys := &Keys{ByName: map[string]*Key{}, Published: map[string]*Key{}}
	for _, a := range algorithms {
		k, err := loadKey(dir, a.file)
		if err != nil {
			return nil, err
		}
		if k.Name != a.name {
			return nil, fmt.Errorf("%s is a %s key, not %s", a.file, k.Name, a.name)
		}
		keys.ByName[a.name] = k
		keys.Published[k.ID] = k
		keys.Published[k.Thumbprint] = k
	}
	did, err := loadKey(dir, didFile)
	if err != nil {
		return nil, err
	}
	keys.Did = did
	keys.Published[did.ID] = did
	return keys, nil
}

// publishedJWKs is the kid map as the public JWKs, for the file the Rust side reads back.
func (k *Keys) publishedJWKs() map[string]any {
	out := map[string]any{}
	for kid, key := range k.Published {
		out[kid] = key.JWK
	}
	return out
}

// order is the group order n of each curve, FIPS 186-5 appendix D.1.2 and SEC 2.
func orderOf(name string) *big.Int {
	switch name {
	case "ES256":
		return elliptic.P256().Params().N
	case "ES384":
		return elliptic.P384().Params().N
	case "ES512":
		return elliptic.P521().Params().N
	}
	return nil
}

func isEcdsa(name string) bool { return orderOf(name) != nil }

// isHighS reports whether the S of an R || S signature is above n / 2, the form this crate
// never writes.
func isHighS(name string, signature []byte) bool {
	n := orderOf(name)
	s := new(big.Int).SetBytes(signature[len(signature)/2:])
	return s.Cmp(new(big.Int).Rsh(n, 1)) > 0
}

// nameOf is the name of a COSE algorithm.
func nameOf(alg cose.Algorithm) string {
	switch alg {
	case cose.AlgorithmEdDSA:
		return "EdDSA"
	case cose.AlgorithmES256:
		return "ES256"
	case cose.AlgorithmES384:
		return "ES384"
	case cose.AlgorithmES512:
		return "ES512"
	}
	return alg.String()
}

// The media types of a credential and a presentation secured with COSE (VC-JOSE-COSE
// 3.3.1, 3.3.2), and the prefix of a COSE envelope's id.
var (
	typOf = map[string]string{"vc": "application/vc+cose", "vp": "application/vp+cose"}
	ctyOf = map[string]string{"vc": "application/vc", "vp": "application/vp"}
)

const vcCosePrefix = "data:application/vc+cose;base64,"

// same reports whether two decoded JSON values are equal. Key order is not meaning, and
// marshalling sorts it.
func same(a, b any) bool {
	x, _ := json.Marshal(a)
	y, _ := json.Marshal(b)
	return bytes.Equal(x, y)
}

// marshalDocument is the JSON text of a document, without Go's HTML escaping.
func marshalDocument(document any) ([]byte, error) {
	var out bytes.Buffer
	enc := json.NewEncoder(&out)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(document); err != nil {
		return nil, err
	}
	return bytes.TrimRight(out.Bytes(), "\n"), nil
}
