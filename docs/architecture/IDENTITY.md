# Identity: `did:web`, JWS and key management

Identity spans two crates: the cryptographic primitives in `dpp-crypto` (key
management, JWS signing) and the trust layer in `dpp-vc` (`did:web` documents,
Verifiable Credentials, status lists). This document also covers the trust model that
binds a physical product to its passport.

The split is deliberate: signing bytes is a different job from deciding whose signature
means what. `dpp-vc` depends on `dpp-crypto`, never the reverse.

---

## 1. What identity gives a passport

A passport without a cryptographic identity is an unverifiable claim. W3C Decentralized
Identifiers (DIDs) and Verifiable Credentials (VCs) provide three properties:

1. **Authenticity:** the data was issued by the named manufacturer.
2. **Integrity:** the data has not changed since it was issued.
3. **Binding:** the credential refers to the specific product being scanned.

The trust root is **DNS and HTTPS**, the same infrastructure that secures the
manufacturer's website. No blockchain is involved.

---

## 2. The `did:web` method

`did:web` is the simplest DID method for an organisation with its own web domain. The
DID document is a JSON file at a well-known HTTPS URL.

```text
did:web:manufacturer.example.com
    -> GET https://manufacturer.example.com/.well-known/did.json

did:web:manufacturer.example.com:path:subpath
    -> GET https://manufacturer.example.com/path/subpath/did.json
```

**Trust model:** if you trust that the manufacturer controls the domain (through DNS,
the HTTPS certificate and domain registration), you trust the public keys in its DID
document.

---

## 3. DID document structure

Every issuer has one DID document. `dpp_vc::did_builder` builds it from the
`dpp-crypto` KeyStore state.

```json
{
  "@context": ["https://www.w3.org/ns/did/v1", "https://www.w3.org/ns/cid/v1"],
  "id": "did:web:manufacturer.example.com",
  "verificationMethod": [
    {
      "id": "did:web:manufacturer.example.com#urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}",
      "type": "JsonWebKey",
      "controller": "did:web:manufacturer.example.com",
      "publicKeyJwk": {
        "kty": "OKP",
        "crv": "Ed25519",
        "x": "{base64url-public-key}",
        "kid": "urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}",
        "alg": "EdDSA"
      }
    }
  ],
  "assertionMethod": ["did:web:manufacturer.example.com#urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}"],
  "authentication": ["did:web:manufacturer.example.com#urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}"]
}
```

- `verificationMethod`: the current key first, then every archived key that has not been
  revoked, so earlier signatures remain verifiable. A **revoked** key is removed, so its
  signatures stop verifying.
- `assertionMethod`: every listed key. This is the relationship a signature is checked
  against.
- `authentication`: the current key only.
- Each method is a `JsonWebKey` (W3C Controlled Identifiers v1.0) whose JWK carries its
  own `kid` and the `alg` the key signs under.

---

## 4. Key management (`dpp-crypto`)

### KeyStore

Ed25519 key storage encrypted with AES-256-GCM, persisted as a JSON file on the local
filesystem. The path is injected, so tests use temporary directories.

```rust
let store = KeyStore::open(&path, passphrase)?;
store.generate_key(&key_id)?;             // new Ed25519 key pair
let key = store.load_key(&key_id)?;       // load an existing key
```

### Key identifiers

A key is identified by its RFC 7638 SHA-256 thumbprint, written as an RFC 9278 URI:
`urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}`. The verification method's
id is the DID, `#`, and that URI. The identifier is derived from the key alone, so it
never changes: not when a newer key is generated, and not when another key is revoked
and drops out of the document. A position in a list could not promise that, and a signed
token's `kid` is fixed for as long as the token exists.

### Key rotation

Rotation does not invalidate existing signatures:

1. The current key is archived with a timestamp.
2. A new Ed25519 key pair is generated and listed first as the current key.
3. Archived keys keep their identifiers and stay under `assertionMethod`.
4. Everything signed from now on uses the new key.
5. A token names the key that signed it in its `kid`, so verifiers use that key, not the
   current one.

---

## 5. JWS signing (`dpp-crypto`)

### Signing

`dpp_crypto::jws::signer::sign()` produces a compact JWS (EdDSA with Ed25519):

