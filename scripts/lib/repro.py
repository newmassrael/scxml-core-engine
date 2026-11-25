#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The smallest command that reproduces what a gate found red, for every runner
# the gates drive.
#
#   scripts/lib/repro.py prepare --dir <dir> -- <command...>
#       the command to run in its place: the same one, asking its runner for
#       the machine-readable report this file reads afterwards (NUL-separated,
#       the first field the filter its output goes through, "go" or "-")
#   scripts/lib/repro.py render-go
#       "go test -json" on stdin, as the text "go test -v" prints, on stdout
#   scripts/lib/repro.py record --out <file> --log <file> [--dir <dir>]
#       [--since <epoch seconds>] [--cwd <dir>] [--first <command>]...
#       [--http-fixture <path>] [--changed <path>]... [--metadata <file>]
#       -- <command...>
#       append to <file> the commands that reproduce each failure of the
#       command, one JSON object a line, read from what its runner reported
#   scripts/lib/repro.py report --slug <slug> --file <file> --fallback <command>
#       [--summary <file>] [--note <text>]...
#       print those commands under the one heading, and add them to <file>
#   scripts/lib/repro.py --self-test <fixtures dir>
#
# WHY. A red gate used to end with "run just this one with: scripts/gate
# <slug>", which builds and runs the whole gate again, or a whole shard of it.
# The workspace suite is 1363 s, and the one failing test of a red shard is a
# few seconds of it (job 138951650414, 2026-11-25: one test of
# every_ledger_binding_names_what_its_file_holds failed, and the advice was the
# whole gate). Each runner names exactly what failed in a report of its own, so
# the gate can say the command that runs that and nothing else.
#
# WHAT IS READ, per runner, and always the runner's own report where it has one:
#   cargo test   libtest's "failures:" list of each test binary, and cargo's
#                "to rerun pass <selector>" for the binary (stable libtest has
#                no machine-readable output; cargo's selector is its own words)
#   ctest        its JUnit report (--output-junit, which prepare adds), else
#                Testing/Temporary/LastTestsFailed.log
#   go test      its -json events (prepare turns -v into -json, and render-go
#                gives the log back its -v text)
#   pytest       its JUnit report (--junitxml, which prepare adds)
#   unittest     its "FAIL:"/"ERROR:" lines, the only report it has
#   gradle       the test-results XML each test task writes
#   swift test   its xUnit report (--xunit-output, through
#                scripts/test_swift_backend.sh)
#   w3c_test_cli its JUnit report (--output)
#   scripts/mutate  its verdict lines, which its ledger importer reads too
#   anything else  the command itself, which is still less than the gate
#
# WHAT A COMMAND MUST BE: runnable from a clean checkout on a build machine,
# from the repository root, saying what it builds first. Paths under the
# checkout are written relative to where the command runs, the runner's own
# flags that change what is built (features, profile, target) are kept, and
# the ones that select a different set of tests are replaced by the one test.
#
# Recording never changes a gate's verdict: whatever goes wrong here is said
# and the command that failed is recorded as it was given.

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import time
import xml.etree.ElementTree as ElementTree
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# The one heading every red gate ends with. A reader and a test find the
# commands by it, so it is written here and nowhere else.
HEADING = "==> reproduce gate[{slug}]"

# More failures than this in one run are reproduced together by one command:
# a list of forty lines is read as noise, and the one command that runs the
# forty is what a reader wants by then.
MAX_COMMANDS = 12


# ── Commands ────────────────────────────────────────────────────────────────


def sh(argv) -> str:
    return " ".join(shlex.quote(str(a)) for a in argv)


def strip_ansi(text: str) -> str:
    return re.sub(r"\x1b\[[0-9;?]*[ -/]*[@-~]", "", text)


def plain_lines(text: str) -> list:
    return [strip_ansi(line).rstrip("\r") for line in text.splitlines()]


class Invocation:
    """What the gate ran: the environment it set, the command, and the
    directory it ran in, relative to the repository root."""

    def __init__(self, argv, cwd=".", first=None, root=REPO_ROOT):
        self.root = Path(root)
        self.cwd = self._relative_dir(cwd)
        self.env = []
        argv = list(argv)
        if argv and os.path.basename(argv[0]) == "env":
            argv = argv[1:]
        while argv and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", argv[0]):
            name, _, value = argv[0].partition("=")
            self.env.append((name, value))
            argv = argv[1:]
        self.argv = argv
        self.first = [self.first_command(c) for c in (first or []) if c and c.strip()]

    def first_command(self, command: str) -> str:
        """A command a gate said runs first, with its paths under the checkout
        written as the repository root sees them; one that holds shell
        operators is kept as the gate wrote it."""
        try:
            words = shlex.split(command)
        except ValueError:
            return command
        if not words or any(w in ("&&", "||", ";", "|", "&", ">", "<", ">>") for w in words):
            return command
        root = self.root
        relative = []
        for word in words:
            head, eq, tail = word.rpartition("=") if "=" in word and not word.startswith("/") else ("", "", word)
            path = tail
            if path.startswith("/"):
                try:
                    Path(path).resolve().relative_to(root.resolve())
                    path = os.path.relpath(path, str(root)) or "."
                except (ValueError, OSError):
                    pass
            relative.append(f"{head}{eq}{path}")
        return sh(relative)

    def _relative_dir(self, cwd) -> str:
        path = Path(cwd)
        if path.is_absolute():
            try:
                return os.path.relpath(str(path), str(self.root))
            except ValueError:
                return str(path)
        return os.path.normpath(str(path))

    def relative(self, token: str) -> str:
        """A path under the checkout, as the command's directory sees it."""
        if not token.startswith("/"):
            return token
        try:
            Path(token).resolve().relative_to(self.root.resolve())
        except (ValueError, OSError):
            return token
        return os.path.relpath(token, str(self.root / self.cwd))

    def token(self, token: str) -> str:
        if "=" in token and not token.startswith("/"):
            head, _, tail = token.partition("=")
            if tail.startswith("/"):
                return head + "=" + self.relative(tail)
            return token
        return self.relative(token)

    def env_value(self, value: str) -> str:
        return ":".join(self.relative(part) for part in value.split(":"))

    def assignment(self, name: str, value: str) -> str:
        # PATH is only ever prepended to by a gate: the directory it put first,
        # and the PATH of whoever runs the line after it.
        if name == "PATH":
            return f'PATH={shlex.quote(self.relative(value.split(":")[0]))}:"$PATH"'
        return f"{name}={shlex.quote(self.env_value(value))}"

    def line(self, argv=None, env=(), cwd=None, first=(), with_first=True) -> str:
        """One line a shell runs from the repository root."""
        cwd = self.cwd if cwd is None else cwd
        words = [self.assignment(name, value) for name, value in self.env]
        words += [f"{name}={shlex.quote(value)}" for name, value in env]
        words.append(sh(self.token(a) for a in (self.argv if argv is None else argv)))
        command = " ".join(words)
        if cwd not in (".", ""):
            command = f"(cd {shlex.quote(cwd)} && {command})"
        before = (list(self.first) if with_first else []) + list(first)
        # A command that runs the generator from the checkout's own build
        # builds it first, as the gate's dependency `codegen-build` did.
        text = " ".join(before + [command])
        if (_GENERATOR_BUILD not in text
                and re.search(r"(^|[\s=/'\"])target/(debug|release)/sce-codegen\b", text)):
            before = [_GENERATOR_BUILD] + before
        return " && ".join(before + [command])

    def resolve(self, path: str) -> Path:
        candidate = Path(path)
        if candidate.is_absolute():
            return candidate
        return self.root / self.cwd / candidate


# What builds the generator a command runs from target/ (scripts/gates/codegen-build.sh).
_GENERATOR_BUILD = "cargo build --bin sce-codegen --features cli -p sce-build"


def entry(what: str, builds: str, command: str) -> dict:
    return {"what": what, "builds": builds, "command": command}


def whole_command(inv: Invocation, why: str) -> list:
    return [entry(why, "what the command builds", inv.line())]


def program(argv) -> str:
    return os.path.basename(argv[0]) if argv else ""


def runner_of(argv) -> str:
    """Which reader a command's report goes to."""
    name = program(argv)
    rest = argv[1:]
    words = [a for a in rest if not a.startswith("-") and not a.startswith("+")]
    if name == "cargo":
        if words[:1] == ["test"] or words[:2] == ["miri", "test"]:
            return "cargo-test"
        if words[:1] and words[0] in ("build", "check", "clippy", "doc", "rustdoc"):
            return "cargo-build"
        return "command"
    if name == "ctest":
        return "ctest"
    if name == "cmake" and "--build" in rest:
        return "cmake-build"
    if name == "go" and words[:1] == ["test"]:
        return "go"
    if name.startswith("python") and "-m" in rest:
        module = rest[rest.index("-m") + 1] if rest.index("-m") + 1 < len(rest) else ""
        if module == "pytest":
            return "pytest"
        if module == "unittest":
            return "unittest"
        return "command"
    if name in ("gradlew", "gradle"):
        return "gradle"
    if name == "test_swift_backend.sh":
        return "swift"
    if name == "w3c_test_cli" or (name == "node" and rest[:1]
                                  and os.path.basename(rest[0]) == "w3c_test_cli.js"):
        return "w3c"
    if name == "mutate":
        return "mutate"
    return "command"


# ── prepare: ask the runner for its report ──────────────────────────────────


