"""Expand every JSON-LD context and document this workspace emits, with PyLD.

`crates/dpp-tests/tests/jsonld_context.rs` holds the contexts to this
repository's own reading of the JSON-LD specification, and says so: nothing
there runs a processor. A transcription error in a term passes it. So does a key
emitted at a position no term reaches, which is the defect JSON-LD tooling hides
best, because a processor drops such a key with everything inside it and reports
nothing.

PyLD is an independent implementation of JSON-LD 1.1 and its API, written by
Digital Bazaar and sharing no code or author with this workspace. The corpus it
is given comes from `crates/dpp-tests/tests/jsonld_oracle_corpus.rs`, which
builds every document with the real builder.

Six checks, each catching something the others do not:

  contexts   every `@context` an emitted document carries processes without
             error, and every term it defines expands to an IRI. A protected
             term redefined, a keyword misused, or a term mapped to nothing is
             caught here, before any document is involved.
  expand     every document expands without a processor error.
  dropped    no property of a document expands to nothing. PyLD calls a handler
             for each one it drops, and the same set is recomputed from an
             expand-and-compact round trip so each drop is reported by path.
  loader     the processor is given the vendored W3C contexts and nothing else.
             A context the oracle cannot serve is a failure, not a fetch, so a
             new remote context has to be looked at before it is trusted.
  named      every node identifier in the expanded form is an absolute IRI or a
             blank node. A relative one names nothing until a base is chosen:
             converting to RDF without a base drops every statement about the
             node. PyLD is passed an explicit null base for this, because when
             the option is left out it resolves a relative IRI against
             `http://example.org/base/` of its own accord, which would hide
             exactly this. The passport's own `id` was such a reference until it
             was written as `urn:uuid:`, and a second processor (jsonld.js)
             found it.
  layered    our own contexts still expand after `credentials/v2` and `did/v1`.
             Both protect `id` as `@id`, and a context of ours that redefined a
             protected term could no longer be layered after them, nor a framed
             passport embedded in a credential.

**One gap is known and listed, not hidden.** The passport context defines a term
for every key of the passport envelope and for the product identifier, and for
nothing beneath those: the keys inside `manufacturer`, `materials` and the rest,
and every key of `productGroupData` except the identifier, have no term. A
processor drops them. They are allowed below as `NESTED_UNDEFINED`, and they are
the register row's stated deviation. The allowance is checked both ways: any
other drop fails, and the day the context defines those terms the allowance is
reported as stale and has to be deleted.

**Passing this is conformance of the documents, not of the vocabulary.** It says
a JSON-LD 1.1 processor accepts the documents and loses nothing outside the
known gap. It says nothing about whether any IRI is the right IRI.

Two deliberately broken inputs are run first, and the processor must reject or
drop them. A processor upgrade that stopped checking would otherwise leave every
real document green.

Exits non-zero, reporting every failure rather than the first.
"""

import copy
import hashlib
import json
import pathlib
import re
import sys
from importlib.metadata import version

from pyld import jsonld

ROOT = pathlib.Path(__file__).resolve().parents[2]
CORPUS = ROOT / "target" / "jsonld-oracle" / "corpus.json"
VENDORED_DIR = ROOT / "crates" / "dpp-tests" / "fixtures" / "jsonld"

# The only remote contexts the oracle will serve. The hashes are recorded in
# the NOTICE.md beside the files and checked here, on every run.
VENDORED = {
    "https://www.w3.org/ns/credentials/v2": (
        "credentials-v2.jsonld",
        "59955ced6697d61e03f2b2556febe5308ab16842846f5b586d7f1f7adec92734",
    ),
    "https://www.w3.org/ns/did/v1": (
        "did-v1.jsonld",
        "4f3eae5568c9c5f036a082088f9e192019ee06faa78973c87ff91d5421b88dad",
    ),
    "https://www.w3.org/ns/cid/v1": (
        "cid-v1.jsonld",
        "ea216ecc1cb02cd39b693dba2250141e270ba0bf95890be107dd9a9e8e43de85",
    ),
}

# What a framed passport's context does not define: any key nested below an
# envelope key, except beneath `productGroupData.productIdentifier`, which has a
# scoped context, and `personalData`, which is kept whole as a JSON literal.
NESTED_UNDEFINED = (
    "keys nested below an envelope key of a framed passport, including every key "
    "of productGroupData except productIdentifier"
)


def is_nested_undefined(label: str, path: str) -> bool:
    if not label.startswith("a framed "):
        return False
    parts = path.strip("/").split("/")
    if len(parts) < 2:
        return False
    if parts[0] == "personalData":
        return False
    return not (parts[0] == "productGroupData" and parts[1] == "productIdentifier")


