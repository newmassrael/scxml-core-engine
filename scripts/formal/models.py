#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Check every TLA+ model under docs/formal with TLC, as docs/formal/models.json
# declares it (docs/adr/0115, layer 5).
#
# A run either holds or finds the violation it names, and both are verdicts:
#
#   * A run that HOLDS explores every reachable state of its configuration and
#     must explore exactly the number of distinct states the manifest records.
#     A model whose behaviour shrank (a guard that became false, a variable that
#     no longer moves) still reports "no error", so the count is what says the
#     model is the one that was judged.
#   * A run that VIOLATES takes one part of the protocol out, and TLC must find a
#     violation of the invariant it names, and of no other. A model that cannot
#     fail shows nothing, and one that fails for another reason shows something
#     else than it says.
#
#   * A module with a PROMISE refines an abstract module (docs/adr/0115, D1 and D2).
#     Its holding configurations check the refinement, and each run that takes a part
#     out says whether the promise is broken without it: where it is, TLC must find a
#     violation of it too, and where it is not, the run says why the part is there.
#
# The manifest is complete in both directions: every configuration and module
# under docs/formal is declared, and every PART of a model (a constant whose
# value switches a piece of the protocol) has a run that takes it out. Exit 1 is a
# verdict about the models, exit 3 is "could not run" (no Java, no TLC).
#
# Usage: models.py [--self-test | --record] [--only <module>] [--workers N]

from __future__ import annotations

import argparse
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
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FORMAL = ROOT / "docs" / "formal"
MANIFEST = FORMAL / "models.json"
EXIT_CANNOT_RUN = 3

# TLC's own exit statuses: 0 no violation, 12 a violated invariant or assertion,
# 13 a violated temporal property. Anything else is a run that did not judge.
TLC_HOLDS = 0
TLC_SAFETY = 12
TLC_LIVENESS = 13

RUN_KEYS = {"config", "expect", "invariant", "property", "without", "against", "states", "promise", "why"}
MODULE_KEYS = {"module", "decision", "parts", "runs", "promise", "refines", "abstract", "progress"}

# The sections of a .cfg, and the ones that take states away from what TLC explores.
SECTION = r"^(CONSTANTS?|INVARIANTS?|PROPERT(?:Y|IES)|SPECIFICATION|CHECK_DEADLOCK|SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b"
REDUCES = r"^(SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b"


class Problem(Exception):
    """A declaration or a model that is not as it must be."""


class CannotRun(Exception):
    """The check could not be made."""


def constants_of(config: Path) -> dict[str, str]:
    """The `Name = value` assignments of a .cfg's CONSTANTS section."""
    found: dict[str, str] = {}
    inside = False
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.split("\\*")[0].strip()
        if re.match(SECTION, line):
            inside = line.startswith("CONSTANT")
            continue
        match = re.match(r"^(\w+)\s*=\s*(.+)$", line)
        if inside and match:
            found[match.group(1)] = match.group(2).strip()
    return found


def properties_of(config: Path) -> set[str]:
    """The names listed under a .cfg's PROPERTIES."""
    found: set[str] = set()
    inside = False
    for line in config.read_text(encoding="utf-8").splitlines():
        line = line.split("\\*")[0].strip()
        if re.match(r"^PROPERT(Y|IES)\b", line):
            inside = True
            found.update(line.split()[1:])
        elif re.match(SECTION, line):
            inside = False
        elif inside and line:
            found.update(line.split())
    return found


def reductions_of(config: Path) -> set[str]:
    """The sections of a .cfg that take states away from what TLC explores."""
    lines = (line.split("\\*")[0].strip() for line in config.read_text(encoding="utf-8").splitlines())
    return {found.group(1) for found in (re.match(REDUCES, line) for line in lines) if found}