def prepare(argv: list, directory: str) -> tuple:
    """The command to run in place of argv, and the filter its output takes."""
    out_dir = Path(directory).resolve()
    inv = Invocation(argv)
    env_words = [f"{n}={v}" for n, v in inv.env]
    command = list(inv.argv)
    kind = runner_of(command)
    render = "-"
    if kind == "ctest" and not any(a == "--output-junit" or a.startswith("--output-junit=")
                                   for a in command):
        command += ["--output-junit", str(out_dir / "ctest-junit.xml")]
    elif kind == "pytest":
        command += [f"--junitxml={out_dir / 'pytest-junit.xml'}", "-o", "junit_family=xunit1"]
    elif kind == "go" and "-json" not in command and ("-v" in command or "-v=true" in command):
        # -json is -v's text with each line an event, so a run that prints
        # -v loses nothing by it. A run without -v keeps its quiet output, and
        # its failures are read from the "--- FAIL" lines that output keeps.
        at = command.index("test") + 1
        command = command[:at] + ["-json"] + [a for a in command[at:] if a not in ("-v", "-v=true")]
        render = "go"
    elif kind == "swift":
        swift_dir = out_dir / "swift"
        swift_dir.mkdir(parents=True, exist_ok=True)
        env_words.append(f"SCE_SWIFT_XUNIT_DIR={swift_dir}")
    if env_words:
        command = ["env"] + env_words + command
    return render, command


def render_go(stream_in, stream_out) -> int:
    """The text of a "go test -json" stream: every event's Output in order,
    and any line that is no event as it is (the build's own errors)."""
    for raw in stream_in:
        text = raw.rstrip("\n")
        event = None
        if text.startswith("{"):
            try:
                event = json.loads(text)
            except ValueError:
                event = None
        if isinstance(event, dict):
            output = event.get("Output")
            if isinstance(output, str):
                stream_out.write(output)
                stream_out.flush()
            continue
        stream_out.write(raw if raw.endswith("\n") else raw + "\n")
        stream_out.flush()
    return 0


# ── cargo ───────────────────────────────────────────────────────────────────

_RUNNING = re.compile(r"^\s*Running (?:(unittests) )?(\S+) \((.+)\)\s*$")
_DOCTESTS = re.compile(r"^\s*Doc-tests (\S+)\s*$")
_RERUN = re.compile(r"to rerun pass [\x60'](.+?)[\x60']")
_TARGETS_FAILED = re.compile(r"^\s*error: \d+ targets? failed:\s*$")
_LISTED_TARGET = re.compile(r"^\s+[\x60'](-.+?)[\x60']\s*$")
_COULD_NOT_COMPILE = re.compile(r"error: could not (?:compile|document) [\x60']([^\x60']+)[\x60'](?: \(([^)]*)\))?")
_TEST_FAILED = re.compile(r"^test (.+) \.\.\. FAILED\s*$")
_RESULT = re.compile(r"^test result: (ok|FAILED)\.")

# The flags of cargo that choose which packages and targets run, and so are
# replaced by the one target that failed; every other flag is kept, since it
# changes what is built (features, profile, target, configuration).
_CARGO_SELECT = {"--workspace", "--all", "--lib", "--bins", "--tests", "--examples",
                 "--benches", "--all-targets", "--doc", "--no-fail-fast", "--no-run"}
_CARGO_SELECT_VALUE = {"-p", "--package", "--exclude", "--bin", "--test", "--example", "--bench"}
_CARGO_PACKAGE_SELECT = {"--workspace", "--all"}
_CARGO_PACKAGE_SELECT_VALUE = {"-p", "--package", "--exclude"}
_CARGO_VALUE = {"--features", "-F", "--target", "--target-dir", "--profile", "--manifest-path",
                "--config", "-Z", "--color", "--message-format", "-j", "--jobs",
                "--lockfile-path", "--artifact-dir"}
# libtest's own flags that take a value; a word after "--" that is no flag
# is a filter, and the filter is what this file replaces.
_LIBTEST_VALUE = {"--test-threads", "--skip", "--logfile", "--format", "--color", "-Z",
                  "--shuffle-seed"}
_LIBTEST_DROP = {"--exact", "--list"}


class Segment:
    """The output of one test binary, from cargo's "Running" line to the next."""

    def __init__(self, kind: str, src: str, stem: str):
        self.kind = kind
        self.src = src
        self.stem = stem
        self.names: list = []
        self.failed_lines: list = []
        self.failed = False
        self.used = False

    def failures(self) -> list:
        return self.names or self.failed_lines


def _stem(artifact: str) -> str:
    base = os.path.basename(artifact.strip())
    if base.endswith(".exe"):
        base = base[:-4]
    return re.sub(r"-[0-9a-f]{16}$", "", base)


def _segment_kind(unittests: bool, src: str) -> str:
    src = src.replace("\\", "/")
    if unittests:
        return "bin" if src.endswith("main.rs") or "/bin/" in src else "lib"
    for prefix, kind in (("benches/", "bench"), ("examples/", "example")):
        if src.startswith(prefix) or ("/" + prefix) in src:
            return kind
    return "test"


def parse_libtest(text: str) -> dict:
    """The binaries a cargo test run reported on, what failed in each, cargo's
    own selector for each binary that failed, and the crates that did not
    compile."""
    segments: list = []
    hints: list = []
    broken: list = []
    current = None
    names = None
    listing = False
    for line in plain_lines(text):
        running = _RUNNING.match(line)
        doctests = _DOCTESTS.match(line)
        if running or doctests:
            if running:
                current = Segment(_segment_kind(bool(running.group(1)), running.group(2)),
                                  running.group(2), _stem(running.group(3)))
            else:
                current = Segment("doc", "", doctests.group(1))
            segments.append(current)
            names = None
            listing = False
            continue
        for hint in _RERUN.findall(line):
            if hint not in hints:
                hints.append(hint)
        if _TARGETS_FAILED.match(line):
            listing = True
            continue
        if listing:
            listed = _LISTED_TARGET.match(line)
            if listed:
                if listed.group(1) not in hints:
                    hints.append(listed.group(1))
                continue
            listing = False
        broke = _COULD_NOT_COMPILE.search(line)
        if broke and (broke.group(1), broke.group(2) or "") not in broken:
            broken.append((broke.group(1), broke.group(2) or ""))
        if current is None:
            continue
        if line.strip() == "failures:":
            names = []
            continue
        if line.startswith("---- "):
            names = None
            continue
        result = _RESULT.match(line)
        if result:
            if result.group(1) == "FAILED":
                current.failed = True
                if names:
                    current.names = names
            names = None
            continue
        failed = _TEST_FAILED.match(line)
        if failed:
            current.failed_lines.append(failed.group(1))
            continue
        if names is not None and line.startswith("    ") and line.strip():
            names.append(line.strip())
    return {"segments": segments, "hints": hints, "broken": broken}


class Metadata:
    """What "cargo metadata --no-deps" says of the workspace's packages: the
    features each declares, the dependencies it names, its library's crate
    name and its targets."""

    def __init__(self, data: dict | None):
        self.packages: dict = {}
        for package in (data or {}).get("packages", []):
            deps = set()
            for dep in package.get("dependencies", []):
                deps.add(dep.get("rename") or dep.get("name"))
            lib = None
            targets = []
            for target in package.get("targets", []):
                kinds = target.get("kind", [])
                targets.append((kinds, target.get("name"), target.get("src_path", "")))
                if lib is None and any(k in ("lib", "rlib", "dylib", "cdylib", "staticlib",
                                             "proc-macro") for k in kinds):
                    lib = target.get("name", "").replace("-", "_")
            self.packages[package.get("name")] = {
                "features": set(package.get("features", {}).keys()),
                "deps": deps,
                "lib": lib,
                "targets": targets,
            }

    def known(self) -> bool:
        return bool(self.packages)

    def lib_name(self, package: str | None) -> str | None:
        if package and package in self.packages:
            return self.packages[package]["lib"]
        return package.replace("-", "_") if package else None

    def features_for(self, package: str | None, features: list) -> list:
        """The features of the run that mean something to the one package: a
        feature it declares, or one of a dependency it names. Asked for with
        -p, cargo refuses a feature the package does not have."""
        if not package or package not in self.packages:
            return list(features)
        info = self.packages[package]
        kept = []
        for feature in features:
            if "/" in feature:
                owner = feature.split("/", 1)[0].rstrip("?")
                if owner in info["deps"] or owner == package:
                    kept.append(feature)
            elif feature in info["features"]:
                kept.append(feature)
        return kept

    def package_of(self, kind: str, stem: str, src: str) -> str | None:
        """The package one test binary belongs to, when cargo's selector did
        not name it."""
        found = []
        for name, info in self.packages.items():
            if kind in ("lib", "doc"):
                if info["lib"] == stem:
                    found.append(name)
                continue
            want = {"test": "test", "bin": "bin", "bench": "bench", "example": "example"}[kind]
            for kinds, target, path in info["targets"]:
                if want in kinds and target.replace("-", "_") == stem.replace("-", "_") and (
                        not src or path.replace("\\", "/").endswith(src)):
                    found.append(name)
        return found[0] if len(found) == 1 else None


def read_metadata(inv: Invocation, cargo_head: list, manifest: str | None) -> Metadata:
    argv = list(cargo_head) + ["metadata", "--no-deps", "--format-version", "1"]
    if manifest:
        argv += ["--manifest-path", manifest]
    env = dict(os.environ)
    env.update({name: value for name, value in inv.env})
    try:
        done = subprocess.run(argv, cwd=str(inv.root / inv.cwd), env=env, capture_output=True,
                              text=True, timeout=120, check=False)
    except (OSError, subprocess.SubprocessError):
        return Metadata(None)
    if done.returncode != 0:
        return Metadata(None)
    try:
        return Metadata(json.loads(done.stdout))
    except ValueError:
        return Metadata(None)