ABSOLUTE_IRI = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")


def relative_ids(node, path=""):
    """Every node identifier in an expanded document that is not an absolute IRI
    or a blank node, with the path it sits at."""
    found = []
    if isinstance(node, dict):
        identifier = node.get("@id")
        if isinstance(identifier, str) and not (
            identifier.startswith("_:") or ABSOLUTE_IRI.match(identifier)
        ):
            found.append((path or "/", identifier))
        for key, value in node.items():
            if key not in ("@id", "@value"):
                found += relative_ids(value, f"{path}/{key}")
    elif isinstance(node, list):
        for item in node:
            found += relative_ids(item, path)
    return found


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_vendored() -> list[str]:
    failures = []
    for url, (name, expected) in VENDORED.items():
        path = VENDORED_DIR / name
        if not path.is_file():
            failures.append(f"{url}: {path} is missing")
        elif sha256(path) != expected:
            failures.append(
                f"{url}: {name} no longer matches the SHA-256 recorded for it. "
                "Either the file was edited, which it must not be, or it was "
                "replaced, in which case update the hash here and in NOTICE.md."
            )
    return failures


def loader(url, options=None):
    """Serve the vendored contexts; refuse everything else, never fetch."""
    if url not in VENDORED:
        raise jsonld.JsonLdError(
            f"the oracle holds no copy of {url}. Vendor it with its SHA-256, or inline its terms.",
            "jsonld.LoadDocumentError",
            code="loading document failed",
        )
    name, _ = VENDORED[url]
    return {
        "contentType": "application/ld+json",
        "contextUrl": None,
        "documentUrl": url,
        "document": json.loads((VENDORED_DIR / name).read_text(encoding="utf-8")),
    }


# `base: None` is load-bearing. Left out, or given as "", PyLD resolves a relative
# IRI against `http://example.org/base/`, a base of its own, and the `named` check
# below would never see one.
OPTIONS = {"processingMode": "json-ld-1.1", "documentLoader": loader, "base": None}


def expand(document):
    """Return the expanded form and how many properties the processor dropped."""
    dropped = []
    expanded = jsonld.expand(
        document, OPTIONS, on_property_dropped=lambda prop: dropped.append(prop)
    )
    return expanded, len(dropped)


def paths(node, prefix=""):
    """Every key path in a JSON tree, array positions ignored.

    A key whose value is null is skipped: JSON-LD drops null by definition, so
    it is not a term that expands to nothing.
    """
    found = set()
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "@context" or value is None:
                continue
            path = f"{prefix}/{key}"
            found.add(path)
            found |= paths(value, path)
    elif isinstance(node, list):
        for item in node:
            found |= paths(item, prefix)
    return found


def lost_roots(document, expanded):
    """Key paths the expansion lost, without the paths beneath a lost key."""
    compacted = jsonld.compact(expanded, document["@context"], OPTIONS)
    lost = paths(document) - paths(compacted)
    return sorted(p for p in lost if not any(p.startswith(q + "/") for q in lost))


def probe(context_entries):
    """A document with one property for every term the inline entries define.

    Prefix declarations and keyword aliases are skipped: they name a vocabulary
    or a keyword, not a property. A term with a scoped context is probed at the
    position it occupies, because that is the only place its terms are reachable.
    """

    def properties(definitions):
        node = {}
        for term, definition in definitions.items():
            if term.startswith("@"):
                continue
            if definition is None:
                # A null definition switches a term off, so the key is dropped.
                # That is never what one of our contexts means to say.
                node[term] = "x"
            elif isinstance(definition, str):
                if definition.startswith("@") or "://" in definition:
                    continue
                node[term] = "x"
            elif isinstance(definition, dict):
                scoped = definition.get("@context")
                node[term] = properties(scoped) if isinstance(scoped, dict) else "x"
        return node

    document = {"@context": context_entries, "@id": "urn:oracle:probe"}
    for entry in context_entries:
        if isinstance(entry, dict):
            document.update(properties(entry))
    return document


