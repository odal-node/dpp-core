# Governance

How decisions are made for `dpp-core`, and how that is expected to change.

## Current model: single maintainer

The project is pre-1.0 and has one maintainer, who holds all design, release and merge
authority:

- **Maintainer:** Aleksandar Temelkov ([@LKSNDRTMLKV](https://github.com/LKSNDRTMLKV))
- **Organisation:** Odal Node

One decision-maker keeps API iteration fast while the regulation it encodes (ESPR
delegated acts, the EU registry specification) is still moving.

## Where decisions are recorded

Significant technical decisions are recorded in the documents under
[`docs/architecture/`](docs/architecture/) and summarised in
[CHANGELOG.md](CHANGELOG.md). Each records the context, the choice, the alternatives
considered and the consequences. A superseded decision links to its replacement.

## Contributions

Every contribution follows [CONTRIBUTING.md](CONTRIBUTING.md):

1. Open an issue describing the change.
2. Fork, implement, and open a pull request.
3. Sign off every commit under the DCO (`Signed-off-by:` line).
4. CI passes (format, clippy, tests, audit). The semver-checks report is read but does
   not block; see [VERSIONING.md](docs/governance/VERSIONING.md).
5. The maintainer reviews and merges.

Nothing merges without maintainer approval. Force-pushes to `main` are blocked.

## Security-sensitive changes

A change to `dpp-crypto`, to schema validation, or to audience and disclosure
enforcement needs:

1. A review focused on cryptographic and access-control correctness.
2. A statement of the security implications in the pull request description.
3. An update to [SECURITY.md](SECURITY.md) if it changes the threat model or the
   cryptographic primitives.

## How the model will change

| Contributors | Model | What changes |
|---|---|---|
| 1 (today) | Single maintainer | All authority with the maintainer |
| 2 to 5 | Maintainer and trusted committers | Named people gain merge rights for specific crates |
| More than 5 | Maintainer council | Lazy consensus, and a written RFC process for breaking changes |

Any change of model is proposed as a pull request to this document and discussed in
public.

## Licensing

`dpp-core` is licensed under Apache-2.0. The maintainer holds copyright in the original
work. Contributors keep copyright in their contributions and license them through the DCO
sign-off; there is no Contributor License Agreement. Changing the project's licence needs
the agreement of every copyright holder.

## Code of conduct

It applies to every space this repository runs: issues, pull requests, discussions,
code review, commit messages, and email to the project's addresses.

Be direct and be decent. Criticise the work, not the person. Say what you think and why;
blunt technical disagreement is welcome. Not acceptable, in any tone: harassment, personal
attacks, discrimination or slurs on any basis, publishing someone's private information
without their permission, sexualised language or unwelcome sexual attention, and sustained
disruption such as reopening settled decisions to wear people down.

This software is used for regulatory compliance, and operators may act on what it says.
**Knowingly stating something false about EU law, about what the software does, or about
its production readiness is a conduct problem here,** and so is describing a plan as a
shipped feature. Honest mistakes are not.

Report to **contact@odal-node.io**: what happened, where and when. A link is enough. You
get an acknowledgement within 72 hours, and your report is not shared with the person you
report without telling you first. If it concerns the maintainer, say so in the report.
GitHub's abuse reporting is available to you independently of this project.

Depending on severity, the outcome is a private correction, a public correction, removal
of the content, a temporary block or a permanent ban. The maintainer is bound by this
section too.

## References

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [SECURITY.md](SECURITY.md)
- [Architecture documents](docs/architecture/)
- [Developer Certificate of Origin](https://developercertificate.org/)
