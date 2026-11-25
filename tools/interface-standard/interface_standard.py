#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Hold SCE's C++ interface to the standard a consumer compiles it at.

ADR 0059 splits the language requirement of every C++ target in two. What a
consumer compiles is the INTERFACE: the public headers under ``sce/include``
and ``backends/cpp/forge-runtime/include``, and every C++ file a generator
writes, which is source compiled in the consumer's own translation units. It is
C++17. What only SCE compiles is the IMPLEMENTATION, and a target may raise
that to C++20 with ``PRIVATE cxx_std_20``. Because one program then holds
translation units of both standards, the interface must also mean the same
thing in both: no header may pick a definition by standard.

A declared standard is a claim until a build holds the code to it, and the
build SCE runs compiles every target at C++20. This tool is the measurement
that turns the claim into a fact. It compiles each piece of the interface on
its own, with ``-Werror``, at every standard it is asked for, and it refuses
the preprocessor guards that would let the interface choose by standard.
``-Werror`` is not decoration: GCC accepts a designated initializer or a
defaulted comparison at ``-std=c++17`` as an extension and only warns, so a
compile without it would pass the very constructs the measurement is for.

Subcommands, in the order ``scripts/gates/cpp-interface-standard.sh`` runs
them:

``guards``
    Refuse an ``#if``, ``#elif``, ``#ifdef`` or ``#ifndef`` that names
    ``__cplusplus`` or a ``__cpp_*`` feature-test macro, in the public headers
    and in the C++ templates. One guard is allowed, with its reason in
    ``GUARD_ALLOWLIST``; an allowlist entry that matches nothing is refused
    too, so an exception cannot outlive the code it excused.

``generation-outputs``
    Read ``ninja -t targets all`` of a configured tree and list the headers
    its custom commands write. Building exactly those runs the generation
    rules (and the few tools they need) without compiling the tree.

``jobs``
    Read the tree's ``compile_commands.json`` and write the compile jobs that
    measure it, as JSON. Every header is compiled alone
    (``-x c++ -include <header> /dev/null``) with the flags of a translation
    unit that carries what it needs; a header no translation unit can vouch
    for is recorded as not measured, by name.

``run``
    Run the jobs at the requested standards in parallel. Print, per kind and
    standard, how many were measured, passed and failed; every failure with
    its first error; and every header that was not measured, with the reason.
    Exit 1 on any failure, and on a count of measured jobs below its floor:
    a run that measured nothing must not read as a run that found nothing.