def split_cargo(argv: list) -> dict:
    """A cargo command in its parts: the program and toolchain, the
    subcommand, the flags kept, the features, the packages it named, and
    libtest's own flags after "--"."""
    head = [argv[0]]
    rest = list(argv[1:])
    while rest and rest[0].startswith("+"):
        head.append(rest.pop(0))
    sub = []
    if rest[:2] == ["miri", "test"]:
        sub, rest = rest[:2], rest[2:]
    elif rest:
        sub, rest = rest[:1], rest[1:]
    test_args: list = []
    if "--" in rest:
        at = rest.index("--")
        rest, test_args = rest[:at], rest[at + 1:]
    kept, features, packages, select = [], [], [], []
    manifest = None
    i = 0
    while i < len(rest):
        word = rest[i]
        name, eq, value = word.partition("=")
        if not word.startswith("-"):
            i += 1
            continue
        if word.startswith("-p") and word not in ("-p",) and not word.startswith("--"):
            name, eq, value = "-p", "=", word[2:]
        takes = name in _CARGO_VALUE or name in _CARGO_SELECT_VALUE
        if takes and not eq:
            value = rest[i + 1] if i + 1 < len(rest) else ""
            i += 1
        i += 1
        if name in ("--features", "-F"):
            features += [f for f in re.split(r"[ ,]+", value) if f]
        elif name in ("-p", "--package"):
            packages.append(value)
            select.append((name, value))
        elif name in _CARGO_SELECT_VALUE:
            select.append((name, value))
        elif name in _CARGO_SELECT:
            select.append((name, None))
        else:
            if name == "--manifest-path":
                manifest = value
            kept += [name, value] if takes else [word]
    libtest = []
    i = 0
    while i < len(test_args):
        word = test_args[i]
        name = word.partition("=")[0]
        if not word.startswith("-"):
            i += 1
            continue
        has_value = name in _LIBTEST_VALUE and "=" not in word
        if name == "--skip" or name in _LIBTEST_DROP:
            i += 2 if has_value else 1
            continue
        libtest += test_args[i:i + 2] if has_value else [word]
        i += 2 if has_value else 1
    return {"head": head, "sub": sub, "kept": kept, "features": features,
            "packages": packages, "select": select, "libtest": libtest, "manifest": manifest}


def _feature_args(features: list) -> list:
    return ["--features", ",".join(features)] if features else []


def _selector(tokens: list) -> tuple:
    """A cargo selector ("-p X --test Y") as its package and its target."""
    package, kind, name = None, None, None
    i = 0
    while i < len(tokens):
        word = tokens[i]
        nxt = tokens[i + 1] if i + 1 < len(tokens) else None
        if word in ("-p", "--package"):
            package, i = nxt, i + 2
            continue
        if word in ("--lib", "--doc"):
            kind, i = word[2:], i + 1
            continue
        if word in ("--test", "--bin", "--example", "--bench"):
            kind, name, i = word[2:], nxt, i + 2
            continue
        i += 1
    return package, kind, name


def _selector_args(package, kind, name) -> list:
    args = ["-p", package] if package else ["--workspace"]
    if kind in ("lib", "doc"):
        args.append("--" + kind)
    elif kind:
        args += ["--" + kind, name]
    return args


def _describe(package, kind, name) -> str:
    owner = package or "the workspace"
    if kind == "lib":
        return f"{owner} library unit tests"
    if kind == "doc":
        return f"{owner} doc tests"
    return f"{owner} {kind} target {name}"


def cargo_test_entries(inv: Invocation, text: str, meta: Metadata | None) -> list:
    parts = split_cargo(inv.argv)
    parsed = parse_libtest(text)
    if meta is None:
        meta = read_metadata(inv, parts["head"], parts["manifest"])
    release = "--release" in parts["kept"]
    named = parts["packages"][0] if len(parts["packages"]) == 1 else None
    plan = []

    def matches(segment, package, kind, name) -> bool:
        if segment.kind != kind or segment.used:
            return False
        if kind in ("lib", "doc"):
            return package is None or segment.stem == meta.lib_name(package)
        return segment.stem == name

    for hint in parsed["hints"]:
        try:
            package, kind, name = _selector(shlex.split(hint))
        except ValueError:
            continue
        if kind is None:
            continue
        package = package or named
        chosen = None
        for segment in parsed["segments"]:
            if matches(segment, package, kind, name) and (segment.failed or segment.failures()):
                chosen = segment
                break
        if chosen is None:
            for segment in parsed["segments"]:
                if matches(segment, package, kind, name):
                    chosen = segment
                    break
        if chosen is not None:
            chosen.used = True
        plan.append((package, kind, name, chosen.failures() if chosen else []))
    for segment in parsed["segments"]:
        if segment.used or not (segment.failed or segment.failed_lines):
            continue
        package = named or meta.package_of(segment.kind, segment.stem, segment.src)
        if segment.kind in ("lib", "doc") and package is None:
            continue
        name = None if segment.kind in ("lib", "doc") else segment.stem
        plan.append((package, segment.kind, name, segment.failures()))

    entries = []
    base = parts["head"] + parts["sub"]
    for package, kind, name, failures in plan:
        features = meta.features_for(package, parts["features"])
        select = _selector_args(package, kind, name)
        prefix = base + parts["kept"] + _feature_args(features) + select
        target = _describe(package, kind, name)
        builds = f"{target}{' (release)' if release else ''} and what it depends on"
        if failures and len(failures) <= MAX_COMMANDS:
            for test in failures:
                entries.append(entry(f"{test} ({target})", builds,
                                     inv.line(prefix + ["--", test, "--exact"] + parts["libtest"])))
        else:
            tail = ["--"] + parts["libtest"] if parts["libtest"] else []
            what = (f"{len(failures)} failing tests of {target}" if failures
                    else f"{target}: the binary failed without naming a test")
            entries.append(entry(what, builds, inv.line(prefix + tail)))
    if entries:
        return entries
    if parsed["broken"]:
        return cargo_broken_entries(inv, parts, parsed["broken"], meta, test=True)
    return whole_command(inv, "the cargo command (it named no failing test or crate)")


def _broken_selector(detail: str) -> list:
    """cargo's "(lib test)", "(test \"x\")", "(bin \"x\")" as target flags."""
    found = re.match(r'^(lib|bin|test|example|bench)(?: "([^"]+)")?', detail or "")
    if not found:
        return []
    kind, name = found.group(1), found.group(2)
    if kind == "lib":
        return ["--lib"]
    return ["--" + kind, name] if name else []


def cargo_broken_entries(inv: Invocation, parts: dict, broken: list, meta: Metadata,
                         test: bool) -> list:
    """The crates that did not compile, each built alone with the run's
    flags, where the crate is a workspace member."""
    entries = []
    base = parts["head"] + parts["sub"]
    for crate, detail in broken:
        if meta.known() and crate not in meta.packages:
            continue
        features = meta.features_for(crate, parts["features"])
        if test:
            argv = base + ["--no-run"] + parts["kept"] + _feature_args(features) + ["-p", crate]
            argv += _broken_selector(detail)
        else:
            targets = []
            for flag, value in parts["select"]:
                if flag in _CARGO_PACKAGE_SELECT or flag in _CARGO_PACKAGE_SELECT_VALUE:
                    continue
                targets += [flag] if value is None else [flag, value]
            argv = base + parts["kept"] + _feature_args(features) + ["-p", crate] + targets
            if parts["libtest"] or "--" in inv.argv:
                argv += ["--"] + inv.argv[inv.argv.index("--") + 1:]
        what = f"{crate} ({detail})" if detail else crate
        command = inv.line(argv)
        if all(e["command"] != command for e in entries):
            entries.append(entry(f"{what} does not compile", f"{crate} with the gate's flags",
                                 command))
    return entries or whole_command(inv, "the cargo command (what failed to compile is no workspace member)")


def cargo_build_entries(inv: Invocation, text: str, meta: Metadata | None) -> list:
    parts = split_cargo(inv.argv)
    broken = parse_libtest(text)["broken"]
    if not broken:
        return whole_command(inv, "the cargo command (it named no crate that failed)")
    if meta is None:
        meta = read_metadata(inv, parts["head"], parts["manifest"])
    return cargo_broken_entries(inv, parts, broken, meta, test=False)


# ── ctest ───────────────────────────────────────────────────────────────────


def _load_ctest_shards():
    """scripts/lib/ctest_shards.py, which knows what a test needs built."""
    import importlib.util
    path = Path(__file__).resolve().parent / "ctest_shards.py"
    spec = importlib.util.spec_from_file_location("sce_ctest_shards", str(path))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _option(argv: list, *names: str) -> str | None:
    for i, word in enumerate(argv):
        for name in names:
            if word == name and i + 1 < len(argv):
                return argv[i + 1]
            if word.startswith(name + "="):
                return word[len(name) + 1:]
    return None


def junit_failures(path: Path) -> list:
    """The <testcase> elements of a JUnit report that did not pass."""
    try:
        root = ElementTree.parse(str(path)).getroot()
    except (OSError, ElementTree.ParseError):
        return []
    failed = []
    for case in root.iter("testcase"):
        status = (case.get("status") or "").lower()
        bad = case.find("failure") is not None or case.find("error") is not None
        if bad or status in ("fail", "failed", "notrun"):
            failed.append(case)
    return failed


def _fresh(path: Path, since: float | None) -> bool:
    try:
        return path.is_file() and (since is None or path.stat().st_mtime >= since - 1)
    except OSError:
        return False


def ctest_failures(inv: Invocation, directory: str | None, since: float | None) -> list:
    argv = inv.argv
    test_dir = inv.resolve(_option(argv, "--test-dir") or ".")
    candidates = []
    junit = _option(argv, "--output-junit")
    if junit:
        candidates += [test_dir / junit, inv.resolve(junit)]
    if directory:
        candidates.append(Path(directory) / "ctest-junit.xml")
    for candidate in candidates:
        if _fresh(candidate, since):
            names = [case.get("name") for case in junit_failures(candidate) if case.get("name")]
            if names:
                return names
    last = test_dir / "Testing" / "Temporary" / "LastTestsFailed.log"
    if _fresh(last, since):
        names = []
        for line in last.read_text(encoding="utf-8", errors="replace").splitlines():
            number, sep, name = line.partition(":")
            if sep and number.strip().isdigit() and name.strip():
                names.append(name.strip())
        return names
    return []


