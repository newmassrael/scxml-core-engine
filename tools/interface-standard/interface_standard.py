#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

Standard library only, so it runs wherever the compiler does.
"""

from __future__ import annotations

import argparse
import collections
import concurrent.futures
import functools
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, Iterator, List, Optional, Sequence, Set, Tuple

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
    found: Set[str] = set()
    for path in walk_files(build_dir, skip_names=("CMakeFiles", "_deps")):
        if path.name.endswith(HEADER_SUFFIXES) and carries_generator_mark(path):
            found.add(os.path.realpath(path))
    for path in tracked_files(root):
        if path.relative_to(root).parts[0] in SKIPPED_SOURCE_DIRS:
            continue
        if path.name.endswith(HEADER_SUFFIXES) and carries_generator_mark(path):
            found.add(os.path.realpath(path))
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
    cxx = sorted((e for e in entries if e.language == "c++"),
                 key=lambda e: (0 if e.is_library else 1, e.file, e.target))
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
    except RuntimeError as exc:
        print(f"jobs: {exc}", file=sys.stderr)
        return 1
    Path(args.out).write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    counts = collections.Counter(job["kind"] for job in doc["jobs"])
    unmeasured = collections.Counter(item["kind"] for item in doc["not_measured"])
    print(f"jobs: {len(entries)} translation unit(s) in {compile_commands}", file=sys.stderr)
    for kind in KINDS:
        print(f"  {kind:5} {counts.get(kind, 0):5} job(s), {unmeasured.get(kind, 0):3} "
              f"not measured  ({KIND_TITLES[kind]})", file=sys.stderr)
    print(f"  {doc['c_headers']} generated header(s) are reached only by C translation "
          f"units and are compiled through them", file=sys.stderr)
    return 0


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


def classify(job: dict, std: Optional[str], returncode: int, output: str,
             index: FileIndex) -> Outcome:
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
        borrowed = job.get("via", {}).get("how") == "sibling"
        if borrowed:
            return Outcome(job, std, "unmeasured",
                           f"the flags of {job['via']['tu']} (which includes a header beside "
                           f"this one) do not reach {name}")
        if not index.provides(name):
            return Outcome(job, std, "unmeasured",
                           f"needs {name}, which neither this machine nor the build tree provides")
    return Outcome(job, std, "fail", error)


def execute(job: dict, std: Optional[str], index: FileIndex) -> Outcome:
    argv = [a.replace(STD_PLACEHOLDER, std) for a in job["argv"]] if std else list(job["argv"])
    try:
        proc = subprocess.run(argv, cwd=job["cwd"], stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, errors="replace",
                              timeout=JOB_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return Outcome(job, std, "fail", f"timed out after {JOB_TIMEOUT_S}s")
    except OSError as exc:
        return Outcome(job, std, "fail", f"could not start {argv[0]}: {exc}")
    return classify(job, std, proc.returncode, proc.stdout, index)


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

    for compiler in sorted({job["argv"][0] for job in doc["jobs"]}):
        print(f"run: compiler {compiler_identity(compiler)}", file=sys.stderr)

    runs, excused = plan_runs(doc["jobs"], stds)
    print(f"run: {len(runs)} compile(s), {args.parallel} at a time", file=sys.stderr)
    outcomes: List[Outcome] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = [pool.submit(execute, job, std, index) for job, std in runs]
        for done, future in enumerate(concurrent.futures.as_completed(futures), 1):
            outcomes.append(future.result())
            if done % 250 == 0:
                print(f"run: {done}/{len(runs)}", file=sys.stderr)
    outcomes.sort(key=lambda o: (KINDS.index(o.job["kind"]), o.job["subject"], o.std or ""))

    table = tally(outcomes)
    unowned = collections.Counter(item["kind"] for item in doc["not_measured"])
    print(f"\n{'kind':6}{'standard':10}{'measured':>9}{'passed':>8}{'failed':>8}"
          f"{'not measured':>14}")
    for kind in KINDS:
        columns = ["own"] if kind == KIND_CTU else stds
        for std in columns:
            row = table.get((kind, std), Tally())
            unmeasured = row.unmeasured + (unowned[kind] if std != "own" else 0)
            note = ""
            if excused.get((kind, std)):
                targets = sorted({job["target"] for job in excused[(kind, std)]})
                note = (f"  ({len(excused[(kind, std)])} not applicable: "
                        f"{', '.join(targets)} declare an implementation above {std})")
            print(f"{kind:6}{std:10}{row.measured:9}{row.passed:8}{row.failed:8}"
                  f"{unmeasured:14}{note}")

    failures = [o for o in outcomes if o.verdict == "fail"]
    if failures:
        print(f"\nFAILED ({len(failures)}):")
        for outcome in failures:
            job = outcome.job
            borrowed = job["via"].get("how") in ("included", "sibling")
            source = f", flags of {job['via']['tu']}" if borrowed else ""
            print(f"  [{job['kind']} {outcome.std or 'own'}] {job['subject']}  "
                  f"(target {job['target']}{source})")
            print(f"      {outcome.detail}")

    unmeasured_runs: Dict[Tuple[str, str, str], List[str]] = collections.defaultdict(list)
    for outcome in outcomes:
        if outcome.verdict == "unmeasured":
            key = (outcome.job["kind"], outcome.job["subject"], outcome.detail)
            unmeasured_runs[key].append(outcome.std or "own")
    total_unmeasured = len(doc["not_measured"]) + len(unmeasured_runs)
    if total_unmeasured:
        print(f"\nNOT MEASURED ({total_unmeasured}), neither passed nor failed:")
        for item in doc["not_measured"]:
            print(f"  [{item['kind']}] {item['subject']}: {item['reason']}")
        for (kind, subject, reason), standards in sorted(unmeasured_runs.items()):
            print(f"  [{kind}] {subject}: {reason} (at {', '.join(standards)})")

    breaches = floor_breaches(table, stds, floors)
    for breach in breaches:
        print(f"\nFLOOR: {breach}")

    if args.results:
        Path(args.results).write_text(json.dumps([
            {"kind": o.job["kind"], "subject": o.job["subject"], "target": o.job["target"],
             "standard": o.std or "own", "verdict": o.verdict, "detail": o.detail}
            for o in outcomes], indent=1) + "\n", encoding="utf-8")

    passed = sum(1 for o in outcomes if o.verdict == "pass")
    print(f"\nrun: {passed} passed, {len(failures)} failed, {total_unmeasured} not measured, "
          f"{len(breaches)} floor breach(es)", file=sys.stderr)
    return 1 if failures or breaches else 0


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
    jobs.set_defaults(func=cmd_jobs)

    run = sub.add_parser("run", help="run the jobs at the requested standards")
    run.add_argument("--jobs", required=True)
    run.add_argument("--std", action="append",
                     help="a C++ standard to measure at; repeat it (default: c++17 and c++20)")
    run.add_argument("--parallel", type=int, default=os.cpu_count() or 1)
    run.add_argument("--floor", action="append", metavar="KIND=N",
                     help="override the fewest jobs of a kind a run must measure")
    run.add_argument("--results", help="also write every outcome to this JSON file")
    run.set_defaults(func=cmd_run)

    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