def promise_only(config_text: str, promise: str) -> str:
    """`config_text` with its INVARIANTS and PROPERTIES replaced by the promise."""
    kept: list[str] = []
    skipping = False
    for line in config_text.splitlines():
        if re.match(r"^(INVARIANTS?|PROPERT(Y|IES))\b", line):
            skipping = True
            continue
        if skipping and re.match(r"^[A-Z]", line):
            skipping = False
        if not skipping:
            kept.append(line)
    return "\n".join([*kept, "PROPERTIES", f"    {promise}", ""])


def declared(manifest: dict, formal: Path) -> list[str]:
    """What is wrong with the declaration itself, each as one line."""
    problems: list[str] = []
    configs: list[str] = []
    modules = [m["module"] for m in manifest["models"]]
    for model in manifest["models"]:
        unknown = set(model) - MODULE_KEYS
        if unknown:
            problems.append(f"{model['module']}: unknown keys {sorted(unknown)}")
        parts: dict[str, str] = model.get("parts", {})
        taken_out: set[str] = set()
        if model.get("abstract"):
            if model["runs"]:
                problems.append(f"{model['module']}: an abstract module is run by what refines it, not by itself")
            if not any(m.get("refines") == model["module"] for m in manifest["models"]):
                problems.append(f"{model['module']}: an abstract module nothing refines")
        elif not any(run["expect"] == "holds" for run in model["runs"]):
            problems.append(f"{model['module']}: no run says the whole protocol holds")
        if ("promise" in model) != ("refines" in model):
            problems.append(f"{model['module']}: a promise and what it refines are declared together")
        if model.get("refines") and not any(m["module"] == model["refines"] and m.get("abstract") for m in manifest["models"]):
            problems.append(f"{model['module']}: refines {model['refines']}, which is no abstract module")
        for run in model["runs"]:
            name = run["config"]
            configs.append(name)
            stray = set(run) - RUN_KEYS
            if stray:
                problems.append(f"{name}: unknown keys {sorted(stray)}")
            if not (formal / name).is_file():
                problems.append(f"{name}: declared, and no such file")
                continue
            # A behaviour cut off by a constraint ends in stuttering, where the fairness it would
            # have met is false, so a temporal property holds of it for nothing (Lamport,
            # Specifying Systems, 14.3.5).
            if properties_of(formal / name) and reductions_of(formal / name):
                problems.append(f"{name}: checks a temporal property under {sorted(reductions_of(formal / name))}")
            held = constants_of(formal / name)
            off = {part for part, whole in parts.items() if held.get(part) != whole}
            if run["expect"] == "holds":
                if off:
                    problems.append(f"{name}: holds, with {sorted(off)} taken out")
                if not isinstance(run.get("states"), int):
                    problems.append(f"{name}: a run that holds records its distinct states")
                if "promise" in model and model["promise"] not in properties_of(formal / name):
                    problems.append(f"{name}: holds, and does not check {model['promise']}")
            elif run["expect"] == "violates":
                if ("invariant" in run) == ("property" in run):
                    problems.append(f"{name}: a violation names its invariant or its property")
                # TLC does not name the temporal property it found violated, so it is the only one.
                if "property" in run and properties_of(formal / name) != {run["property"]}:
                    problems.append(f"{name}: violates {run['property']}, and checks {sorted(properties_of(formal / name))}")
                if ("without" in run) == ("against" in run):
                    problems.append(f"{name}: says either what part it takes out or what it goes against")
                if "without" in run:
                    taken_out.add(run["without"])
                    if off != {run["without"]}:
                        problems.append(
                            f"{name}: says it takes out {run['without']}, and takes out {sorted(off) or 'nothing'}"
                        )
                elif off:
                    problems.append(f"{name}: takes out {sorted(off)} without saying so")
                if "promise" in model:
                    if run.get("promise") not in ("broken", "kept"):
                        problems.append(f"{name}: says whether the promise is broken or kept")
                    if run.get("promise") == "kept" and not run.get("why"):
                        problems.append(f"{name}: the promise is kept without the part, and the run says why it is there")
            else:
                problems.append(f"{name}: expect is 'holds' or 'violates', not {run['expect']!r}")
        present = [run for run in model["runs"] if (formal / run["config"]).is_file()]
        for progress in model.get("progress", []):
            if not any(r["expect"] == "holds" and progress in properties_of(formal / r["config"]) for r in present):
                problems.append(f"{model['module']}: no run says {progress} holds")
            if not any(r["expect"] == "violates" and r.get("property") == progress for r in present):
                problems.append(f"{model['module']}: no run shows {progress} fail")
        for part in sorted(set(parts) - taken_out):
            problems.append(f"{model['module']}: no run takes out {part}")
    for missing in sorted({p.name for p in formal.glob("*.cfg")} - set(configs)):
        problems.append(f"{missing}: a configuration the manifest does not declare")
    for twice in sorted({c for c in configs if configs.count(c) > 1}):
        problems.append(f"{twice}: declared twice")
    for missing in sorted({p.stem for p in formal.glob("*.tla")} - set(modules)):
        problems.append(f"{missing}.tla: a module the manifest does not declare")
    for missing in sorted(set(modules) - {p.stem for p in formal.glob("*.tla")}):
        problems.append(f"{missing}: a declared module with no .tla")
    return problems


