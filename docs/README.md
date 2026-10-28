# dpp-core documentation

These documents describe the library: what a Digital Product Passport is in code, and
how it is validated, signed and verified. Running a service around the library is
outside its scope. The API reference for each crate is on [docs.rs](https://docs.rs/dpp-domain).

## Start here

1. **[architecture/OVERVIEW.md](architecture/OVERVIEW.md):** the shape of the library and
   its boundaries.
2. **[architecture/DATA-MODEL.md](architecture/DATA-MODEL.md):** what a passport contains,
   aligned with ESPR and the Battery Regulation.
3. **[regulatory/COMPLIANCE.md](regulatory/COMPLIANCE.md):** how the code maps to the law,
   article by article. A claim that cannot be pinned to Official Journal text is marked as
   such.

## By question

| Question | Read |
|---|---|
| How is the library structured, and why hexagonal? | [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md), [architecture/DESIGN-PATTERNS.md](architecture/DESIGN-PATTERNS.md) |
| Where does a new file go, and what checks it? | [architecture/CODE-LAYOUT.md](architecture/CODE-LAYOUT.md) |
| Can I change a struct that is already stored somewhere? | [architecture/PERSISTED-SHAPES.md](architecture/PERSISTED-SHAPES.md): the one place where a pre-1.0 breaking change is not free |
| Which port traits exist, and who implements them? | [architecture/PORTS.md](architecture/PORTS.md) |
| How do identity, signing and Verifiable Credentials work? | [architecture/IDENTITY.md](architecture/IDENTITY.md) |
| How do product group plugins run in a sandbox? | [architecture/PLUGIN-HOST.md](architecture/PLUGIN-HOST.md) |
| How do passports link to their components and predecessors? | [architecture/PRODUCT-LINEAGE.md](architecture/PRODUCT-LINEAGE.md): the bill-of-materials and second-life model, all five phases implemented |
| Which acts apply to a product group, and from when? | [architecture/DATA-MODEL.md](architecture/DATA-MODEL.md) §3.5: the instrument catalog, and why applicable law is a set |
| Why is a date pending rather than computed? | [architecture/EFFECTIVE-DATES.md](architecture/EFFECTIVE-DATES.md) |
| How does a schema change, and what counts as breaking? | [architecture/SCHEMA-CHANGES.md](architecture/SCHEMA-CHANGES.md) |
| Why is personal data kept out of the passport? | [architecture/PERSONAL-DATA.md](architecture/PERSONAL-DATA.md); the rule itself is in the `personal_data` module |
| Which RFC, W3C, GS1, IDTA or ETSI text does the code cite, at which revision? | [architecture/STANDARDS.md](architecture/STANDARDS.md): one row per specification, with the date its status was checked. A test holds the IETF rows |
| Where does code meet regulation formally? | [regulatory/CONFORMITY.md](regulatory/CONFORMITY.md), written for assessment bodies |
| Which regulations are in scope? | [regulatory/REGULATORY.md](regulatory/REGULATORY.md) |
| What is the project's scope, and what is out of it? | [project/BLUEPRINT.md](project/BLUEPRINT.md) |
| How are releases, versions and branches managed? | [governance/](governance/): VERSIONING, RELEASE, DEVELOPMENT, GIT-STRATEGY. CONTRIBUTING and CHANGELOG are at the repository root |

## Three design rules

**Sign locally, verify anywhere.** The manufacturer validates and signs a passport with
its own key. A verifier needs the passport and the signer's public `did:web` document,
nothing else. The port traits, the fail-closed verifiers and the evidence primitives all
follow from this.

**The build proves the boundary.** `cargo build --workspace` succeeds with no database,
no services and no environment variables. Anything that needs a database or an HTTP
client lives behind a port trait in `dpp-domain/src/ports/`, implemented by the
application that uses the library.

**Placeholders and unverified citations are labelled.** Stub port implementations
(`GhostBackup`, `GhostRegistrySync`, `GhostSeal`) mark their output as placeholder, so a
host can refuse to run on them in production. A regulatory citation that cannot be pinned
to the Official Journal is flagged, not asserted. A binding compliance verdict needs an
act that is in force **and** imposes a passport, checked separately: an act can bind today
and require no passport (ESPR Arts. 24 and 25), or have its passport duty met by another
system (Art. 9(4)(b), EPREL).
