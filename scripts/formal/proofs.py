#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Check every proof of the host core with Verus, as docs/formal/proofs.json declares
# it (docs/adr/0115, D13).
#
#   * A file is PROVED when Verus verifies exactly the number of functions the manifest
#     records, with no error and no warning. A contract that was taken out leaves fewer.
#   * A proof that cannot fail shows nothing, so each MUTANT is a change of the code that
#     breaks something a user relies on: it must compile, and Verus must refuse it.
#   * What is proved is what runs: the file is compiled by the module the manifest names,
#     and nothing in it is taken on trust (an assumption, an admitted proof, a body Verus
#     does not read).
#
# The manifest is complete: every file that carries a contract is declared. Exit 1 is a
# verdict about the code, exit 3 is "could not run" (no Verus, or not its toolchain).
#
# Usage: proofs.py [--self-test | --install [<directory>]]

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "docs" / "formal" / "proofs.json"
EXIT_CANNOT_RUN = 3

PROOF_KEYS = {"file", "compiled_by", "decision", "verified", "mutants"}
MUTANT_KEYS = {"breaks", "from", "to"}

# What makes Verus take something on trust.
TRUSTED = re.compile(
    r"\b(assume|admit|assume_specification|external_body|external_fn_specification|external_type_specification"
    r"|external_trait_specification|axiom)\b|verifier::external|exec_allows_no_decreases_clause"
)
RESULTS = re.compile(r"verification results:: (\d+) verified, (\d+) errors")


class CannotRun(Exception):
    """The check could not be made."""


def judged(output: str, status: int, verified: int) -> str | None:
    """What is wrong with a run of a proof that must hold, or None."""
    found = RESULTS.search(output)
    if found is None:
        return "Verus judged nothing: the file does not compile"
    if int(found.group(2)) or status:
        return f"{found.group(2)} error(s)"
    if int(found.group(1)) != verified:
        return f"{found.group(1)} verified where {verified} are recorded"
    if re.search(r"^(warning|WARNING)", output, re.MULTILINE):
        return "a warning"
    return None


def mutant_judged(output: str) -> str | None:
    """What is wrong with a run of a mutant, which must be refused, or None."""
    found = RESULTS.search(output)
    if found is None:
        return "Verus judged nothing: the mutant does not compile"
    if not int(found.group(2)):
        return "the proof holds of it, so it says nothing of what the mutant breaks"
    return None


def declared(manifest: dict, root: Path, contracted: list[str]) -> list[str]:
    """What the manifest and the files it declares get wrong, with no Verus."""
    problems: list[str] = []
    for proof in manifest["proofs"]:
        name = proof.get("file", "?")
        for key in sorted(PROOF_KEYS ^ set(proof)):
            problems.append(f"{name}: {'unknown' if key in proof else 'missing'} key {key}")
        path = root / name
        if not path.is_file():
            problems.append(f"{name}: no such file")
            continue
        text = path.read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), 1):
            code = line.split("//")[0]
            if TRUSTED.search(code):
                problems.append(f"{name}:{number}: taken on trust: {line.strip()}")
        module = root / proof.get("compiled_by", "")
        if not module.is_file() or not re.search(rf"^\s*(pub(\([a-z]+\))? )?mod {path.stem};", module.read_text(encoding="utf-8"), re.MULTILINE):
            problems.append(f"{name}: {proof.get('compiled_by')} does not compile it, so the proof is of nothing that runs")
        if not proof.get("mutants"):
            problems.append(f"{name}: no mutant, so nothing shows that the proof can fail")
        for mutant in proof.get("mutants", []):
            if set(mutant) != MUTANT_KEYS:
                problems.append(f"{name}: a mutant has the keys {sorted(mutant)}")
                continue
            times = text.count(mutant["from"])
            if times != 1:
                problems.append(f"{name}: mutant \"{mutant['breaks']}\": its text is there {times} times")
            if mutant["from"] == mutant["to"]:
                problems.append(f"{name}: mutant \"{mutant['breaks']}\": changes nothing")
    names = {proof.get("file") for proof in manifest["proofs"]}
    for name in contracted:
        if name not in names:
            problems.append(f"{name}: carries a contract, and the manifest does not declare it")
    return problems


