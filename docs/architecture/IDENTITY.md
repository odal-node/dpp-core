# Identity Layer — `did:web`, JWS, and Key Management

This document covers the cryptographic identity primitives in `dpp-crypto`: key management, JWS signing, DID document construction, and the trust model that binds a physical product to a verifiable digital passport.

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

Every issuer has one DID Document. `dpp-crypto::did_builder` constructs it from the KeyStore state.

```json
{
  "@context": ["https://www.w3.org/ns/did/v1", "https://w3id.org/security/suites/jws-2020/v1"],
  "id": "did:web:manufacturer.example.com",
  "verificationMethod": [
    {
      "id": "did:web:manufacturer.example.com#key-1",
      "type": "JsonWebKey2020",
      "controller": "did:web:manufacturer.example.com",
      "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": "{base64url-public-key}" }
    }
  ],
  "assertionMethod": ["did:web:manufacturer.example.com#key-1"],
  "authentication": ["did:web:manufacturer.example.com#key-1"]
}
```

- `verificationMethod`: All public keys, including archived ones from rotation. Old keys are **never removed** — they are retained so previously signed VCs remain verifiable.
- `assertionMethod`: Only the current active key (permitted to sign new VCs).
- `authentication`: Keys permitted for authentication flows.

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

Key IDs follow the pattern `{did}#key-{n}` where `n` increments on each rotation:
- `#key-1` — initial key (primary, in `authentication` + `assertionMethod`)
- `#key-2` — after first rotation (`key-1` retained in `verificationMethod` only)

### Key Rotation

Key rotation does not invalidate existing signatures:

1. Current key is archived with a timestamp
2. New Ed25519 keypair generated, becomes `#key-1` (primary)
3. Archived keys retained as `#key-2`, `#key-3`, etc. under `assertionMethod`
4. All future VCs are signed with the new key
5. All existing VCs reference the old key in their `proof.verificationMethod` — verifiers use the specific key referenced in the proof, not the "current" key

---

## 5. JWS Signing (dpp-crypto)

### Signing

`dpp-crypto::signer::sign()` produces a JWS compact serialisation (EdDSA with Ed25519):

1. Serialize the VC payload deterministically (sorted keys, no extra whitespace)
2. Build the JWS Protected Header: `{"alg": "EdDSA", "b64": false, "crit": ["b64"], "kid": "{did}#key-1"}`
3. Signing input: `base64url(header) || "." || payload_bytes`
4. Sign with Ed25519
5. Compact serialisation: `{header_b64}..{signature_b64}` (double dot — payload carried separately in the VC)

### Verification

`dpp-crypto::jws_verifier` provides the single source of truth for JWS verification:

```rust
verify_jws(jws_compact, public_key_b64)?;
extract_primary_public_key(did_document)?;
```

The verifier:
1. Fetches the issuer's DID Document (via the `did:web` resolution rule)
2. Extracts the Ed25519 public key for the `kid` referenced in the JWS header
3. Reconstructs the signing input and verifies the Ed25519 signature

---

## 6. QR Code Trust Anchor

The carrier (QR or Data Matrix) on a physical product encodes a **GS1 Digital Link** URI, not a proprietary path:

```
{resolver_base}/01/{gtin}/10/{batchId}/21/{dpp_id}
```

- `resolver_base` is per-deployment configuration (`RESOLVER_BASE_URL`). A self-hoster sets it to their **own domain**, so the printed label carries the same trust root as their `did:web` identity; Odal's managed default is `https://id.odal-node.io`.
- The `/10/{batchId}` (batch/lot) segment is omitted when the passport carries no batch.
- The GTIN and identifier come from the **verified** passport fields — the resolver checks the JWS before building the URI and never trusts a stored `qrCodeUrl` value.
- The `/21/` serial segment carries a GS1-conformant 20-character serial derived from the passport id — a raw 36-character UUID exceeds the GS1 AI 21 limit.

The carrier **fails closed**: if the passport does not verify, no URI is produced; if the sector data has no GTIN (for example an unsold-goods report), resolution returns `422` rather than a misleading code. Because the carrier is standard GS1 Digital Link, any conformant resolver serving the same path answers the same scan — re-homing a passport is a DNS or registry change, not a reprint.

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
