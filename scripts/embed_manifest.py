#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""What embed/MANIFEST.json is made of, and whether it was made from them.

The manifest is generated: `emit_embed_manifest.sh` parses each public header
`package_embed.sh` puts under embed/include/ and records the declarations. So
it is a function of its inputs, and it is stale exactly when an input changed
and it was not emitted again. Until this module that was found only in CI,
after the push, by a 311-second gate; the repair was a separate commit, five
of eight manifest commits in October and November 2026.

This module is the one place that says what the inputs are:

* the headers package_embed.sh copies: sce/include/*.h, every directory
  `SCE_BASE_INCLUDE_DIRS` names in sce/sce_base_sources.cmake, and the Forge
  C++ runtime's headers;
* what the emitter puts on clang's include path besides them: QuickJS and
  cpp-httplib from the source tree, nlohmann/json and pugixml through the
  package;
* the tools that turn them into the manifest.

The emitter writes embed/MANIFEST.inputs beside the manifest: the git object id
and path of each input, a submodule as the commit it pins. `check` compares the
inputs at a snapshot, the index or a commit, with the stamp at that snapshot,
which costs no clang and under a second, so a commit that changes an input
without emitting the manifest again is refused at the commit that does it. The
CI lane `embed-vendor` stays the check of the content.

Subcommands:
  include-dirs           the SCE_BASE_INCLUDE_DIRS, one per line (package_embed.sh)
  include-roots          the source-tree include roots the emitter adds (-I)
  stamp                  the stamp of the working tree, on stdout
  check --staged         the index against its stamp (pre-commit)
  check --rev REV        a commit against its stamp (pre-push, CI)
  canon FILE             the manifest without its label fields, as JSON
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SOURCES_CMAKE = "sce/sce_base_sources.cmake"
STAMP = "embed/MANIFEST.inputs"

#: Include roots in the source tree the emitter puts on clang's path: public
#: headers include them, and the package does not carry them (the consumer
#: provides each tier).
INCLUDE_ROOTS = ["third_party/quickjs", "third_party/cpp-httplib"]

#: What the package carries for clang besides the public headers.
PACKAGED_DEPENDENCIES = ["third_party/nlohmann_json/include/nlohmann/json.hpp", "third_party/pugixml"]

#: The tools that turn the inputs into the manifest.
TOOLS = [
    "scripts/package_embed.sh",
    "scripts/emit_embed_manifest.sh",
    "scripts/_embed_manifest_filter.py",
    "scripts/embed_manifest.py",
    SOURCES_CMAKE,
]

#: The fields of the manifest that label an emit rather than describe the
#: headers: the HEAD it ran at, and the compiler. Two emits of the same
#: headers agree on everything else.
LABELS = ["embed_version", "clang_version"]

HEADER = """\
# The inputs embed/MANIFEST.json was emitted from: the git object id and path of
# each file, a submodule as the commit it pins (scripts/embed_manifest.py).
# Written by scripts/emit_embed_manifest.sh; regenerate both with
# `bx --action regen-embed-manifest`. A commit whose inputs differ from these
# is refused (scripts/gates/embed-manifest-stamp.sh).
"""


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(ROOT), *args], check=True, capture_output=True, text=True
    ).stdout


class Snapshot:
    """The tree being judged: the working tree, the index, or a commit."""

    def __init__(self, rev: str | None = None, staged: bool = False) -> None:
        self.rev = rev
        self.staged = staged

    def read(self, path: str) -> str | None:
        """The text of `path` in this snapshot, or `None` where it has none."""
        if self.rev is None and not self.staged:
            file = ROOT / path
            return file.read_text(encoding="utf-8") if file.is_file() else None
        spec = f":{path}" if self.staged else f"{self.rev}:{path}"
        result = subprocess.run(
            ["git", "-C", str(ROOT), "show", spec], capture_output=True, text=True
        )
        return result.stdout if result.returncode == 0 else None

    def entries(self, specs: list[str]) -> list[tuple[str, str, str]]:
        """Each tracked path the pathspecs name, with its mode and object id."""
        if self.rev is not None:
            text = git("ls-tree", "-r", "-z", "--full-tree", self.rev, "--", *specs)
            out = []
            for record in filter(None, text.split("\0")):
                meta, path = record.split("\t", 1)
                mode, _, oid = meta.split(" ")
                out.append((path, mode, oid))
            return out
        text = git("ls-files", "-s", "-z", "--", *specs)
        out = []
        for record in filter(None, text.split("\0")):
            meta, path = record.split("\t", 1)
            mode, oid, _ = meta.split(" ")
            out.append((path, mode, oid))
        return out


def include_dirs(snapshot: Snapshot) -> list[str]:
    """`SCE_BASE_INCLUDE_DIRS` of sce/sce_base_sources.cmake: the directories
    under sce/include/ the package carries."""
    text = snapshot.read(SOURCES_CMAKE)
    if text is None:
        raise SystemExit(f"embed_manifest: {SOURCES_CMAKE} not found")
    block = re.search(r"set\(SCE_BASE_INCLUDE_DIRS\s+(.*?)\)", text, re.S)
    if block is None:
        raise SystemExit(f"embed_manifest: {SOURCES_CMAKE} sets no SCE_BASE_INCLUDE_DIRS")
    lines = (line.split("#", 1)[0].strip() for line in block.group(1).splitlines())
    return [line for line in lines if line]


#: Where the headers package_embed.sh copies come from: the root of
#: sce/include/ (its own headers, not its subdirectories) and the Forge C++
#: runtime's header directory.
PUBLIC_HEADERS = "sce/include"
FORGE_HEADERS = "backends/cpp/forge-runtime/include/sce/forge"


def is_input(path: str, dirs: list[str]) -> bool:
    """Whether `path` is an input of the manifest, as package_embed.sh copies
    headers: `*.h` at the root of sce/include/ and of the Forge runtime's
    header directory, every file of each directory `SCE_BASE_INCLUDE_DIRS`
    names, and the include roots, dependencies and tools by name."""
    for flat in (PUBLIC_HEADERS, FORGE_HEADERS):
        rest = path.removeprefix(flat + "/")
        if rest != path and "/" not in rest and rest.endswith(".h"):
            return True
    if any(path.startswith(f"{PUBLIC_HEADERS}/{d}/") for d in dirs):
        return True
    return path in INCLUDE_ROOTS + PACKAGED_DEPENDENCIES + TOOLS


def submodules(snapshot: Snapshot) -> list[str]:
    """The paths .gitmodules declares submodules at."""
    text = snapshot.read(".gitmodules") or ""
    return re.findall(r"^\s*path\s*=\s*(\S+)\s*$", text, re.M)


def checked_out(path: str) -> str:
    """The commit the submodule at `path` has checked out, which is what an
    emit in this working tree reads."""
    if not (ROOT / path / ".git").exists():
        raise SystemExit(
            f"embed_manifest: submodule {path} is not checked out, so no emit here can have "
            "read it (git submodule update --init)"
        )
    return subprocess.run(
        ["git", "-C", str(ROOT / path), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()


def inputs(snapshot: Snapshot) -> list[tuple[str, str]]:
    """Each input as (object id, path), sorted by path. In a commit or the
    index a submodule is the commit it pins. In the working tree, where the
    emit reads what is on disk, a file is its content as git would store it
    and a submodule the commit it has checked out: an index is not trusted
    for those, since a copy of the tree (a build machine's) need not record
    them as the checkout's does."""
    dirs = include_dirs(snapshot)
    # Literal prefixes, which `ls-files` and `ls-tree` both read alike; the
    # rule itself is `is_input`.
    prefixes = [PUBLIC_HEADERS, FORGE_HEADERS, *INCLUDE_ROOTS, *PACKAGED_DEPENDENCIES, *TOOLS]
    entries = [e for e in snapshot.entries(prefixes) if is_input(e[0], dirs)]
    if snapshot.rev is None and not snapshot.staged:
        held = set(submodules(snapshot))
        files = [path for path, mode, _ in entries if mode != "160000" and path not in held]
        hashed = dict(zip(files, git("hash-object", "--", *files).split())) if files else {}
        entries = [(path, mode, hashed[path]) for path, mode, _ in entries if path in hashed]
        entries += [(path, "160000", checked_out(path)) for path in sorted(held)
                    if is_input(path, dirs)]
    return sorted((oid, path) for path, _, oid in entries)


def stamp_text(entries: list[tuple[str, str]]) -> str:
    return HEADER + "".join(f"{oid} {path}\n" for oid, path in sorted(entries, key=lambda e: e[1]))


def stamped(text: str) -> dict[str, str]:
    """The inputs a stamp records, by path."""
    out = {}
    for line in text.splitlines():
        if line and not line.startswith("#"):
            oid, path = line.split(" ", 1)
            out[path] = oid
    return out


def check(snapshot: Snapshot, where: str) -> int:
    """Whether the stamp at `snapshot` records its inputs: 0 if so, 1 naming
    each input that differs."""
    recorded_text = snapshot.read(STAMP)
    if recorded_text is None:
        print(f"embed-manifest-stamp: {where} has no {STAMP}.", file=sys.stderr)
        recorded: dict[str, str] = {}
    else:
        recorded = stamped(recorded_text)
    current = {path: oid for oid, path in inputs(snapshot)}
    changed = sorted(
        path for path in set(current) | set(recorded) if current.get(path) != recorded.get(path)
    )
    if not changed and recorded_text is not None:
        return 0
    print(
        f"embed-manifest-stamp: {where} changes what embed/MANIFEST.json is emitted from,\n"
        "and the manifest was not emitted again. Each input that differs from its stamp:",
        file=sys.stderr,
    )
    for path in changed:
        print(f"  {path}", file=sys.stderr)
    print(
        "\nRegenerate on a build machine and commit both files with the change:\n"
        "    bx --action regen-embed-manifest\n"
        "    git add embed/MANIFEST.json embed/MANIFEST.inputs",
        file=sys.stderr,
    )
    return 1


def canon(path: str) -> str:
    """The manifest at `path` without its label fields, as canonical JSON."""
    with open(path, encoding="utf-8") as fh:
        manifest = json.load(fh)
    for label in LABELS:
        manifest.pop(label, None)
    return json.dumps(manifest, sort_keys=True, indent=2)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("include-dirs")
    sub.add_parser("include-roots")
    sub.add_parser("stamp")
    checking = sub.add_parser("check")
    which = checking.add_mutually_exclusive_group(required=True)
    which.add_argument("--staged", action="store_true")
    which.add_argument("--rev")
    canonical = sub.add_parser("canon")
    canonical.add_argument("file")
    args = parser.parse_args()

    if args.command == "include-dirs":
        print("\n".join(include_dirs(Snapshot())))
    elif args.command == "include-roots":
        print("\n".join(str(ROOT / root) for root in INCLUDE_ROOTS))
    elif args.command == "stamp":
        sys.stdout.write(stamp_text(inputs(Snapshot())))
    elif args.command == "check":
        if args.staged:
            return check(Snapshot(staged=True), "the staged commit")
        return check(Snapshot(rev=args.rev), args.rev)
    elif args.command == "canon":
        sys.stdout.write(canon(args.file))
    return 0


if __name__ == "__main__":
    sys.exit(main())