1. Serialise the payload as RFC 8785 (JCS) canonical JSON.
2. Build the protected header: `{"alg": "EdDSA", "kid": "{kid}"}`, plus `typ` where the
   token type defines one. For anything verified through the DID document, the `kid` is
   the absolute DID URL of the signing key's verification method,
   `{did}#urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}`. An SD-JWT VC
   carries the thumbprint URI alone, which is what its issuer metadata's key set names.
3. Signing input: `base64url(header) || "." || base64url(payload)`.
4. Sign with Ed25519.
5. Compact serialisation: `{header_b64}.{payload_b64}.{signature_b64}`.

`EdDSA` is the identifier RFC 9864 deprecates in favour of `Ed25519`. It is still the one
written, because the European Commission's DSS validator maps only `EdDSA` for JOSE and
the W3C VC-JOSE-COSE test suite signs with it. Verification accepts both.

### Verification

`dpp_crypto::jws::verifier` is the single place JWS verification lives:

```rust
let key = resolve_verification_key(&did_document, jws_compact)?;
verify_jws(jws_compact, &key)?;
```

The verifier:

1. Fetches the issuer's DID document (by the `did:web` resolution rule).
2. Resolves the verification method the JWS `kid` names, with the binding checks of
   Controlled Identifiers v1.0 §3.3: the document is the one the `kid` names, the method
   belongs to that document, and `assertionMethod` references it. A `kid` that is a
   thumbprint URI must be the key's own, and a JWK that declares an `alg` must match the
   header. A token without a `kid` is refused.
3. Rebuilds the signing input and verifies the Ed25519 signature.

---

## 6. The QR code as trust anchor

The carrier (QR or Data Matrix) on a physical product encodes a **GS1 Digital Link** URI,
not a proprietary path:

```text
{resolver_base}/01/{gtin}                        model level
{resolver_base}/01/{gtin}/10/{batch}             batch level
{resolver_base}/01/{gtin}/21/{carrier serial}    item level, or no level stated
```

- `resolver_base` is deployment configuration. An operator sets it to their **own
  domain**, so the printed label carries the same trust root as their `did:web`
  identity.
- The GTIN and identifier come from the **verified** passport fields. The URI is built
  only after the JWS checks out, never from a stored `qrCodeUrl` value.
- What follows the GTIN is `Passport::carrier_qualifier`, chosen by the passport's
  `granularity`. A GTIN with AI 21 is a serialised GTIN, which GS1 defines as identifying
  **one individual** item, so a model-level or batch-level carrier, printed on every unit
  it covers, carries no serial. Regulation (EU) 2024/1781 Art. 10(1)(f) draws the same
  line: the data *"shall refer to the product model, batch or item"*. A passport that
  states no level prints a serial, as every carrier did before the level was read. A label
  resolves back to its passport through `PassportRepository::find_by_carrier`, which
  compares the same value; a serial label resolves at any level, so labels printed before
  a passport stated its level keep working.
- The `/21/` segment is the passport's **carrier serial**,
  `Passport::effective_carrier_serial`: the serial the operator attributes (the act Art.
  77(3) of Regulation (EU) 2023/1542 names) or, when it attributes none, twenty hex
  characters from the random tail of the passport id. Either way it is one to twenty GS1
  CSET 82 characters.
- The `/10/` segment is printed only at batch level, where the lot is what the carrier
  identifies, and `Passport::validate` then holds `batch_id` to GS1 AI 10's one to twenty
  CSET 82 characters. Below batch level the serial alone resolves the label, and the lot
  stays operator free text that no carrier prints.

The carrier **fails closed**: if the passport does not verify, no URI is produced, and if
the product group data has no GTIN (an unsold-goods report, for example), resolution
returns `422` rather than a misleading code. Because the carrier is standard GS1 Digital
Link, any conformant resolver serving the same path answers the same scan, so moving a
passport is a DNS or registry change, not a reprint.

---

## 7. Moving to EBSI later

EBSI (`did:ebsi`) is the EU's own DID infrastructure for regulated credentials. This
library does not support it yet. An issuer can move to it later without re-issuing
existing passports:

1. Register the issuer on EBSI, which creates a `did:ebsi` DID.
2. Add the EBSI DID to the `did:web` document as a `sameAs` service endpoint.
3. Issue new passports with `did:ebsi` as the issuer.
4. Old passports keep `did:web` and remain verifiable through it.
5. Both DIDs are valid during the transition.

No passport is invalidated and no QR code needs reprinting.