In shards (the gate's SCE_INTERFACE_STANDARD_SHARD), `shard-plan` runs after
`generation-outputs`, the generation builds only the shard's part, `jobs` is
given the plan, and `verify-shards` holds the shards of one run together:

``shard-plan``
    Plan the whole measurement of the configured tree as N shards, before
    anything is generated, and write the plan and this shard's outputs.

``verify-shards``
    Read every shard's plan, account and outcomes, and judge the run as the
    whole measurement would: one plan, every subject accounted for, the
    floors held over the shards together, no failure.

Standard library only, so it runs wherever the compiler does; the shard plan
reads ninja's graph and spreads its units through `scripts/lib/ctest_shards.py`.
"""

from __future__ import annotations

import argparse
import collections
import concurrent.futures
import functools
import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, FrozenSet, Iterable, Iterator, List, Optional, Sequence, Set, Tuple

# The repository this file lives in: tools/interface-standard/<this file>.
REPO_ROOT = Path(__file__).resolve().parents[2]

# The token every job's `-std=` carries until `run` substitutes a standard.
STD_PLACEHOLDER = "@STD@"

HEADER_SUFFIXES = (".h", ".hpp", ".hh", ".hxx")
# What the generation step builds: headers, and the include fragments a
# generated header may pull in beside them.
GENERATION_SUFFIXES = HEADER_SUFFIXES + (".inc", ".inl", ".ipp")
CXX_SOURCE_SUFFIXES = (".cpp", ".cc", ".cxx", ".c++")
C_SOURCE_SUFFIXES = (".c",)

PUBLIC_HEADER_ROOT = "sce/include"
FORGE_RUNTIME_INCLUDE = "backends/cpp/forge-runtime/include"
FORGE_GOLDENS = "tests/forge/expected"
LIBRARY_SOURCES = "sce/src"
# The C translation units the gate compiles at their own standard.
C_TU_ROOT = "backends"

KIND_TU = "tu"
KIND_HDR = "hdr"
KIND_GEN = "gen"
KIND_FORGE = "forge"
KIND_CTU = "ctu"
KINDS = (KIND_TU, KIND_HDR, KIND_GEN, KIND_FORGE, KIND_CTU)
KIND_TITLES = {
    KIND_TU: "library translation units under sce/src",
    KIND_HDR: "public headers, each compiled alone",
    KIND_GEN: "generated C++ headers, each compiled alone",
    KIND_FORGE: "Forge C++ goldens, each compiled alone",
    KIND_CTU: "C backend translation units, at their own standard",
}

# The fewest jobs of a kind that a run may measure at one standard and still
# be believed. Each sits near half of what a configure with every optional
# dependency measured on 2026-10-27 (about 100 library translation units, 250
# public headers, 670 generated headers, 165 goldens and 325 C translation
# units), so a runner without the mesh SDKs stays above it and a run whose
# generation step or include reader went blind does not. The library floor is
# low because only the targets with a C++17 implementation are compiled at
# C++17 (see `implementation_floors`).
DEFAULT_FLOORS = {KIND_TU: 10, KIND_HDR: 150, KIND_GEN: 300, KIND_FORGE: 100, KIND_CTU: 50}

# One syntax-only compile of the heaviest generated header takes seconds; a
# job still running after this is hung, and is reported as a failure.
JOB_TIMEOUT_S = 600

# Programs a compile command can be prefixed with that are not the compiler.
LAUNCHERS = frozenset({"ccache", "sccache", "distcc", "icecc"})

# The first lines of every file `sce-codegen` writes carry this mark (the
# drift header), in every backend language. It is what tells a generated
# header from a hand-written one without a list of directories.
GENERATOR_MARK = "SCE-GENERATED"
MARK_WINDOW = 400

# Top-level directories whose files are not SCE's interface even when tracked:
# vendored code SCE does not own, cargo's output, and node's. The repository
# side of a population is read from git (`tracked_files`); a walk of the source
# tree (`FileIndex`) also skips hidden directories (`.git`, a checkout's
# worktrees under `.claude/`) and top-level `build`/`build_*` trees.
SKIPPED_SOURCE_DIRS = frozenset({"third_party", "target", "node_modules"})
_BUILD_TREE_NAME = re.compile(r"^build(_.*)?$")

# ── Standards ─────────────────────────────────────────────────────

_STD_YEARS = {
    "98": 1998, "03": 2003, "0x": 2011, "11": 2011, "1y": 2014, "14": 2014,
    "1z": 2017, "17": 2017, "2a": 2020, "20": 2020, "2b": 2023, "23": 2023,
    "2c": 2026, "26": 2026,
}


def std_year(std: str) -> int:
    """The year of a ``-std=`` value: ``c++17`` and ``gnu++1z`` are 2017.

    Raises ``ValueError`` for anything that does not name a C++ standard, so a
    typo on the command line is refused rather than compiled.
    """
    match = re.fullmatch(r"(?:c|gnu)\+\+(\w\w)", std)
    if not match or match.group(1) not in _STD_YEARS:
        raise ValueError(f"not a C++ standard: {std!r}")
    return _STD_YEARS[match.group(1)]


def feature_year(feature: str) -> Optional[int]:
    """The year a CMake ``cxx_std_NN`` compile feature names, or None."""
    match = re.fullmatch(r"cxx_std_(\d\d)", feature)
    if not match:
        return None
    return _STD_YEARS.get(match.group(1))


# ── Walking the trees ─────────────────────────────────────────────


def walk_files(base: Path, *, skip_names: Iterable[str] = (),
               skip_build_trees: bool = False) -> Iterator[Path]:
    """Every file under ``base``, in a stable order.

    Hidden directories are never entered. ``skip_names`` are directory names
    pruned at any depth; ``skip_build_trees`` also prunes ``build`` and
    ``build_*`` directly under ``base``, which is where a checkout keeps the
    trees it has configured.
    """
    skip = frozenset(skip_names)
    for dirpath, dirnames, filenames in os.walk(base):
        at_top = Path(dirpath) == base
        dirnames[:] = sorted(
            d for d in dirnames
            if not d.startswith(".")
            and d not in skip
            and not (at_top and skip_build_trees and _BUILD_TREE_NAME.match(d))
        )
        for name in sorted(filenames):
            yield Path(dirpath) / name


def carries_generator_mark(path: Path) -> bool:
    """Whether ``path`` opens with the drift header ``sce-codegen`` writes."""
    try:
        with open(path, "rb") as handle:
            head = handle.read(MARK_WINDOW)
    except OSError:
        return False
    return GENERATOR_MARK.encode() in head


def display(path: str, root: Path, build_dir: Optional[Path]) -> str:
    """A path as a reader wants it: repository-relative, or under ``<build>/``."""
    for base, prefix in ((build_dir, "<build>/"), (root, "")):
        if base is None:
            continue
        try:
            return prefix + Path(path).relative_to(base).as_posix()
        except ValueError:
            continue
    return path


# ── guards ────────────────────────────────────────────────────────

TEMPLATE_SCOPE = "tools/codegen/templates"
GUARD_SCOPES = (PUBLIC_HEADER_ROOT, FORGE_RUNTIME_INCLUDE, TEMPLATE_SCOPE)
# Template subtrees that emit another language. The C ones carry the
# `#ifdef __cplusplus` / `extern "C"` idiom, which is right for a C header.
NON_CXX_TEMPLATE_TREES = frozenset({"c", "rust", "go", "kotlin", "python", "swift"})

# The only standard-macro guards the interface may carry, each with why.
# Keyed by file and macro, so the same file naming any OTHER standard macro is
# still refused.
GUARD_ALLOWLIST: Dict[Tuple[str, str], str] = {
    ("sce/include/common/JsonText.h", "__cpp_lib_to_chars"): (
        "names a C++17 library feature (201611), so it reports the library and "
        "not the standard: one library gives it the same value in a C++17 and a "
        "C++20 translation unit, which ADR 0059 section 3 allows"
    ),
}

# Fewer files than this means a scope moved or the walk went blind.
MIN_GUARD_FILES = 100

_DIRECTIVE = re.compile(r"^\s*#\s*(if|elif|ifdef|ifndef|elifdef|elifndef)\b(.*)$")
_STANDARD_MACRO = re.compile(r"\b(__cplusplus|__cpp_[A-Za-z0-9_]+)\b")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/")


@dataclass(frozen=True)
class Guard:
    """One preprocessor conditional that names a standard macro."""
    path: str
    line: int
    text: str
    macros: Tuple[str, ...]


def logical_lines(text: str) -> Iterator[Tuple[int, str]]:
    """``(first line number, line)`` with backslash continuations joined.

    A directive split over lines is one directive; reading it a physical line
    at a time would miss a macro on its second line.
    """
    pending: List[str] = []
    start = 0
    for number, line in enumerate(text.splitlines(), 1):
        if not pending:
            start = number
        if line.endswith("\\"):
            pending.append(line[:-1])
            continue
        pending.append(line)
        yield start, " ".join(pending)
        pending = []
    if pending:
        yield start, " ".join(pending)


def standard_macros(line: str) -> Tuple[str, ...]:
    """The standard macros a conditional directive tests; empty otherwise.

    Comments are removed first, so ``#endif // __cplusplus`` and
    ``#if FOO /* not __cplusplus */`` name nothing.
    """
    match = _DIRECTIVE.match(line)
    if not match:
        return ()
    condition = _BLOCK_COMMENT.sub(" ", match.group(2)).split("//", 1)[0]
    return tuple(sorted(set(_STANDARD_MACRO.findall(condition))))


def find_guards(root: Path, files: Iterable[Path]) -> Iterator[Guard]:
    """Every standard-macro conditional in ``files``."""
    for path in files:
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        rel = path.relative_to(root).as_posix()
        for number, line in logical_lines(text):
            macros = standard_macros(line)
            if macros:
                yield Guard(rel, number, line.strip(), macros)


@dataclass
class GuardReport:
    """What `guards` found: refused guards, allowed ones, unused exceptions."""
    files: int
    refused: List[Guard]
    allowed: List[Guard]
    unused_allowances: List[Tuple[str, str]]
    missing_scopes: List[str]


def check_guards(root: Path, allowlist: Dict[Tuple[str, str], str] = GUARD_ALLOWLIST,
                 scopes: Sequence[str] = GUARD_SCOPES) -> GuardReport:
    """Judge every guard in the scopes against the allowlist.

    The scopes are the public headers and the C++ templates; under the
    templates, the subtrees that emit another language are not read.
    """
    missing = [s for s in scopes if not (root / s).is_dir()]
    files = []
    for scope in scopes:
        skip = NON_CXX_TEMPLATE_TREES if scope == TEMPLATE_SCOPE else ()
        files.extend(walk_files(root / scope, skip_names=skip))
    refused: List[Guard] = []
    allowed: List[Guard] = []
    used: Set[Tuple[str, str]] = set()
    for guard in find_guards(root, files):
        keys = [(guard.path, macro) for macro in guard.macros]
        if all(key in allowlist for key in keys):
            allowed.append(guard)
            used.update(keys)
        else:
            refused.append(guard)
    unused = sorted(set(allowlist) - used)
    return GuardReport(len(files), refused, allowed, unused, missing)


def cmd_guards(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    report = check_guards(root)
    status = 0
    for scope in report.missing_scopes:
        print(f"guards: {scope} does not exist, so it was not read", file=sys.stderr)
        status = 1
    if report.files < args.min_files:
        print(f"guards: read only {report.files} file(s), fewer than {args.min_files}; "
              f"the scopes moved or the walk is blind, and a blind scan finds nothing",
              file=sys.stderr)
        status = 1
    for guard in report.refused:
        print(f"{guard.path}:{guard.line}: {guard.text}")
    for path, macro in report.unused_allowances:
        print(f"guards: the allowance for {macro} in {path} matches no directive; "
              f"remove it from GUARD_ALLOWLIST, or an exception outlives what it excused",
              file=sys.stderr)
        status = 1
    if report.refused:
        print(f"guards: {len(report.refused)} directive(s) select by language standard. "
              f"ADR 0059 section 3: the interface reads the same in every standard, so "
              f"write the C++17 form once (a trait checked by static_assert, not a concept).",
              file=sys.stderr)
        status = 1
    print(f"guards: {report.files} file(s) read, {len(report.refused)} refused, "
          f"{len(report.allowed)} allowed", file=sys.stderr)
    return status


# ── generation-outputs ────────────────────────────────────────────


def generation_outputs(targets_text: str, build_dir: Optional[Path] = None) -> List[str]:
    """The generated files a Ninja tree's custom commands write.

    ``targets_text`` is the output of ``ninja -t targets all``: one
    ``<output>: <rule>`` line per output. CMake names every custom command's
    rule ``CUSTOM_COMMAND``. Kept: outputs of that rule with a header or
    include-fragment suffix, inside the build tree. An output in the source
    tree is dropped, because a rule that writes there is a regeneration of
    committed files and not part of compiling this configure.
    """
    found: Set[str] = set()
    for line in targets_text.splitlines():
        path, sep, rule = line.rpartition(": ")
        if not sep or rule.strip() != "CUSTOM_COMMAND":
            continue
        if not path.endswith(GENERATION_SUFFIXES):
            continue
        if os.path.isabs(path):
            if build_dir is None:
                continue
            try:
                path = Path(path).relative_to(build_dir).as_posix()
            except ValueError:
                continue
        if path.startswith("..") or path.startswith("$"):
            continue
        found.add(path)
    return sorted(found)


def cmd_generation_outputs(args: argparse.Namespace) -> int:
    text = Path(args.ninja_targets).read_text(encoding="utf-8", errors="replace")
    build_dir = Path(args.build_dir).resolve() if args.build_dir else None
    outputs = generation_outputs(text, build_dir)
    Path(args.out).write_text("".join(f"{o}\n" for o in outputs), encoding="utf-8")
    print(f"generation-outputs: {len(outputs)} generated file(s) in the build graph",
          file=sys.stderr)
    if not outputs:
        print("generation-outputs: the build graph names no generated header; the "
              "configure registered none, or this is not the output of "
              "`ninja -t targets all`", file=sys.stderr)
        return 1
    return 0


# ── The compile database ──────────────────────────────────────────


@dataclass
class Entry:
    """One translation unit of the configured tree, as the compiler sees it."""
    directory: str
    file: str
    args: List[str]
    target: str
    declared_in: str
    language: str

    @property
    def is_library(self) -> bool:
        """A translation unit of a target SCE declares under ``sce/``.

        The declaring directory comes from the object path, so a test target
        that compiles an ``sce/src`` file into itself is not mistaken for the
        library that owns it.
        """
        return self.declared_in == "sce" or self.declared_in.startswith("sce/")


def language_of(path: str) -> Optional[str]:
    if path.endswith(CXX_SOURCE_SUFFIXES):
        return "c++"
    if path.endswith(C_SOURCE_SUFFIXES):
        return "c"
    return None


def strip_launcher(args: List[str]) -> List[str]:
    """``args`` with a leading ``ccache``-style launcher removed."""
    while len(args) > 1 and os.path.basename(args[0]) in LAUNCHERS:
        args = args[1:]
    return args


@functools.lru_cache(maxsize=None)
def real_compiler(path: str) -> str:
    """The compiler a command names, seen through a launcher's masquerade.

    Debian's ``/usr/lib/ccache/c++`` is a link to ``ccache`` that runs the
    ``c++`` found next on PATH. The jobs name that compiler directly, so a
    syntax-only compile is not routed through a cache that cannot hold it.
    """
    resolved = shutil.which(path) or path
    if os.path.basename(os.path.realpath(resolved)) not in LAUNCHERS:
        return resolved
    name = os.path.basename(path)
    for directory in os.environ.get("PATH", "").split(os.pathsep):
        candidate = os.path.join(directory, name)
        if not (os.path.isfile(candidate) and os.access(candidate, os.X_OK)):
            continue
        if os.path.basename(os.path.realpath(candidate)) in LAUNCHERS:
            continue
        return candidate
    return resolved


def object_path(item: dict, args: List[str], directory: str) -> str:
    """The absolute object file a compile command writes."""
    for i, arg in enumerate(args):
        if arg == "-o" and i + 1 < len(args):
            return os.path.normpath(os.path.join(directory, args[i + 1]))
        if arg.startswith("-o") and len(arg) > 2:
            return os.path.normpath(os.path.join(directory, arg[2:]))
    return os.path.normpath(os.path.join(directory, item.get("output", "")))


def target_of(obj: str, build_dir: Path) -> Tuple[str, str]:
    """``(target, directory that declared it)`` from an object path.

    CMake writes a target's objects under
    ``<declaring dir>/CMakeFiles/<target>.dir/``, with either generator.
    """
    try:
        rel = Path(obj).relative_to(build_dir).as_posix()
    except ValueError:
        rel = obj
    match = re.search(r"(?:^|/)CMakeFiles/([^/]+)\.dir/", rel)
    if not match:
        return "?", ""
    declared = rel[:match.start()].strip("/")
    return match.group(1), declared


def load_entries(compile_commands: Path, build_dir: Path) -> List[Entry]:
    """Every C and C++ translation unit ``compile_commands.json`` records."""
    raw = json.loads(compile_commands.read_text(encoding="utf-8"))
    entries: List[Entry] = []
    for item in raw:
        args = item.get("arguments") or shlex.split(item["command"])
        args = strip_launcher(list(args))
        directory = item["directory"]
        file = os.path.realpath(os.path.join(directory, item["file"]))
        language = language_of(file)
        if language is None:
            continue
        target, declared = target_of(object_path(item, args, directory), build_dir)
        entries.append(Entry(directory, file, args, target, declared, language))
    return entries


# ── Flags ─────────────────────────────────────────────────────────

_DROP_WITH_VALUE = frozenset({"-o", "-MF", "-MT", "-MQ"})
_DROP_ALONE = frozenset({"-c", "-MD", "-MMD", "-MP", "-fsyntax-only"})
_FORCED_INCLUDES = frozenset({"-include", "-imacros"})


def compile_flags(entry: Entry, *, keep_source: bool, keep_forced_includes: bool,
                  substitute_std: bool) -> List[str]:
    """An entry's flags, without its compiler, output or dependency file.

    ``substitute_std`` turns the ``-std=`` into the placeholder `run` fills
    (adding one when the command carried none). ``keep_forced_includes`` is
    false for a header compiled alone: a forced include (`-include
    common/DisableStdOut.h`, which brings `<iostream>`) is a header a consumer
    does not have, and leaving it in is exactly how a header that names
    ``uint64_t`` without ``<cstdint>`` passes here and fails downstream.
    """
    args = entry.args
    out: List[str] = []
    saw_std = False
    i = 1
    while i < len(args):
        arg = args[i]
        if arg in _DROP_WITH_VALUE:
            i += 2
            continue
        if arg.startswith(("-MF", "-MT", "-MQ")) or (arg.startswith("-o") and len(arg) > 2):
            i += 1
            continue
        if arg in _DROP_ALONE or arg.startswith("-gsplit-dwarf"):
            i += 1
            continue
        if arg in _FORCED_INCLUDES:
            if keep_forced_includes and i + 1 < len(args):
                out.extend(args[i:i + 2])
            i += 2
            continue
        if arg.startswith("-std="):
            out.append(f"-std={STD_PLACEHOLDER}" if substitute_std else arg)
            saw_std = True
            i += 1
            continue
        if not arg.startswith("-") and os.path.realpath(
                os.path.join(entry.directory, arg)) == entry.file:
            if keep_source:
                out.append(arg)
            i += 1
            continue
        out.append(arg)
        i += 1
    if substitute_std and not saw_std:
        out.insert(0, f"-std={STD_PLACEHOLDER}")
    return out


@dataclass(frozen=True)
class SearchPath:
    """Where an entry's compiler looks for ``#include "x"`` and ``<x>``."""
    quote: Tuple[str, ...]
    angle: Tuple[str, ...]


_INCLUDE_OPTIONS = (("-iquote", "quote"), ("-isystem", "system"),
                    ("-idirafter", "after"), ("-I", "normal"))


def search_path(entry: Entry) -> SearchPath:
    """The include directories an entry names, in the compiler's order.

    A quoted include searches ``-iquote`` first and then everything an
    angle-bracket include does: ``-I``, ``-isystem``, then ``-idirafter``.
    The compiler's own system directories are left out, because nothing this
    tool follows lives there.
    """
    buckets: Dict[str, List[str]] = {"quote": [], "system": [], "after": [], "normal": []}

    def directory(value: str) -> str:
        return os.path.normpath(os.path.join(entry.directory, value))

    args = entry.args
    i = 1
    while i < len(args):
        arg = args[i]
        for option, bucket in _INCLUDE_OPTIONS:
            if arg == option and i + 1 < len(args):
                buckets[bucket].append(directory(args[i + 1]))
                i += 1
                break
            if arg.startswith(option) and len(arg) > len(option):
                buckets[bucket].append(directory(arg[len(option):]))
                break
        i += 1
    angle = tuple(buckets["normal"] + buckets["system"] + buckets["after"])
    return SearchPath(tuple(buckets["quote"]) + angle, angle)


def forced_includes(entry: Entry) -> List[str]:
    """The files an entry's command includes before its source."""
    out = []
    for i, arg in enumerate(entry.args[:-1]):
        if arg in _FORCED_INCLUDES:
            out.append(entry.args[i + 1])
    return out


# ── Which translation unit includes which header ──────────────────

_INCLUDE_DIRECTIVE = re.compile(r'^\s*#\s*include\s*([<"])([^>"]+)[>"]', re.M)


class IncludeGraph:
    """The headers each translation unit reaches, read from the sources.

    Conditional compilation is not evaluated: a header included under an
    ``#ifdef`` that is off for a translation unit still counts as reached.
    That over-reaches in one direction only, toward flags configured for an
    option that is off, and where it matters it shows as a missing file;
    `run` reports a file this machine does not have as not measured, by name.

    Only files under ``follow`` are opened; vendored code and the compiler's
    own headers are leaves.
    """

    def __init__(self, follow: Sequence[Path], leaves: Sequence[Path] = ()):
        self._follow = tuple(str(p) + os.sep for p in follow)
        self._leaves = tuple(str(p) + os.sep for p in leaves)
        self._includes: Dict[str, List[Tuple[str, str]]] = {}
        self._isfile: Dict[str, bool] = {}
        self._real: Dict[str, str] = {}
        # Search paths are interned: every unit of a target shares one, and a
        # small integer is a cheaper memo key than a tuple of thirty paths.
        self._search_ids: Dict[SearchPath, int] = {}
        self._found: Dict[Tuple[int, str, str], Optional[str]] = {}
        self._children: Dict[Tuple[int, str], Tuple[str, ...]] = {}

    def _search_id(self, search: SearchPath) -> int:
        return self._search_ids.setdefault(search, len(self._search_ids))

    def follows(self, path: str) -> bool:
        return path.startswith(self._follow) and not path.startswith(self._leaves)

    def _is_file(self, path: str) -> bool:
        known = self._isfile.get(path)
        if known is None:
            known = self._isfile[path] = os.path.isfile(path)
        return known

    def _realpath(self, path: str) -> str:
        real = self._real.get(path)
        if real is None:
            real = self._real[path] = os.path.realpath(path)
        return real

    def includes_of(self, path: str) -> List[Tuple[str, str]]:
        known = self._includes.get(path)
        if known is None:
            try:
                with open(path, encoding="utf-8", errors="replace") as handle:
                    text = handle.read()
            except OSError:
                text = ""
            known = self._includes[path] = _INCLUDE_DIRECTIVE.findall(text)
        return known

    def resolve(self, search: SearchPath, kind: str, name: str, includer_dir: str) -> Optional[str]:
        """Where ``#include <kind>name`` from a file in ``includer_dir`` lands."""
        if kind == '"':
            local = os.path.normpath(os.path.join(includer_dir, name))
            if self._is_file(local):
                return self._realpath(local)
        key = (self._search_id(search), kind, name)
        if key in self._found:
            return self._found[key]
        found = None
        for directory in (search.quote if kind == '"' else search.angle):
            candidate = os.path.normpath(os.path.join(directory, name))
            if self._is_file(candidate):
                found = self._realpath(candidate)
                break
        self._found[key] = found
        return found

    def _children_of(self, search: SearchPath, path: str) -> Tuple[str, ...]:
        key = (self._search_id(search), path)
        known = self._children.get(key)
        if known is None:
            directory = os.path.dirname(path)
            resolved = (self.resolve(search, kind, name, directory)
                        for kind, name in self.includes_of(path))
            known = self._children[key] = tuple(p for p in resolved if p)
        return known

    def closure(self, entry: Entry) -> Set[str]:
        """Every file ``entry`` reaches, its own source and forced includes first."""
        search = search_path(entry)
        start = [entry.file]
        for name in forced_includes(entry):
            here = os.path.normpath(os.path.join(entry.directory, name))
            start.append(self._realpath(here) if self._is_file(here)
                         else self.resolve(search, '"', name, entry.directory) or "")
        seen: Set[str] = set()
        stack = [p for p in start if p]
        while stack:
            path = stack.pop()
            if path in seen:
                continue
            seen.add(path)
            if not self.follows(path):
                continue
            stack.extend(c for c in self._children_of(search, path) if c not in seen)
        return seen


# ── jobs ──────────────────────────────────────────────────────────

_CMAKE_FEATURES = re.compile(r"target_compile_features\s*\(([^)]*)\)", re.I | re.S)


def strip_cmake_comments(text: str) -> str:
    """CMake source with ``#`` line comments removed (outside quotes)."""
    out = []
    for line in text.splitlines():
        quoted = False
        for i, char in enumerate(line):
            if char == '"':
                quoted = not quoted
            elif char == "#" and not quoted:
                line = line[:i]
                break
        out.append(line)
    return "\n".join(out)


def implementation_floors(cmake_texts: Iterable[str]) -> Dict[str, int]:
    """The standard each target's own sources are declared to need.

    ADR 0059 section 2: a target whose sources use C++20 says so with
    ``PRIVATE cxx_std_20`` (``PUBLIC`` also applies to its own sources). Its
    translation units are then not held to C++17, because no consumer
    compiles them. A target that declares nothing above C++17 claims a C++17
    implementation, and the gate holds it to that: the vendoring channels
    compile those sources at the consumer's standard.
    """
    floors: Dict[str, int] = {}
    for text in cmake_texts:
        for call in _CMAKE_FEATURES.finditer(strip_cmake_comments(text)):
            tokens = call.group(1).split()
            if not tokens or "${" in tokens[0]:
                continue
            target, scope = tokens[0], None
            for token in tokens[1:]:
                if token in ("PUBLIC", "PRIVATE", "INTERFACE"):
                    scope = token
                    continue
                year = feature_year(token)
                if year is None or scope == "INTERFACE":
                    continue
                floors[target] = max(floors.get(target, 2017), year)
    return floors


def tracked_files(root: Path, *pathspecs: str) -> List[Path]:
    """The files git tracks under ``root`` (matching ``pathspecs``), present on disk.

    The repository side of every population is what is COMMITTED, so a local
    run and a CI checkout measure the same set. Walking the directory instead
    counted 1,711 untracked scratch headers in one checkout on 2026-10-27
    (experiment outputs in an ignored directory, a local ``examples/*/build/``),
    each listed as not measured and each a failure waiting to happen. The
    build tree is not asked here: its generated headers are this configure's.

    ``-z`` so a path with an unusual byte is read whole. ``GIT_*`` variables
    are dropped so a hook's ``GIT_DIR`` cannot point the question at another
    repository. A git that fails is an error, not an empty tree: an empty
    list would read as "nothing committed to measure".
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    proc = subprocess.run(["git", "-C", str(root), "ls-files", "-z", "--", *pathspecs],
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    if proc.returncode != 0:
        raise RuntimeError(f"git could not list the tracked files under {root}: "
                           f"{proc.stderr.decode('utf-8', 'replace').strip()}")
    paths = (root / p.decode("utf-8", "surrogateescape")
             for p in proc.stdout.split(b"\0") if p)
    return [p for p in paths if p.is_file()]


def cmake_sources(root: Path) -> Iterator[str]:
    """The text of every committed CMake file SCE owns."""
    for path in tracked_files(root):
        if path.relative_to(root).parts[0] in SKIPPED_SOURCE_DIRS:
            continue
        if path.name == "CMakeLists.txt" or path.suffix == ".cmake":
            try:
                yield path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue


def public_headers(root: Path) -> List[str]:
    return [os.path.realpath(p) for p in tracked_files(root, PUBLIC_HEADER_ROOT)
            if p.name.endswith(HEADER_SUFFIXES)]


def forge_runtime_headers(root: Path) -> List[str]:
    return [os.path.realpath(p) for p in tracked_files(root, FORGE_RUNTIME_INCLUDE)
            if p.name.endswith(HEADER_SUFFIXES)]


def forge_goldens(root: Path) -> List[str]:
    """The C++ goldens: ``tests/forge/expected/*.h``, not the C ``*.c.h``."""
    base = root / FORGE_GOLDENS
    return sorted(os.path.realpath(p) for p in tracked_files(root, FORGE_GOLDENS)
                  if p.parent == base and p.name.endswith(".h")
                  and not p.name.endswith(".c.h"))


def generated_headers(root: Path, build_dir: Path) -> List[str]:
    """Every header carrying the generator's mark: in the build tree, and committed.

    The build tree is walked, because what this configure generated is the
    population. The repository is asked of git, because only what is
    committed belongs to the interface (see `tracked_files`).
    """
    found: Set[str] = set(tracked_generated_headers(root))
    for path in walk_files(build_dir, skip_names=("CMakeFiles", "_deps")):
        if path.name.endswith(HEADER_SUFFIXES) and carries_generator_mark(path):
            found.add(os.path.realpath(path))
    return sorted(found)


def tracked_generated_headers(root: Path) -> List[str]:
    """The committed headers that carry the generator's mark (see `generated_headers`)."""
    found = []
    for path in tracked_files(root):
        if path.relative_to(root).parts[0] in SKIPPED_SOURCE_DIRS:
            continue
        if path.name.endswith(HEADER_SUFFIXES) and carries_generator_mark(path):
            found.append(os.path.realpath(path))
    return sorted(found)


@dataclass
class Assignment:
    """The translation unit whose flags a header is compiled with, and why."""
    entry: Entry
    how: str  # "included" or "sibling"


@dataclass
class HeaderPlan:
    assigned: Dict[str, Assignment]
    c_only: Set[str]
    unowned: List[str]


def owner_order(entries: Sequence[Entry]) -> List[Entry]:
    """The C++ translation units in the order a header's owner is chosen from:
    library units first (they belong to the target that owns the header), then
    by file and target. A shard plan reads the same order (`plan_shards`)."""
    return sorted((e for e in entries if e.language == "c++"),
                  key=lambda e: (0 if e.is_library else 1, e.file, e.target))


def assign_headers(wanted: Iterable[str], entries: Sequence[Entry],
                   graph: IncludeGraph) -> HeaderPlan:
    """Pair each header with a translation unit that carries the flags it needs.

    First choice, a C++ translation unit that includes the header (directly
    or through other headers), library units first: they belong to the target
    that owns the header. Then, for a header nothing includes, a C++ unit that
    includes a header BESIDE it, which is how a generated machine whose
    consumer only includes its sibling gets its target's flags. A header that
    only C units reach is a C header and outside this measurement. Anything
    else is unowned: no target of this configure carries what it needs.
    """
    wanted_set = set(wanted)
    wanted_dirs = {os.path.dirname(p) for p in wanted_set}
    cxx = owner_order(entries)
    c_units = sorted((e for e in entries if e.language == "c"), key=lambda e: (e.file, e.target))

    assigned: Dict[str, Assignment] = {}
    cxx_dir_owner: Dict[str, Entry] = {}
    for entry in cxx:
        for path in graph.closure(entry):
            if path in wanted_set and path not in assigned:
                assigned[path] = Assignment(entry, "included")
            directory = os.path.dirname(path)
            if directory in wanted_dirs and directory not in cxx_dir_owner:
                cxx_dir_owner[directory] = entry

    remaining = wanted_set - set(assigned)
    c_reached: Set[str] = set()
    c_dirs: Set[str] = set()
    if remaining:
        for entry in c_units:
            for path in graph.closure(entry):
                if path in remaining:
                    c_reached.add(path)
                c_dirs.add(os.path.dirname(path))

    c_only: Set[str] = set()
    unowned: List[str] = []
    for path in sorted(remaining):
        directory = os.path.dirname(path)
        if path in c_reached:
            c_only.add(path)
        elif directory in cxx_dir_owner:
            assigned[path] = Assignment(cxx_dir_owner[directory], "sibling")
        elif directory in c_dirs:
            c_only.add(path)
        else:
            unowned.append(path)
    return HeaderPlan(assigned, c_only, unowned)


def header_alone_argv(compiler: str, flags: List[str], header: str) -> List[str]:
    """Compile ``header`` as the only thing in a translation unit.

    ``-include <header> /dev/null`` rather than the header as the main file,
    so ``#pragma once`` behaves as it does for a consumer. ``-Werror`` comes
    first so that a target's own later ``-Wno-error=...`` still wins.
    """
    return [compiler, "-Werror", *flags, "-fsyntax-only", "-x", "c++",
            "-include", header, os.devnull]


def forge_flags(root: Path) -> List[str]:
    """The flags of ``sce_forge_runtime``'s consumers: its include directory."""
    return [f"-std={STD_PLACEHOLDER}", "-Wall",
            "-I", str(root / FORGE_RUNTIME_INCLUDE), "-I", str(root / FORGE_GOLDENS)]


def build_jobs(root: Path, build_dir: Path, entries: Sequence[Entry],
               floors: Dict[str, int], generated: Sequence[str],
               cxx_override: Optional[str] = None,
               c_override: Optional[str] = None) -> dict:
    """The whole measurement of one configured tree, as a JSON-ready document."""
    graph = IncludeGraph(follow=(root, build_dir),
                         leaves=(root / "third_party", build_dir / "_deps"))
    jobs: List[dict] = []
    not_measured: List[dict] = []

    def compiler_for(entry: Entry) -> str:
        override = cxx_override if entry.language == "c++" else c_override
        return override or real_compiler(entry.args[0])

    def subject(path: str) -> str:
        return display(path, root, build_dir)

    def via(entry: Entry, how: str) -> dict:
        return {"how": how, "tu": subject(entry.file), "target": entry.target}

    cxx_entries = [e for e in entries if e.language == "c++"]
    default_cxx = cxx_override or (
        real_compiler(collections.Counter(e.args[0] for e in cxx_entries).most_common(1)[0][0])
        if cxx_entries else "c++")

    library_root = str(root / LIBRARY_SOURCES) + os.sep
    for entry in sorted(cxx_entries, key=lambda e: (e.file, e.target)):
        if not (entry.is_library and entry.file.startswith(library_root)):
            continue
        flags = compile_flags(entry, keep_source=True, keep_forced_includes=True,
                              substitute_std=True)
        jobs.append({
            "kind": KIND_TU, "subject": subject(entry.file), "target": entry.target,
            "via": via(entry, "own"), "cwd": entry.directory,
            "argv": [compiler_for(entry), "-Werror", *flags, "-fsyntax-only"],
            "floor": floors.get(entry.target, 2017),
        })

    publics = public_headers(root)
    plan = assign_headers(list(publics) + list(generated), entries, graph)
    for kind, population in ((KIND_HDR, publics), (KIND_GEN, generated)):
        for path in population:
            if path in plan.c_only and kind == KIND_GEN:
                # A generated C header: its C translation units compile it.
                continue
            choice = plan.assigned.get(path)
            if choice is None:
                reason = ("only C translation units include it, and the C++ interface is "
                          "measured with the flags of a C++ one"
                          if path in plan.c_only else
                          "no C++ translation unit of this configure includes it or a "
                          "header beside it, so no target carries the flags it needs")
                not_measured.append({"kind": kind, "subject": subject(path), "reason": reason})
                continue
            flags = compile_flags(choice.entry, keep_source=False, keep_forced_includes=False,
                                  substitute_std=True)
            jobs.append({
                "kind": kind, "subject": subject(path), "target": choice.entry.target,
                "via": via(choice.entry, choice.how), "cwd": choice.entry.directory,
                "argv": header_alone_argv(compiler_for(choice.entry), flags, path),
            })

    forge = forge_flags(root)
    for path in forge_runtime_headers(root):
        jobs.append({
            "kind": KIND_HDR, "subject": subject(path), "target": "sce_forge_runtime",
            "via": {"how": "interface", "tu": "", "target": "sce_forge_runtime"},
            "cwd": str(root), "argv": header_alone_argv(default_cxx, forge, path),
        })
    for path in forge_goldens(root):
        jobs.append({
            "kind": KIND_FORGE, "subject": subject(path), "target": "sce_forge_runtime",
            "via": {"how": "interface", "tu": "", "target": "sce_forge_runtime"},
            "cwd": str(root), "argv": header_alone_argv(default_cxx, forge, path),
        })

    c_root = str(root / C_TU_ROOT) + os.sep
    c_entries = [e for e in entries if e.language == "c"]
    for entry in sorted(c_entries, key=lambda e: (e.file, e.target)):
        if not entry.file.startswith(c_root):
            continue
        flags = compile_flags(entry, keep_source=True, keep_forced_includes=True,
                              substitute_std=False)
        jobs.append({
            "kind": KIND_CTU, "subject": subject(entry.file), "target": entry.target,
            "via": via(entry, "own"), "cwd": entry.directory,
            "argv": [compiler_for(entry), *flags, "-fsyntax-only"], "standard": "own",
        })

    return {
        "format": 1,
        "root": str(root),
        "build_dir": str(build_dir),
        "jobs": jobs,
        "not_measured": not_measured,
        "c_headers": len(plan.c_only & set(generated)),
        # Which generated headers those are, by subject: a shard accounts for
        # each header its plan gave it (`filter_for_shard`).
        "c_only": sorted(subject(p) for p in plan.c_only & set(generated)),
    }


def cmd_jobs(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    compile_commands = Path(args.compile_commands).resolve()
    build_dir = Path(args.build_dir).resolve() if args.build_dir else compile_commands.parent
    entries = load_entries(compile_commands, build_dir)
    try:
        floors = implementation_floors(cmake_sources(root))
        generated = generated_headers(root, build_dir)
        doc = build_jobs(root, build_dir, entries, floors, generated,
                         cxx_override=args.cxx, c_override=args.cc)
        if args.plan:
            plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
            doc = filter_for_shard(doc, plan, entries, root, build_dir, generated)
    except RuntimeError as exc:
        print(f"jobs: {exc}", file=sys.stderr)
        return 1
    Path(args.out).write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    if "shard" in doc:
        shard = doc["shard"]
        late = sum(1 for job in doc["jobs"] if job.get("late"))
        print(f"jobs: shard {shard['index']} of {shard['count']} (plan "
              f"{shard['digest'][:16]}) keeps {len(doc['jobs']) - late} job(s) of its own "
              f"and {late} late one(s)", file=sys.stderr)
    counts = collections.Counter(job["kind"] for job in doc["jobs"] if not job.get("late"))
    unmeasured = collections.Counter(item["kind"] for item in doc["not_measured"]
                                     if not item.get("late"))
    print(f"jobs: {len(entries)} translation unit(s) in {compile_commands}", file=sys.stderr)
    for kind in KINDS:
        print(f"  {kind:5} {counts.get(kind, 0):5} job(s), {unmeasured.get(kind, 0):3} "
              f"not measured  ({KIND_TITLES[kind]})", file=sys.stderr)
    print(f"  {doc['c_headers']} generated header(s) are reached only by C translation "
          f"units and are compiled through them", file=sys.stderr)
    faults = sorted(key for key, status in doc.get("accounting", {}).items()
                    if status in ACCOUNTING_FAULTS)
    for key in faults:
        print(f"jobs: {key} is planned in this shard and is {doc['accounting'][key]}",
              file=sys.stderr)
    return 1 if faults else 0


# ── run ───────────────────────────────────────────────────────────

# `error:` with its colon, so a path such as `common/error.h:12` in an
# "In file included from" line is not mistaken for the diagnostic.
_FIRST_ERROR = re.compile(r"^.*\b(?:fatal error|error):.*$", re.M)
_MISSING_FILE = re.compile(r"fatal error: ([^\n:]+): No such file or directory")


class FileIndex:
    """Which file names exist anywhere SCE's build could find them.

    Built only when a compile reports a missing file, because only then is the
    question asked: is the file absent from this machine (an optional
    dependency not installed here), or present and out of the flags' reach?
    """

    def __init__(self, bases: Sequence[Path]):
        self._bases = tuple(bases)
        self._by_name: Optional[Dict[str, List[str]]] = None

    def _build(self) -> Dict[str, List[str]]:
        by_name: Dict[str, List[str]] = collections.defaultdict(list)
        for base in self._bases:
            if not base.is_dir():
                continue
            for path in walk_files(base, skip_names=("target", "node_modules"),
                                   skip_build_trees=True):
                by_name[path.name].append(path.as_posix())
        return by_name

    def provides(self, name: str) -> bool:
        if os.path.isabs(name):
            return os.path.isfile(name)
        if self._by_name is None:
            self._by_name = self._build()
        wanted = "/" + os.path.normpath(name).replace(os.sep, "/")
        return any(p.endswith(wanted) for p in self._by_name.get(os.path.basename(name), ()))


@dataclass
class Outcome:
    job: dict
    std: Optional[str]
    verdict: str  # "pass", "fail" or "unmeasured"
    detail: str = ""


def first_error(output: str) -> str:
    """The first line of compiler output that reports an error."""
    match = _FIRST_ERROR.search(output)
    if match:
        found = match.group(0)
    else:
        found = next((text for text in output.splitlines() if text.strip()), "(no output)")
    return found.strip()[:400]


def generated_elsewhere(name: str, unbuilt: Sequence[str]) -> bool:
    """Whether a missing `name` is a header the shard plan generates in another
    shard (`unbuilt`, build-relative), spelled as an include or absolutely."""
    wanted = os.path.normpath(name).replace(os.sep, "/")
    for rel in unbuilt:
        if rel == wanted or rel.endswith("/" + wanted) or wanted.endswith("/" + rel):
            return True
    return False


def classify(job: dict, std: Optional[str], returncode: int, output: str,
             index: FileIndex, unbuilt: Sequence[str] = ()) -> Outcome:
    """Pass, fail, or not measured, from one compile's status and output.

    A compile whose first error is a missing file says nothing about the
    standard when the file is a dependency this machine lacks (the mesh SDKs
    on a runner without them), so that is "not measured", named. When the
    flags were borrowed from a translation unit that includes a header BESIDE
    this one, any missing file is the borrowing's limit and not the header's
    defect. A missing file the tree does hold, under flags of a unit that
    includes the header, is a failure.
    """
    if returncode == 0:
        return Outcome(job, std, "pass")
    error = first_error(output)
    missing = _MISSING_FILE.search(error)
    if missing:
        name = missing.group(1).strip()
        # In a shard, a header the plan generates elsewhere is the plan's
        # defect: it split what this compile joins. Read as "not measured" it
        # would drop the header from the measurement without a word.
        if generated_elsewhere(name, unbuilt):
            return Outcome(job, std, "fail",
                           f"needs {name}, which the shard plan generates in another shard: "
                           f"the plan split what this compile joins ({error})")
        borrowed = job.get("via", {}).get("how") == "sibling"
        if borrowed:
            return Outcome(job, std, "unmeasured",
                           f"the flags of {job['via']['tu']} (which includes a header beside "
                           f"this one) do not reach {name}")
        if not index.provides(name):
            return Outcome(job, std, "unmeasured",
                           f"needs {name}, which neither this machine nor the build tree provides")
    return Outcome(job, std, "fail", error)


def execute(job: dict, std: Optional[str], index: FileIndex,
            unbuilt: Sequence[str] = ()) -> Outcome:
    argv = [a.replace(STD_PLACEHOLDER, std) for a in job["argv"]] if std else list(job["argv"])
    try:
        proc = subprocess.run(argv, cwd=job["cwd"], stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, errors="replace",
                              timeout=JOB_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return Outcome(job, std, "fail", f"timed out after {JOB_TIMEOUT_S}s")
    except OSError as exc:
        return Outcome(job, std, "fail", f"could not start {argv[0]}: {exc}")
    return classify(job, std, proc.returncode, proc.stdout, index, unbuilt)


Run = Tuple[dict, Optional[str]]
Excused = Dict[Tuple[str, str], List[dict]]


def plan_runs(jobs: Sequence[dict], stds: Sequence[str]) -> Tuple[List[Run], Excused]:
    """``(job, standard)`` pairs to run, and the pairs a declaration excuses.

    A library unit of a target with a C++20 implementation is not compiled at
    C++17; it is counted as not applicable, per standard, so the summary says
    how many were excused and by whom.
    """
    runs: List[Run] = []
    excused: Excused = collections.defaultdict(list)
    for job in jobs:
        if job.get("standard") == "own":
            runs.append((job, None))
            continue
        floor = job.get("floor")
        for std in stds:
            if floor is not None and std_year(std) < floor:
                excused[(job["kind"], std)].append(job)
                continue
            runs.append((job, std))
    return runs, excused


def parse_floors(specs: Sequence[str]) -> Dict[str, int]:
    """``DEFAULT_FLOORS`` with ``kind=N`` overrides applied."""
    floors = dict(DEFAULT_FLOORS)
    for spec in specs:
        kind, sep, value = spec.partition("=")
        if not sep or kind not in KINDS or not value.isdigit():
            raise ValueError(f"--floor takes <kind>=<count>, the kind one of "
                             f"{', '.join(KINDS)}; got {spec!r}")
        floors[kind] = int(value)
    return floors


@dataclass
class Tally:
    passed: int = 0
    failed: int = 0
    unmeasured: int = 0

    @property
    def measured(self) -> int:
        return self.passed + self.failed


def tally(outcomes: Sequence[Outcome]) -> Dict[Tuple[str, str], Tally]:
    """Counts per ``(kind, standard)``; a job at its own standard reads ``own``."""
    table: Dict[Tuple[str, str], Tally] = collections.defaultdict(Tally)
    for outcome in outcomes:
        row = table[(outcome.job["kind"], outcome.std or "own")]
        if outcome.verdict == "pass":
            row.passed += 1
        elif outcome.verdict == "fail":
            row.failed += 1
        else:
            row.unmeasured += 1
    return table


def floor_breaches(table: Dict[Tuple[str, str], Tally], stds: Sequence[str],
                   floors: Dict[str, int]) -> List[str]:
    """Every ``(kind, standard)`` that measured fewer jobs than its floor."""
    breaches = []
    for kind in KINDS:
        columns = ["own"] if kind == KIND_CTU else list(stds)
        for std in columns:
            measured = table.get((kind, std), Tally()).measured
            if measured < floors[kind]:
                breaches.append(
                    f"{kind} at {std}: measured {measured}, below the floor of {floors[kind]} "
                    f"({KIND_TITLES[kind]}). A run that compiled this little is not "
                    f"evidence the interface holds; find why the population shrank.")
    return breaches


def compiler_identity(argv0: str) -> str:
    try:
        proc = subprocess.run([argv0, "-dumpfullversion"], stdout=subprocess.PIPE,
                              stderr=subprocess.DEVNULL, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return f"{argv0} (did not report a version)"
    return f"{argv0} {proc.stdout.strip() or '(no version)'}"


def scaled_floors(floors: Dict[str, int], counts: Dict[str, int],
                  totals: Dict[str, int]) -> Dict[str, int]:
    """A shard's floors: each kind's floor in the share of that kind's planned
    population the shard holds, rounded down. The verdict across the shards
    holds the whole floors (`verify-shards`); a shard run alone, as a landing
    runs it, still refuses a part that measured far below its share."""
    return {kind: (floors[kind] * counts.get(kind, 0)) // totals[kind] if totals.get(kind) else 0
            for kind in KINDS}


def outcome_record(outcome: Outcome) -> dict:
    job = outcome.job
    return {"key": job.get("key") or job_key(job), "kind": job["kind"],
            "subject": job["subject"], "target": job["target"],
            "standard": outcome.std or "own", "verdict": outcome.verdict,
            "detail": outcome.detail, "how": job.get("via", {}).get("how", ""),
            "tu": job.get("via", {}).get("tu", ""), "late": job.get("late")}


def print_table(table: Dict[Tuple[str, str], Tally], stds: Sequence[str],
                unowned: Dict[str, int], excused: Excused) -> None:
    print(f"\n{'kind':6}{'standard':10}{'measured':>9}{'passed':>8}{'failed':>8}"
          f"{'not measured':>14}")
    for kind in KINDS:
        columns = ["own"] if kind == KIND_CTU else stds
        for std in columns:
            row = table.get((kind, std), Tally())
            unmeasured = row.unmeasured + (unowned.get(kind, 0) if std != "own" else 0)
            note = ""
            if excused.get((kind, std)):
                targets = sorted({job["target"] for job in excused[(kind, std)]})
                note = (f"  ({len(excused[(kind, std)])} not applicable: "
                        f"{', '.join(targets)} declare an implementation above {std})")
            print(f"{kind:6}{std:10}{row.measured:9}{row.passed:8}{row.failed:8}"
                  f"{unmeasured:14}{note}")


def print_failures(failures: Sequence[Outcome]) -> None:
    if not failures:
        return
    print(f"\nFAILED ({len(failures)}):")
    for outcome in failures:
        job = outcome.job
        borrowed = job["via"].get("how") in ("included", "sibling")
        source = f", flags of {job['via']['tu']}" if borrowed else ""
        print(f"  [{job['kind']} {outcome.std or 'own'}] {job['subject']}  "
              f"(target {job['target']}{source})")
        print(f"      {outcome.detail}")


def cmd_run(args: argparse.Namespace) -> int:
    stds = args.std or ["c++17", "c++20"]
    try:
        for std in stds:
            std_year(std)
        floors = parse_floors(args.floor or [])
    except ValueError as exc:
        print(f"run: {exc}", file=sys.stderr)
        return 2
    doc = json.loads(Path(args.jobs).read_text(encoding="utf-8"))
    root = Path(doc["root"])
    build_dir = Path(doc["build_dir"])
    index = FileIndex((root, build_dir))
    # A shard's part (`jobs --plan`): its floors are its share of the whole,
    # a header the plan generates in another shard is the plan's defect, and
    # a late header is compiled here and judged across the shards.
    shard = doc.get("shard")
    if shard:
        floors = scaled_floors(floors, shard.get("counts", {}), shard.get("totals", {}))
    unbuilt = tuple(doc.get("unbuilt", ()))

    for compiler in sorted({job["argv"][0] for job in doc["jobs"]}):
        print(f"run: compiler {compiler_identity(compiler)}", file=sys.stderr)

    runs, excused = plan_runs(doc["jobs"], stds)
    print(f"run: {len(runs)} compile(s), {args.parallel} at a time", file=sys.stderr)
    every: List[Outcome] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = [pool.submit(execute, job, std, index, unbuilt) for job, std in runs]
        for done, future in enumerate(concurrent.futures.as_completed(futures), 1):
            every.append(future.result())
            if done % 250 == 0:
                print(f"run: {done}/{len(runs)}", file=sys.stderr)
    every.sort(key=lambda o: (KINDS.index(o.job["kind"]), o.job["subject"], o.std or ""))
    late = [o for o in every if o.job.get("late")]
    outcomes = [o for o in every if not o.job.get("late")]

    table = tally(outcomes)
    own_not_measured = [item for item in doc["not_measured"] if not item.get("late")]
    unowned = collections.Counter(item["kind"] for item in own_not_measured)
    print_table(table, stds, unowned, excused)

    failures = [o for o in outcomes if o.verdict == "fail"]
    print_failures(failures)

    unmeasured_runs: Dict[Tuple[str, str, str], List[str]] = collections.defaultdict(list)
    for outcome in outcomes:
        if outcome.verdict == "unmeasured":
            key = (outcome.job["kind"], outcome.job["subject"], outcome.detail)
            unmeasured_runs[key].append(outcome.std or "own")
    total_unmeasured = len(own_not_measured) + len(unmeasured_runs)
    if total_unmeasured:
        print(f"\nNOT MEASURED ({total_unmeasured}), neither passed nor failed:")
        for item in own_not_measured:
            print(f"  [{item['kind']}] {item['subject']}: {item['reason']}")
        for (kind, subject, reason), standards in sorted(unmeasured_runs.items()):
            print(f"  [{kind}] {subject}: {reason} (at {', '.join(standards)})")

    if late:
        print(f"\nLATE ({len(late)} compile(s)): headers no translation unit includes "
              f"before generation, compiled in every shard that can own them; the "
              f"verdict across the shards counts the one the whole run would:")
        for outcome in late:
            job = outcome.job
            detail = f": {outcome.detail}" if outcome.detail else ""
            print(f"  [{job['kind']} {outcome.std or 'own'}] {job['subject']} "
                  f"{outcome.verdict}{detail}  ({job['via'].get('how', '')}, flags of "
                  f"{job['via'].get('tu', '')})")

    breaches = floor_breaches(table, stds, floors)
    for breach in breaches:
        print(f"\nFLOOR: {breach}")

    if args.results:
        Path(args.results).write_text(json.dumps(
            {"stds": list(stds), "outcomes": [outcome_record(o) for o in every],
             "not_measured": doc["not_measured"]}, indent=1) + "\n", encoding="utf-8")

    passed = sum(1 for o in outcomes if o.verdict == "pass")
    print(f"\nrun: {passed} passed, {len(failures)} failed, {total_unmeasured} not measured, "
          f"{len(breaches)} floor breach(es)", file=sys.stderr)
    return 1 if failures or breaches else 0


# ── Shards ────────────────────────────────────────────────────────
#
# The measurement split over N jobs (ADR 0120, D3), each generating only what
# its own part compiles. Run whole, the generation rules took 984 s and the
# compiles 1252 s of a 2811 s job (job 138978378267, 57602a3bb8), beside a
# configure and a generator build every job pays alike; splitting the compiles
# alone would leave every job paying the whole generation.
#
# WHAT A SHARD OWNS is read from the configured tree before anything is
# generated, the same way in every job, so every job holds the whole plan:
#   * a library translation unit, and a C translation unit, are each their own
#     subject;
#   * a generation family is the planned headers one build couples: a header
#     and every header whose rule runs on a file that rule writes (a child
#     machine generated from the document its parent's generation wrote). Steps
#     that write no planned header (a staged document, a stamp) couple nothing;
#   * a public header, and a generated header that exists before the generation
#     rules run, goes with the first translation unit in owner order
#     (`owner_order`) whose include graph reaches it, read with every planned
#     header present as a file that includes nothing (`PlannedIncludeGraph`);
#   * a family goes with the first C++ unit that reaches one of its headers so,
#     else the first C unit, else the first C++ unit that reaches a file beside
#     one of them (the owner `assign_headers` would borrow flags from), else a
#     unit of its own;
#   * a Forge golden and a Forge runtime header are each their own subject.
# A source header no unit reaches before generation can only be reached through
# a generated header. It is LATE: every shard compiles it if it can own it, and
# `verify-shards` counts the outcome of the shard whose owner the whole run
# would have chosen, which is the earliest unit in owner order that reaches it,
# an including unit before a sibling's.
#
# WHAT A SHARD BUILDS is the families its subjects need (a translation unit's
# reach, a header's reach under its owner's flags, an owned family) and the
# families the library reaches, which every compile of the interface needs.
#
# HOW THE UNITS ARE SPREAD is `scripts/lib/ctest_shards.py`'s rule over
# unions: a unit's cost is its compiles and its generation steps, a step
# weighed against a compile by the measured ratio above over this plan's own
# counts, and a step two units share is counted once in a shard holding both.
#
# WHAT A SHARD MAY NOT DO is lose a subject. The plan refuses unless its shards
# are every subject once; a shard accounts for each subject it was given
# (`filter_for_shard`) and fails on one it cannot; a compile that misses a
# header the plan generates elsewhere is a failure, not "not measured"; and
# `verify-shards` holds the shards of one run to one plan, every subject to
# its shard's account, and the floors to the whole.

SHARD_FORMAT = 1
GENERATION_SECONDS = 984
COMPILE_SECONDS = 1252
CUSTOM_COMMAND = "CUSTOM_COMMAND"
BUILD_PREFIX = "<build>/"

# What a shard's account of a subject it was given may say. The last two are
# faults: a planned header the generation did not write, or a subject no
# category of the measurement holds.
ACCOUNTING_FAULTS = frozenset({"absent", "unaccounted"})


def ctest_shards_module():
    """`scripts/lib/ctest_shards.py`: its reading of ninja's graph and its spreading
    of units over shards, which the ctest lanes and this gate share."""
    lib = str(REPO_ROOT / "scripts" / "lib")
    if lib not in sys.path:
        sys.path.insert(0, lib)
    import ctest_shards  # noqa: E402  (the path above is what finds it)
    return ctest_shards


def shard_spec(text: str) -> Tuple[int, int]:
    return ctest_shards_module().shard_arg(text)


def job_key(job: dict) -> str:
    """The subject a job measures, as a shard plan names it: a translation unit
    with its target (one file can be compiled into two), a header by path."""
    if job["kind"] in (KIND_TU, KIND_CTU):
        return f"{job['kind']}:{job['subject']}@{job['target']}"
    return f"{job['kind']}:{job['subject']}"


class PlannedIncludeGraph(IncludeGraph):
    """An include graph in which every header the generation rules will write is
    present already, as a file that includes nothing.

    Before generation it says which translation unit reaches a planned header
    through the files that exist; what a generated header includes is known
    only once it is written, which is why a family is generated whole.
    """

    def __init__(self, follow: Sequence[Path], leaves: Sequence[Path],
                 planned: Iterable[str]):
        super().__init__(follow, leaves)
        self._planned = frozenset(planned)

    def _is_file(self, path: str) -> bool:
        return path in self._planned or super()._is_file(path)

    def _realpath(self, path: str) -> str:
        return path if path in self._planned else super()._realpath(path)

    def reach(self, entry: Entry, start: str) -> Set[str]:
        """Every file `start` reaches under `entry`'s search path, itself included."""
        search = search_path(entry)
        seen: Set[str] = set()
        stack = [start]
        while stack:
            path = stack.pop()
            if path in seen:
                continue
            seen.add(path)
            if not self.follows(path):
                continue
            stack.extend(c for c in self._children_of(search, path) if c not in seen)
        return seen


def generation_families(outputs: Sequence[str], graph,
                        build_dir: Path) -> Tuple[Dict[str, str], Dict[str, FrozenSet[str]]]:
    """Each planned output's family, and each family's custom-command steps.

    Two outputs are one family when the steps that build one include a step
    that writes a planned output and builds the other too. A family is named by
    its least output.
    """
    def spelled(rel: str) -> Optional[str]:
        for spelling in (rel, str(build_dir / rel)):
            if spelling in graph.node_of:
                return spelling
        return None

    steps: Dict[str, FrozenSet[str]] = {}
    writers: Set[str] = set()
    for rel in outputs:
        name = spelled(rel)
        if name is None:
            steps[rel] = frozenset()
            continue
        steps[rel] = graph.steps(name, lambda rule: rule == CUSTOM_COMMAND)
        writer = graph.made_by.get(graph.node_of[name])
        if writer:
            writers.add(writer)

    parent = {rel: rel for rel in outputs}

    def find(rel: str) -> str:
        while parent[rel] != rel:
            parent[rel] = parent[parent[rel]]
            rel = parent[rel]
        return rel

    holder: Dict[str, str] = {}
    for rel in sorted(outputs):
        for step in sorted(steps[rel] & writers):
            if step in holder:
                a, b = find(rel), find(holder[step])
                if a != b:
                    parent[max(a, b)] = min(a, b)
            else:
                holder[step] = rel
    family_of = {rel: find(rel) for rel in outputs}
    family_steps: Dict[str, Set[str]] = collections.defaultdict(set)
    for rel in outputs:
        family_steps[family_of[rel]] |= steps[rel]
    return family_of, {family: frozenset(s) for family, s in family_steps.items()}


def plan_shards(root: Path, build_dir: Path, entries: Sequence[Entry],
                outputs: Sequence[str], graph, count: int,
                stds: Sequence[str] = ("c++17", "c++20"),
                floors: Optional[Dict[str, int]] = None) -> dict:
    """The whole shard plan of one configured tree, as a JSON-ready document."""
    if count < 1:
        raise RuntimeError(f"a plan of {count} shard(s)")
    shards_lib = ctest_shards_module()
    floors = floors or {}

    def subject(path: str) -> str:
        return display(path, root, build_dir)

    def spellings(rel: str) -> Tuple[str, str]:
        joined = os.path.join(str(build_dir), rel)
        return os.path.normpath(joined), os.path.realpath(joined)

    planned: Dict[str, str] = {}
    for rel in outputs:
        for spelling in spellings(rel):
            planned[spelling] = rel
    includes = PlannedIncludeGraph(follow=(root, build_dir),
                                   leaves=(root / "third_party", build_dir / "_deps"),
                                   planned=planned)
    family_of, family_steps = generation_families(outputs, graph, build_dir)
    members: Dict[str, List[str]] = collections.defaultdict(list)
    for rel in sorted(outputs):
        members[family_of[rel]].append(rel)

    cxx = owner_order(entries)
    c_units = sorted((e for e in entries if e.language == "c"), key=lambda e: (e.file, e.target))
    ordered = cxx + c_units
    reach = [includes.closure(e) for e in ordered]

    def families_in(paths: Iterable[str]) -> Set[str]:
        return {family_of[planned[p]] for p in paths if p in planned}

    first_cxx: Dict[str, int] = {}
    first_cxx_rel: Dict[str, int] = {}
    first_cxx_dir: Dict[str, int] = {}
    first_c_rel: Dict[str, int] = {}
    for i, paths in enumerate(reach):
        for path in paths:
            if i < len(cxx):
                first_cxx.setdefault(path, i)
                first_cxx_dir.setdefault(os.path.dirname(path), i)
                if path in planned:
                    first_cxx_rel.setdefault(planned[path], i)
            elif path in planned:
                first_c_rel.setdefault(planned[path], i)

    always: Set[str] = set()
    for i, entry in enumerate(cxx):
        if entry.is_library:
            always |= families_in(reach[i])

    units: Dict[str, dict] = {}

    def unit(name: str) -> dict:
        return units.setdefault(name, {"subjects": set(), "families": set(), "compiles": 0})

    def entry_unit(i: int) -> dict:
        return unit(f"{subject(ordered[i].file)}@{ordered[i].target}")

    def compiles_at(target: str) -> int:
        return sum(1 for std in stds if std_year(std) >= floors.get(target, 2017))

    floor_keys: Set[str] = set()
    library_root = str(root / LIBRARY_SOURCES) + os.sep
    c_root = str(root / C_TU_ROOT) + os.sep
    for i, entry in enumerate(ordered):
        if entry.language == "c++" and entry.is_library and entry.file.startswith(library_root):
            kind, compiles = KIND_TU, compiles_at(entry.target)
        elif entry.language == "c" and entry.file.startswith(c_root):
            kind, compiles = KIND_CTU, 1
        else:
            continue
        held = entry_unit(i)
        key = f"{kind}:{subject(entry.file)}@{entry.target}"
        held["subjects"].add(key)
        held["families"] |= families_in(reach[i])
        held["compiles"] += compiles
        floor_keys.add(key)

    late: List[str] = []
    sources = [(KIND_HDR, p) for p in public_headers(root)]
    sources += [(KIND_GEN, p) for p in generated_headers(root, build_dir) if p not in planned]
    for kind, path in sources:
        key = f"{kind}:{subject(path)}"
        owner = first_cxx.get(path)
        if owner is None:
            late.append(key)
            continue
        held = entry_unit(owner)
        held["subjects"].add(key)
        held["families"] |= families_in(includes.reach(cxx[owner], path))
        held["compiles"] += len(stds)
        floor_keys.add(key)

    for family in sorted(members):
        rels = members[family]
        owner: Optional[int] = None
        measured_by_cxx = True
        hits = [first_cxx_rel[rel] for rel in rels if rel in first_cxx_rel]
        if hits:
            owner = min(hits)
        else:
            c_hits = [first_c_rel[rel] for rel in rels if rel in first_c_rel]
            if c_hits:
                owner, measured_by_cxx = min(c_hits), False
            else:
                dirs = {os.path.dirname(s) for rel in rels for s in spellings(rel)}
                d_hits = [first_cxx_dir[d] for d in dirs if d in first_cxx_dir]
                if d_hits:
                    owner = min(d_hits)
                else:
                    measured_by_cxx = False
        held = entry_unit(owner) if owner is not None else unit(f"family:{family}")
        held["families"].add(family)
        for rel in rels:
            key = f"{KIND_GEN}:{BUILD_PREFIX}{rel}"
            held["subjects"].add(key)
            if rel.endswith(HEADER_SUFFIXES):
                held["compiles"] += len(stds)
                if measured_by_cxx:
                    floor_keys.add(key)

    for path in forge_runtime_headers(root):
        key = f"{KIND_HDR}:{subject(path)}"
        held = unit(key)
        held["subjects"].add(key)
        held["compiles"] += len(stds)
        floor_keys.add(key)
    for path in forge_goldens(root):
        key = f"{KIND_FORGE}:{subject(path)}"
        held = unit(key)
        held["subjects"].add(key)
        held["compiles"] += len(stds)
        floor_keys.add(key)

    def steps_of(families: Iterable[str]) -> Set[str]:
        found: Set[str] = set()
        for family in families:
            found |= family_steps.get(family, frozenset())
        return found

    always_steps = steps_of(always)
    every_step = steps_of(family_steps) - always_steps
    compiles_total = sum(u["compiles"] for u in units.values())
    weight = max(1, round(GENERATION_SECONDS * compiles_total
                          / (COMPILE_SECONDS * max(1, len(every_step)))))

    items = []
    held_by: Dict[str, dict] = {}
    for name in sorted(units):
        data = units[name]
        item = shards_lib.Unit(sorted(data["subjects"]))
        tokens = {f"c:{name}#{n}" for n in range(data["compiles"])}
        tokens |= {f"g:{step}#{n}" for step in steps_of(data["families"]) - always_steps
                   for n in range(weight)}
        item.closure = frozenset(tokens)
        items.append(item)
        held_by[item.name] = data

    spread = []
    for part in shards_lib.assign(items, count):
        subjects = sorted(k for item in part for k in item.tests)
        families = set(always)
        for item in part:
            families |= held_by[item.name]["families"]
        cost = len(frozenset().union(*(item.closure for item in part))) if part else 0
        counts = collections.Counter(k.split(":", 1)[0] for k in subjects if k in floor_keys)
        spread.append({"subjects": subjects,
                       "outputs": sorted(rel for f in families for rel in members.get(f, ())),
                       "cost": cost, "counts": {kind: counts.get(kind, 0) for kind in KINDS}})

    everything = sorted(k for data in units.values() for k in data["subjects"])
    assigned = sorted(k for part in spread for k in part["subjects"])
    if assigned != everything or len(set(assigned)) != len(assigned) or set(late) & set(assigned):
        raise RuntimeError("the shards are not every subject of the plan, each once")
    totals = collections.Counter(k.split(":", 1)[0] for k in everything if k in floor_keys)
    shape = {"count": count, "stds": list(stds), "late": sorted(late),
             "shards": [part["subjects"] for part in spread],
             "outputs": [part["outputs"] for part in spread]}
    digest = hashlib.sha256(json.dumps(shape, sort_keys=True).encode()).hexdigest()
    return {"format": SHARD_FORMAT, "count": count, "digest": digest, "stds": list(stds),
            "weight": weight, "steps": len(every_step), "subjects": len(everything),
            "late": sorted(late), "totals": {kind: totals.get(kind, 0) for kind in KINDS},
            "shards": spread}


def describe_plan(plan: dict, index: int) -> str:
    lines = [f"plan {plan['digest'][:16]}: {plan['subjects']} subject(s) and "
             f"{len(plan['late'])} late header(s) in {plan['count']} shard(s); "
             f"{plan['steps']} generation step(s), each weighed as {plan['weight']} compile(s)"]
    for i, part in enumerate(plan["shards"], 1):
        mark = "  <- this job" if i == index else ""
        lines.append(f"  shard {i}: {len(part['subjects'])} subject(s), "
                     f"{len(part['outputs'])} generated file(s), cost {part['cost']}{mark}")
    return "\n".join(lines)


def cmd_shard_plan(args: argparse.Namespace) -> int:
    root = Path(args.root).resolve()
    compile_commands = Path(args.compile_commands).resolve()
    build_dir = Path(args.build_dir).resolve() if args.build_dir else compile_commands.parent
    index, count = args.shard
    entries = load_entries(compile_commands, build_dir)
    outputs = [line.strip() for line in
               Path(args.generation).read_text(encoding="utf-8").splitlines() if line.strip()]
    graph = ctest_shards_module().Graph(
        Path(args.ninja_graph).read_text(encoding="utf-8", errors="replace"))
    try:
        floors = implementation_floors(cmake_sources(root))
        plan = plan_shards(root, build_dir, entries, outputs, graph, count,
                           stds=args.std or ["c++17", "c++20"], floors=floors)
    except RuntimeError as exc:
        print(f"shard-plan: {exc}", file=sys.stderr)
        return 1
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "plan.json").write_text(json.dumps({**plan, "shard": index}, indent=1) + "\n",
                                   encoding="utf-8")
    (out / "outputs").write_text(
        "".join(f"{rel}\n" for rel in plan["shards"][index - 1]["outputs"]), encoding="utf-8")
    print(describe_plan(plan, index), file=sys.stderr)
    return 0


def filter_for_shard(doc: dict, plan: dict, entries: Sequence[Entry], root: Path,
                     build_dir: Path, generated: Sequence[str]) -> dict:
    """The jobs document of one shard: its own subjects' jobs, every late
    header's, and its account of each subject the plan gave it."""
    index = plan["shard"]
    part = plan["shards"][index - 1]
    mine = set(part["subjects"])
    late = set(plan["late"])
    ranks: Dict[Tuple[str, str], int] = {}
    for rank, entry in enumerate(owner_order(entries)):
        ranks.setdefault((display(entry.file, root, build_dir), entry.target), rank)

    jobs: List[dict] = []
    not_measured: List[dict] = []
    accounting: Dict[str, str] = {}
    for job in doc["jobs"]:
        key = job_key(job)
        if key in mine:
            jobs.append({**job, "key": key})
            accounting[key] = "measured"
        elif key in late:
            via = job.get("via", {})
            rank = ranks.get((via.get("tu", ""), via.get("target", "")), len(ranks))
            jobs.append({**job, "key": key, "late": {"rank": rank, "how": via.get("how", "")}})
    for item in doc["not_measured"]:
        key = f"{item['kind']}:{item['subject']}"
        if key in mine:
            not_measured.append({**item, "key": key})
            accounting[key] = "not measured"
        elif key in late:
            not_measured.append({**item, "key": key, "late": True})
    for header in doc.get("c_only", []):
        key = f"{KIND_GEN}:{header}"
        if key in mine:
            accounting.setdefault(key, "compiled through C units")
    marked = {display(p, root, build_dir) for p in generated}
    for key in sorted(mine - set(accounting)):
        kind, _, rest = key.partition(":")
        if kind == KIND_GEN and rest.startswith(BUILD_PREFIX):
            path = build_dir / rest[len(BUILD_PREFIX):]
            if not path.is_file():
                accounting[key] = "absent"
            elif rest not in marked:
                accounting[key] = "not a generated header"
            else:
                accounting[key] = "unaccounted"
        else:
            accounting[key] = "unaccounted"

    built = set(part["outputs"])
    everywhere = {rel for other in plan["shards"] for rel in other["outputs"]}
    return {**doc, "jobs": jobs, "not_measured": not_measured, "accounting": accounting,
            "unbuilt": sorted(everywhere - built),
            "shard": {"index": index, "count": plan["count"], "digest": plan["digest"],
                      "counts": part["counts"], "totals": plan["totals"]}}


def verify_shards(directory: Path, floors: Dict[str, int]) -> Tuple[List[str], dict]:
    """What is wrong with the shards of one run, and the counted outcomes.

    Each shard leaves `plan.json`, `jobs.json` and `results.json` in one
    directory. Nothing is wrong when every shard planned alike, accounted for
    every subject it was given and measured it, and the counted outcomes hold
    the whole floors with no failure.
    """
    faults: List[str] = []
    plans: Dict[int, dict] = {}
    docs: Dict[int, dict] = {}
    results: Dict[int, dict] = {}
    for plan_path in sorted(directory.rglob("plan.json")):
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        index = plan.get("shard")
        if index in plans:
            faults.append(f"shard {index} left two plans ({plan_path})")
            continue
        plans[index] = plan
        for name, into in (("jobs.json", docs), ("results.json", results)):
            path = plan_path.with_name(name)
            if path.is_file():
                into[index] = json.loads(path.read_text(encoding="utf-8"))
    if not plans:
        return [f"no shard left a plan under {directory}"], {}
    digests = {p.get("digest") for p in plans.values()}
    if len(digests) != 1:
        faults.append("the shards planned differently, so a subject can fall between them: "
                      f"{sorted((str(i), str(p.get('digest'))[:16]) for i, p in plans.items())}")
    plan = plans[min(plans, key=str)]
    count = plan["count"]
    extra = sorted(str(i) for i in plans if not isinstance(i, int) or not 1 <= i <= count)
    if extra:
        faults.append(f"plans for shards outside 1..{count}: {extra}")
    given = [k for part in plan["shards"] for k in part["subjects"]]
    if len(given) != len(set(given)) or len(given) != plan["subjects"]:
        faults.append(f"the plan does not give each of its {plan['subjects']} subject(s) to "
                      f"one shard ({len(given)} given, {len(set(given))} distinct)")

    counted: List[Outcome] = []
    late: Dict[str, List[Tuple[Tuple[int, int, int], dict]]] = collections.defaultdict(list)
    late_reasons: Dict[str, str] = {}
    for index in range(1, count + 1):
        if index not in plans:
            faults.append(f"shard {index} of {count} left no plan")
            continue
        if index not in docs or index not in results:
            faults.append(f"shard {index} of {count} left no account or no results: its "
                          f"part did not run to the end")
            continue
        mine = set(plan["shards"][index - 1]["subjects"])
        accounting = docs[index].get("accounting", {})
        if set(accounting) != mine:
            missing = sorted(mine - set(accounting))[:10]
            foreign = sorted(set(accounting) - mine)[:10]
            faults.append(f"shard {index} accounted for other than its subjects: missing "
                          f"{missing}, not its own {foreign}")
        for key, status in sorted(accounting.items()):
            if status in ACCOUNTING_FAULTS:
                faults.append(f"shard {index}: {key} is {status}")
        for item in docs[index].get("not_measured", []):
            if item.get("late"):
                late_reasons.setdefault(item["key"], item.get("reason", ""))
        for record in results[index].get("outcomes", []):
            job = {"kind": record["kind"], "subject": record["subject"],
                   "target": record["target"], "via": {"how": record.get("how", ""),
                                                       "tu": record.get("tu", "")}}
            std = None if record["standard"] == "own" else record["standard"]
            outcome = Outcome(job, std, record["verdict"], record.get("detail", ""))
            if record.get("late"):
                info = record["late"]
                order = (0 if info.get("how") == "included" else 1, info.get("rank", 0), index)
                late[record["key"]].append((order, {"outcome": outcome, "shard": index}))
            elif record["key"] in mine:
                counted.append(outcome)
            else:
                faults.append(f"shard {index} measured {record['key']}, which is not its own")

    unmeasured_late = []
    for key in plan["late"]:
        candidates = late.get(key)
        if not candidates:
            unmeasured_late.append((key, late_reasons.get(key, "no shard could own it")))
            continue
        best = min(order for order, _ in candidates)
        counted.extend(c["outcome"] for order, c in candidates if order == best)

    stds = plan["stds"]
    table = tally(counted)
    failures = [o for o in counted if o.verdict == "fail"]
    breaches = floor_breaches(table, stds, floors)
    return faults, {"table": table, "stds": stds, "failures": failures, "breaches": breaches,
                    "unmeasured_late": unmeasured_late, "count": count,
                    "digest": plan.get("digest", "")}


def cmd_verify_shards(args: argparse.Namespace) -> int:
    try:
        floors = parse_floors(args.floor or [])
    except ValueError as exc:
        print(f"verify-shards: {exc}", file=sys.stderr)
        return 2
    faults, verdict = verify_shards(Path(args.directory), floors)
    for fault in faults:
        print(f"verify-shards: {fault}", file=sys.stderr)
    if not verdict:
        return 1
    print(f"verify-shards: plan {verdict['digest'][:16]}, {verdict['count']} shard(s)")
    print_table(verdict["table"], verdict["stds"], {}, {})
    print_failures(verdict["failures"])
    for key, reason in verdict["unmeasured_late"]:
        print(f"  NOT MEASURED (late) {key}: {reason}")
    for breach in verdict["breaches"]:
        print(f"\nFLOOR: {breach}")
    ok = not faults and not verdict["failures"] and not verdict["breaches"]
    print(f"\nverify-shards: {len(verdict['failures'])} failed, "
          f"{len(verdict['breaches'])} floor breach(es), {len(faults)} fault(s) in the shards",
          file=sys.stderr)
    return 0 if ok else 1


# ── CLI ───────────────────────────────────────────────────────────


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = parser.add_subparsers(dest="command", required=True)

    guards = sub.add_parser("guards", help="refuse a standard-selecting #if in the interface")
    guards.add_argument("--root", default=str(REPO_ROOT))
    guards.add_argument("--min-files", type=int, default=MIN_GUARD_FILES)
    guards.set_defaults(func=cmd_guards)

    outputs = sub.add_parser("generation-outputs",
                             help="the generated headers a Ninja tree's custom commands write")
    outputs.add_argument("--ninja-targets", required=True,
                         help="a file holding the output of `ninja -t targets all`")
    outputs.add_argument("--build-dir", help="the tree, to relativise absolute outputs")
    outputs.add_argument("--out", required=True)
    outputs.set_defaults(func=cmd_generation_outputs)

    jobs = sub.add_parser("jobs", help="the compile jobs that measure a configured tree")
    jobs.add_argument("--compile-commands", required=True)
    jobs.add_argument("--build-dir", help="default: the directory of compile_commands.json")
    jobs.add_argument("--root", default=str(REPO_ROOT))
    jobs.add_argument("--out", required=True)
    jobs.add_argument("--cxx", help="compile with this C++ compiler instead of the configured one")
    jobs.add_argument("--cc", help="compile with this C compiler instead of the configured one")
    jobs.add_argument("--plan", help="a shard's plan.json (`shard-plan`): keep its subjects' "
                                     "jobs and the late ones, and account for each subject")
    jobs.set_defaults(func=cmd_jobs)

    plan = sub.add_parser("shard-plan",
                          help="plan the measurement of a configured tree as N shards")
    plan.add_argument("--compile-commands", required=True)
    plan.add_argument("--build-dir", help="default: the directory of compile_commands.json")
    plan.add_argument("--root", default=str(REPO_ROOT))
    plan.add_argument("--generation", required=True,
                      help="the file `generation-outputs` wrote: every planned output")
    plan.add_argument("--ninja-graph", required=True,
                      help="`ninja -t graph` of those outputs")
    plan.add_argument("--std", action="append",
                      help="a C++ standard the run measures at (default: c++17 and c++20)")
    plan.add_argument("--out", required=True,
                      help="where plan.json and this shard's outputs are written")
    plan.add_argument("shard", type=shard_spec, help="k/N, this job's shard")
    plan.set_defaults(func=cmd_shard_plan)

    run = sub.add_parser("run", help="run the jobs at the requested standards")
    run.add_argument("--jobs", required=True)
    run.add_argument("--std", action="append",
                     help="a C++ standard to measure at; repeat it (default: c++17 and c++20)")
    run.add_argument("--parallel", type=int, default=os.cpu_count() or 1)
    run.add_argument("--floor", action="append", metavar="KIND=N",
                     help="override the fewest jobs of a kind a run must measure")
    run.add_argument("--results", help="also write every outcome to this JSON file")
    run.set_defaults(func=cmd_run)

    verify = sub.add_parser("verify-shards",
                            help="hold the shards of one run to one plan and the whole floors")
    verify.add_argument("directory", help="holds each shard's plan.json, jobs.json and "
                                          "results.json, a directory per shard")
    verify.add_argument("--floor", action="append", metavar="KIND=N",
                        help="override the fewest jobs of a kind the run must measure")
    verify.set_defaults(func=cmd_verify_shards)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
