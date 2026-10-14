# `vocabularies/` — one file per authority, never mixed

Each file describes **one** upstream vocabulary: who publishes it, where its
terms live, under what licence, and what a person here has actually verified
about it. One authority per file, mirroring `dpp-domain/product-groups/*.json`.

## The rule this directory exists to enforce

> Naming another organisation's term asserts, to a machine, that our field means
> what that organisation says its identifier means. That is a claim about
> somebody else's vocabulary, and it is recorded only once a person has read
> their published source.

There is deliberately no third category between "verified" and "not permitted".

## Fixed key set

Every file carries every key. `null` means **not established**, never "none" and
never "not applicable" — a missing licence blocks reuse rather than allowing it.

| Key | Meaning |
|---|---|
| `key` | Stable lookup key. Must equal the filename stem |
| `title` | The vocabulary's own name for itself |
| `authority` | The organisation that publishes it |
| `layer` | `foundational` · `community-profile` · `sectoral` · `conformance-bridge` |
| `namespaceIri` | Prefix IRI its terms sit under. `null` where the authority publishes none |
| `usedFor` | What we would use it for. Intent, not a statement that we use it |
| `licence` | SPDX identifier or a plain description. `null` = unstated by the source |
| `status` | `verified` · `tracked` · `surveyed` |
| `checkedOn` | ISO date of the most recent look |
| `source` | Where that look happened |
| `finding` | What was learned. The load-bearing field |
| `nextStep` | What would move it to the next status |
| `terms` | The terms somebody has read, each with `iri`, `readOn`, `source` and `finding`. `[]` = none |
| `pins` | The upstream state the findings rest on: each `kind` (`github-head`, `github-release`, `url-sha256`), `target`, optional `path`, `value` and `observedOn`. `[]` = nothing pinned |

A term is listed only on a `verified` record at a layer we may speak, and only
after its definition was read. Listing it is what permits emitting it: the
register permits a term, not a namespace. The IRI normally sits under
`namespaceIri`; one that does not must not fall under any other record's
namespace.

A `verified` record must pin its source. `just vocab-drift` observes every pin
again and names the records whose source has moved; it cannot say a record is
wrong, only that it is due a reading. After reading again, update the `finding`,
`checkedOn` and the pin together. Pin the narrowest thing whose change would
matter (a release-notes file, not a directory of generated artefacts), and only
state that was actually read.

## Statuses

- **`verified`** — a person read the authority's own publication. Only a
  `verified` vocabulary can carry permitted terms, and then only the ones its
  `terms` list.
- **`tracked`** — evaluated and not adopted, with a recorded finding. Refused.
- **`surveyed`** — known to exist from a secondary source, or from a directory
  listing or release notes alone. **No term has been read.** Refused, and the
  weakest state: a survey can be wrong about the IRI, the version, the licence
  and the scope at once.

Six of the twenty records are `verified`; the crate README names them. The rest
stay refused. That is the accurate state, not an omission.

## Term counts are deliberately absent

Every published term count for these vocabularies traces to one third-party
site's own summaries. Two independent readings of the same CIRPASS-2 version
produced **233** and **338**. A count is not load-bearing, and an unverified
number in a published crate is the defect class this directory exists to
prevent. The survey figures are recorded outside this repository, marked
as reported rather than measured.
