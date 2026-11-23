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
  `vc+jwt` or `vp+jwt`, as an SD-JWT, `vc+sd-jwt` or `vp+sd-jwt`, or as a
  `COSE_Sign1`, `vc+cose` or `vp+cose`, with a key you supply. See `jose`.

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
revoked, so a signature it made still verifies. A revoked key is listed in neither
document, the current one included, and signs nothing.

## Any credential and presentation, as VC-JOSE-COSE

```rust
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::ClaimPath;
use dpp_vc::jose::{
    KeyBindingRequirement, VerifyOptions, sign_credential, sign_credential_sd_jwt,
    sign_presentation, verify_credential, verify_credential_sd_jwt, verify_presentation,
};
use serde_json::json;

# fn demo(issuer_key: &KeyEntry, holder_key: &KeyEntry, now: i64) -> Result<(), Box<dyn std::error::Error>> {
let issuer_kid = "https://issuer.example/keys#1";
let credential = json!({
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential"],
    "issuer": "https://issuer.example",
    "credentialSubject": {"id": "did:example:subject", "name": "Example", "role": "manufacturer"},
    "productPassport": {"id": "urn:example:passport:1"},
});

// `productPassport` is not a VC term, so the signer is told to expect it.
let token = sign_credential(&credential, issuer_key, issuer_kid, &["productPassport"])?;
let issuer_public = issuer_key.public_key();
// The options carry the time and the audience this verifier identifies as.
let options = VerifyOptions::new().at(now).for_audience("https://verifier.example");
let verified = verify_credential(&token, &issuer_public, options)?;
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
let presented = verify_presentation(&vp, &holder_key.public_key(), options)?;
assert_eq!(presented.envelopes().len(), 1);

// Verify each credential under the key of its issuer's `kid`. `None` refuses the set.
let credentials = presented.verify_credentials(options, &KeyBindingRequirement::NotRequired, |envelope| {
    (envelope.kid.as_deref() == Some(issuer_kid)).then(|| issuer_public.clone())
})?;
assert_eq!(credentials.len(), 1);

// The same credential as an SD-JWT, with `name` concealed. `role` stays in the clear,
// because a subject that is only its `id` is refused. The holder chooses what to
// disclose, and the verifier reads what it is given.
let name = ClaimPath::claim("credentialSubject").key("name");
// Key Binding is the verifier's decision, and there is no default: here it is not required.
// Give a holder key instead of `None` to write it as `cnf`, and pass
// `KeyBindingRequirement::Required(policy)` to require the holder's proof of it.
let not_bound = KeyBindingRequirement::NotRequired;
let sd_jwt = sign_credential_sd_jwt(&credential, issuer_key, issuer_kid, &["productPassport"], &[name], None)?;
let disclosed = verify_credential_sd_jwt(&sd_jwt.serialise(), &issuer_public, options, &not_bound)?;
assert_eq!(disclosed["credentialSubject"]["name"], "Example");
let withheld = sd_jwt.present(&[])?;
let hidden = verify_credential_sd_jwt(&withheld.serialise(), &issuer_public, options, &not_bound)?;
assert!(hidden["credentialSubject"].get("name").is_none());
# Ok(())
# }
#
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#     let key = KeyEntry::from_private_jwk(&json!({
#         "kty": "OKP",
#         "crv": "Ed25519",
#         "d": "nWGxne_9WmC6hEr0kuwsxERJxWl7MmkZcDusAxyuf2A",
#         "x": "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo"
#     }))?;
#     demo(&key, &key, 1_800_000_000)
# }
```

`jose` signs and verifies a credential or a presentation of any shape, given as
JSON, as a compact JWS: `vc+jwt` for a credential and `vp+jwt` for a presentation
(W3C Securing Verifiable Credentials using JOSE and COSE, Recommendation of 13 October
2026, §3.1), as an SD-JWT, `vc+sd-jwt` and `vp+sd-jwt` (§3.2), where you name the
claims to conceal by path, or as a `COSE_Sign1`, `vc+cose` and `vp+cose` (§3.3). It
resolves no key and performs no I/O. You hand it the signing key when
you sign, the holder's public key to verify a presentation, and each issuer's public
key to verify that issuer's credential, under EdDSA, ES256, ES384 or ES512. This is a
different path from the access credential above, which is one typed document verified
through a `did:web` document. No conformance is claimed.

Four rules are the caller's to know:

- **Verifying a presentation and verifying its credentials are two operations.**
  VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as validating
  its own JWT, §5.3 defines it for COSE as the `COSE_Sign1` process on the presentation
  itself, and §5 assumes the key is found before verification begins.
  `verify_presentation`, `verify_presentation_sd_jwt` and `verify_presentation_cose`
  therefore check the presentation's token and the shape of each envelope (§3.1.2,
  §3.2.2, §3.3.2), and return the envelopes unverified. Verify the credentials
  with `verify_credentials`, which needs a key for every envelope and fails on the
  first one that fails, or with `UnverifiedEnvelope::verify`, one at a time. A
  `vc+jwt`, a `vc+sd-jwt` and a `vc+cose` can be verified: an envelope of any other
  format makes `verify_credentials` fail.
