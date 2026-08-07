"""Load every committed AAS Environment through an external AAS implementation.

The JSON Schema gate in `dpp-tests` establishes that our documents are *shaped*
the way the metamodel says. It cannot establish that another implementation can
ingest them, and the difference is not academic: IDTA sets `additionalProperties`
nowhere in 3.0, 3.1 or 3.2, so a member that is not part of a class validates in
silence against all three. Two shipped that way — `kind` on the shell, and `unit`
on `Property`, which made every Environment unloadable while every schema passed.

This runs the documents through `aas-core3.0`, the Python member of
aas-core-works' SDK family. All of those SDKs are generated from one
machine-readable metamodel definition, so they agree with the specification by
construction rather than by discipline.

Three checks, each catching something the others do not:

  deserialise   members not in the class, wrong JSON types, bad enum values
  verify        the specification's own constraints (idShort patterns,
                cardinality, identifier uniqueness)
  round-trip    information dropped or invented on the way back out

**Passing this is not IDTA conformance.** It says a reference implementation
accepts the document. Conformance is a separate process against IDTA's own test
tooling, and neither says anything about whether a submodel matches a published
submodel template.

Note that a lenient implementation is worth nothing here: Eclipse BaSyx accepts
unknown members and would have reported success on the `unit` defect. An oracle
has to be strict to be an oracle.

Exits non-zero, reporting every failing document rather than the first.
"""

import json
import pathlib
import sys

import aas_core3.jsonization as jsonization
import aas_core3.verification as verification

ENVIRONMENTS = (
    pathlib.Path(__file__).resolve().parents[2]
    / "crates"
    / "dpp-tests"
    / "fixtures"
    / "aas"
    / "environments"
)

# Every product group must be covered. A glob that silently matched nothing, or
# a fixture directory that moved, would otherwise make this pass having checked
# nothing at all.
MINIMUM_EXPECTED = 11


def check(path: pathlib.Path) -> list[str]:
    """Return the failures for one Environment; empty means it passed."""
    raw = json.loads(path.read_text(encoding="utf-8"))

    try:
        environment = jsonization.environment_from_jsonable(raw)
    except Exception as error:  # noqa: BLE001 - any rejection is a failure
        return [f"rejected by the loader: {type(error).__name__}: {error}"]

    failures = [
        f"constraint violation at {error.path}: {error.cause}"
        for error in verification.verify(environment)
    ]

    if jsonization.to_jsonable(environment) != raw:
        failures.append(
            "does not round-trip: serialising the parsed document back does not "
            "reproduce the input, so the loader read something other than what we wrote"
        )

    return failures


def main() -> int:
    if not ENVIRONMENTS.is_dir():
        print(f"error: no Environment fixtures at {ENVIRONMENTS}", file=sys.stderr)
        return 1

    documents = sorted(ENVIRONMENTS.glob("*.json"))
    failed = 0

    for path in documents:
        failures = check(path)
        if failures:
            failed += 1
            print(f"FAIL  {path.name}")
            for failure in failures:
                print(f"        {failure}")
        else:
            print(f"ok    {path.name}")

    print()
    if len(documents) < MINIMUM_EXPECTED:
        print(
            f"error: only {len(documents)} Environment(s) found, expected at least "
            f"{MINIMUM_EXPECTED} — this gate is not covering what it claims to",
            file=sys.stderr,
        )
        return 1

    if failed:
        print(
            f"error: {failed} of {len(documents)} Environments are not loadable by an "
            "AAS implementation. Regenerate with `UPDATE_AAS_FIXTURES=1 cargo test -p "
            "dpp-tests` if the mappers changed; otherwise this is a metamodel defect "
            "no JSON Schema can see.",
            file=sys.stderr,
        )
        return 1

    print(f"{len(documents)} Environments load, verify and round-trip.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
