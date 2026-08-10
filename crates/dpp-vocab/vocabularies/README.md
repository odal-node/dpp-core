# `vocabularies/` — one file per authority, never mixed

Each file describes **one** upstream vocabulary: who publishes it, where its
terms live, under what licence, and what a person here has actually verified
about it. One authority per file, mirroring `dpp-domain/sectors/*.json`.

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

## Statuses

- **`verified`** — a person read the authority's own publication. Only a
  `verified` vocabulary can carry permitted terms.
- **`tracked`** — evaluated and not adopted, with a recorded finding. Refused.
- **`surveyed`** — known to exist from a secondary source. **Nothing has been
  read.** Refused, and the weakest state: a survey can be wrong about the IRI,
  the version, the licence and the scope at once.

Nothing here is `verified` today. That is the accurate state, not an omission.

## Term counts are deliberately absent

Every published term count for these vocabularies traces to one third-party
site's own summaries. Two independent readings of the same CIRPASS-2 version
produced **233** and **338**. A count is not load-bearing, and an unverified
number in a published crate is the defect class this directory exists to
prevent. The survey figures are recorded in the private research notes, marked
as reported rather than measured.