def judged(run: dict, status: int, output: str) -> str | None:
    """What is wrong with TLC's answer to `run`, or None where it is as declared."""
    if run["expect"] == "holds":
        if status != TLC_HOLDS:
            found = re.search(r"Invariant (\w+) is violated|Temporal properties were violated", output)
            return f"expected no violation, and TLC answered {status} ({found.group(0) if found else 'no violation named'})"
        counts = re.findall(r"(\d+) distinct states found", output)
        if not counts:
            return "TLC reported no state count"
        if int(counts[-1]) != run["states"]:
            return f"{counts[-1]} distinct states, and the manifest records {run['states']}"
        return None
    if status not in (TLC_SAFETY, TLC_LIVENESS):
        return f"expected a violation, and TLC answered {status}"
    if "property" in run:
        return None if status == TLC_LIVENESS else f"expected a violation of {run['property']}, and TLC answered {status}"
    violated = re.findall(r"Invariant (\w+) is violated", output)
    if violated != [run["invariant"]]:
        return f"expected a violation of {run['invariant']}, and TLC found {violated or 'no invariant'}"
    return None


def promise_judged(run: dict, status: int) -> str | None:
    """What is wrong with TLC's answer to the promise of a run that takes a part out."""
    if run["promise"] == "broken" and status != TLC_LIVENESS:
        return f"the promise is said to be broken, and TLC answered {status}"
    if run["promise"] == "kept" and status != TLC_HOLDS:
        return f"the promise is said to be kept, and TLC answered {status}"
    return None


def tlc_jar(manifest: dict) -> Path:
    """The pinned TLC, fetched once and checked.

    The pin is a stable release: `v1.8.0` is a rolling pre-release whose asset is
    replaced (its digest moved between 2026-10-17 and 2026-11-19), so a machine
    that fetched it later got another program under the same name.
    """
    named = os.environ.get("TLA2TOOLS")
    if named:
        return Path(named)
    pin = manifest["tlc"]
    jar = Path.home() / ".cache" / "sce-formal" / f"tla2tools-{pin['version']}.jar"
    if not jar.is_file():
        jar.parent.mkdir(parents=True, exist_ok=True)
        partial = jar.with_suffix(".partial")
        try:
            urllib.request.urlretrieve(pin["url"], partial)
        except OSError as error:
            raise CannotRun(f"TLC could not be fetched: {error}") from error
        partial.replace(jar)
    actual = hashlib.sha256(jar.read_bytes()).hexdigest()
    if actual != pin["sha256"]:
        raise CannotRun(f"{jar} is not the TLC this manifest pins ({actual})")
    return jar


