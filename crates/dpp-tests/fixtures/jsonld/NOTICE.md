# Vendored W3C JSON-LD contexts

The three `.jsonld` files here are not our files. Each is a verbatim copy of a
context document the W3C publishes at the URL named below, vendored so the
JSON-LD oracle (`.github/workflows/jsonld-oracle.yml`) is hermetic. The W3C
remains authoritative; these copies exist so the check neither depends on
network access nor changes under us.

A JSON-LD processor fetches every string entry in an `@context` array. The
documents this workspace emits reference exactly these three, so the oracle's
document loader serves these files and refuses any other URL. A context added to
a builder therefore fails the oracle by name until somebody vendors it, which is
the point at which its URL gets checked.

| | `credentials-v2.jsonld` | `did-v1.jsonld` | `cid-v1.jsonld` |
|---|---|---|---|
| **Source** | `https://www.w3.org/ns/credentials/v2` | `https://www.w3.org/ns/did/v1` | `https://www.w3.org/ns/cid/v1` |
| **Retrieved** | 2026-10-07 | 2026-10-07 | 2026-10-07 |
| **Content type served** | `application/ld+json` | `application/ld+json` | `application/ld+json` |
| **Size (bytes)** | 10 131 | 1 474 | 3 248 |
| **SHA-256** | `59955ced6697d61e03f2b2556febe5308ab16842846f5b586d7f1f7adec92734` | `4f3eae5568c9c5f036a082088f9e192019ee06faa78973c87ff91d5421b88dad` | `ea216ecc1cb02cd39b693dba2250141e270ba0bf95890be107dd9a9e8e43de85` |
| **Used by** | credentials: the passport credential and the access credential | DID documents | DID documents |

The hashes are checked by the oracle script itself, on every run, against the
constants in `.github/scripts/jsonld_oracle.py`. All three files are pinned
`-text` in `.gitattributes`: line-ending translation would change the bytes on
disk without changing the blob, and the recorded hash would then fail to verify
on a checkout that normalises, which is the default on Windows.

## Why `credentials-v2` matters here

It declares `@protected: true` and defines `name` as `https://schema.org/name`.
A later context in the same array may not redefine a protected term, and a
conforming processor fails the whole document if it tries. It also declares no
`@vocab`, so a key it does not define is dropped on expansion with no error.
Both behaviours are what the oracle exists to catch.

## Updating

Re-download from the URL, record the retrieval date, size and hash above and in
the script, and say why. Do not edit these files: a locally modified copy of
someone else's context is no longer evidence of anything.
