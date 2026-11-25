# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# What selects a gate, derived: a gate's trigger globs from its entry, the
# workflows it mirrors, the builds it names (`builds`, "What a build reads"
# below) and the ledgers it names (`enrolled_by`); and what its builds read
# that does not select it ("What a build reads that does not select it").
#
# Apart from `gate_table.py` because a gate's run seldom asks it. The
# registry asks it for every gate, to choose what a change reaches and to key
# a gate on its triggers (`INPUTS_ARE_TRIGGERS`); a run asks it only when
# its submodule preflight needs the reach of such a gate
# (`submodule_needs` in gate_table.py). Kept out of `gate_table.py`, an edit
# to how a build's inputs are derived moves the keys of the units that read
# it and of no other. Loaded by `gate_table.sibling`, never imported, for the
# reason that function gives.

from __future__ import annotations

import importlib.util
import posixpath
import re
import subprocess
from pathlib import Path

from gate_table import glob_to_regex


def workflow_paths(repo_root: Path, name: str) -> list[str]:
    """The deduped ALLOW-side (`paths:`) globs a workflow declares under `on:`.

    A workflow with no `paths:` filter runs on every push, so it returns the
    catch-all — the honest translation of "CI always runs this".

    ⚠ Allow-side ONLY, and the asymmetry is deliberate rather than an
    oversight. A `paths-ignore:` workflow really does start on every push in
    this sense: there is no path list CI checks a change INTO. What it has is
    a list it checks changes OUT of, which is a different question with a
    different reader — `workflow_ignored_paths` below — because the two are
    consumed in different places. `narrows` asks this one ("is CI's trigger
    the catch-all, so the local table may state the real inputs instead?")
    and the answer stays yes. `ci_owed` asks the other ("will CI actually
    start for THIS change?"), and that is where a deny-list bites.

    Reading a deny-list as "unfiltered" here and stopping would have made the
    narrowing self-test pass by blindness the moment the first `paths-ignore`
    landed: `.github/workflows/cpp-suite.yml` acquired one on 2026-08-29 and
    every check in this file went on reporting the lane as unfiltered.
    `sce-build/tests/workflow_trigger_coverage.rs` already counted it
    ("`paths-ignore` counts: it is the same filter with the sense flipped"),
    so two readers of one file disagreed about whether it was filtered, and
    nothing held them in step. `both_filter_readers_agree` in the self-test
    below is now what does.
    """
    wf = repo_root / ".github" / "workflows" / name
    if not wf.is_file():
        # Missing workflow: refuse to guess. The caller turns an empty
        # trigger set into "always run", which is the safe direction.
        return ["**"]
    text = wf.read_text(encoding="utf-8")
    on_block = re.search(r"^on:\n(.*?)(?=^\S)", text, re.S | re.M)
    if not on_block:
        return ["**"]
    globs: list[str] = []
    for block in re.finditer(r"^(\s+)paths:\s*\n((?:\1\s+-.*\n|\s*#.*\n)*)",
                             on_block.group(1), re.M):
        for line in block.group(2).splitlines():
            line = line.strip()
            if line.startswith("- "):
                globs.append(line[2:].strip().strip("'\""))
    if not globs:
        return ["**"]
    return list(dict.fromkeys(globs))


def tracked_paths(repo_root: Path, *pathspecs: str) -> list[str]:
    """Every path git tracks that matches one of `pathspecs` — every tracked
    path when there are none — repository-relative, as git stores it.

    `-z`, always. Without it git's default `core.quotePath` writes a path
    holding any byte outside printable ASCII as a quoted octal escape,
    which names no file on disk, and a caller splitting on whitespace also
    cuts a path with a space in two — either way the file is skipped in
    silence. A git that fails is an error, not an empty tree: an empty list
    here reads as "nothing to judge" to every caller.
    """
    proc = subprocess.run(
        ["git", "-C", str(repo_root), "ls-" + "files", "-z", "--", *pathspecs],
        capture_output=True, check=True,
    )
    return [p.decode("utf-8", "surrogateescape") for p in proc.stdout.split(b"\0") if p]


