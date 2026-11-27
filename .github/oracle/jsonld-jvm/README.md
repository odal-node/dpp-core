# JSON-LD JVM oracle

Titanium 2.0.0-M2 judges the corpus emitted by the Rust builders, alongside PyLD.
It uses JSON-LD 1.1, a null base, an allowlisted loader for the three hash-checked
W3C context fixtures, and fails on undefined terms. Its controls check protected
term rejection, missing-term detection, unvendored-context rejection, and
relative IDs.

Titanium currently rejects identical protected terms repeated across layered
contexts. To preserve the W3C-defined result, the oracle inlines only the
hash-verified vendored contexts and removes only term definitions that are
exactly repeated; conflicting definitions are left for Titanium to reject.
Titanium also returns JSON-LD `@json` values through `TreeIO`, which the oracle
materializes before checking round-trip paths. Both adaptations are confined to
this judge; the generated corpus is unchanged. Titanium 2.0.0-M2 is a milestone
release, so upgrade it deliberately and rerun both judges.

Run these commands from the repository root (Java 25, Maven, Rust toolchain):

```sh
python3 -m venv target/jsonld-oracle/venv
target/jsonld-oracle/venv/bin/python -m pip install --require-hashes -r .github/oracle/jsonld/requirements.txt
EMIT_JSONLD_CORPUS=1 cargo test -p dpp-tests --test jsonld_oracle_corpus
target/jsonld-oracle/venv/bin/python .github/scripts/jsonld_oracle.py
mvn -q -B -f .github/oracle/jsonld-jvm/pom.xml package
java -cp '.github/oracle/jsonld-jvm/target/classes:.github/oracle/jsonld-jvm/target/lib/*' \
  io.odalnode.oracle.JsonLdJvmOracle
```

The Rust command writes `target/jsonld-oracle/corpus.json`; both judges read that
same file. Neither judge fetches contexts at runtime. The only network access is
for installing the pinned Python and Maven dependencies.