def run_tlc(jar: Path, module: str, config: str, workers: str, promise: str | None = None) -> tuple[int, str]:
    if not shutil.which("java"):
        raise CannotRun("no java on PATH (Java 11 or later)")
    with tempfile.TemporaryDirectory() as work:
        for source in FORMAL.glob("*.tla"):
            shutil.copy(source, work)
        text = (FORMAL / config).read_text(encoding="utf-8")
        (Path(work) / config).write_text(promise_only(text, promise) if promise else text, encoding="utf-8")
        done = subprocess.run(
            ["java", "-XX:+UseParallelGC", "-cp", str(jar), "tlc2.TLC", "-workers", workers, "-config", config, f"{module}.tla"],
            cwd=work, capture_output=True, text=True, check=False,
        )
    return done.returncode, done.stdout + done.stderr


def check(only: str | None, workers: str, record: bool) -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    problems = declared(manifest, FORMAL)
    if problems and not record:
        for line in problems:
            print(f"models.py: {line}", file=sys.stderr)
        return 1
    jar = tlc_jar(manifest)
    failed = 0
    for model in manifest["models"]:
        if only and model["module"] != only:
            continue
        for run in model["runs"]:
            status, output = run_tlc(jar, model["module"], run["config"], workers)
            if record and run["expect"] == "holds":
                counts = re.findall(r"(\d+) distinct states found", output)
                if status == TLC_HOLDS and counts:
                    run["states"] = int(counts[-1])
            wrong = judged(run, status, output)
            if wrong is None and "promise" in run:
                promised, output = run_tlc(jar, model["module"], run["config"], workers, model["promise"])
                wrong = promise_judged(run, promised)
            print(f"{'ok  ' if wrong is None else 'FAIL'} {run['config']}: {wrong or run['expect']}", flush=True)
            if wrong is not None:
                failed += 1
                print("\n".join(output.splitlines()[-25:]), file=sys.stderr)
    if record:
        MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    if failed:
        print(f"models.py: {failed} run(s) are not as docs/formal/models.json declares", file=sys.stderr)
        return 1
    print("models.py: every model is as declared")
    return 0