def enrolled_globs(repo_root: Path, configs: str):
    """The trees the ledgers matching `configs` enrol, as trigger globs.

    A ledger's `mnemosyne.toml` lists, under
    `[plugins.set_equality_validator]`, the paths whose citations it judges.
    They are read with the reader the citation gate itself uses
    (`paths_from_toml` in tools/mnemosyne-adoption/migrate_citations.py), so
    the two cannot parse the list differently. A directory becomes
    `<dir>/**` and a file stays itself; each ledger's own directory joins as
    well, since its config decides the set and its store is what the
    citations are judged against.

    `None` when the set cannot be read: no config matches, the reader cannot
    be loaded, or a config has no such list. The caller reads that as the
    catch-all, the safe direction, the same answer `workflow_paths` gives for
    a workflow it cannot find.

    Read once per process and root: `select` derives the triggers for every
    call, and the self-test calls it dozens of times.
    """
    key = (str(repo_root), configs)
    if key not in _ENROLLED:
        _ENROLLED[key] = _read_enrolled(repo_root, configs)
    cached = _ENROLLED[key]
    return None if cached is None else list(cached)


_ENROLLED: dict[tuple[str, str], list[str] | None] = {}


def _read_enrolled(repo_root: Path, configs: str):
    """`enrolled_globs` without the cache."""
    found = sorted(repo_root.glob(configs))
    reader = repo_root / "tools" / "mnemosyne-adoption" / "migrate_citations.py"
    if not found or not reader.is_file():
        return None
    try:
        spec = importlib.util.spec_from_file_location(
            "_sce_enrolment_reader", reader)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        globs: list[str] = []
        for config in found:
            # The ledger's own directory: the config, its store and the
            # document it is made from, which the ledger's tooling reads.
            globs.append(f"{config.parent.relative_to(repo_root).as_posix()}/**")
            for absolute in module.paths_from_toml(str(config)):
                rel = Path(absolute).relative_to(module.REPO_ROOT).as_posix()
                globs.append(f"{rel}/**" if (repo_root / rel).is_dir()
                             else rel)
    except (Exception, SystemExit):
        return None
    return list(dict.fromkeys(globs))


