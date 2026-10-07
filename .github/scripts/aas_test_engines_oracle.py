"""Check every committed AAS Environment with IDTA's own test tooling.

`aas_loader_oracle.py` runs the Environments through `aas-core3.0`, an
independent implementation written by the aas-core-works project. This is the
second oracle over the same files, and it comes from the specification's owner:
`aas-test-engines` is published by IDTA's `admin-shell-io` organisation as the
"official test tooling for the Asset Administration Shell". Two implementations
that share no code and no author, agreeing on the same document, is stronger
evidence than either alone — and where they disagree, the disagreement is the
finding.

What the tool does with a file, in the order it does it:

  check meta model   every member is one the class defines, every type and enum
                     value is right (an unknown member such as `unit` on a
                     `Property` is rejected here, which a JSON Schema without
                     `additionalProperties: false` cannot do)
  check constraints  the specification's own constraints, `AASd-xxx`: idShort
                     rules, uniqueness within a namespace, value against
                     `valueType`, reference shapes. Skipped when the meta model
                     check already failed, so a failure here is a shape problem
                     first and a constraint problem second.

**The revision is IDTA-01001-3-0, and nothing here can say otherwise.** The
tool's own `supported_versions()` lists `3.0` and no other, and its file check
ignores the version argument. Moving the claim to a later revision needs an
independent implementation of that revision to exist; this script fails the day
the tool starts to list a second version, so the pin cannot be moved past that
point by accident.

**Known gap in the tool, covered by the other oracle.** The tool's source marks
`AASd-021` (one qualifier per `type`) and `AASd-077` (one extension per `name`)
as not implemented. `aas-core3.0` verifies both.

**Passing this is not IDTA certification.** Nobody certifies an AAS file; this
says IDTA's own checker accepts the document under the revision it implements.
It says nothing about whether a submodel matches a published submodel template,
and every `semanticId` here is `urn:odal-node:*` by design.

A tool that cannot fail proves nothing, so two deliberately broken Environments
are run first and the tool must reject both. A tool upgrade that stopped
checking constraints would otherwise leave every real Environment green.

Exits non-zero, reporting every failing document rather than the first.
"""

import copy
import json
import pathlib
import sys
from importlib.metadata import version

from aas_test_engines import file

ROOT = pathlib.Path(__file__).resolve().parents[2]
ENVIRONMENTS = ROOT / "crates" / "dpp-tests" / "fixtures" / "aas" / "environments"
SCHEMAS = ROOT / "crates" / "dpp-domain" / "schemas"

# The Environments that belong to no product group on their own. See the comment
# on `MINIMUM_EXPECTED` in `aas_loader_oracle.py` for why this one exists.
SCENARIOS = {"scenario-model-level-did"}

# The one revision the tool implements. If it ever lists another, the claim in
# docs/architecture/STANDARDS.md needs a decision, not a silent pin bump.
IMPLEMENTED_REVISION = "3.0"


def strip_ansi(line: str) -> str:
    """The tool colours its report with escape codes; CI logs should not."""
    out = []
    skipping = False
    for char in line:
        if char == "\x1b":
            skipping = True
        elif skipping:
            skipping = char != "m"
        else:
            out.append(char)
    return "".join(out)


def check(document: dict) -> tuple[bool, list[str]]:
    """Run the tool; return its verdict and its report, one line per entry."""
    result = file.check_json_data(document, model_type="Environment")
    return result.ok(), [strip_ansi(line) for line in result.to_lines()]


def first_submodel_elements(environment: dict) -> list:
    for submodel in environment["submodels"]:
        if submodel.get("submodelElements"):
            return submodel["submodelElements"]
    raise SystemExit("error: no Environment has any submodel elements to break")


def controls(sample: dict) -> list[str]:
    """Return the ways the tool failed to reject a document it must reject."""
    escaped = []

    duplicated = copy.deepcopy(sample)
    elements = first_submodel_elements(duplicated)
    elements.append(copy.deepcopy(elements[0]))
    accepted, _ = check(duplicated)
    if accepted:
        escaped.append(
            "accepted a duplicated idShort (AASd-022), so the constraint check is not running"
        )

    unknown = copy.deepcopy(sample)
    first_submodel_elements(unknown)[0]["unit"] = "kg"
    accepted, _ = check(unknown)
    if accepted:
        escaped.append(
            "accepted a member the class does not define, so the meta-model check is not running"
        )

    return escaped


def expected_names() -> set[str]:
    """One Environment per product group, named by the schema directories.

    The product groups are the directories under the schemas, which is the same
    set the catalog is built from. Reading them here, rather than counting
    files, means a product group added without an Environment fails this
    script by name instead of passing with one fewer.
    """
    groups = {path.name for path in SCHEMAS.iterdir() if path.is_dir()}
    return groups | SCENARIOS


def main() -> int:
    tool = version("aas-test-engines")
    supported = sorted(file.supported_versions())
    print(f"aas-test-engines {tool}, AAS metamodel revisions it checks: {supported}")
    if supported != [IMPLEMENTED_REVISION]:
        print(
            f"error: the tool now lists {supported}, not [{IMPLEMENTED_REVISION!r}]. "
            "Decide which revision the claim in docs/architecture/STANDARDS.md is made "
            "against, and update IMPLEMENTED_REVISION and the register row together.",
            file=sys.stderr,
        )
        return 1

    if not ENVIRONMENTS.is_dir() or not SCHEMAS.is_dir():
        print(f"error: no fixtures at {ENVIRONMENTS} or schemas at {SCHEMAS}", file=sys.stderr)
        return 1

    documents = {path.stem: path for path in sorted(ENVIRONMENTS.glob("*.json"))}
    missing = sorted(expected_names() - documents.keys())
    unexplained = sorted(documents.keys() - expected_names())
    if missing or unexplained:
        print(
            f"error: Environments do not match the product groups. Missing: {missing}. "
            f"Present but belonging to no product group or scenario: {unexplained}. "
            "This gate is not covering what it claims to.",
            file=sys.stderr,
        )
        return 1

    sample = json.loads(next(iter(documents.values())).read_text(encoding="utf-8"))
    escaped = controls(sample)
    if escaped:
        for reason in escaped:
            print(f"error: control failed: the tool {reason}", file=sys.stderr)
        return 1
    print("controls: the tool rejects a duplicated idShort and an unknown member\n")

    failed = 0
    for name, path in documents.items():
        accepted, report = check(json.loads(path.read_text(encoding="utf-8")))
        if accepted:
            print(f"ok    {path.name}")
            for line in report:
                # Only a warning is worth showing on a pass; the framing lines
                # ("Check", "Check meta model", ...) are noise on a green file.
                if "warning" in line.lower():
                    print(f"        {line.strip()}")
        else:
            failed += 1
            print(f"FAIL  {path.name}")
            for line in report:
                print(f"        {line}")

    print()
    if failed:
        print(
            f"error: IDTA's own checker rejects {failed} of {len(documents)} Environments. "
            "Regenerate with `UPDATE_AAS_FIXTURES=1 cargo test -p dpp-tests` if the mappers "
            "changed; otherwise this is a metamodel defect no JSON Schema can see.",
            file=sys.stderr,
        )
        return 1

    print(
        f"{len(documents)} Environments pass the meta-model and constraint checks of "
        f"aas-test-engines {tool} (AAS {IMPLEMENTED_REVISION})."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