def contracted_files() -> list[str]:
    """Every Rust file of the repository that Verus would read a contract of."""
    found = subprocess.run(
        ["git", "grep", "-l", "--untracked", "cfg_attr(verus_only", "--", "*.rs"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    if found.returncode > 1:
        raise CannotRun(f"git grep: {found.stderr.strip()}")
    return found.stdout.split()


def place_of(pin: dict) -> Path:
    """Where the pinned Verus is, on a build machine (bx.toml) and anywhere else."""
    return Path.home() / ".local" / "verus" / pin["version"]


def toolchain_of(pin: dict) -> Path:
    """The Rust release Verus runs on, where every declared one is (scripts/lib/rust_toolchains.sh)."""
    return Path.home() / ".local" / "rust" / pin["toolchain"]


# Verus asks rustup for its Rust release unless it is told not to; then it is the one
# whose libraries it finds. No gate here reads an account's rustup home.
WRAPPER = """#!/bin/sh
# Written by scripts/formal/proofs.py --install: Verus {version} on Rust {toolchain}, with no rustup.
if [ "$1" = --version ]; then
    echo "verus {version}"
    exit 0
fi
rust="$HOME/.local/rust/{toolchain}"
VERUS_USE_RUSTUP=0 LD_LIBRARY_PATH="$rust/lib" exec "$(dirname "$(readlink -f "$0")")/../verus" --sysroot "$rust" "$@"
"""


def install(pin: dict, place: Path) -> None:
    """Fetch the pinned Verus, check it against its digest, and unpack it into `place`."""
    with tempfile.TemporaryDirectory() as scratch:
        archive = Path(scratch) / "verus.zip"
        try:
            urllib.request.urlretrieve(pin["url"], archive)
        except OSError as error:
            raise CannotRun(f"cannot fetch {pin['url']}: {error}") from error
        actual = hashlib.sha256(archive.read_bytes()).hexdigest()
        if actual != pin["sha256"]:
            raise CannotRun(f"{pin['url']} is not the Verus this manifest pins ({actual})")
        unpacked = Path(scratch) / "unpacked"
        with zipfile.ZipFile(archive) as opened:
            opened.extractall(unpacked)
            for entry in opened.infolist():
                mode = entry.external_attr >> 16
                if mode:
                    os.chmod(unpacked / entry.filename, mode & 0o755)
        tree = unpacked / "verus-x86-linux"
        if not (tree / "verus").is_file():
            raise CannotRun(f"{pin['url']} holds no verus-x86-linux/verus")
        place.mkdir(parents=True, exist_ok=True)
        for entry in tree.iterdir():
            shutil.move(str(entry), str(place / entry.name))
    wrapper = place / "bin" / "verus"
    wrapper.parent.mkdir(exist_ok=True)
    wrapper.write_text(WRAPPER.format(version=pin["version"], toolchain=pin["toolchain"]), encoding="utf-8")
    wrapper.chmod(0o755)


def verus(pin: dict) -> Path:
    """The pinned Verus, which this never fetches: a gate installs no tool."""
    tool = place_of(pin) / "bin" / "verus"
    if not tool.is_file():
        raise CannotRun(f"no Verus at {tool}: scripts/formal/proofs.py --install")
    if not (toolchain_of(pin) / "lib").is_dir():
        raise CannotRun(f"no Rust at {toolchain_of(pin)}: scripts/install_rust.sh --declared {pin['toolchain']}")
    return tool


def run_verus(tool: Path, file: Path) -> tuple[int, str]:
    ran = subprocess.run(
        [str(tool), "--crate-type=lib", str(file)], capture_output=True, text=True, check=False, cwd=file.parent,
    )
    return ran.returncode, ran.stdout + ran.stderr


def check() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    problems = declared(manifest, ROOT, contracted_files())
    if problems:
        for line in problems:
            print(f"proofs.py: {line}", file=sys.stderr)
        return 1
    tool = verus(manifest["verus"])
    failed = 0
    for proof in manifest["proofs"]:
        source = ROOT / proof["file"]
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch) / source.name
            text = source.read_text(encoding="utf-8")
            runs = [(None, text)] + [(m["breaks"], text.replace(m["from"], m["to"])) for m in proof["mutants"]]
            for breaks, body in runs:
                copy.write_text(body, encoding="utf-8")
                status, output = run_verus(tool, copy)
                wrong = judged(output, status, proof["verified"]) if breaks is None else mutant_judged(output)
                what = f"{source.name}: {proof['verified']} verified" if breaks is None else f"{source.name}: refused: {breaks}"
                print(f"{'ok  ' if wrong is None else 'FAIL'} {what}{'' if wrong is None else f': {wrong}'}", flush=True)
                if wrong is not None:
                    failed += 1
                    print("\n".join(output.splitlines()[-30:]), file=sys.stderr)
    if failed:
        print(f"proofs.py: {failed} run(s) are not as docs/formal/proofs.json declares", file=sys.stderr)
        return 1
    print("proofs.py: every proof is as declared")
    return 0


class SelfTest(unittest.TestCase):
    """The checks that judge a proof are judged here, with no Verus."""

    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "rules.rs").write_text("#[cfg_attr(verus_only, verus_spec())]\nfn f() -> u64 { 1 }\n")
        (self.root / "mod.rs").write_text("mod rules;\n")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def manifest(self, **over: object) -> dict:
        proof = {
            "file": "rules.rs", "compiled_by": "mod.rs", "decision": "docs/adr/0115", "verified": 1,
            "mutants": [{"breaks": "one", "from": "{ 1 }", "to": "{ 2 }"}],
        }
        proof.update(over)
        return {"proofs": [proof]}

    def problems(self, **over: object) -> list[str]:
        return declared(self.manifest(**over), self.root, ["rules.rs"])

    def test_a_whole_declaration_is_clean(self) -> None:
        self.assertEqual(self.problems(), [])

    def test_a_contract_nobody_declared_is_found(self) -> None:
        self.assertTrue(any("does not declare" in p for p in declared(self.manifest(), self.root, ["other.rs"])))

    def test_a_file_that_is_not_there_is_found(self) -> None:
        (self.root / "rules.rs").unlink()
        self.assertTrue(any("no such file" in p for p in self.problems()))

    def test_a_file_nothing_compiles_is_found(self) -> None:
        (self.root / "mod.rs").write_text("// mod rules;\n")
        self.assertTrue(any("nothing that runs" in p for p in self.problems()))

    def test_what_is_taken_on_trust_is_found(self) -> None:
        for line in ("assume(false);", "admit();", "#[verifier::external_body]", "axiom fn a();"):
            (self.root / "rules.rs").write_text(f"{line}\nfn f() -> u64 {{ 1 }}\n")
            self.assertTrue(any("taken on trust" in p for p in self.problems()), line)

    def test_a_word_of_a_comment_is_not_trust(self) -> None:
        (self.root / "rules.rs").write_text("// we assume nothing\nfn f() -> u64 { 1 }\n")
        self.assertEqual(self.problems(), [])

    def test_a_mutant_that_changes_no_text_is_found(self) -> None:
        self.assertTrue(any("0 times" in p for p in self.problems(mutants=[{"breaks": "x", "from": "{ 3 }", "to": ""}])))
        self.assertTrue(any("changes nothing" in p for p in self.problems(mutants=[{"breaks": "x", "from": "{ 1 }", "to": "{ 1 }"}])))

    def test_a_proof_without_a_mutant_is_found(self) -> None:
        self.assertTrue(any("no mutant" in p for p in self.problems(mutants=[])))

    def test_an_unknown_key_is_found(self) -> None:
        self.assertTrue(any("unknown key" in p for p in self.problems(trust=True)))

    def test_a_proof_that_holds_is_as_recorded(self) -> None:
        self.assertIsNone(judged("verification results:: 3 verified, 0 errors", 0, 3))
        self.assertIsNotNone(judged("verification results:: 2 verified, 0 errors", 0, 3))
        self.assertIsNotNone(judged("verification results:: 3 verified, 1 errors", 1, 3))
        self.assertIsNotNone(judged("WARNING: x\nverification results:: 3 verified, 0 errors", 0, 3))
        self.assertIsNotNone(judged("error: aborting", 1, 3))

    def test_a_mutant_is_refused_by_the_proof_and_not_by_the_compiler(self) -> None:
        self.assertIsNone(mutant_judged("verification results:: 2 verified, 1 errors"))
        self.assertIsNotNone(mutant_judged("verification results:: 3 verified, 0 errors"))
        self.assertIsNotNone(mutant_judged("error[E0308]: mismatched types"))


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return 0 if unittest.main(argv=[sys.argv[0]], exit=False).result.wasSuccessful() else 1
    try:
        if sys.argv[1:2] == ["--install"] and len(sys.argv) <= 3:
            pin = json.loads(MANIFEST.read_text(encoding="utf-8"))["verus"]
            place = Path(sys.argv[2]) if len(sys.argv) == 3 else place_of(pin)
            if (place / "bin" / "verus").is_file():
                print(f"proofs.py: Verus {pin['version']} is at {place}")
                return 0
            install(pin, place)
            return 0
        if sys.argv[1:]:
            print("usage: proofs.py [--self-test | --install [<directory>]]", file=sys.stderr)
            return 2
        return check()
    except CannotRun as reason:
        print(f"proofs.py: could not run: {reason}", file=sys.stderr)
        return EXIT_CANNOT_RUN


if __name__ == "__main__":
    sys.exit(main())