def ctest_entries(inv: Invocation, directory: str | None, since: float | None,
                  closure_reader=None) -> list:
    names = ctest_failures(inv, directory, since)
    if not names:
        return whole_command(inv, "the ctest run (it reported no failing test by name)")
    test_dir_word = _option(inv.argv, "--test-dir") or "."
    test_dir = inv.resolve(test_dir_word)
    config = _option(inv.argv, "-C", "--build-config")
    kept = ["-C", config] if config else []
    for flag in ("--timeout", "--repeat"):
        value = _option(inv.argv, flag)
        if value:
            kept += [flag, value]
    unloaded = ""
    if closure_reader is None:
        try:
            closure_reader = _load_ctest_shards().ClosureReader().closure
        except Exception as error:  # the module is ours; a fault in it is said, not fatal
            unloaded = f"scripts/lib/ctest_shards.py could not be loaded: {error}"
    groups = [[n] for n in names] if len(names) <= MAX_COMMANDS else [names]
    entries = []
    for group in groups:
        closure, reason = None, unloaded
        if closure_reader is not None:
            try:
                closure = closure_reader(str(test_dir), group)
            except Exception as error:  # CannotPlan, and anything the tools said
                reason = str(error)
        regex = "^(" + "|".join(re.escape(n) for n in group) + ")$"
        run = [program(inv.argv), "--test-dir", test_dir_word, "-R", regex]
        if closure and closure.get("fixtures"):
            run += ["--fixture-exclude-setup",
                    "^(" + "|".join(re.escape(f) for f in closure["fixtures"]) + ")$"]
        run += kept + ["--output-on-failure"]
        build = ["cmake", "--build", test_dir_word]
        if config:
            build += ["--config", config]
        if closure is not None:
            targets = closure.get("targets", [])
            builds = (f"the {len(targets)} output(s) the test runs and what they are built from"
                      if targets else "nothing: the test runs no built program")
            first = [inv.line(build + ["--target"] + targets, with_first=False)] if targets else []
        else:
            builds = (f"the whole tree in {test_dir_word}, since the test's closure could not be "
                      f"read ({reason or 'no reader'})")
            first = [inv.line(build, with_first=False)]
        what = group[0] if len(group) == 1 else f"{len(group)} failing ctest tests"
        entries.append(entry(what, builds, inv.line(run, first=first)))
    return entries


# ── go test ─────────────────────────────────────────────────────────────────

_GO_VALUE = {"-run", "-count", "-tags", "-timeout", "-p", "-parallel", "-cpu", "-bench",
             "-benchtime", "-coverprofile", "-skip", "-o", "-exec", "-covermode", "-coverpkg"}
_GO_DROP = {"-v", "-json", "-run", "-skip"}
_GO_FAIL_TEST = re.compile(r"^\s*--- FAIL: (\S+)")
_GO_FAIL_PACKAGE = re.compile(r"^FAIL\s+(\S+)(?:\s+\[[^\]]*\]|\s+[0-9.]+s)?\s*$")


def go_failures_from_events(text: str) -> dict:
    """Package to its failing tests, from "go test -json"; a package that
    failed with no failing test (it did not build, or its TestMain failed)
    maps to an empty list."""
    failed: dict = {}
    for line in text.splitlines():
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except ValueError:
            continue
        action = event.get("Action")
        package = event.get("Package") or event.get("ImportPath")
        if action not in ("fail", "build-fail") or not package:
            continue
        failed.setdefault(package, [])
        test = event.get("Test")
        if test and test not in failed[package]:
            failed[package].append(test)
    return failed


def go_failures_from_text(text: str) -> dict:
    failed: dict = {}
    pending: list = []
    for line in plain_lines(text):
        test = _GO_FAIL_TEST.match(line)
        if test:
            if test.group(1) not in pending:
                pending.append(test.group(1))
            continue
        package = _GO_FAIL_PACKAGE.match(line)
        if package:
            failed.setdefault(package.group(1), []).extend(t for t in pending)
            pending = []
    return failed


def _deepest(tests: list) -> list:
    """The failing tests, less each one that fails only because a subtest of
    it did."""
    return [t for t in tests if not any(o != t and o.startswith(t + "/") for o in tests)]


def _go_module(start: Path) -> tuple:
    for directory in [start] + list(start.parents):
        mod = directory / "go.mod"
        if mod.is_file():
            for line in mod.read_text(encoding="utf-8", errors="replace").splitlines():
                found = re.match(r"^module\s+(\S+)", line.strip())
                if found:
                    return directory, found.group(1)
            return directory, None
    return None, None


def go_entries(inv: Invocation, text: str, directory: str | None) -> list:
    events = ""
    if directory and (Path(directory) / "go.json").is_file():
        events = (Path(directory) / "go.json").read_text(encoding="utf-8", errors="replace")
    failed = go_failures_from_events(events or text)
    if not failed:
        failed = go_failures_from_text(text)
    if not failed:
        return whole_command(inv, "the go test run (it reported no failing test by name)")
    argv = inv.argv
    at = argv.index("test") + 1
    kept = []
    i = at
    while i < len(argv):
        word = argv[i]
        name = word.partition("=")[0]
        has_value = name in _GO_VALUE and "=" not in word
        if word.startswith("-") and name not in _GO_DROP:
            kept += argv[i:i + 2] if has_value else [word]
        i += 2 if (word.startswith("-") and has_value) else 1
    module_dir, module = _go_module(inv.root / inv.cwd)
    cwd = inv.cwd
    if module_dir is not None:
        cwd = os.path.relpath(str(module_dir), str(inv.root))
    entries = []
    for package, tests in failed.items():
        if module and (package == module or package.startswith(module + "/")):
            where = "." if package == module else "./" + package[len(module) + 1:]
        else:
            where = package
        tests = _deepest(tests)
        if not tests:
            entries.append(entry(f"{package} (it failed with no test named)",
                                 f"the package {package} and its tests",
                                 inv.line(argv[:at] + kept + [where], cwd=cwd)))
            continue
        groups = [[t] for t in tests] if len(tests) <= MAX_COMMANDS else [tests]
        for group in groups:
            if len(group) == 1:
                pattern = "/".join("^" + re.escape(part) + "$" for part in group[0].split("/"))
                what = f"{group[0]} ({package})"
            else:
                pattern = "^(" + "|".join(re.escape(t.split("/")[0]) for t in group) + ")$"
                what = f"{len(group)} failing tests of {package}"
            entries.append(entry(what, f"the package {package} and its tests",
                                 inv.line(argv[:at] + kept + ["-run", pattern, where], cwd=cwd)))
    return entries


# ── pytest and unittest ─────────────────────────────────────────────────────

_PYTEST_DROP_VALUE = {"-k", "-m", "-o", "--junitxml", "--junit-xml", "--deselect"}


def pytest_entries(inv: Invocation, directory: str | None, since: float | None) -> list:
    report = Path(directory) / "pytest-junit.xml" if directory else None
    if report is None or not _fresh(report, since):
        return whole_command(inv, "the pytest run (it left no JUnit report)")
    cases = junit_failures(report)
    if not cases:
        return whole_command(inv, "the pytest run (its report names no failing test)")
    argv = inv.argv
    at = argv.index("-m") + 2
    head = argv[:at]
    kept, paths = [], []
    i = at
    while i < len(argv):
        word = argv[i]
        name = word.partition("=")[0]
        if word.startswith("-"):
            if name in _PYTEST_DROP_VALUE:
                i += 1 if "=" in word else 2
                continue
            kept.append(word)
        else:
            paths.append(word)
        i += 1
    run_dir = inv.root / inv.cwd
    ids = []
    for case in cases:
        node = _pytest_node(case, run_dir, paths, inv.root)
        if node and node not in ids:
            ids.append(node)
    if not ids:
        return whole_command(inv, "the pytest run (its report names no file for a failing test)")
    if len(ids) > MAX_COMMANDS:
        return [entry(f"{len(ids)} failing pytest tests", "nothing: pytest runs the sources",
                      inv.line(head + kept + ids))]
    return [entry(node, "nothing: pytest runs the sources", inv.line(head + kept + [node]))
            for node in ids]


def _pytest_node(case, run_dir: Path, paths: list, root: Path) -> str | None:
    """A JUnit testcase (junit_family=xunit1) as the node id pytest takes."""
    file_attr = case.get("file")
    name = case.get("name") or ""
    classname = case.get("classname") or ""
    if not file_attr:
        return None
    module = file_attr[:-3].replace("/", ".") if file_attr.endswith(".py") else file_attr
    chain = []
    if classname.startswith(module + "."):
        chain = classname[len(module) + 1:].split(".")
    run_dir = run_dir.resolve()
    top = root.resolve()
    bases = [run_dir]
    for path in paths:
        start = (run_dir / path).resolve()
        for base in [start] + list(start.parents):
            bases.append(base)
            if base == top:
                break
    found = next((b / file_attr for b in bases if (b / file_attr).is_file()), None)
    where = os.path.relpath(str(found), str(run_dir)) if found else file_attr
    return "::".join([where] + chain + [name])


_UNITTEST_LINE = re.compile(r"^(?:FAIL|ERROR): (\S+) \(([^)]+)\)")


def unittest_entries(inv: Invocation, text: str) -> list:
    ids = []
    for line in plain_lines(text):
        found = _UNITTEST_LINE.match(line)
        if not found:
            continue
        name, where = found.group(1), found.group(2)
        if where.startswith("unittest.loader._FailedTest"):
            identifier = name
        elif where == name or where.endswith("." + name):
            identifier = where
        elif name in ("setUpClass", "tearDownClass", "setUpModule", "tearDownModule"):
            identifier = where
        else:
            identifier = where + "." + name
        if identifier not in ids:
            ids.append(identifier)
    if not ids:
        return whole_command(inv, "the unittest run (it named no failing test)")
    argv = inv.argv
    at = argv.index("-m") + 2
    flags = [a for a in argv[at:] if a.startswith("-")]
    groups = [[i] for i in ids] if len(ids) <= MAX_COMMANDS else [ids]
    return [entry(group[0] if len(group) == 1 else f"{len(group)} failing unittest tests",
                  "nothing: unittest runs the sources", inv.line(argv[:at] + flags + group))
            for group in groups]