def controls() -> list[str]:
    """Return the ways the processor failed to reject what it must reject."""
    escaped = []

    redefined = {
        "@context": [
            "https://www.w3.org/ns/credentials/v2",
            {"name": "https://example.org/another-name"},
        ],
        "name": "x",
    }
    try:
        expand(redefined)
        escaped.append(
            "redefined a protected term without an error, so protected terms are not enforced"
        )
    except jsonld.JsonLdError as error:
        if getattr(error, "code", None) != "protected term redefinition":
            escaped.append(f"failed for another reason than a protected term: {error}")

    undefined = {"@context": {"dpp": "https://example.org/dpp#"}, "undefinedKey": "x"}
    expanded, dropped = expand(undefined)
    if dropped != 1 or lost_roots(undefined, expanded) != ["/undefinedKey"]:
        escaped.append("did not report a key with no term as dropped, so drops would go unseen")

    try:
        expand({"@context": "https://example.org/never-vendored", "a": "x"})
        escaped.append("served a context that is not vendored, so the oracle could fetch")
    except jsonld.JsonLdError:
        pass

    unnamed = {"@context": {"dpp": "https://example.org/dpp#"}, "@id": "a-bare-uuid", "dpp:x": "y"}
    expanded, _ = expand(unnamed)
    if relative_ids(expanded) != [("/", "a-bare-uuid")]:
        escaped.append("did not report a relative node identifier, so an unnamed node would go unseen")

    return escaped


def main() -> int:
    print(f"PyLD {version('pyld')}, processing mode json-ld-1.1")

    failures = check_vendored()
    if failures:
        for failure in failures:
            print(f"error: vendored context: {failure}", file=sys.stderr)
        return 1

    if not CORPUS.is_file():
        print(
            f"error: no corpus at {CORPUS}. Build it with `EMIT_JSONLD_CORPUS=1 cargo test "
            "-p dpp-tests --test jsonld_oracle_corpus`.",
            file=sys.stderr,
        )
        return 1
    corpus = json.loads(CORPUS.read_text(encoding="utf-8"))

    escaped = controls()
    if escaped:
        for reason in escaped:
            print(f"error: control failed: the processor {reason}", file=sys.stderr)
        return 1
    print(
        "controls: a redefined protected term fails, an undefined key is reported, "
        "an unvendored context is refused, a relative node identifier is reported\n"
    )

    failed = 0

    for entry in corpus["contexts"]:
        label = entry["label"]
        problems = []
        try:
            document = probe(copy.deepcopy(entry["context"]))
            _, dropped = expand(document)
            if dropped:
                problems.append(f"{dropped} defined term(s) expand to nothing")
        except jsonld.JsonLdError as error:
            problems.append(describe(error))
        # Our contexts have to stay usable under the W3C ones, which protect `id`.
        for base in ("https://www.w3.org/ns/credentials/v2", "https://www.w3.org/ns/did/v1"):
            own = entry["context"] if isinstance(entry["context"], list) else [entry["context"]]
            if base in own:
                continue
            try:
                expand(probe(copy.deepcopy([base] + own)))
            except jsonld.JsonLdError as error:
                problems.append(f"cannot be layered after {base}: {describe(error)}")
        report("context ", label, problems)
        failed += bool(problems)

    allowed_seen = 0
    for entry in corpus["documents"]:
        label = entry["label"]
        document = entry["document"]
        problems = []
        try:
            expanded, dropped = expand(document)
            roots = lost_roots(document, expanded)
            if bool(dropped) != bool(roots):
                problems.append(
                    f"the processor reports {dropped} dropped and the round trip finds "
                    f"{len(roots)}: the oracle disagrees with itself"
                )
            for path in roots:
                if is_nested_undefined(label, path):
                    allowed_seen += 1
                else:
                    problems.append(f"{path} expands to nothing")
            for path, identifier in relative_ids(expanded):
                problems.append(
                    f"{path} is named {identifier!r}, a relative IRI: without a base it names nothing"
                )
        except jsonld.JsonLdError as error:
            problems.append(describe(error))
        report("document", label, problems)
        failed += bool(problems)

    print()
    if allowed_seen == 0:
        print(
            f"error: the known gap ({NESTED_UNDEFINED}) no longer drops anything. "
            "Delete NESTED_UNDEFINED and the register's deviation for it.",
            file=sys.stderr,
        )
        failed += 1
    else:
        print(f"known gap, listed in the register: {allowed_seen} key paths ({NESTED_UNDEFINED})")

    if failed:
        print(f"error: {failed} failure(s)", file=sys.stderr)
        return 1

    print(
        f"{len(corpus['contexts'])} contexts and {len(corpus['documents'])} documents "
        f"expand under PyLD {version('pyld')} with nothing lost outside the known gap."
    )
    return 0


def describe(error: jsonld.JsonLdError) -> str:
    """The processor's own code and message, on one line."""
    return f"{getattr(error, 'code', 'error')}: {error.args[0]}"


def report(kind: str, label: str, problems: list[str]) -> None:
    if not problems:
        print(f"ok    {kind} {label}")
        return
    print(f"FAIL  {kind} {label}")
    for problem in problems:
        print(f"        {problem}")


if __name__ == "__main__":
    sys.exit(main())
