# Identity Layer — `did:web`, JWS, and Key Management

This document covers identity across two crates: the cryptographic primitives in
`dpp-crypto` (key management, JWS signing) and the trust layer in `dpp-vc`
(`did:web` documents, Verifiable Credentials, status lists), plus the trust model
that binds a physical product to a verifiable digital passport.

The split is deliberate: signing bytes is a different job from deciding whose
signature means what. `dpp-vc` depends on `dpp-crypto`, never the reverse.

---

## 1. Why Identity Matters for DPPs

A DPP without a cryptographic identity is a claim, not a credential. W3C Decentralized Identifiers (DIDs) and Verifiable Credentials (VCs) solve three problems:

1. **Authenticity** — the data was issued by the named manufacturer
2. **Integrity** — the data has not been modified since issuance
3. **Binding** — the credential refers to the specific product being scanned

The trust root is **DNS + HTTPS** — the same infrastructure that secures the manufacturer's website. No blockchain required.

---

## 2. `did:web` Method

`did:web` is the simplest DID method suitable for organisations with their own web domain. The DID Document is served as a JSON file at a well-known HTTPS URL.

```
did:web:manufacturer.example.com
    -> GET https://manufacturer.example.com/.well-known/did.json

did:web:manufacturer.example.com:path:subpath
    -> GET https://manufacturer.example.com/path/subpath/did.json
```

**Trust model:** If you trust that the domain is controlled by the manufacturer (via DNS, HTTPS certificate, domain registration), then you trust the public keys in its DID Document.

---

## 3. DID Document Structure

Every issuer has one DID Document. `dpp_vc::did_builder` constructs it from the `dpp-crypto` KeyStore state.

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

- `verificationMethod`: the current key first, then every archived key that has not been revoked, so previously signed VCs remain verifiable. A **revoked** key is removed, so its signatures stop verifying.
- `assertionMethod`: every listed key. This is the relationship a signature is checked against.
- `authentication`: the current key only.
- Each method is a `JsonWebKey` (W3C Controlled Identifiers v1.0) whose JWK carries its own `kid` and the `alg` the key signs under.

---

## 4. Key Management (dpp-crypto)

### KeyStore

AES-256-GCM encrypted Ed25519 key storage. Keys are persisted as JSON files on the local filesystem. The path is injected, making it testable with temp directories.

```rust
let store = KeyStore::open(&path, passphrase)?;
store.generate_key(&key_id)?;             // new Ed25519 keypair
let key = store.load_key(&key_id)?;       // load existing key
```

### Key IDs

A key is identified by its RFC 7638 SHA-256 thumbprint, written as an RFC 9278 URI: `urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}`. The verification method's id is the DID, `#`, and that URI. The identifier is derived from the key alone, so it never changes: not when a newer key is generated, and not when another key is revoked and drops out of the document. A position in a list could not promise that, and a signed token's `kid` is fixed for as long as the token exists.

### Key Rotation

Key rotation does not invalidate existing signatures:

1. Current key is archived with a timestamp
2. New Ed25519 keypair generated, and listed first as the current key
3. Archived keys keep their identifiers and stay under `assertionMethod`
4. All future VCs are signed with the new key
5. A token names the key that signed it in its `kid`, so verifiers use that key, not the "current" one

---

## 5. JWS Signing (dpp-crypto)

### Signing

`dpp_crypto::jws::signer::sign()` produces a JWS compact serialisation (EdDSA with Ed25519):

1. Serialize the payload as RFC 8785 (JCS) canonical JSON
2. Build the JWS Protected Header: `{"alg": "EdDSA", "kid": "{kid}"}`, plus `typ` where the token type has one. For anything verified through the DID document, the `kid` is the absolute DID URL of the signing key's verification method, `{did}#urn:ietf:params:oauth:jwk-thumbprint:sha-256:{thumbprint}`. An SD-JWT VC carries the thumbprint URI alone, which is what its issuer metadata's key set names
3. Signing input: `base64url(header) || "." || base64url(payload)`
4. Sign with Ed25519
5. Compact serialisation: `{header_b64}.{payload_b64}.{signature_b64}`