class SelfTest(unittest.TestCase):
    """The checks that judge a model are judged here, with no TLC."""

    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.formal = Path(self.dir.name)
        (self.formal / "M.tla").write_text("")
        self.write("M.cfg", Part="TRUE")
        self.write("M_without_part.cfg", Part="FALSE")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def write(self, name: str, **constants: str) -> None:
        body = "\n".join(f"    {k} = {v}" for k, v in constants.items())
        (self.formal / name).write_text(f"SPECIFICATION Spec\nCONSTANTS\n{body}\nINVARIANTS\n    Safe\n")

    def manifest(self) -> dict:
        return {"models": [{
            "module": "M", "parts": {"Part": "TRUE"},
            "runs": [
                {"config": "M.cfg", "expect": "holds", "states": 7},
                {"config": "M_without_part.cfg", "expect": "violates", "invariant": "Safe", "without": "Part"},
            ],
        }]}

    def test_a_whole_declaration_is_clean(self) -> None:
        self.assertEqual(declared(self.manifest(), self.formal), [])

    def test_a_configuration_nobody_declared_is_found(self) -> None:
        self.write("M_stray.cfg", Part="TRUE")
        self.assertTrue(any("does not declare" in p for p in declared(self.manifest(), self.formal)))

    def test_a_module_nobody_declared_is_found(self) -> None:
        (self.formal / "N.tla").write_text("")
        self.assertTrue(any("N.tla" in p for p in declared(self.manifest(), self.formal)))

    def test_a_declared_configuration_that_is_not_there_is_found(self) -> None:
        (self.formal / "M.cfg").unlink()
        self.assertTrue(any("no such file" in p for p in declared(self.manifest(), self.formal)))

    def test_a_part_no_run_takes_out_is_found(self) -> None:
        manifest = self.manifest()
        manifest["models"][0]["parts"]["Other"] = "TRUE"
        self.assertTrue(any("no run takes out Other" in p for p in declared(manifest, self.formal)))

    def test_a_run_that_takes_out_nothing_is_found(self) -> None:
        self.write("M_without_part.cfg", Part="TRUE")
        self.assertTrue(any("takes out nothing" in p for p in declared(self.manifest(), self.formal)))

    def test_a_holding_run_with_a_part_out_is_found(self) -> None:
        self.write("M.cfg", Part="FALSE")
        self.assertTrue(any("holds, with" in p for p in declared(self.manifest(), self.formal)))

    def test_a_holding_run_without_a_count_is_found(self) -> None:
        manifest = self.manifest()
        del manifest["models"][0]["runs"][0]["states"]
        self.assertTrue(any("records its distinct states" in p for p in declared(manifest, self.formal)))

    def test_a_state_space_that_moved_is_found(self) -> None:
        run = {"expect": "holds", "states": 7}
        self.assertIsNone(judged(run, 0, "7 distinct states found"))
        self.assertIsNotNone(judged(run, 0, "6 distinct states found"))
        self.assertIsNotNone(judged(run, 0, "nothing"))
        self.assertIsNotNone(judged(run, 12, "Invariant Safe is violated. 7 distinct states found"))

    def promising(self) -> dict:
        manifest = self.manifest()
        manifest["models"][0]["promise"] = "Refines"
        manifest["models"][0]["refines"] = "S"
        manifest["models"].append({"module": "S", "abstract": True, "runs": []})
        (self.formal / "S.tla").write_text("")
        (self.formal / "M.cfg").write_text((self.formal / "M.cfg").read_text() + "PROPERTIES\n    Refines\n")
        manifest["models"][0]["runs"][1]["promise"] = "broken"
        return manifest

    def test_an_abstract_module_nothing_refines_is_found(self) -> None:
        manifest = self.manifest()
        (self.formal / "S.tla").write_text("")
        manifest["models"].append({"module": "S", "abstract": True, "runs": []})
        self.assertTrue(any("nothing refines" in p for p in declared(manifest, self.formal)))

    def test_a_promise_that_refines_no_abstract_module_is_found(self) -> None:
        manifest = self.promising()
        manifest["models"][0]["refines"] = "Nobody"
        self.assertTrue(any("no abstract module" in p for p in declared(manifest, self.formal)))

    def test_a_promise_that_is_checked_and_broken_is_clean(self) -> None:
        self.assertEqual(declared(self.promising(), self.formal), [])

    def test_a_holding_run_that_does_not_check_the_promise_is_found(self) -> None:
        manifest = self.promising()
        self.write("M.cfg", Part="TRUE")
        self.assertTrue(any("does not check Refines" in p for p in declared(manifest, self.formal)))

    def test_a_part_taken_out_that_says_nothing_of_the_promise_is_found(self) -> None:
        manifest = self.promising()
        del manifest["models"][0]["runs"][1]["promise"]
        self.assertTrue(any("broken or kept" in p for p in declared(manifest, self.formal)))

    def test_a_kept_promise_without_a_reason_is_found(self) -> None:
        manifest = self.promising()
        manifest["models"][0]["runs"][1]["promise"] = "kept"
        self.assertTrue(any("says why" in p for p in declared(manifest, self.formal)))

    def progressing(self) -> dict:
        manifest = self.manifest()
        manifest["models"][0]["progress"] = ["Ends"]
        (self.formal / "M.cfg").write_text((self.formal / "M.cfg").read_text() + "PROPERTIES\n    Ends\n")
        (self.formal / "M_unfair.cfg").write_text("SPECIFICATION Spec\nCONSTANTS\n    Part = TRUE\nPROPERTIES\n    Ends\n")
        manifest["models"][0]["runs"].append(
            {"config": "M_unfair.cfg", "expect": "violates", "property": "Ends", "against": "fairness"})
        return manifest

    def test_a_progress_that_holds_and_can_fail_is_clean(self) -> None:
        self.assertEqual(declared(self.progressing(), self.formal), [])

    def test_a_progress_that_never_fails_is_found(self) -> None:
        manifest = self.progressing()
        manifest["models"][0]["runs"].pop()
        (self.formal / "M_unfair.cfg").unlink()
        self.assertTrue(any("shows Ends fail" in p for p in declared(manifest, self.formal)))

    def test_a_progress_no_run_checks_is_found(self) -> None:
        manifest = self.progressing()
        self.write("M.cfg", Part="TRUE")
        self.assertTrue(any("says Ends holds" in p for p in declared(manifest, self.formal)))

    def test_a_temporal_property_under_a_constraint_is_found(self) -> None:
        for section in ("CONSTRAINT", "ACTION_CONSTRAINT", "SYMMETRY", "VIEW"):
            manifest = self.progressing()
            (self.formal / "M.cfg").write_text((self.formal / "M.cfg").read_text() + f"{section} Small\n")
            self.assertTrue(any("under" in p for p in declared(manifest, self.formal)), section)
            self.write("M.cfg", Part="TRUE")

    def test_a_violated_property_beside_another_is_found(self) -> None:
        manifest = self.progressing()
        (self.formal / "M_unfair.cfg").write_text((self.formal / "M_unfair.cfg").read_text() + "    Other\n")
        self.assertTrue(any("and checks" in p for p in declared(manifest, self.formal)))

    def test_a_property_judged_by_its_own_status(self) -> None:
        run = {"expect": "violates", "property": "Ends"}
        self.assertIsNone(judged(run, 13, "Temporal properties were violated."))
        self.assertIsNotNone(judged(run, 12, "Invariant Safe is violated."))
        self.assertIsNotNone(judged(run, 0, "7 distinct states found"))

    def test_a_promise_judged_against_what_it_says(self) -> None:
        self.assertIsNone(promise_judged({"promise": "broken"}, 13))
        self.assertIsNotNone(promise_judged({"promise": "broken"}, 0))
        self.assertIsNone(promise_judged({"promise": "kept"}, 0))
        self.assertIsNotNone(promise_judged({"promise": "kept"}, 13))

    def test_a_promise_only_configuration_keeps_its_constants(self) -> None:
        text = "SPECIFICATION Spec\nCONSTANTS\n    A = 1\nINVARIANTS\n    Safe\nPROPERTIES\n    Ends\n"
        self.assertEqual(promise_only(text, "Refines"), "SPECIFICATION Spec\nCONSTANTS\n    A = 1\nPROPERTIES\n    Refines\n")

    def test_a_violation_of_another_invariant_is_found(self) -> None:
        run = {"expect": "violates", "invariant": "Safe"}
        self.assertIsNone(judged(run, 12, "Invariant Safe is violated."))
        self.assertIsNotNone(judged(run, 12, "Invariant Other is violated."))
        self.assertIsNotNone(judged(run, 0, "7 distinct states found"))
        self.assertIsNotNone(judged(run, 150, "parse error"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Check the TLA+ models as docs/formal/models.json declares them.")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--record", action="store_true", help="write the state counts TLC found into the manifest")
    parser.add_argument("--only", help="check one module")
    parser.add_argument("--workers", default="auto")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTest)
        return 0 if unittest.TextTestRunner(verbosity=1).run(suite).wasSuccessful() else 1
    try:
        return check(args.only, args.workers, args.record)
    except CannotRun as error:
        print(f"models.py: cannot run: {error}", file=sys.stderr)
        return EXIT_CANNOT_RUN


if __name__ == "__main__":
    sys.exit(main())