- **Time is the `now` you pass.** `VerifyOptions::at(seconds)` holds `exp` and `nbf` to
  it, with no leeway, and refuses one that is not a number. Without it neither is
  read. `iat` is never checked when verifying, because the specification says it
  describes the signature, and a credential's own `validFrom` and `validUntil` are
  yours to check. Signing refuses an `iat` that is not a number (RFC 7519 §4.1.6).
- **The audience is the one you identify as.** RFC 7519 §4.1.3 says a JWT whose `aud`
  does not name the principal processing it "MUST be rejected". `for_audience` gives
  the value you identify with. A token whose `aud`, a string or an array of strings,
  does not name it is refused, so is one that carries `aud` when you gave none, and so
  is an `aud` of any other shape. It applies to the credentials in a presentation's
  envelopes as well, through the options you pass to `verify_credentials`.
- **Undeclared top-level members are refused at signing.** A member must be a term
  of the VC Data Model 2.0 vocabulary, a JWT registered claim name (`iss`, `sub`,
  `aud`, `exp`, `nbf`, `iat`, `jti`), or named in the `extension_terms` you pass.
  `vc` and `vp` are refused even then. Signing also refuses what verification would
  refuse: an `iss` that is not the issuer, and an `exp` or `nbf` that is not a number.
  Verification ignores members it does not know.

Five more are the SD-JWT form's:

- **A signer refuses to conceal what a verifier needs.** A path that starts at
  `@context`, `type`, `credentialStatus`, `credentialSchema` or `relatedResource`
  (§3.2.1: "SHOULD be disclosed"), or at `iss`, `aud`, `exp`, `nbf` or `cnf` (RFC 9901
  clause 9.7), is refused. A path that selects nothing is refused too.
- **The checks read what was disclosed.** Verification applies the Disclosures first and
  then holds the result to the rules of the `vc+jwt` form. A required claim that was
  concealed and not disclosed is absent, and refused for it.
- **Whether Key Binding is required is your decision, and there is no default.** RFC 9901
  §9.5 says verifiers "MUST decide whether Key Binding is required for a particular use
  case before verifying a credential", so every SD-JWT verifier, and the envelope path,
  takes a `KeyBindingRequirement`. `NotRequired` ignores a Key Binding JWT the token
  carries, as §7.3 steps 3 and 4 say. `Required(policy)` checks it as step 5 says, under
  the key in the token's `cnf`, with the audience, nonce and window of the policy, and
  refuses a token that has no Key Binding JWT, and a `vc+jwt` or `vc+cose` envelope,
  which cannot carry one. The signers take an optional holder key and write it as `cnf`.
- **A claim that was disclosed is checked as one in the clear.** A disclosed `exp` in the
  past is refused as expired when your `VerifyOptions` carry a `now`, as one in the clear
  is, and a disclosed `aud` entry is part of the `aud` your audience is compared with. RFC 9901 clause 9.7 puts the duty not to conceal them on the
  issuer, and the signer here refuses to.
- **A member named `_sd`, `_sd_alg` or `...` cannot be signed,** because a reader would
  take it for the mechanism.

Four more are the COSE form's:

- **The payload is the document, not a claims set.** §3.3.1 and §3.3.2 say the
  unsecured credential or presentation "is the unencoded COSE_Sign1 payload". So the
  COSE signers allow only the vocabulary and the `extension_terms` you pass: a JWT claim
  name such as `exp` or `iss` is an undeclared member like any other, and `vc` and `vp`
  are refused only as undeclared members, since §1.1.2.1 is stated for a JWT Claims Set.
- **Time, audience and issuer come from the CWT claims.** `exp`, `nbf`, `aud` and `iss`
  are read from the CWT Claims header parameter of the protected header (§3.3.3, RFC
  9597) and checked as the same claims are in the `vc+jwt` form. The signers write none,
  and the parameter is refused in the unprotected header, where nothing protects it.
- **`typ` is required.** It must name `vc+cose` or `vp+cose`, and the content type, when
  present, `vc` or `vp`, each with or without `application/`, though §3.3.1 and §3.3.2
  say SHOULD for both.
- **An envelope's base64 is read forgivingly.** A `vc+cose` envelope is
  `data:application/vc+cose;base64,<base64>` (§3.3.1), and its base64 is read with the
  WHATWG Infra Standard's "forgiving-base64 decode", so padding is optional and ASCII
  whitespace is removed: the W3C suite's own envelope has no padding.

## Relationship to other crates

| Crate | Role |
|---|---|
| `dpp-domain` | Provides `Audience`, `Disclosure`, `PassportId` and `IdentityPort`; required |
| `dpp-crypto` | Provides JWS signing and verification and the keystore; required |

The HTTP surface that issues and verifies these credentials is not part of this
workspace; it belongs to your application. Everything below that
boundary (credential shapes, signing, status lists) is here and works without one.

## Minimum Rust version

1.100, enforced in CI.

## Licence

Apache-2.0. See [LICENSE](../../LICENSE).
