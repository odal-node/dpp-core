#!/usr/bin/env python3
"""Fail the build when a file in this public repo references private material.

`dpp-core` is published to crates.io. Anything committed here is public the
moment it is pushed, and a released crate version can never be unpublished — so
a reference that leaks the existence or structure of a non-public repository is
a defect that cannot be walked back, only superseded.

CLAUDE.md section 6 states the rule. This script is the enforcement, and it
exists because the rule was stated and then broken: a vocabulary record shipped
in 0.17.0 carrying a filesystem path into a private sibling repository, compiled
into the published artefact via `include_str!`. Review did not catch it because
the string was inside a JSON data field rather than code.

That is the point. Every previous instance of this class has been in **prose** —
module docs, schema `description` fields, README paragraphs, planning narrative —
and a check that reads only code comments is not a check. This one reads every
text file in the tree.

What it looks for:

* **Names of, or paths into, sibling repositories that are not public.** Only
  `dpp-core` and `dpp-engine` are public; naming any other repo discloses that it
  exists, which is itself something a public reader should not learn here.
* **Internal decision-record references.** Their numbering and structure are
  private, and they point a reader at something they cannot open.
* **The name of a non-customer** that internal planning material once described
  as a pilot. It is a real company that has signed nothing.

What it deliberately does not look for: general planning vocabulary, priority
codes, phase letters. Those are real parts of the rule but too ambiguous to grep
without drowning the signal in false positives — `P0` is a legitimate string in
a dozen contexts. A tripwire people switch off is worse than no tripwire, so
this one only fires on patterns that are unambiguous.

Usage:
    python .github/scripts/check_private_refs.py [--root DIR]

Exit code 0 when clean, 1 when a reference is found.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# Repositories in this project that are NOT public. `dpp-core` (this one) and
# `dpp-engine` are public and may be named freely; everything else may not.
PRIVATE_REPOS = (
    "dpp-docs",
    "dpp-control-plane",
    "dpp-infra",
    "dpp-legacy",
    "dpp-web",
)

PATTERNS: list[tuple[str, re.Pattern[str], str]] = [
    (
        "private-repo",
        re.compile(r"\b(" + "|".join(PRIVATE_REPOS) + r")\b"),
        "names a repository that is not public; state the substance inline instead",
    ),
    (
        "decision-record",
        re.compile(r"\bADR[-\s]\d+"),
        "references an internal decision record; cite the OJ text or standard instead",
    ),
    (
        "non-customer",
        re.compile(r"\bAmor\b"),
        "names a company that is not a customer and has consented to nothing",
    ),
]

# Extensions worth reading. Prose counts: the three recorded instances of this
# failure class were all in prose, not code.
TEXT_SUFFIXES = {
    ".rs", ".md", ".json", ".toml", ".yml", ".yaml", ".sh", ".py", ".txt",
}

SKIP_DIRS = {".git", "target", "node_modules", ".claude"}

# Files allowed to contain the patterns, because stating the rule requires
# naming what it forbids. Kept as an explicit list rather than a marker comment:
# an inline opt-out is one edit away from being sprinkled onto a real leak.
ALLOWLIST = {
    Path(".github/scripts/check_private_refs.py"),
    Path("CLAUDE.md"),
}


def iter_files(root: Path):
    """Yield (path, path-relative-to-root) for every text file worth reading.

    Skip directories are matched against the **relative** path. Matching the
    absolute one silently scans nothing when the checkout itself sits under a
    skipped name — which is exactly what a git worktree under `.claude/` does,
    and a scanner that reports success having read zero files is worse than no
    scanner at all.
    """
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in TEXT_SUFFIXES:
            continue
        rel = path.relative_to(root)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        yield path, rel


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()

    findings: list[str] = []
    scanned = 0

    for path, rel in iter_files(root):
        if rel in ALLOWLIST:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        scanned += 1
        for lineno, line in enumerate(text.splitlines(), start=1):
            for label, pattern, why in PATTERNS:
                match = pattern.search(line)
                if match:
                    findings.append(
                        f"{rel.as_posix()}:{lineno}: [{label}] {match.group(0)!r} — {why}\n"
                        f"    {line.strip()[:160]}"
                    )

    if findings:
        print("Private-material references found in a public repository:\n")
        for finding in findings:
            print(f"  {finding}\n")
        print(
            f"{len(findings)} reference(s) across {scanned} files.\n"
            "See CLAUDE.md section 6. Write the substance, drop the pointer:\n"
            "the regulatory and technical reasoning is nearly always public-safe;\n"
            "the internal record that decided how to read it is not."
        )
        return 1

    print(f"No private-material references found ({scanned} files scanned).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
