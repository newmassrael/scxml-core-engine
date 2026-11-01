#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Name the paths of a commit range that a downstream consumer would meet.

ADR 0078 §2: a landing that changes what a consumer compiles against, or what a
decode refuses, says how to follow it in `docs/SCE_CODEC_CONSUMER_MIGRATION.md`.
This script answers the first half for a range of commits: it prints each path
whose change a consumer meets, one per line, and nothing when there is none.

What a consumer meets is something it may already use that is gone or changed:
a public declaration REMOVED or rewritten (a `-` line of the diff that declares a
public item) in

- a generated codec golden that already existed (`tests/forge/expected/**`,
  status M): the code a consumer's regeneration produces;
- a codec runtime's own source (`backends/*/forge-runtime/**`, its tests left
  out): the library the generated code links against;
- the codec oracle's reject vectors (`tests/forge/conformance/numerical_reference.json`),
  where a removed `"error"` line is a frame that is now refused differently.

A line that only ADDS is not counted: an addition breaks no consumer. A golden
that is added or removed whole is a fixture of SCE's own tests coming or going,
not a change to code a consumer had. A comment line is not counted either. The
predicates are a tripwire, not a parser: they look at one line at a time, and a
commit that trips them without changing anything a consumer uses says so in its
message (see `consumer-guide-gate.sh`).

Usage: consumer_visible_change.py <base>..<head>
"""

import re
import subprocess
import sys

GUIDE = "docs/SCE_CODEC_CONSUMER_MIGRATION.md"
ORACLE = "tests/forge/conformance/numerical_reference.json"

# One predicate per language: does this source line declare a public item? Each
# asks for a declaring keyword or shape at the start of the line, where a comment
# marker (`//`, `///`, `#`, ` *`) cannot stand, so a comment is never one.
RUST = re.compile(r"\s*pub\s")
GO = re.compile(
    r"(func|type|var|const)\s+(\([^)]*\)\s*)?[A-Z]"  # an exported top-level name
    r"|\t[A-Z]\w*\s+[\w\[\]\*\.]+(\s+`[^`]*`)?\s*$"  # an exported struct field, `Name Type`
)
# A Kotlin declaration at the top level, in a class, or in its companion: at most
# eight columns in. A function's own locals sit deeper and are no API.
KOTLIN = re.compile(
    r"\s{0,8}(public\s+)?((data|sealed|enum|abstract|open|inline|value|companion|override)\s+)*"
    r"(class|object|interface|fun|val|var)\s"
)
PYTHON = re.compile(r"\s*(class|def)\s+[A-Za-z]|\s{4}[a-z]\w*:\s")
C_FAMILY = re.compile(
    r"\s*(typedef|struct|class|enum|union|static inline|#define|template)\b"
    r"|\s*[A-Za-z_][\w:<>,\s*&]*\s\**[A-Za-z_]\w*\s*\("  # a function declaration
    r"|\s*[A-Za-z_][\w:<>]*\s+\**[A-Za-z_]\w*(\[\w*\])?;"  # a member
)
C_KEYWORDS = re.compile(r"\s*(return|if|while|for|switch|else|case|do|goto|sizeof)\b")


def declares_public(path: str, line: str) -> bool:
    if not line.strip():
        return False
    if path.endswith(".rs"):
        return bool(RUST.match(line))
    if path.endswith(".go"):
        return bool(GO.match(line))
    if path.endswith((".kt", ".kts")):
        return bool(KOTLIN.match(line)) and not re.search(r"\b(private|internal)\b", line)
    if path.endswith(".py"):
        return bool(PYTHON.match(line)) and not re.match(r"\s*(class|def)\s+_", line)
    if path.endswith((".h", ".hpp", ".c")):
        if C_KEYWORDS.match(line) or "=" in line:
            return False
        return bool(C_FAMILY.match(line))
    return False


def in_scope(path: str, status: str) -> bool:
    if path.startswith("tests/forge/expected/"):
        return status == "M"
    if re.match(r"backends/[^/]+/forge-runtime/", path):
        return not re.search(r"/(tests?|jvmTest|commonTest|Tests)/|_test\.go$|/test_", path)
    return path == ORACLE


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True
    ).stdout


def visible_paths(rev_range: str) -> list:
    found = []
    for row in git("diff", "--name-status", "--no-renames", rev_range).splitlines():
        status, path = row.split("\t", 1)
        if not in_scope(path, status[:1]):
            continue
        diff = git("diff", "-U0", rev_range, "--", path)
        removed = [l[1:] for l in diff.splitlines() if l.startswith("-") and not l.startswith("---")]
        if path == ORACLE:
            hit = any('"error"' in l for l in removed)
        else:
            hit = any(declares_public(path, l) for l in removed)
        if hit:
            found.append(path)
    return found


def main() -> int:
    if len(sys.argv) != 2 or ".." not in sys.argv[1]:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    for path in visible_paths(sys.argv[1]):
        print(path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