# ── Gradle ──────────────────────────────────────────────────────────────────

def gradle_reports(inv: Invocation, directories: list, since: float | None) -> list:
    """The test-results XML each named test task wrote since the run began,
    <project>/build/test-results/<task>/TEST-*.xml, as (project, task,
    report). The gate names the directories: it knows the projects it runs,
    and a search of the tree would read far past what its key names."""
    found = []
    for directory in directories:
        results = inv.resolve(directory) if not Path(directory).is_absolute() else Path(directory)
        if not results.is_dir():
            continue
        project = results.parent.parent.parent
        for report in sorted(results.glob("TEST-*.xml")):
            if _fresh(report, since):
                found.append((project, results.name, report))
    return found


def _gradle_filter(classname: str, name: str) -> str:
    """The --tests pattern of one JUnit testcase: the class and method, the
    method's parameters and display suffix replaced by a wildcard."""
    method = name[:-2] if name.endswith("()") else name
    cut = re.search(r"[(\[]", method)
    if cut:
        method = method[:cut.start()]
    # A Kotlin name in backticks holds spaces and commas, which a filter
    # cannot spell; each run of them is matched by a wildcard instead.
    pattern = re.sub(r"[^A-Za-z0-9_$]+", "*", method).strip("*")
    if cut and pattern:
        pattern += "*"
    return f"{classname}.{pattern}" if pattern else classname


def gradle_entries(inv: Invocation, directories: list, since: float | None) -> list:
    reports = gradle_reports(inv, directories, since)
    argv = inv.argv
    tasks = [a for a in argv[1:] if a.startswith(":") or (not a.startswith("-") and a)]
    # The project properties the run was given, less those naming a path the
    # gate made for itself outside the checkout (a census file, a generated
    # overlay, a heap log): gone when the gate ended, so no clean checkout
    # has them. A property left out is said in each entry.
    flags, left_out = [], []
    for word in argv[1:]:
        if word in ("--console=plain", "--no-daemon", "--offline"):
            flags.append(word)
        elif word.startswith("-P"):
            value = word.partition("=")[2]
            if value.startswith("/") and inv.relative(value) == value:
                left_out.append(word.partition("=")[0])
            else:
                flags.append(word)
    unsaid = f" (without {', '.join(left_out)}, which named a directory of the gate's own)" \
        if left_out else ""
    project = _option(argv, "-p", "--project-dir")
    entries = []
    for project_dir, task_name, report in reports:
        for case in junit_failures(report):
            classname = case.get("classname") or ""
            name = case.get("name") or ""
            if not classname:
                continue
            named = [t for t in tasks if t.split(":")[-1] == task_name]
            if len(named) == 1:
                selection = [named[0]]
            elif project:
                selection = ["-p", project, task_name]
            else:
                selection = ["-p", os.path.relpath(str(project_dir), str(inv.root / inv.cwd)),
                             task_name]
            pattern = _gradle_filter(classname, name)
            command = inv.line([argv[0]] + flags + selection + ["--tests", pattern])
            if all(e["command"] != command for e in entries):
                entries.append(entry(f"{classname}.{name} ({task_name}){unsaid}",
                                     f"what the task {selection[-1]} compiles, then that test",
                                     command))
    if not entries:
        return whole_command(inv, "the Gradle run (no test-results XML names a failing test)")
    if len(entries) > MAX_COMMANDS:
        return entries[:MAX_COMMANDS] + whole_command(
            inv, f"the whole Gradle run, for the {len(entries) - MAX_COMMANDS} failures not listed")
    return entries


# ── swift test, through scripts/test_swift_backend.sh ───────────────────────


def swift_entries(inv: Invocation, directory: str | None) -> list:
    swift_dir = Path(directory) / "swift" if directory else None
    entries = []
    if swift_dir is not None and swift_dir.is_dir():
        for marker in sorted(swift_dir.glob("run-*.package")):
            stem = marker.name[:-len(".package")]
            package_dir, first, env = None, [], []
            for line in marker.read_text(encoding="utf-8", errors="replace").splitlines():
                key, _, value = line.partition(" ")
                if key == "dir":
                    package_dir = value
                elif key == "first":
                    first.append(value)
                elif key == "env":
                    name, _, setting = value.partition("=")
                    if setting.startswith("/") and package_dir:
                        # A path under the checkout, as the package's directory sees it.
                        try:
                            Path(setting).resolve().relative_to(inv.root.resolve())
                            setting = os.path.relpath(setting, str(inv.root / package_dir))
                        except (ValueError, OSError):
                            pass
                    env.append((name, setting))
            if not package_dir:
                continue
            reports = [swift_dir / f"{stem}.xml", swift_dir / f"{stem}-swift-testing.xml"]
            for report in (r for r in reports if r.is_file()):
                for case in junit_failures(report):
                    classname = case.get("classname") or ""
                    name = case.get("name") or ""
                    identifier = f"{classname}/{name}" if classname else name
                    pattern = "^" + re.escape(identifier) + "$"
                    command = inv.line(["swift", "test", "--filter", pattern], env=env,
                                       cwd=package_dir, first=first)
                    if all(e["command"] != command for e in entries):
                        entries.append(entry(f"{identifier} ({package_dir})",
                                             f"the Swift package {package_dir} and its tests",
                                             command))
    if not entries:
        return whole_command(inv, "the Swift run (no xUnit report names a failing test)")
    return entries[:MAX_COMMANDS] + (whole_command(inv, "the whole Swift run")
                                     if len(entries) > MAX_COMMANDS else [])


# ── The W3C runner ──────────────────────────────────────────────────────────


def w3c_entries(inv: Invocation, since: float | None) -> list:
    argv = inv.argv
    report = inv.resolve(_option(argv, "--output") or "w3c_test_results.xml")
    if not _fresh(report, since):
        return whole_command(inv, "the W3C run (it left no report)")
    by_id: dict = {}
    for case in junit_failures(report):
        test_id = case.get("id") or (case.get("name") or "").replace("Test_", "").split("@")[0]
        if not test_id:
            continue
        engine = "aot" if case.get("classname") == "W3C_AOT" else "interpreter"
        by_id.setdefault(test_id, set()).add(engine)
    if not by_id:
        return whole_command(inv, "the W3C run (its report names no failing case)")
    head = argv[:2] if program(argv) == "node" else argv[:1]
    keep = []
    engine_flag = _option(argv, "--engine")
    if engine_flag:
        keep += ["--engine", engine_flag]
    entries = []
    for test_id, engines in by_id.items():
        number = re.sub(r"[a-z]+$", "", test_id)
        if engines == {"aot"}:
            run, what = [test_id, "--aot-only"], f"W3C {test_id} on the AOT engine"
        elif engines == {"interpreter"}:
            run, what = [number, "--interpreter-only"], f"W3C {number} on the Interpreter"
        else:
            run, what = [number], f"W3C {number} on both engines"
        entries.append(entry(what, "the W3C runner, by the commands ahead of it",
                             inv.line(head + run + keep)))
    if len(entries) > MAX_COMMANDS:
        return entries[:MAX_COMMANDS] + whole_command(inv, "the whole W3C run")
    return entries


# ── scripts/mutate ──────────────────────────────────────────────────────────

_VERDICT = re.compile(r"^(SURVIVED|INCONCLUSIVE)\s+(.+?)\s*$")


def mutate_entries(inv: Invocation, text: str) -> list:
    argv = inv.argv
    casefile = next((a for a in reversed(argv[1:]) if not a.startswith("-")), None)
    labels = []
    for line in plain_lines(text):
        found = _VERDICT.match(line)
        if not found:
            continue
        label = found.group(2)
        if found.group(1) == "SURVIVED" and "  (" in label:
            label = label[:label.index("  (")]
        if label not in labels:
            labels.append(label)
    if not labels or casefile is None:
        return whole_command(inv, "the mutation round (it judged no case it could name)")
    # `--check` asks whether a case still applies and builds nothing; a full
    # round builds the baseline and then the case.
    check_only = "--check" in argv[1:]
    mode = ["--check"] if check_only else []
    builds = ("nothing: --check applies and restores the case" if check_only
              else "the casefile's baseline, then that one case")
    return [entry(f"{label} ({casefile})", builds,
                  inv.line([argv[0]] + mode + ["--case", label, casefile]))
            for label in labels]


# ── A file a regeneration should have left as it was ────────────────────────


def tree_roots(root: Path) -> dict:
    """Each declared tree (tools/codegen/trees/<tree>.toml) and the
    directories it writes, its "roots"."""
    trees = {}
    for manifest in sorted((root / "tools" / "codegen" / "trees").glob("*.toml")):
        text = manifest.read_text(encoding="utf-8", errors="replace")
        found = re.search(r"^roots\s*=\s*\[(.*?)\]", text, re.S | re.M)
        if found:
            trees[manifest.stem] = re.findall(r'"([^"]+)"', found.group(1))
    return trees


def changed_entries(inv: Invocation, changed: list) -> list:
    """Files a regeneration changed: the declared trees that write them,
    materialized alone, or the regeneration the gate ran where a file is in
    no declared tree; then what git sees of the files."""
    shown = changed[:MAX_COMMANDS]
    trees = []
    roots = tree_roots(inv.root)
    for path in changed:
        owner = next((tree for tree, dirs in roots.items()
                      if any(path == d or path.startswith(d.rstrip("/") + "/") for d in dirs)), None)
        if owner is None:
            trees = []
            break
        if owner not in trees:
            trees.append(owner)
    look = (" && " + sh(["git", "status", "--porcelain", "--"] + shown)
            + " && " + sh(["git", "--no-pager", "diff", "--exit-code", "--"] + shown))
    what = shown[0] if len(changed) == 1 else f"{len(changed)} files the regeneration changed"
    if trees:
        return [entry(what, "sce-codegen (scripts/materialize.sh builds it when missing), then "
                            f"the tree(s) {', '.join(trees)}",
                      " && ".join(inv.first + [sh(["scripts/materialize.sh"] + trees)]) + look)]
    return [entry(what, "what the regeneration builds", inv.line() + look)]


