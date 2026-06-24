# Project Governance

This document describes the decision-making structure for dpp-core and how it
is expected to evolve.

## Current Model: BDFL (Benevolent Dictator For Life)

dpp-core is maintained by a single author and the project is in its early
pre-1.0 phase. All design decisions, release approvals, and merge authority
rest with:

- **Maintainer**: Aleksandar Temelkov (LKSNDRTMLKV)
- **Organisation**: Odal Node
- **Contact**: https://github.com/LKSNDRTMLKV

This model is appropriate for the current stage. It allows fast iteration on
API design while the EU regulatory landscape (ESPR delegated acts, EU Registry
specification) is still evolving.

## Decision Records

Significant technical decisions are recorded in the architecture and design
docs under `docs/architecture/` and `docs/design/`, and summarised in
`CHANGELOG.md`. Each decision captures the context, what was chosen, the
alternatives considered, and the consequences. Superseded decisions are marked
as such with a link to the replacement.

## Contribution Governance

All contributions follow the process described in [CONTRIBUTING.md](CONTRIBUTING.md):

1. Open an issue describing the proposed change.
2. Fork, implement, and submit a pull request.
3. All commits must carry a DCO sign-off (`Signed-off-by:` line).
4. CI must pass (fmt, clippy, nextest, audit, semver-checks).
5. The maintainer reviews and merges.

No pull request is merged without maintainer approval. Force-pushes to `main`
are prohibited.

## Security Decisions

Security-sensitive changes (anything touching `dpp-crypto`, schema validation
logic, or access-tier enforcement) require:

1. A dedicated review focusing on cryptographic correctness.
2. Explicit sign-off in the PR description noting the security implications.
3. An update to [SECURITY.md](../project/SECURITY.md) if the change affects the
   threat model or supported cryptographic primitives.

## Evolution Path

As the contributor base grows, governance will evolve:

| Contributors | Model | Change |
|---|---|---|
| 1 (current) | BDFL | All authority with maintainer |
| 2 - 5 | BDFL + Trusted Committers | Named individuals gain merge rights for specific crates |
| 5+ | Maintainer Council | Decisions by lazy consensus, formal RFC process for breaking changes |

Any governance transition will be proposed as a PR updating this document,
discussed publicly, and recorded here.

## Licensing Authority

dpp-core is licensed under Apache-2.0. The maintainer holds copyright over the
original work. Contributors retain copyright over their contributions and
grant the license via the DCO sign-off. No Contributor License Agreement (CLA)
is required.

Decisions to change the project license require agreement from all copyright
holders.

## Code of Conduct

All participants are expected to behave professionally and respectfully.
A formal Code of Conduct will be adopted before the project actively solicits
external contributions.

## References

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [SECURITY.md](../project/SECURITY.md)
- [Architecture docs](../architecture/)
- [Developer Certificate of Origin](https://developercertificate.org/)