# ── What a build reads: `builds` ──────────────────────────────────
#
# Eleven gates configure and build the main CMake tree, and every one of them
# compiles what the tree compiles whatever else it does: the engine, its
# runtimes, the submodules, and, since the ECMAScript frontend is linked into
# `sce_scripting` (sce/CMakeLists.txt, `sce_link_ecmascript_frontend`), the
# `sce-build` crate through cargo. Each gate used to write its own share of
# that into `extra`, and the shares had drifted: none named the root
# `Cargo.toml` or `Cargo.lock`, `w3c-python-bindings` and `visualizer-wasm`
# named no `sce/` at all (2026-11-16), so an edit to what they compile neither
# selected them nor moved the key of a unit kept for them.
#
# So what a shared build reads is written once, here, and a gate names it in
# `builds`; `gate_triggers` adds it to the gate's trigger, which is also the
# unit's input set (`unit_inputs`). The cargo half is DERIVED, from the files
# cargo and the crate itself declare, so a new path dependency, a new member
# or a new compiled-in file joins every such gate on the commit that adds it:
#   * the workspace: the root `Cargo.toml`, `Cargo.lock`, and every member's
#     manifest (`cargo_workspace_manifests`), which cargo reads to load the
#     workspace; a build from a crate outside it reads the root's and those
#     of the crates it builds;
#   * `.cargo/`, whose configuration every cargo run reads;
#   * each crate the build compiles: the crate named and its path
#     dependencies (`[dependencies]`, `[build-dependencies]` and their target
#     forms, never `[dev-dependencies]`), each as its `src/**`, its build
#     script and the roots its manifest declares (`cargo_build_targets`).
#     Not the whole directory: `tests/`, `benches/` and `examples/` are
#     compiled by `cargo test` or `--all-targets`, which a gate runs as its
#     own command and names its own trigger for. Taken whole, one integration
#     test of `sce-build` selected 39 gates of a landing (2026-11-24);
#   * every file a compiled module names by path, found by walking the module
#     tree as rustc does (`rust_module_closure`): a `#[path]` module and the
#     directory it owns, and each `include_str!` / `include_bytes!` /
#     `include!` target. A module behind `#[cfg(test)]` is not walked;
#   * what `sce-build`'s build script reads: the generator witness (its
#     WITNESS_FILES, WITNESS_TREES and RUN_TIME_TREE, in
#     sce-build/src/generator_witness.rs, which the script digests and embeds);
# `MAIN_TREE_CONFIGURE` is the CMake half: what the root CMakeLists.txt
# configures for every tree. A subdirectory a gate turns on with an option of
# its own (`tests/`, the examples, the Python bindings) stays in its `extra`.
#
# `every-tree-builder-names-what-it-builds` in the self-test finds the gates
# whose script configures the root tree and refuses one that does not name
# `main-tree`, and `ci-only-coverage` holds a filtered workflow's `paths:` to
# what its gate builds.
MAIN_TREE_CONFIGURE = (
    "CMakeLists.txt", "cmake/**", "sce/**", "third_party/**",
    "backends/cpp/forge-runtime/**", "backends/c/runtime/**",
    "tools/txml_converter/**",
)
# Each shared build: the CMake globs it configures, and the crates cargo
# builds for it.
# The third field says whether cargo builds the crates as members of this
# workspace (`cargo build -p`), which loads every member's manifest, or from a
# crate of the gate's own that names them as path dependencies, which reads
# their manifests and the root's (the fields they inherit) and no other.
BUILT_INPUTS: dict[str, tuple[tuple[str, ...], tuple[str, ...], bool]] = {
    "sce-codegen": ((), ("sce-build",), True),
    "main-tree": (MAIN_TREE_CONFIGURE, ("sce-build",), True),
    "codegen-wasm": ((), ("sce-build-wasm",), True),
    # The Rust forge runtime a generated Rust codec compiles against, built
    # by the codec gates in a crate of their own (scripts/check_*_codec.sh).
    # `bx --checks prepush --learn codec-clippy` (2026-11-24) read the root
    # manifest and the runtime's, and no other member's.
    "forge-runtime-rust": ((), ("backends/rust/forge-runtime",), False),
}
# The file whose constants name what `sce-build`'s build script reads.
GENERATOR_WITNESS = "sce-build/src/generator_witness.rs"

_CARGO_DEP_TABLE = re.compile(
    r"^\[(?:target\.[^\]]+\.)?(?:dependencies|build-dependencies)"
    r"(?:\.[A-Za-z0-9_-]+)?\]\s*$")
_CARGO_PATH_VALUE = re.compile(r'\bpath\s*=\s*"([^"]+)"')
# A compiled-in file (`include_str!`, `include_bytes!`, `include!`) and a
# module whose file a `#[path]` names, each by a literal path relative to the
# source file that names it.
_RUST_INCLUDE = re.compile(r'include(?:_str|_bytes)?!\(\s*"([^"]+)"\s*\)')
_RUST_PATH_ATTR = re.compile(r'^\s*#\[path\s*=\s*"([^"]+)"\s*\]')
_RUST_MOD_DECL = re.compile(
    r'^(\s*)(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;')
_RUST_TEST_ONLY = re.compile(r'^\s*#\[cfg\(test\)\]\s*$')
_RUST_ATTRIBUTE = re.compile(r'^\s*(?:#\[|///|//!)')
_CARGO_TABLE = re.compile(r'^\[\[?\s*([A-Za-z0-9_.-]+)\s*\]\]?\s*$')
_CARGO_STRING_VALUE = re.compile(r'^\s*(path|build)\s*=\s*"([^"]+)"')
_CARGO_BUILD_OFF = re.compile(r'^\s*build\s*=\s*false\b')