# ── cmake --build ───────────────────────────────────────────────────────────

_NINJA_FAILED = re.compile(r"^FAILED: (?:\[code=\d+\] )?(\S+)")


def cmake_build_entries(inv: Invocation, text: str) -> list:
    """A build that failed, as the one output ninja could not make: building
    that output alone is the compiler's error and nothing else."""
    argv = inv.argv
    build_dir = _option(argv, "--build")
    outputs = []
    for line in plain_lines(text):
        found = _NINJA_FAILED.match(line)
        if found and found.group(1) not in outputs:
            outputs.append(found.group(1))
    if not build_dir or not outputs:
        return whole_command(inv, "the build (ninja named no output it could not make)")
    config = _option(argv, "--config")
    tail = ["--config", config] if config else []
    entries = []
    for output in outputs[:MAX_COMMANDS]:
        entries.append(entry(f"{output} does not build",
                             f"what {output} is made from, in {build_dir}",
                             inv.line([argv[0], "--build", build_dir, "--target", output] + tail)))
    return entries


# ── record and report ───────────────────────────────────────────────────────


def entries_for(inv: Invocation, text: str, directory: str | None, since: float | None,
                changed: list, metadata: Metadata | None = None, reports: list = ()) -> list:
    if changed:
        return changed_entries(inv, changed)
    kind = runner_of(inv.argv)
    if kind == "cargo-test":
        return cargo_test_entries(inv, text, metadata)
    if kind == "cargo-build":
        return cargo_build_entries(inv, text, metadata)
    if kind == "ctest":
        return ctest_entries(inv, directory, since)
    if kind == "cmake-build":
        return cmake_build_entries(inv, text)
    if kind == "go":
        return go_entries(inv, text, directory)
    if kind == "pytest":
        return pytest_entries(inv, directory, since)
    if kind == "unittest":
        return unittest_entries(inv, text)
    if kind == "gradle":
        return gradle_entries(inv, list(reports), since)
    if kind == "swift":
        return swift_entries(inv, directory)
    if kind == "w3c":
        return w3c_entries(inv, since)
    if kind == "mutate":
        return mutate_entries(inv, text)
    return whole_command(inv, "the command that failed")


def with_http_fixture(inv: Invocation, path: str) -> None:
    """A command that needs the W3C BasicHTTP fixture server starts one first,
    on a port of its own, and is told it."""
    port = "18080"
    inv.first.insert(0, f"(node tests/w3c/standalone_http_server.js {port} {shlex.quote(path)} &)"
                        " && sleep 1")
    inv.env = [(n, v) for n, v in inv.env if n != "SCE_W3C_HTTP_PORT"] + [("SCE_W3C_HTTP_PORT", port)]


def tree_first(inv: Invocation, trees: list) -> list:
    """How the CMake tree a ctest run or a build reads was configured, as the
    gate's library registered it (sce_gate_configure_tree and its siblings):
    the command a clean checkout runs before it can build in that tree."""
    named = _option(inv.argv, "--test-dir") or _option(inv.argv, "--build")
    if not named:
        return []
    want = os.path.normpath(os.path.relpath(str(inv.resolve(named)), str(inv.root)))
    for directory, configure in trees:
        here = Path(directory)
        if not here.is_absolute():
            here = inv.root / here
        if os.path.normpath(os.path.relpath(str(here), str(inv.root))) != want:
            continue
        try:
            words = shlex.split(configure)
        except ValueError:
            return [configure]
        return [Invocation(words, root=inv.root).line(with_first=False)]
    return []


def record(args) -> int:
    try:
        text = Path(args.log).read_text(encoding="utf-8", errors="replace") if args.log else ""
    except OSError:
        text = ""
    inv = Invocation(args.command, cwd=args.cwd or ".", first=args.first)
    if not inv.first and args.tree:
        inv.first = tree_first(inv, args.tree)
    if args.http_fixture:
        with_http_fixture(inv, args.http_fixture)
    metadata = None
    if args.metadata:
        metadata = Metadata(json.loads(Path(args.metadata).read_text(encoding="utf-8")))
    try:
        entries = entries_for(inv, text, args.dir, args.since, args.changed or [], metadata,
                              args.report or [])
    except Exception as error:  # never the gate's verdict: say it, keep the command
        print(f"repro: could not read what {program(inv.argv)} reported ({error}); "
              f"recording the command itself", file=sys.stderr)
        entries = whole_command(inv, "the command that failed")
    lines = "".join(json.dumps(e, sort_keys=True) + "\n" for e in entries)
    if args.out == "-":
        sys.stdout.write(lines)
    else:
        with open(args.out, "a", encoding="utf-8") as out:
            out.write(lines)
    return 0


def read_entries(path: str) -> list:
    entries = []
    try:
        text = Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return entries
    for line in text.splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if isinstance(item, dict) and item.get("command") and \
                all(e["command"] != item["command"] for e in entries):
            entries.append(item)
    return entries


def render(slug: str, entries: list, fallback: str, notes: list) -> str:
    lines = ["", HEADING.format(slug=slug)]
    if entries:
        lines.append(f"  {len(entries)} command(s), each the smallest that reproduces one "
                     f"failure, from the root of a clean checkout:")
        for item in entries:
            lines.append(f"  # {item['what']}")
            lines.append(f"  #   builds first: {item['builds']}")
            lines.append(f"  {item['command']}")
    else:
        lines.append("  nothing narrower than the gate was recorded for this failure")
    for note in notes:
        lines.append(f"  # {note}")
    lines.append(f"  the whole gate: {fallback}")
    return "\n".join(lines) + "\n"


def render_summary(slug: str, entries: list, fallback: str, notes: list) -> str:
    out = ["", f"### Reproduce gate \x60{slug}\x60", ""]
    if entries:
        out.append("Each command is the smallest that reproduces one failure, run from the "
                   "root of a clean checkout.")
    else:
        out.append("Nothing narrower than the gate was recorded for this failure.")
    out += ["", "\x60\x60\x60bash"]
    for item in entries:
        out.append(f"# {item['what']}")
        out.append(f"#   builds first: {item['builds']}")
        out.append(item["command"])
    for note in notes:
        out.append(f"# {note}")
    out.append("# the whole gate:")
    out.append(fallback)
    out += ["\x60\x60\x60", ""]
    return "\n".join(out) + "\n"


def report(args) -> int:
    entries = read_entries(args.file) if args.file else []
    notes = args.note or []
    sys.stderr.write(render(args.slug, entries, args.fallback, notes))
    if args.summary:
        try:
            with open(args.summary, "a", encoding="utf-8") as summary:
                summary.write(render_summary(args.slug, entries, args.fallback, notes))
        except OSError as error:
            print(f"repro: could not add to the step summary {args.summary}: {error}",
                  file=sys.stderr)
    return 0


# ── Self-test ───────────────────────────────────────────────────────────────


