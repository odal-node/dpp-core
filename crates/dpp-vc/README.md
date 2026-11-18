# dpp-vc

[![crates.io](https://img.shields.io/crates/v/dpp-vc.svg)](https://crates.io/crates/dpp-vc)
[![docs.rs](https://img.shields.io/docsrs/dpp-vc)](https://docs.rs/dpp-vc)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](../../LICENSE)

W3C Verifiable Credentials, `did:web` documents, Bitstring Status List revocation, and
the JSON-LD context they are expressed in, for EU Digital Product Passports. Part of
[dpp-core](https://github.com/odal-node/dpp-core) by [Odal Node](https://odal-node.io).

Pure Rust, no I/O, no network calls. It does not build for `wasm32-unknown-unknown`,
because it depends on `dpp-crypto`, whose RNG needs a platform entropy source.

## When to use this crate

- You issue or verify a credential that proves a supply-chain actor's role: an
  authorised repairer, a recycler, a market-surveillance authority.
- You publish or resolve a `did:web` document for a manufacturer or operator.
- You check whether a credential has been revoked against a Bitstring Status List.
- You build the JSON-LD envelope for a passport or a credential.
- You read a *copy* of a passport's public view and need to know whether its time
  bound still holds. See `snapshot`.
- You issue a passport as an **SD-JWT VC**, so a holder can present part of it to a
  third party without the issuing node being reachable. See `sd_jwt_vc`. Keys there are
  published through JWT VC Issuer Metadata rather than `did:web`, because the credential
  profile defines no DID mechanism.

  This crate does not *resolve* those keys, and nothing here does I/O.
  `build_issuer_metadata` produces the document an issuer publishes, and `verify` takes
  the key the caller has already chosen. Fetching the metadata, deciding whether to
  trust it, and picking the key are your application's job, and so is the network policy.
- You sign or verify **any** W3C credential or presentation as a VC-JOSE-COSE
  `vc+jwt` or `vp+jwt`, with a key you supply. See `jose`.

## What this crate does not cover

| Concern | Where it lives | Why not here |
|---|---|---|
| Cryptographic primitives: JWS, keys, keystore | `dpp-crypto` | Signing bytes is a different job from deciding whose signature means what |
| The disclosure contract: `Audience`, `Disclosure`, the per-field map and its filter | `dpp-domain` | Which fields a role may see describes what a passport *is*, not who is asking |
| Projections: GS1, AAS | `dpp-digital-link`, `dpp-aas` | Rendering a passport for an ecosystem says nothing about trust |

The boundary: **a credential establishes which `Audience` a caller holds; the
disclosure policy maps that `Audience` to fields.** Two questions, two places.

## Example

```rust
use dpp_vc::{CredentialBuilder, CredentialRole, DppCredentialSubject};

let subject = DppCredentialSubject {
    id: "did:web:repairer.example.com".into(),
    name: "Example Repair GmbH".into(),
    role: CredentialRole::AuthorisedRepairer,
    country: "DE".into(),
    product_groups: vec!["battery".into()],
    product_categories: vec![],
};

let credential = CredentialBuilder::new(
    "did:web:issuer.example.com".into(),
    subject,
)
.expires_in_days(365)
.build();
```

A runnable version covering issuance and transfer is in
[`examples/credential_and_transfer.rs`](examples/credential_and_transfer.rs).

## Issuer keys: Ed25519 and the EC curves

An issuer's signing key can be Ed25519, signing JWS with `EdDSA`, or an EC key on
P-256, P-384 or P-521, signing with `ES256`, `ES384` or `ES512`. A key store makes the
key you ask for (`KeyStore::generate_key_with`), and `rotate_key_to` moves an issuer
from one algorithm to another. The archived key stays published under its own shape,
so a verifier can check the signature of a token made before the rotation for as long
as that key is published and not revoked. Keeping the key published does not keep a
credential valid past its expiry or its revocation: those are separate checks.

Each key is published with the JWK shape of its algorithm:

| Key | `kty` | `crv` | `alg` | Length of `x` and `y` |
|---|---|---|---|---|
| Ed25519 | `OKP` | `Ed25519` | `EdDSA` | `x` only, 32 octets |
| P-256 | `EC` | `P-256` | `ES256` | 32 octets each |
| P-384 | `EC` | `P-384` | `ES384` | 48 octets each |
| P-521 | `EC` | `P-521` | `ES512` | 66 octets each |

In a `did:web` document, a P-256 issuer key is one verification method. The `x`, `y`
and thumbprint values below are placeholders:

```json
{
  "id": "did:web:issuer.example.com#urn:ietf:params:oauth:jwk-thumbprint:sha-256:<thumbprint>",
  "type": "JsonWebKey",
  "controller": "did:web:issuer.example.com",
  "publicKeyJwk": {
    "kty": "EC",
    "crv": "P-256",
    "x": "<43 base64url characters>",
    "y": "<43 base64url characters>",
    "kid": "urn:ietf:params:oauth:jwk-thumbprint:sha-256:<thumbprint>",
    "alg": "ES256"
  }
}
```

In the JWT VC issuer metadata of an SD-JWT VC, the same key is one entry of
`jwks.keys`, with `use` set to `sig`:

```json
{
  "issuer": "https://passports.example.com",
  "jwks": {
    "keys": [
      {
        "kty": "EC",
        "crv": "P-256",
        "x": "<43 base64url characters>",
        "y": "<43 base64url characters>",
        "kid": "urn:ietf:params:oauth:jwk-thumbprint:sha-256:<thumbprint>",
        "alg": "ES256",
        "use": "sig"
      }
    ]
  }
}
```

A token names its key in the protected header's `kid`. In a `did:web` document that
is the verification method's absolute DID URL, and in the SD-JWT VC metadata it is the
thumbprint URI alone. A key that was rotated out is still listed while it is not
revoked, so a signature it made still verifies. A revoked archived key is listed in
neither document.

## Any credential and presentation, as VC-JOSE-COSE

```rust
use dpp_crypto::keystore::KeyEntry;
use dpp_vc::jose::{sign_credential, sign_presentation, verify_credential, verify_presentation};
use serde_json::json;

# fn demo(issuer_key: &KeyEntry, holder_key: &KeyEntry, now: i64) -> Result<(), Box<dyn std::error::Error>> {
let issuer_kid = "https://issuer.example/keys#1";
let credential = json!({
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential"],
    "issuer": "https://issuer.example",
    "credentialSubject": {"id": "did:example:subject"},
    "productPassport": {"id": "urn:example:passport:1"},
});

// `productPassport` is not a VC term, so the signer is told to expect it.
let token = sign_credential(&credential, issuer_key, issuer_kid, &["productPassport"])?;
let issuer_public = issuer_key.public_key();
let verified = verify_credential(&token, &issuer_public, Some(now))?;
assert_eq!(verified["issuer"], "https://issuer.example");

let presentation = json!({
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiablePresentation"],
    "verifiableCredential": [{
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": format!("data:application/vc+jwt,{token}"),
    }],
});
let vp = sign_presentation(&presentation, holder_key, "https://holder.example/keys#1", &[])?;

// Verify the presentation's own token. Its credentials are listed, not verified.
let presented = verify_presentation(&vp, &holder_key.public_key(), Some(now))?;
assert_eq!(presented.envelopes.len(), 1);

// Verify each credential under the key of its issuer's `kid`. `None` refuses the set.
let credentials = presented.verify_credentials(Some(now), |envelope| {
    (envelope.kid.as_deref() == Some(issuer_kid)).then(|| issuer_public.clone())
})?;
assert_eq!(credentials.len(), 1);
# Ok(())
# }
```

`jose` signs and verifies a credential or a presentation of any shape, given as
JSON, as a compact JWS: `vc+jwt` for a credential and `vp+jwt` for a presentation
(W3C Securing Verifiable Credentials using JOSE and COSE, Recommendation of 15 May
2025, §3.1). It resolves no key and performs no I/O. You hand it the signing key when
you sign, the holder's public key to verify a presentation, and each issuer's public
key to verify that issuer's credential, under EdDSA, ES256, ES384 or ES512. This is a
different path from the access credential above, which is one typed document verified
through a `did:web` document. No conformance is claimed.

Three rules are the caller's to know:

- **Verifying a presentation and verifying its credentials are two operations.**
  VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as validating
  its own JWT, and §5 assumes the key is found before verification begins.
  `verify_presentation` therefore checks the presentation's token and the shape of
  each envelope (§3.1.2), and returns the envelopes unverified. Verify the credentials
  with `verify_credentials`, which needs a key for every envelope and fails on the
  first one that fails, or with `UnverifiedEnvelope::verify`, one at a time. Only a
  `vc+jwt` can be verified: a `vc+sd-jwt` or a `vc+cose` envelope makes
  `verify_credentials` fail.
- **Time is the `now` you pass.** `Some(seconds)` holds `exp` and `nbf` to it, with
  no leeway, and refuses one that is not a number. `None` reads neither. `iat` is
  never checked, because the specification says it describes the signature, and a
  credential's own `validFrom` and `validUntil` are yours to check.
- **Undeclared top-level members are refused at signing.** A member must be a term
  of the VC Data Model 2.0 vocabulary, a JWT registered claim name (`iss`, `sub`,
  `aud`, `exp`, `nbf`, `iat`, `jti`), or named in the `extension_terms` you pass.
  `vc` and `vp` are refused even then. Signing also refuses what verification would
  refuse: an `iss` that is not the issuer, and an `exp` or `nbf` that is not a number.
  Verification ignores members it does not know.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Audience`, `Disclosure`, `PassportId` and `IdentityPort`; required |
| `dpp-crypto` | Provides JWS signing and verification and the keystore; required |

The HTTP surface that issues and verifies these credentials is not part of this
workspace; it belongs to your application. Everything below that
boundary (credential shapes, signing, status lists) is here and works without one.

## Minimum Rust version

1.99, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