def cargo_path_deps(repo_root: Path, crate: str) -> list[str]:
    """The directories of the path dependencies `crate/Cargo.toml` builds.

    Read by its lines, as the registry reads every TOML file (see
    `fleet_platforms`): a table header opens a dependency table or closes
    one, and a `path = "..."` inside one names a directory relative to the
    crate. `[dev-dependencies]` is not a dependency table here: a build of
    the crate compiles none of it.
    """
    try:
        text = (repo_root / crate / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return []
    found: list[str] = []
    in_deps = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_deps = bool(_CARGO_DEP_TABLE.match(stripped))
            continue
        if in_deps and not stripped.startswith("#"):
            found.extend(posixpath.normpath(f"{crate}/{rel}")
                         for rel in _CARGO_PATH_VALUE.findall(stripped))
    return found


def generator_witness_globs(repo_root: Path) -> list[str]:
    """What `sce-build`'s build script reads: the generator witness.

    `WITNESS_FILES` as paths, `WITNESS_TREES` and `RUN_TIME_TREE` as trees,
    read out of GENERATOR_WITNESS, the one place they are written.
    """
    try:
        text = (repo_root / GENERATOR_WITNESS).read_text(encoding="utf-8")
    except OSError:
        return []
    globs: list[str] = []
    for name, tree in (("WITNESS_FILES", False), ("WITNESS_TREES", True)):
        found = re.search(r"pub const " + name + r": &\[&str\] = &\[(.*?)\];",
                          text, re.S)
        if found:
            globs.extend(f"{v}/**" if tree else v
                         for v in re.findall(r'"([^"]+)"', found.group(1)))
    found = re.search(r'pub const RUN_TIME_TREE: &str = "([^"]+)";', text)
    if found:
        globs.append(f"{found.group(1)}/**")
    return globs


def cargo_build_targets(repo_root: Path, crate: str):
    """`(build script or None, [lib and bin roots it declares])` of `crate`.

    Cargo's own target rules (the Cargo Book, "Target auto-discovery" and
    "Configuring a target"): the build script is `build.rs` unless
    `[package] build` names another file or turns it off, and a `[lib]` or
    `[[bin]]` table may name its root with `path`. The roots cargo finds by
    itself (`src/lib.rs`, `src/main.rs`, `src/bin/`) are not listed here.
    `[[test]]`, `[[bench]]` and `[[example]]` roots are never among these:
    `cargo build` compiles none of them, and `cargo test` or `--all-targets`
    is a gate's own command. Read by its lines, as the registry reads every
    TOML file. Paths are from the repository root.
    """
    try:
        text = (repo_root / crate / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return None, []
    table = ""
    script = "build.rs"
    declared: list[str] = []
    for line in text.splitlines():
        header = _CARGO_TABLE.match(line.strip())
        if header:
            table = header.group(1)
            continue
        if table == "package" and _CARGO_BUILD_OFF.match(line):
            script = None
            continue
        value = _CARGO_STRING_VALUE.match(line)
        if not value:
            continue
        key, rel = value.groups()
        if table == "package" and key == "build":
            script = rel
        elif table in ("lib", "bin") and key == "path":
            declared.append(posixpath.normpath(f"{crate}/{rel}"))
    if script is not None:
        script = posixpath.normpath(f"{crate}/{script}")
        if not (repo_root / script).is_file():
            script = None
    return script, declared


def rust_module_closure(repo_root: Path, roots) -> tuple[list[str], list[str]]:
    """What a build that is not a test compiles from `roots`, by path.

    Walks the module tree the way rustc resolves it (the Rust Reference,
    "Modules"): `mod x;` in a crate root or a `mod.rs` is `x.rs` or
    `x/mod.rs` beside it, in any other file `f.rs` it is under `f/`, and a
    `#[path]` names the file relative to the directory of the file that
    declares it. A module whose attributes say `#[cfg(test)]` is not walked:
    a build that is not a test compiles none of it. Every file walked is read
    for the literal paths of `include_str!`, `include_bytes!` and `include!`,
    which resolve against that file's directory. A declaration that is
    indented sits in an inline module, whose files this does not place, so
    the walk takes every source below the directory instead.

    Returns the files named by path (`#[path]` and include targets, which
    may lie outside `src/`) and the directories a `#[path]` module owns.
    """
    root = repo_root.resolve()
    named: list[str] = []
    trees: list[str] = []
    pending = [(r, [posixpath.dirname(r)]) for r in roots]
    seen: set[str] = set()

    def resolved(base, rel):
        try:
            return (repo_root / base).joinpath(rel).resolve() \
                .relative_to(root).as_posix()
        except ValueError:
            return None

    def sources_below(folder):
        return [p for p in tracked_paths(repo_root, folder) if p.endswith(".rs")]

    while pending:
        source, owns = pending.pop()
        if source in seen:
            continue
        seen.add(source)
        try:
            text = (repo_root / source).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        here = posixpath.dirname(source)
        # Over the whole text: rustfmt breaks a long call over three lines.
        for rel in _RUST_INCLUDE.findall(text):
            if (path := resolved(here, rel)) is not None:
                named.append(path)
                if path.endswith(".rs"):
                    pending.append((path, owns))
        lines = text.splitlines()
        for i, line in enumerate(lines):
            decl = _RUST_MOD_DECL.match(line)
            if not decl:
                continue
            indent, name = decl.groups()
            above, test_only, path_attr = i - 1, False, None
            while above >= 0 and _RUST_ATTRIBUTE.match(lines[above]):
                test_only |= bool(_RUST_TEST_ONLY.match(lines[above]))
                if (attr := _RUST_PATH_ATTR.match(lines[above])):
                    path_attr = attr.group(1)
                above -= 1
            if test_only:
                continue
            if indent:
                for folder in owns:
                    pending.extend((p, [p[:-3]]) for p in sources_below(folder))
                continue
            if path_attr is not None:
                if (path := resolved(here, path_attr)) is None:
                    continue
                folder = posixpath.dirname(path)
                named.append(path)
                trees.append(f"{folder}/**")
                pending.append((path, [folder, path[:-3]]))
                pending.extend((p, [p[:-3]]) for p in sources_below(folder))
                continue
            for folder in owns:
                flat = f"{folder}/{name}.rs"
                nested = f"{folder}/{name}/mod.rs"
                if (repo_root / flat).is_file():
                    pending.append((flat, [f"{folder}/{name}"]))
                elif (repo_root / nested).is_file():
                    pending.append((nested, [f"{folder}/{name}"]))
    return named, trees


_CARGO_MEMBERS = re.compile(r"^\s*members\s*=\s*\[(.*?)\]", re.S | re.M)


def cargo_workspace_manifests(repo_root: Path) -> list[str]:
    """The manifests cargo reads to load this workspace: the root's and each
    member's (`[workspace] members`, its globs expanded), sorted.

    A build of any member loads them all, and a crate outside the workspace
    that names a member as a path dependency reads the member's and, for the
    fields the member inherits, the root's. Named one by one rather than as
    `**/Cargo.toml`, which also names the manifests of crates no build here
    loads (a probe with a workspace of its own, a submodule's)."""
    root = repo_root / "Cargo.toml"
    try:
        text = root.read_text(encoding="utf-8")
    except OSError:
        return []
    section = text.split("[workspace]", 1)[1] if "[workspace]" in text else ""
    section = re.split(r"^\[(?!workspace\])", section, maxsplit=1, flags=re.M)[0]
    found = _CARGO_MEMBERS.search(section)
    members = re.findall(r'"([^"]+)"', re.sub(r"#[^\n]*", "", found.group(1))) \
        if found else []
    manifests = ["Cargo.toml"]
    for member in members:
        for path in sorted(repo_root.glob(member)) if "*" in member \
                else [repo_root / member]:
            if (path / "Cargo.toml").is_file():
                manifests.append(f"{path.relative_to(repo_root)}/Cargo.toml")
    return sorted(dict.fromkeys(manifests))


def cargo_build_closure(repo_root: Path, crates) -> list[str]:
    """`crates` and the path dependencies a build of them compiles
    (`cargo_path_deps`), transitively, sorted."""
    closure: list[str] = []
    pending = list(crates)
    while pending:
        crate = pending.pop()
        if crate in closure:
            continue
        closure.append(crate)
        pending.extend(cargo_path_deps(repo_root, crate))
    return sorted(closure)


def cargo_build_globs(repo_root: Path, crates, in_workspace=True) -> list[str]:
    """What a cargo build of `crates` reads, as globs: as members of this
    workspace (`in_workspace`), or as path dependencies of a crate outside it.

    See "What a build reads". Exact paths a directory glob already covers
    are left out, so each file is named once.
    """
    closure = cargo_build_closure(repo_root, crates)
    globs = [".cargo/**"] + (["Cargo.lock"] if in_workspace else [])
    named: list[str] = (cargo_workspace_manifests(repo_root) if in_workspace
                        else ["Cargo.toml"]) + [
        f"{crate}/Cargo.toml" for crate in sorted(closure)]
    if "sce-build" in closure:
        named.extend(generator_witness_globs(repo_root))
    trees: list[str] = []
    roots: list[str] = []
    for crate in sorted(closure):
        trees.append(f"{crate}/src/**")
        script, declared = cargo_build_targets(repo_root, crate)
        declared = declared + ([script] if script else [])
        named.extend(declared)
        roots.extend(declared)
        roots.extend(p for p in ([f"{crate}/src/lib.rs", f"{crate}/src/main.rs"]
                                 + tracked_paths(repo_root, f"{crate}/src/bin"))
                     if p.endswith(".rs") and p not in declared
                     and (repo_root / p).is_file())
    reached, owned = rust_module_closure(repo_root, roots)
    named.extend(reached)
    trees.extend(owned)
    for tree in dict.fromkeys(trees):
        # A tree beneath another one adds nothing to it.
        if not any(other != tree and glob_to_regex(other).match(tree[:-3])
                   for other in trees):
            globs.append(tree)
    wide = [g for g in globs + named if "*" in g]
    covered = [glob_to_regex(g) for g in wide]
    out = list(globs)
    for g in named:
        if "*" in g or not any(p.match(g) for p in covered):
            out.append(g)
    return list(dict.fromkeys(out))


_BUILT_CACHE: dict[tuple[str, str], list[str]] = {}


def built_inputs(repo_root: Path, name: str) -> list[str]:
    """The globs a gate that `builds` `name` reads for it (`BUILT_INPUTS`)."""
    key = (str(repo_root.resolve()), name)
    if key not in _BUILT_CACHE:
        configure, crates, in_workspace = BUILT_INPUTS[name]
        joined = list(dict.fromkeys(
            list(configure) + (cargo_build_globs(repo_root, crates, in_workspace)
                               if crates else [])))
        # An exact path a tree of the other half already covers is named once.
        trees = [glob_to_regex(g) for g in joined if "*" in g]
        _BUILT_CACHE[key] = [g for g in joined
                             if "*" in g or not any(p.match(g) for p in trees)]
    return list(_BUILT_CACHE[key])


# ── What a build reads that does not select it ─────────────────────
#
# A gate's trigger is what can change its verdict; its key has to hold every
# file its run looks at, or bx's trace calls the run "could not measure" (bx
# RFC 0020, 5.2). For a cargo build the two differ, and the difference is
# cargo's own, so it is derived here from the manifests as the trigger half is
# and joins the KEY of a gate that `builds` (`gate_reads`), never its
# trigger:
#   * loading the workspace (the Cargo Book, "Target auto-discovery"): every
#     package the resolver loads, which is each member and each package a
#     member names by `path` in any dependency table, dev-dependencies
#     included (`cargo_loaded_packages`). Of each, cargo reads the manifest,
#     looks for `src/lib.rs` and `src/main.rs`, looks for `build.rs` when
#     `[package] build` does not name the script, and lists `src/bin/`,
#     `tests/`, `examples/` and `benches/` where they exist, with each
#     `<dir>/<name>/main.rs` a listed directory holds
#     (`cargo_package_load_reads`). It compiles none of that unless it builds
#     the package, so an edit there changes no verdict of these gates and
#     selects none of them;
#   * a build script cargo has not run in that target directory before: no
#     earlier run has told cargo what the script reads (`rerun-if-changed`),
#     and a script that has told it nothing is fingerprinted by every file of
#     its package (the Cargo Book, "Build Scripts", "Change Detection"), so a
#     crate the build compiles that has a build script is read whole
#     (`<crate>/**`).
# Measured 2026-11-24 with `bx --checks prepush --learn` (bx 340e9f9e):
# `forge-c11` and `forge-cpp` read every member's manifest and target roots,
# their `tests/` and `examples/` listings, and every file of `sce-build`, of
# which the triggers named only what the generator compiles.
_CARGO_ANY_DEP_TABLE = re.compile(
    r"^\[(?:target\.[^\]]+\.)?(?:dependencies|build-dependencies|dev-dependencies)"
    r"(?:\.[A-Za-z0-9_-]+)?\]\s*$")
# The directories cargo lists in a package it loads, for the targets its
# manifest does not name.
CARGO_DISCOVERED_DIRS = ("src/bin", "tests", "examples", "benches")


def cargo_loaded_packages(repo_root: Path) -> list[str]:
    """The packages cargo loads to resolve this workspace, sorted: each member
    (`cargo_workspace_manifests`) and each package one names by `path` in any
    dependency table, `[dev-dependencies]` included, transitively. Read by
    lines, as `cargo_path_deps` reads a manifest; a path outside the
    repository names no tracked file and is left out."""
    members = [m[:-len("/Cargo.toml")] for m in cargo_workspace_manifests(repo_root)
               if m != "Cargo.toml"]
    seen: list[str] = []
    pending = list(members)
    while pending:
        package = pending.pop()
        if package in seen or package.startswith("../") or package == "..":
            continue
        seen.append(package)
        try:
            text = (repo_root / package / "Cargo.toml").read_text(encoding="utf-8")
        except OSError:
            continue
        in_deps = False
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("["):
                in_deps = bool(_CARGO_ANY_DEP_TABLE.match(stripped))
                continue
            if in_deps and not stripped.startswith("#"):
                pending.extend(posixpath.normpath(f"{package}/{rel}")
                               for rel in _CARGO_PATH_VALUE.findall(stripped))
    return sorted(seen)


def cargo_package_load_reads(repo_root: Path, package: str) -> list[str]:
    """What cargo reads of `package` to load it, built or not, as input
    patterns: the manifest, the target roots it looks for that are tracked,
    and a listing (`<dir>/`, bx RFC 0020 4.2) of each discovery directory that
    holds a tracked file, with the `<name>/main.rs` of each target directory
    in it. See "What a build reads that does not select it"."""
    try:
        text = (repo_root / package / "Cargo.toml").read_text(encoding="utf-8")
    except OSError:
        return []
    table, script_named = "", False
    for line in text.splitlines():
        header = _CARGO_TABLE.match(line.strip())
        if header:
            table = header.group(1)
        elif table == "package" and re.match(r"^\s*build\s*=", line):
            script_named = True
    tracked = set(tracked_paths(repo_root, package))
    reads = [f"{package}/Cargo.toml"]
    looked_for = ["src/lib.rs", "src/main.rs"] + ([] if script_named else ["build.rs"])
    reads.extend(f"{package}/{rel}" for rel in looked_for
                 if f"{package}/{rel}" in tracked)
    for folder in CARGO_DISCOVERED_DIRS:
        prefix = f"{package}/{folder}/"
        held = sorted(p for p in tracked if p.startswith(prefix))
        if held:
            reads.append(prefix)
            reads.extend(p for p in held
                         if re.fullmatch(r"[^/]+/main\.rs", p[len(prefix):]))
    return reads


_READS_CACHE: dict[tuple[str, str], list[str]] = {}


def built_reads(repo_root: Path, name: str) -> list[str]:
    """What a gate that `builds` `name` reads for it beyond `built_inputs`:
    the workspace's load (`cargo_package_load_reads` of every package cargo
    loads) for a build as members of this workspace, and the whole of each
    crate the build compiles that has a build script. A pattern `built_inputs`
    or a wider one here already gives is left out. Empty for a build cargo does
    not run. See "What a build reads that does not select it"."""
    key = (str(repo_root.resolve()), name)
    if key not in _READS_CACHE:
        _, crates, in_workspace = BUILT_INPUTS[name]
        reads: list[str] = []
        if crates:
            if in_workspace:
                for package in cargo_loaded_packages(repo_root):
                    reads.extend(cargo_package_load_reads(repo_root, package))
            for crate in cargo_build_closure(repo_root, crates):
                if cargo_build_targets(repo_root, crate)[0] is not None:
                    reads.append(f"{crate}/**")
        built = built_inputs(repo_root, name)
        trees = [g for g in built + reads if g.endswith("/**")]
        # A listing is given by a tree that holds the directory's entries,
        # which is a tree that matches a name inside it.
        covering = [(t, glob_to_regex(t)) for t in trees]
        _READS_CACHE[key] = [
            p for p in dict.fromkeys(reads)
            if p not in built and not any(
                t != p and r.match(f"{p}x" if p.endswith("/") else p)
                for t, r in covering)]
    return list(_READS_CACHE[key])


def triggers_of(repo_root: Path, spec: dict) -> list[str]:
    """One gate's trigger globs, derived from its entry `spec`: its
    workflows' `paths:`, its own lists, what it builds and what the ledgers
    it names enrol.

    `narrows` is the one case where a gate does not inherit its workflow's
    trigger. A workflow with no `paths:` filter is telling CI "run this on
    every push", which is a statement about a runner that is paid for by the
    minute and not about what the check reads. Inheriting it locally makes
    the catch-all the trigger for the gate, so the whole conformance suite
    would run for a README edit. A narrowing gate states its real inputs in
    `local`/`extra` instead, and the self-test holds the two properties that
    keep the narrowing from becoming a coverage hole: the workflow it
    narrows must actually be unfiltered (or there is nothing to narrow), and
    the gate must be left with triggers of its own (or it never runs).
    """
    globs: list[str] = list(spec.get("local", []))
    for wf in spec.get("workflows", []):
        wf_globs = workflow_paths(repo_root, wf)
        if spec.get("narrows") and wf_globs == ["**"]:
            # The catch-all is dropped; the workflow FILE is not. Editing
            # a lane is a reason to run what that lane runs, and every
            # filtered workflow classifies its own edits by naming itself
            # in `paths:`. Without this line an unfiltered lane's file is
            # unclassified, so touching it buys the full suite — measured
            # at 27 gates including the 408s vendor smoke.
            globs.append(f".github/workflows/{wf}")
            continue
        globs.extend(wf_globs)
    globs.extend(spec.get("extra", []))
    for built in spec.get("builds", []):
        globs.extend(built_inputs(repo_root, built))
    if spec.get("enrolled_by"):
        # What the ledgers enrol, read from their configs on every call:
        # a tree enrolled tomorrow selects the gate on the same commit.
        globs.extend(enrolled_globs(repo_root, spec["enrolled_by"])
                     or ["**"])
    return list(dict.fromkeys(globs))


def reads_of(repo_root: Path, spec: dict) -> list[str]:
    """What one gate's run reads that does not select it, from its entry
    `spec`: `built_reads` of each shared build it names (`builds`), sorted.
    A gate keyed on its triggers is keyed on these too (`gate_inputs` in
    gate_table.py); see "What a build reads that does not select it"."""
    return sorted({p for built in spec.get("builds", [])
                   for p in built_reads(repo_root, built)})