def _fixture(directory: Path, name: str) -> str:
    """A fixture as the gate saw it: the hosted runner's timestamp column,
    which the recorded job logs carry and a run's own output does not, taken
    off each line."""
    text = (directory / name).read_text(encoding="utf-8")
    return "\n".join(re.sub(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+Z ?", "", line)
                     for line in text.splitlines()) + "\n"


def self_test(fixtures: Path) -> int:
    import tempfile
    checks = 0

    def check(condition: bool, what: str) -> None:
        nonlocal checks
        checks += 1
        if not condition:
            raise AssertionError(what)

    def commands(entries: list) -> list:
        return [e["command"] for e in entries]

    sce_build = {"name": "sce-build", "features": {"cli": [], "ffi": [], "remote-cache": []},
                 "dependencies": [{"name": "serde"}],
                 "targets": [{"kind": ["lib"], "name": "sce_build", "src_path": "/r/sce-build/src/lib.rs"},
                             {"kind": ["test"], "name": "b9_drift_detection",
                              "src_path": "/r/sce-build/tests/b9_drift_detection.rs"}]}
    probe = {"name": "sce-portable-emit-probe", "features": {}, "dependencies": [],
             "targets": [{"kind": ["lib"], "name": "sce_portable_emit_probe",
                          "src_path": "/r/probe/src/lib.rs"}]}
    conformance = {"name": "sce-forge-conformance", "features": {"alloc": []},
                   "dependencies": [{"name": "sce-forge-runtime"}],
                   "targets": [{"kind": ["test"], "name": "numerical_conformance",
                                "src_path": "/r/c/tests/numerical_conformance.rs"}]}
    meta = Metadata({"packages": [sce_build, probe, conformance]})

    # cargo test, one shard of the workspace suite (job 138951650414): cargo's
    # selector names the package and target, libtest's list the test, and the
    # features the package does not have are left out of a -p run.
    inv = Invocation(["cargo", "test", "--workspace", "--features", "cli,sce-rust-mesh/wss",
                      "--no-fail-fast", "--test", "a_kotlin_kind_keeps_an_authors_names_apart_from_its_own",
                      "--test", "every_ledger_binding_names_what_its_file_holds"])
    got = commands(cargo_test_entries(inv, _fixture(fixtures, "cargo_test_workspace_shard.txt"), meta))
    check(got == ["cargo test --features cli -p sce-build --test "
                  "every_ledger_binding_names_what_its_file_holds -- "
                  "every_ledger_binding_names_what_its_file_holds --exact"],
          f"the workspace shard's one failure: {got}")

    # cargo test with cargo's stderr ahead of the binary's stdout (job
    # 138963137845): the selector came before the failures list.
    inv = Invocation(["cargo", "test", "-p", "sce-build", "--features", "cli", "--test",
                      "b9_drift_detection", "--", "--test-threads=1"])
    got = commands(cargo_test_entries(inv, _fixture(fixtures, "cargo_test_interleaved.txt"), meta))
    check(got == ["cargo test --features cli -p sce-build --test b9_drift_detection -- "
                  "verify_passes_on_the_materialized_rust_static_datamodel_tree --exact "
                  "--test-threads=1"], f"the drift suite's failure, read out of order: {got}")

    # A binary that died without a failures list still has cargo's selector.
    crashed = ("     Running tests/x.rs (target/debug/deps/x-0123456789abcdef)\n\nrunning 3 tests\n"
               "error: test failed, to rerun pass \x60-p sce-build --test x\x60\n\nCaused by:\n"
               "  process didn't exit successfully: (signal: 11, SIGSEGV)\n")
    got = commands(cargo_test_entries(Invocation(["cargo", "test", "-p", "sce-build", "--release"]),
                                      crashed, meta))
    check(got == ["cargo test --release -p sce-build --test x"],
          f"a crashed binary is run as its target: {got}")

    # cargo build that did not compile (job 138978848853), and clippy that did
    # not (job 138878333220): the crate alone, with the run's flags.
    inv = Invocation(["cargo", "check", "--manifest-path", "backends/rust/probes/feature-unification/Cargo.toml",
                      "--workspace"])
    got = commands(cargo_build_entries(inv, _fixture(fixtures, "cargo_build_broken.txt"), meta))
    check(got == ["cargo check --manifest-path backends/rust/probes/feature-unification/Cargo.toml "
                  "-p sce-portable-emit-probe"], f"the crate that did not compile: {got}")
    inv = Invocation(["cargo", "clippy", "--workspace", "--all-targets", "--features",
                      "cli,sce-rust-mesh/wss,sce-forge-conformance/alloc", "--", "-D", "warnings"])
    got = commands(cargo_build_entries(inv, _fixture(fixtures, "cargo_clippy_broken.txt"), meta))
    check(got == ["cargo clippy --features sce-forge-conformance/alloc -p sce-forge-conformance "
                  "--all-targets -- -D warnings"],
          f"clippy narrowed to the crate it refused, once for its two targets: {got}")

    # ctest (job 138089290997): the run's own record of what failed, the
    # closure of each test, and the whole-tree build fixture left out.
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        tree = root / "build_lua" / "Testing" / "Temporary"
        tree.mkdir(parents=True)
        (tree / "LastTestsFailed.log").write_text(
            _fixture(fixtures, "ctest_last_tests_failed.txt"), encoding="utf-8")
        inv = Invocation(["ctest", "--test-dir", "build_lua", "-R", "^LoweredEcma262$", "--verbose"],
                         root=root, first=["scripts/prepare_ctest_tree.sh"])
        asked = []

        def closure(build, names):
            asked.append((Path(build).name, list(names)))
            return {"targets": ["tests/lowered_ecma262"] if names == ["LoweredEcma262"] else [],
                    "fixtures": ["sce_built"]}

        got = commands(ctest_entries(inv, None, None, closure_reader=closure))
        check(asked == [("build_lua", ["LoweredEcma262"]), ("build_lua", ["sce_build_is_current"])],
              f"each failed test's closure is asked of its own tree: {asked}")
        check(got[0] == "scripts/prepare_ctest_tree.sh && cmake --build build_lua --target "
                        "tests/lowered_ecma262 && ctest --test-dir build_lua -R '^(LoweredEcma262)$' "
                        "--fixture-exclude-setup '^(sce_built)$' --output-on-failure",
              f"the ctest command builds the closure and runs the one test: {got}")
        check(got[1].endswith("ctest --test-dir build_lua -R '^(sce_build_is_current)$' "
                              "--fixture-exclude-setup '^(sce_built)$' --output-on-failure")
              and "cmake --build" not in got[1],
              f"a test that runs no built program builds nothing first: {got}")
        stale = time.time() + 3600
        got = commands(ctest_entries(inv, None, stale, closure_reader=closure))
        check(len(got) == 1 and "-R '^LoweredEcma262$'" in got[0],
              f"a LastTestsFailed.log older than the run is not this run's: {got}")

        # The JUnit report prepare asks for wins over the log.
        report = root / "junit"
        report.mkdir()
        (report / "ctest-junit.xml").write_text(
            '<testsuite><testcase name="A" status="run"/><testcase name="B" status="fail">'
            '<failure message="Failed"/></testcase><testcase name="C" status="disabled"/>'
            '</testsuite>', encoding="utf-8")
        got = commands(ctest_entries(inv, str(report), None, closure_reader=closure))
        check(len(got) == 1 and "-R '^(B)$'" in got[0], f"the JUnit report's one failure: {got}")

        def unreadable(build, names):
            raise RuntimeError("build_lua is built by 'Unix Makefiles'")

        got = commands(ctest_entries(inv, str(report), None, closure_reader=unreadable))
        check("cmake --build build_lua && ctest" in got[0],
              f"an unreadable closure builds the whole tree, and says so: {got}")

    # cmake --build (job 137573088547): ninja's FAILED edge, built alone.
    inv = Invocation(["cmake", "--build", "build", "--parallel", "8"],
                     first=["scripts/prepare_ctest_tree.sh"])
    got = commands(cmake_build_entries(inv, _fixture(fixtures, "cmake_build_failed.txt")))
    check(got == ["scripts/prepare_ctest_tree.sh && cmake --build build --target "
                  "tests/CMakeFiles/saved_state_test.dir/saved/SavedStateTest.cpp.o"],
          f"the object that did not compile: {got}")

    # go test: the -json events, and the -v text where there are none.
    events = _fixture(fixtures, "go_test.json")
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        module = root / "backends" / "go" / "tests"
        module.mkdir(parents=True)
        (module / "go.mod").write_text("module github.com/newmassrael/sce-go-tests\n\ngo 1.23.0\n",
                                       encoding="utf-8")
        (root / "events").mkdir()
        (root / "events" / "go.json").write_text(events, encoding="utf-8")
        inv = Invocation(["go", "test", "./...", "-v", "-count=1", "-race"],
                         cwd="backends/go/tests", root=root)
        got = commands(go_entries(inv, "", str(root / "events")))
        check(got == ["(cd backends/go/tests && go test -count=1 -race -run '^TestW3C$/^test144$' "
                      "./generated/test144)",
                      "(cd backends/go/tests && go test -count=1 -race ./harness)"],
              f"a failing subtest and a package that did not build: {got}")
        text = "".join(json.loads(line).get("Output", "") for line in events.splitlines()
                       if line.startswith("{"))
        got = commands(go_entries(inv, text, None))
        check(got[0] == "(cd backends/go/tests && go test -count=1 -race -run "
                        "'^TestW3C$/^test144$' ./generated/test144)",
              f"the -v text gives the same failing test: {got}")

    # pytest, from its xunit1 JUnit report.
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        (root / "backends" / "python" / "tests" / "generated").mkdir(parents=True)
        (root / "backends" / "python" / "tests" / "generated" / "test_w3c_144.py").write_text("")
        (root / "out").mkdir()
        (root / "out" / "pytest-junit.xml").write_text(
            '<testsuites><testsuite><testcase classname="generated.test_w3c_144.TestW3C144" '
            'file="generated/test_w3c_144.py" name="test_passes[lua]"><failure message="x"/>'
            '</testcase><testcase classname="generated.test_w3c_144" '
            'file="generated/test_w3c_144.py" name="test_ok"/></testsuite></testsuites>',
            encoding="utf-8")
        inv = Invocation(["env", "PYTHONPATH=" + str(root / "backends/python/runtime"), "python3",
                          "-m", "pytest", "backends/python/tests/generated/", "--no-header", "-v",
                          "--junitxml=/tmp/x.xml", "-o", "junit_family=xunit1"], root=root)
        got = commands(pytest_entries(inv, str(root / "out"), None))
        check(got == ["PYTHONPATH=backends/python/runtime python3 -m pytest --no-header -v "
                      "'backends/python/tests/generated/test_w3c_144.py::TestW3C144::test_passes[lua]'"],
              f"the pytest node id, found from the run's own path: {got}")

    # unittest, from its FAIL and ERROR lines, both spellings.
    inv = Invocation(["env", "PYTHONPATH=tests", "python3", "-m", "unittest",
                      "tests.test_numerical_conformance"], cwd="backends/python/forge-runtime")
    got = commands(unittest_entries(inv, _fixture(fixtures, "unittest.txt")))
    check(got == ["(cd backends/python/forge-runtime && PYTHONPATH=tests python3 -m unittest "
                  "tests.test_numerical_conformance.NumericalConformanceTest.test_saturating_add)",
                  "(cd backends/python/forge-runtime && PYTHONPATH=tests python3 -m unittest "
                  "tests.test_queue_history.QueueHistoryTest.test_every_history_is_linearizable)"],
          f"the unittest ids: {got}")

    # Gradle (job 137251729632): the test-results XML of the task that ran,
    # its display name narrowed to the method by wildcards.
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        results = root / "backends" / "kotlin" / "tests" / "build" / "test-results" / "test"
        results.mkdir(parents=True)
        (results / "TEST-com.sce.integration.AChildTakesAHandedValueTest.xml").write_text(
            _fixture(fixtures, "gradle_test_results.xml"), encoding="utf-8")
        inv = Invocation(["./gradlew", "--console=plain", ":sce-kotlin-tests:test",
                          "-Psce.script.engine=lua", "-Psce.lua.loweringCensus=/tmp/x/census.tsv",
                          "-Dorg.gradle.jvmargs=-Xmx2g"], root=root)
        listed = gradle_entries(inv, ["backends/kotlin/tests/build/test-results/test"], None)
        check("without -Psce.lua.loweringCensus" in listed[0]["what"],
              f"a property naming the gate's own directory is left out, and said: {listed}")
        got = commands(listed)
        check(got == ["./gradlew --console=plain -Psce.script.engine=lua :sce-kotlin-tests:test "
                      "--tests 'com.sce.integration.AChildTakesAHandedValueTest.an*object*a*parent*"
                      "hands*arrives*whole*on*every*engine*as*a*copy'"],
              f"the one Gradle test, by its class and method: {got}")
        check(_gradle_filter("a.B", "plainName()") == "a.B.plainName",
              "a method name that is an identifier is matched exactly")
        inv = Invocation(["./gradlew", "--console=plain", ":sce-kotlin-w3c:test",
                          ":sce-kotlin-tests:test"], root=root)
        got = commands(gradle_entries(inv, ["backends/kotlin/tests/build/test-results/test"], None))
        check(len(got) == 1 and " -p backends/kotlin/tests test --tests " in got[0],
              f"two tasks of one name are told apart by the project's directory: {got}")
        stale = time.time() + 3600
        got = commands(gradle_entries(inv, ["backends/kotlin/tests/build/test-results/test"], stale))
        check(got == [inv.line()], f"a report older than the run is not this run's: {got}")

    # swift test, from the xUnit report test_swift_backend.sh wrote.
    with tempfile.TemporaryDirectory() as scratch:
        swift = Path(scratch) / "swift"
        swift.mkdir()
        (swift / "run-1.package").write_text("dir backends/swift/tests\nfirst scripts/regen_swift_w3c.sh\n")
        (swift / "run-1.xml").write_text(
            '<testsuites><testsuite><testcase classname="W3cMachinesTests.W3cMachinesTests" '
            'name="test144"><failure message="x"/></testcase></testsuite></testsuites>')
        (swift / "run-10.package").write_text("dir backends/swift/mesh\n")
        (swift / "run-10.xml").write_text('<testsuites><testsuite><testcase classname="M.T" '
                                          'name="ok"/></testsuite></testsuites>')
        inv = Invocation(["scripts/test_swift_backend.sh", "w3c"])
        got = commands(swift_entries(inv, scratch))
        check(got == ["scripts/regen_swift_w3c.sh && (cd backends/swift/tests && swift test "
                      "--filter '^W3cMachinesTests\\.W3cMachinesTests/test144$')"],
              f"the Swift test, in its package, after what writes it: {got}")

    # The W3C runner's report: an AOT-only failure, and one on both engines.
    with tempfile.TemporaryDirectory() as scratch:
        report = Path(scratch) / "w3c_test_results.xml"
        report.write_text(
            '<testsuites><testsuite name="W3C_SCXML_Interpreter">'
            '<testcase classname="W3C_Interpreter" name="Test_201" id="201"><failure message="x"/></testcase>'
            '</testsuite><testsuite name="W3C_SCXML_AOT">'
            '<testcase classname="W3C_AOT" name="Test_201" id="201"><failure message="x"/></testcase>'
            '<testcase classname="W3C_AOT" name="Test_403a" id="403a"><failure message="x"/></testcase>'
            '</testsuite></testsuites>')
        inv = Invocation(["./w3c_test_cli", "--output", str(report)], cwd="build/tests")
        got = commands(w3c_entries(inv, None))
        check(got == ["(cd build/tests && ./w3c_test_cli 201)",
                      "(cd build/tests && ./w3c_test_cli 403a --aot-only)"],
              f"one W3C case each, on the engine that failed it: {got}")

    # scripts/mutate (job 138776920543): the cases that did not turn red.
    inv = Invocation(["scripts/mutate", "--shard", "1/1",
                      "sce-build/tests/mutations/a_form_holds_what_its_deployment_says.cases"])
    stale_anchor = "INCONCLUSIVE  a frame is called too big\n              the anchor is absent\n"
    got = commands(mutate_entries(Invocation(["scripts/mutate", "--check", "x.cases"]), stale_anchor))
    check(got == ["scripts/mutate --check --case 'a frame is called too big' x.cases"],
          f"a stale case of --check is checked again alone, building nothing: {got}")
    got = commands(mutate_entries(inv, _fixture(fixtures, "mutate_round.txt")))
    check(got == ["scripts/mutate --case 'a frame is called too big to hold before it is judged "
                  "whole' sce-build/tests/mutations/a_form_holds_what_its_deployment_says.cases",
                  "scripts/mutate --case 'a capped text holds a view past its limit' "
                  "sce-build/tests/mutations/a_form_holds_what_its_deployment_says.cases"],
          f"each case that survived or could not be judged: {got}")

    # A regeneration that changed files (job 137522402631): the tree they are in.
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        (root / "tools" / "codegen" / "trees").mkdir(parents=True)
        (root / "tools" / "codegen" / "trees" / "swift-static-datamodel.toml").write_text(
            '[tree]\nname = "swift-static-datamodel"\nroots = [\n'
            '    "backends/swift/tests/Sources/StaticDatamodelMachines",\n'
            '    "backends/swift/tests/Tests/StaticDatamodelTests",\n]\n')
        inv = Invocation(["scripts/regen_all_committed_trees.sh"], root=root)
        changed = ["backends/swift/tests/Sources/StaticDatamodelMachines/LinkstateQinfo.swift",
                   "backends/swift/tests/Sources/StaticDatamodelMachines/SchemaMapRow.swift"]
        got = commands(changed_entries(inv, changed))
        check(got == ["scripts/materialize.sh swift-static-datamodel && git status --porcelain -- "
                      + " ".join(changed) + " && git --no-pager diff --exit-code -- " + " ".join(changed)],
              f"the declared tree the changed files are in: {got}")
        got = commands(changed_entries(inv, ["embed/MANIFEST.json"]))
        check(got[0].startswith("scripts/regen_all_committed_trees.sh && git status"),
              f"a file no tree declares is reproduced by the gate's regeneration: {got}")

    # prepare asks each runner for its report, and leaves the rest alone.
    with tempfile.TemporaryDirectory() as scratch:
        render_mode, argv = prepare(["ctest", "--test-dir", "build", "-L", "c11"], scratch)
        check(render_mode == "-" and argv[-2] == "--output-junit", f"ctest JUnit: {argv}")
        render_mode, argv = prepare(["go", "test", "./...", "-v", "-count=1"], scratch)
        check(render_mode == "go" and argv == ["go", "test", "-json", "./...", "-count=1"],
              f"go -json: {argv}")
        render_mode, argv = prepare(["go", "test", "./conformance/", "-count=1"], scratch)
        check(render_mode == "-" and argv == ["go", "test", "./conformance/", "-count=1"],
              f"a go run that prints no -v text is run as given: {argv}")
        render_mode, argv = prepare(["env", "A=b", "python3", "-m", "pytest", "x"], scratch)
        check(argv[:3] == ["env", "A=b", "python3"] and "-o" in argv
              and any(a.startswith("--junitxml=") for a in argv), f"pytest JUnit: {argv}")
        render_mode, argv = prepare(["scripts/test_swift_backend.sh", "w3c"], scratch)
        check(argv[0] == "env" and argv[1].startswith("SCE_SWIFT_XUNIT_DIR="), f"swift: {argv}")
        render_mode, argv = prepare(["cargo", "test", "-p", "x"], scratch)
        check(argv == ["cargo", "test", "-p", "x"], f"cargo is run as given: {argv}")

    # render-go gives back the text "go test -v" prints.
    import io
    out = io.StringIO()
    render_go(io.StringIO('{"Action":"output","Output":"=== RUN   TestA\\n"}\n# pkg\nbad.go:1: x\n'), out)
    check(out.getvalue() == "=== RUN   TestA\n# pkg\nbad.go:1: x\n", f"render-go: {out.getvalue()!r}")

    # The report: one heading, every command, the whole gate last, and the
    # same in the step summary.
    rendered = render("workspace-tests", [entry("t (x)", "b", "cargo test -p x")],
                      "scripts/gate --shard 2/4 workspace-tests", [])
    check(HEADING.format(slug="workspace-tests") in rendered and "cargo test -p x" in rendered
          and rendered.rstrip().endswith("scripts/gate --shard 2/4 workspace-tests"),
          f"the report: {rendered}")
    summary = render_summary("workspace-tests", [entry("t (x)", "b", "cargo test -p x")],
                             "scripts/gate workspace-tests", [])
    check("cargo test -p x" in summary and "workspace-tests" in summary, f"summary: {summary}")
    check("nothing narrower" in render("x", [], "scripts/gate x", []),
          "a failure nothing narrower was recorded for says so")

    print(f"repro self-test: {checks} checks passed")
    return 0


# ── Command line ────────────────────────────────────────────────────────────


def main(argv: list) -> int:
    if argv[:1] == ["--self-test"]:
        if len(argv) != 2:
            print("usage: repro.py --self-test <fixtures dir>", file=sys.stderr)
            return 2
        return self_test(Path(argv[1]))
    if argv[:1] == ["render-go"]:
        return render_go(sys.stdin, sys.stdout)
    command = []
    if "--" in argv:
        at = argv.index("--")
        argv, command = argv[:at], argv[at + 1:]
    parser = argparse.ArgumentParser(prog="repro.py")
    sub = parser.add_subparsers(dest="action", required=True)
    preparing = sub.add_parser("prepare")
    preparing.add_argument("--dir", required=True)
    recording = sub.add_parser("record")
    recording.add_argument("--out", required=True)
    recording.add_argument("--log")
    recording.add_argument("--dir")
    recording.add_argument("--since", type=float)
    recording.add_argument("--cwd")
    recording.add_argument("--first", action="append")
    recording.add_argument("--tree", nargs=2, action="append", metavar=("DIR", "CONFIGURE"))
    recording.add_argument("--http-fixture")
    recording.add_argument("--changed", action="append")
    recording.add_argument("--report", action="append")
    recording.add_argument("--metadata")
    reporting = sub.add_parser("report")
    reporting.add_argument("--slug", required=True)
    reporting.add_argument("--file")
    reporting.add_argument("--fallback", required=True)
    reporting.add_argument("--summary")
    reporting.add_argument("--note", action="append")
    args = parser.parse_args(argv)
    if args.action == "report":
        return report(args)
    if not command:
        parser.error(f"{args.action} needs the command after --")
    if args.action == "prepare":
        render_mode, run = prepare(command, args.dir)
        sys.stdout.write("\0".join([render_mode] + run) + "\0")
        return 0
    args.command = command
    return record(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
