# dpp-core Documentation — start here

This folder documents **the standard, not the product**: what a Digital Product Passport *is* in code, how it's signed, validated, and verified. If you're looking for how to *run* a node, that's the engine repo's docs.

## If you're new, read these three, in this order

1. **[architecture/OVERVIEW.md](architecture/OVERVIEW.md)** — the shape of the library and the open-core boundary in ten minutes.
2. **[architecture/DATA-MODEL.md](architecture/DATA-MODEL.md)** — what a passport contains and why (ESPR / Battery Regulation aligned).
3. **[regulatory/COMPLIANCE.md](regulatory/COMPLIANCE.md)** — how code maps to law, article by article, with our honesty conventions (a claim we can't pin to the OJ text is marked, not asserted).

## By question

| You're asking… | Read |
|---|---|
| "How is the library structured, and why hexagonal?" | [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) · [architecture/DESIGN-PATTERNS.md](architecture/DESIGN-PATTERNS.md) |
| "How do identity, signing, and verifiable credentials work?" | [architecture/IDENTITY.md](architecture/IDENTITY.md) |
| "How do sector plugins run safely?" | [architecture/PLUGIN-HOST.md](architecture/PLUGIN-HOST.md) |
| "Can a third party verify a passport without trusting anyone?" | [../crates/dpp-evidence/spec/dossier-v1.md](../crates/dpp-evidence/spec/dossier-v1.md) — the evidence-dossier wire format and its offline checks |
| "Where does code meet regulation, formally?" | [regulatory/CONFORMITY.md](regulatory/CONFORMITY.md) — written for assessment bodies |
| "How are releases, versions, and contributions governed?" | [governance/](governance/) — VERSIONING, RELEASE, CONTRIBUTING, CHANGELOG |

## The three ideas that explain everything else

**Proof-bound.** The manufacturer validates and signs locally with their own key; the world receives a verifiable proof, not a promise. Every design decision — the port seam, the evidence dossier, the fail-closed verifiers — follows from this.

**The compiler enforces the boundary.** Core builds with zero infrastructure (`cargo build --workspace`, no DB, no env). Anything that needs a database or an HTTP client lives across the seam in the engine. The port traits in `dpp-domain/src/ports/` *are* the boundary — the module is authoritative, prose never quotes a hardcoded count.

**Honesty is a feature.** Placeholder implementations (the Ghost family) are clearly marked, regulatory citations that can't be pinned to the Official Journal are flagged rather than asserted, and provisional sectors can never emit a binding compliance verdict.