`EdDSA` is the identifier RFC 9864 deprecates in favour of `Ed25519`. It is still the one written, because the European Commission's DSS validator maps only `EdDSA` for JOSE and the W3C VC-JOSE-COSE test suite signs with it. Verification accepts both.

### Verification

`dpp_crypto::jws::verifier` provides the single source of truth for JWS verification:

```rust
let key = resolve_verification_key(&did_document, jws_compact)?;
verify_jws(jws_compact, &key)?;
```

The verifier:
1. Fetches the issuer's DID Document (via the `did:web` resolution rule)
2. Resolves the verification method the JWS `kid` names, with the binding checks of Controlled Identifiers v1.0 §3.3: the document is the one the `kid` names, the method is that document's, and `assertionMethod` references it. A `kid` that is a thumbprint URI must be the key's own, and a JWK that declares an `alg` must match the header. A token without a `kid` is refused
3. Reconstructs the signing input and verifies the Ed25519 signature

---

## 6. QR Code Trust Anchor

The carrier (QR or Data Matrix) on a physical product encodes a **GS1 Digital Link** URI, not a proprietary path:

```
{resolver_base}/01/{gtin}                        model level
{resolver_base}/01/{gtin}/10/{batch}             batch level
{resolver_base}/01/{gtin}/21/{carrier serial}    item level, or no level stated
```

- `resolver_base` is per-deployment configuration (`RESOLVER_BASE_URL`). A self-hoster sets it to their **own domain**, so the printed label carries the same trust root as their `did:web` identity; Odal's managed default is `https://id.odal-node.io`.
- The GTIN and identifier come from the **verified** passport fields — the resolver checks the JWS before building the URI and never trusts a stored `qrCodeUrl` value.
- What follows the GTIN is `Passport::carrier_qualifier`, chosen by the passport's `granularity`. A GTIN with AI 21 is a serialised GTIN, which GS1 defines as identifying **one individual** item, so a model- or batch-level carrier — printed on every unit it covers — carries no serial. Regulation (EU) 2024/1781 Art. 10(1)(f) draws the same line: the data *"shall refer to the product model, batch or item"*. A passport that states no level prints a serial, as every carrier did before the level was read. A label resolves back to its passport through `PassportRepository::find_by_carrier`, which compares the same value; a serial label resolves at any level, so labels printed before a passport stated its level keep working.
- The `/21/` segment is the passport's **carrier serial**, `Passport::effective_carrier_serial`: the serial the operator attributes — the act Art. 77(3) of Regulation (EU) 2023/1542 names — or, when it attributes none, twenty hex characters from the random tail of the passport id. It is one to twenty GS1 CSET 82 characters either way.
- The `/10/` segment is printed only at batch level, where the lot is what the carrier identifies; `Passport::validate` then holds `batch_id` to GS1 AI 10's one to twenty CSET 82 characters. Below batch level the serial alone resolves the label, and the lot stays operator free text that no carrier prints.

The carrier **fails closed**: if the passport does not verify, no URI is produced; if the product group data has no GTIN (for example an unsold-goods report), resolution returns `422` rather than a misleading code. Because the carrier is standard GS1 Digital Link, any conformant resolver serving the same path answers the same scan — re-homing a passport is a DNS or registry change, not a reprint.

---

## 7. EBSI Upgrade Path

EBSI (`did:ebsi`) is the EU Commission's preferred DID infrastructure for regulated credentials. As of Q1 2026, EBSI has 29 EU member state pilots but zero production DPP deployments and no Rust library.

An issuer can be migrated to EBSI credentials without re-issuing existing passports:

1. Register the issuer on EBSI (creates a `did:ebsi` DID)
2. Add the EBSI DID to the `did:web` DID Document as a `sameAs` service endpoint
3. New passports issued with `did:ebsi` as issuer
4. Old passports retain `did:web` — they remain verifiable via the `did:web` path
5. Both DIDs are valid simultaneously during transition

This is a non-breaking migration. No passports are invalidated. No QR codes need reprinting.
