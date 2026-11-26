#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Refuse a commit that changes what is checked together with what is judged by it.

docs/adr/0115, D11: who is asked to make a check green must not be able to do it by
changing the check. So what a check says is changed by a commit that changes nothing
else and gives its reason. What says it, and is read here out of the commit's PARENT, so
that a commit cannot say that something is no check any more:

- an abstract module of docs/formal (a promise), whole;
- in any other module, the definition of each property a holding run checks, and what only
  those are made of;
- in a configuration, the properties it checks; a configuration that is taken out;
- in models.json, a model, a run, a part, a promise, a progress, what must stay possible,
  and what a run must answer; in proofs.json, a proof, a change it must refuse, and fewer
  functions verified;
- in a file that is proved, the contracts and the checked callers.

What is ADDED is no change of a check: a new module, property, run, part, or a file that
had no contract. A comment is none either.

A commit that changes one of these may carry documents beside it (docs/**.md) and nothing
more, not the rest of the same module, the counts of states a run must find, a bound of a
configuration, the code of the proved file, the judges, or any other file. It names its
reason in a bullet of its message that begins `- Spec: `.

Judged commit by commit over a range, and only where the commit's parent holds this file:
a rule judges what comes after it. A merge is judged by the commits it brings.

Usage: spec_isolation.py <base>..<head> | --self-test
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SELF = "tools/git-hooks/spec_isolation.py"
FORMAL = "docs/formal/"
MODELS = FORMAL + "models.json"
PROOFS = FORMAL + "proofs.json"
REASON = re.compile(r"^- Spec: \S.+", re.MULTILINE)

SECTION = re.compile(
    r"^(CONSTANTS?|INVARIANTS?|PROPERT(?:Y|IES)|SPECIFICATION|CHECK_DEADLOCK|SYMMETRY|VIEW|(?:ACTION_)?CONSTRAINTS?)\b"
)
DEFINITION = re.compile(r"^([A-Za-z]\w*)(\([^)]*\))?\s*==")
TOP_LEVEL = re.compile(r"^([A-Za-z]\w*(\([^)]*\))?\s*==|VARIABLES?\b|CONSTANTS?\b|ASSUME\b|THEOREM\b|EXTENDS\b|-{4,}|={4,})")


class Unreadable(Exception):
    """The range could not be read, which is no verdict about it."""


class Repository:
    """The commits of one checkout, read through git."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def git(self, *args: str) -> str:
        done = subprocess.run(["git", *args], cwd=self.root, capture_output=True, text=True, check=False)
        if done.returncode:
            raise Unreadable(f"git {' '.join(args)}: {done.stderr.strip()}")
        return done.stdout

    def show(self, commit: str, path: str) -> str | None:
        """A file as `commit` holds it, or None."""
        done = subprocess.run(["git", "show", f"{commit}:{path}"], cwd=self.root, capture_output=True, check=False)
        return done.stdout.decode("utf-8", errors="replace") if done.returncode == 0 else None

    def commits(self, revisions: str) -> list[str]:
        return self.git("rev-list", "--reverse", "--no-merges", revisions).split()

    def changed(self, commit: str) -> list[str]:
        return self.git("diff-tree", "--no-commit-id", "--name-only", "--no-renames", "-r", "-z", commit).split("\0")[:-1]

    def message(self, commit: str) -> str:
        return self.git("log", "-1", "--format=%B", commit)


def loaded(text: str | None) -> dict:
    """A manifest, or an empty one: a file that does not parse says nothing here, and the judge refuses it."""
    try:
        value = json.loads(text) if text is not None else {}
    except ValueError:
        return {}
    return value if isinstance(value, dict) else {}


def uncommented(module: str) -> str:
    """A TLA+ module without its comments and the space they left."""
    text = re.sub(r"\(\*.*?\*\)", " ", module, flags=re.DOTALL)
    lines = (re.sub(r"\s+", " ", line.split("\\*")[0]).strip() for line in text.splitlines())
    return "\n".join(line for line in lines if line)


def definitions(module: str) -> dict[str, str]:
    """Each top-level definition of a module without comments, by its name."""
    found: dict[str, str] = {}
    name = None
    for line in uncommented(module).splitlines():
        if TOP_LEVEL.match(line):
            started = DEFINITION.match(line)
            name = started.group(1) if started else None
        if name is not None:
            found[name] = found.get(name, "") + line + "\n"
    return found


def rest_of(module: str, names: set[str]) -> str:
    """A module without comments and without the definitions `names`."""
    kept: list[str] = []
    skipping = False
    for line in uncommented(module).splitlines():
        if TOP_LEVEL.match(line):
            started = DEFINITION.match(line)
            skipping = bool(started and started.group(1) in names)
        if not skipping:
            kept.append(line)
    return "\n".join(kept)


def said_by(module: str, checked: set[str]) -> set[str]:
    """The definitions that say what `checked` says: those, and what only they are made of.

    A definition the protocol's own steps use too is the model's: changing it changes what is
    judged, and the runs that take a part out are what see a check it made hollow.
    """
    found = definitions(module)
    uses = {name: set(re.findall(r"[A-Za-z]\w*", text)) & set(found) - {name} for name, text in found.items()}

    def reached(start: set[str]) -> set[str]:
        seen: set[str] = set()
        left = [name for name in start if name in found]
        while left:
            name = left.pop()
            if name not in seen:
                seen.add(name)
                left += uses[name]
        return seen

    return (reached(checked) - reached({"Init", "Next", "Fair"})) | (checked & set(found))


def checks_of(config: str) -> tuple[set[str], str]:
    """The properties a configuration checks, and what is left of it."""
    checks: set[str] = set()
    rest: list[str] = []
    inside = False
    for line in config.splitlines():
        line = line.split("\\*")[0].strip()
        if not line:
            continue
        section = SECTION.match(line)
        if section:
            inside = section.group(1).startswith(("INVARIANT", "PROPERT"))
            line_names = line.split()[1:] if inside else []
        else:
            line_names = line.split() if inside else []
        if inside:
            checks.update(line_names)
        else:
            rest.append(line)
    return checks, "\n".join(rest)


def contract_of(source: str) -> tuple[str, str]:
    """A proved file's contracts and checked callers, and its code; each without comments and space."""
    text = re.sub(r"//[^\n]*", "", source)
    contract: list[str] = []
    code: list[str] = []
    at = 0
    while at < len(text):
        opened = re.compile(r"#\[cfg_attr\(verus_only,|#\[cfg\(verus_only\)\]").search(text, at)
        if opened is None:
            code.append(text[at:])
            break
        code.append(text[at:opened.start()])
        end = balanced(text, opened.start() + 1)
        if opened.group(0).startswith("#[cfg(verus_only)"):
            # The item the attribute is on: to the end of its block, or of its statement.
            brace, stop = text.find("{", end), text.find(";", end)
            end = balanced(text, brace) if brace != -1 and (stop == -1 or brace < stop) else stop + 1
        contract.append(text[opened.start():end])
        at = end
    squeeze = re.compile(r"\s+")
    return squeeze.sub(" ", " ".join(contract)).strip(), squeeze.sub(" ", " ".join(code)).strip()


def balanced(text: str, start: int) -> int:
    """Just past the bracket that closes the one at `start`."""
    depth = 0
    for at in range(start, len(text)):
        if text[at] in "([{":
            depth += 1
        elif text[at] in ")]}":
            depth -= 1
            if depth == 0:
                return at + 1
    return len(text)


def models_changed(before: dict, after: dict) -> tuple[list[str], list[str]]:
    """What models.json says less or otherwise, and what a run must find that moved."""
    spec: list[str] = []
    judged: list[str] = []
    now = {m.get("module"): m for m in after.get("models", [])}
    for model in before.get("models", []):
        name = model.get("module")
        if name not in now:
            spec.append(f"models.json: {name} is taken out")
            continue
        new = now[name]
        for key in sorted(set(model) - {"runs", "parts", "progress", "possible"}):
            if model[key] != new.get(key):
                spec.append(f"models.json: {name}: {key}")
        for part, whole in model.get("parts", {}).items():
            if new.get("parts", {}).get(part) != whole:
                spec.append(f"models.json: {name}: part {part}")
        for key in ("progress", "possible"):
            for lost in [p for p in model.get(key, []) if p not in new.get(key, [])]:
                spec.append(f"models.json: {name}: {key} {lost}")
        runs = {run.get("config"): run for run in new.get("runs", [])}
        for run in model.get("runs", []):
            config = run.get("config")
            if config not in runs:
                spec.append(f"models.json: {config} is taken out")
                continue
            for key in sorted((set(run) | set(runs[config])) - {"states"}):
                if run.get(key) != runs[config].get(key):
                    spec.append(f"models.json: {config}: {key}")
            if run.get("states") != runs[config].get("states"):
                judged.append(f"models.json: {config}: the states it must find")
    return spec, judged


def proofs_changed(before: dict, after: dict) -> list[str]:
    """What proofs.json says less."""
    spec: list[str] = []
    now = {p.get("file"): p for p in after.get("proofs", [])}
    for proof in before.get("proofs", []):
        name = proof.get("file")
        if name not in now:
            spec.append(f"proofs.json: {name} is taken out")
            continue
        if now[name].get("verified", 0) < proof.get("verified", 0):
            spec.append(f"proofs.json: {name}: fewer functions verified")
        kept = {m.get("breaks") for m in now[name].get("mutants", [])}
        for mutant in proof.get("mutants", []):
            if mutant.get("breaks") not in kept:
                spec.append(f"proofs.json: {name}: no longer refuses: {mutant.get('breaks')}")
    return spec


def is_judge(path: str) -> bool:
    return bool(re.match(r"(sce-build/src/formal/|scripts/gates/formal-|\.github/workflows/formal-|tools/git-hooks/(spec_isolation|spec-isolation|pre-push))", path))


def judged_commit(repository: Repository, commit: str) -> list[str]:
    """What is wrong with one commit, each as one line; none when it isolates what it changes of a check."""
    parent = f"{commit}^"
    if repository.show(parent, SELF) is None:
        return []
    models, proofs = loaded(repository.show(parent, MODELS)), loaded(repository.show(parent, PROOFS))
    declared = {m.get("module"): m for m in models.get("models", [])}
    proved = {p.get("file") for p in proofs.get("proofs", [])}
    checked: dict[str, set[str]] = {}
    for name, model in declared.items():
        for run in model.get("runs", []):
            config = repository.show(parent, FORMAL + str(run.get("config")))
            if run.get("expect") == "holds" and config is not None:
                checked.setdefault(name, set()).update(checks_of(config)[0])

    spec: list[str] = []
    beside: list[str] = []
    for path in repository.changed(commit):
        old, new = repository.show(parent, path), repository.show(commit, path)
        module = Path(path).stem
        if old is None:
            # What is new says nothing less than before. A new file of code is still beside.
            if not path.startswith("docs/"):
                beside.append(path)
        elif path == MODELS:
            less, moved = models_changed(loaded(old), loaded(new))
            spec += less
            beside += moved
        elif path == PROOFS:
            spec += proofs_changed(loaded(old), loaded(new))
        elif path.startswith(FORMAL) and path.endswith(".tla"):
            if declared.get(module, {}).get("abstract"):
                if uncommented(old) != uncommented(new or ""):
                    spec.append(f"{path}: a promise")
                continue
            names = said_by(old, checked.get(module, set()))
            before, after = definitions(old), definitions(new or "")
            for name in sorted(names & set(before)):
                if before[name] != after.get(name):
                    spec.append(f"{path}: {name}")
            if rest_of(old, names) != rest_of(new or "", names):
                beside.append(f"{path}: the model itself")
        elif path.startswith(FORMAL) and path.endswith(".cfg"):
            if new is None:
                spec.append(f"{path}: taken out")
                continue
            (had, bounds), (has, now) = checks_of(old), checks_of(new)
            for name in sorted(had - has):
                spec.append(f"{path}: no longer checks {name}")
            if bounds != now:
                beside.append(f"{path}: what it explores")
        elif path in proved:
            (contract, code), (new_contract, new_code) = contract_of(old), contract_of(new or "")
            if contract != new_contract:
                spec.append(f"{path}: its contracts")
            if code != new_code:
                beside.append(f"{path}: its code")
        elif is_judge(path):
            beside.append(f"{path}: a judge")
        elif not (path.startswith("docs/") and path.endswith(".md")):
            beside.append(path)

    if not spec:
        return []
    problems: list[str] = []
    if beside:
        problems.append("changes what is checked:")
        problems += [f"  {line}" for line in spec]
        problems.append("together with:")
        problems += [f"  {line}" for line in beside[:12]]
        if len(beside) > 12:
            problems.append(f"  and {len(beside) - 12} more")
    if not REASON.search(repository.message(commit)):
        if not beside:
            problems.append("changes what is checked:")
            problems += [f"  {line}" for line in spec]
        problems.append("and gives no reason in a bullet `- Spec: <why>`")
    return problems


def judged_range(repository: Repository, revisions: str) -> list[str]:
    problems: list[str] = []
    for commit in repository.commits(revisions):
        found = judged_commit(repository, commit)
        if found:
            problems.append(f"{commit[:11]} {repository.message(commit).splitlines()[0]}")
            problems += [f"  {line}" for line in found]
    return problems


class SelfTest(unittest.TestCase):
    """Commits of a repository made here, each judged."""

    PROMISE = "---- MODULE S ----\nVARIABLES x\nSpec == x = 0\n====\n"
    MODEL = "---- MODULE M ----\nVARIABLES x\nInit == x = 0\n\\* what is promised\nSafe == x < 3\nNext == x' = x\n====\n"
    CONFIG = "SPECIFICATION Spec\nCONSTANTS\n    N = 2\nINVARIANTS\n    Safe\n"
    PROVED = ("#[cfg(verus_only)]\nuse vstd::prelude::*;\n"
              "#[cfg_attr(verus_only, verus_spec(r => ensures r <= a))]\npub fn low(a: u64) -> u64 { a }\n"
              "#[cfg(verus_only)]\nverus! { proof fn l() ensures true {} }\n")

    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        self.repository = Repository(self.root)
        for args in (["init", "-q"], ["config", "user.name", "t"], ["config", "user.email", "t@example.com"]):
            self.repository.git(*args)
        self.put(SELF, "")
        self.put(FORMAL + "S.tla", self.PROMISE)
        self.put(FORMAL + "M.tla", self.MODEL)
        self.put(FORMAL + "M.cfg", self.CONFIG)
        self.put(MODELS, json.dumps({"models": [
            {"module": "M", "parts": {"P": "TRUE"}, "progress": ["Ends"],
             "runs": [{"config": "M.cfg", "expect": "holds", "states": 7}]},
            {"module": "S", "abstract": True, "runs": []}]}))
        self.put("rules.rs", self.PROVED)
        self.put(PROOFS, json.dumps({"proofs": [{"file": "rules.rs", "verified": 2, "mutants": [{"breaks": "b", "from": "a", "to": "c"}]}]}))
        self.commit("feat: Begin")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def put(self, path: str, text: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def swap(self, path: str, old: str, new: str) -> None:
        text = (self.root / path).read_text(encoding="utf-8")
        assert old in text
        self.put(path, text.replace(old, new))

    def commit(self, message: str) -> str:
        self.repository.git("add", "-A")
        self.repository.git("commit", "-q", "--no-verify", "-m", message)
        return self.repository.git("rev-parse", "HEAD").strip()

    def judged(self, message: str = "feat: X\n\n- y") -> str:
        return "\n".join(judged_commit(self.repository, self.commit(message)))

    REASONED = "docs: X\n\n- Spec: it said too much"

    def test_a_model_changed_by_itself_is_clean(self) -> None:
        self.swap(FORMAL + "M.tla", "x' = x", "x' = x + 1")
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_property_changed_by_itself_for_a_reason_is_clean(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.put("docs/adr/1.md", "why")
        self.assertEqual(self.judged(self.REASONED), "")

    def test_a_property_changed_without_a_reason_is_refused(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.assertIn("gives no reason", self.judged())

    def test_a_property_changed_with_its_model_is_refused(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.swap(FORMAL + "M.tla", "x' = x", "x' = x + 1")
        found = self.judged(self.REASONED)
        self.assertIn("M.tla: Safe", found)
        self.assertIn("the model itself", found)

    def test_a_property_changed_with_code_is_refused(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.put("code.rs", "fn f() {}")
        self.assertIn("code.rs", self.judged(self.REASONED))

    def test_a_property_changed_with_a_judge_is_refused(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.put("sce-build/src/formal/models.rs", "pass")
        self.commit("chore: judge")
        self.swap(FORMAL + "M.tla", "x < 9", "x < 11")
        self.swap("sce-build/src/formal/models.rs", "pass", "pass  // all")
        self.assertIn("a judge", self.judged(self.REASONED))

    def test_a_property_taken_out_is_seen(self) -> None:
        self.swap(FORMAL + "M.tla", "Safe == x < 3\n", "")
        self.assertIn("M.tla: Safe", self.judged())

    def test_what_only_a_property_is_made_of_is_a_check(self) -> None:
        self.swap(FORMAL + "M.tla", "Safe == x < 3\n", "Bound == 3\nSafe == x < Bound\n")
        self.commit("docs: X\n\n- Spec: name the bound")
        self.swap(FORMAL + "M.tla", "Bound == 3", "Bound == 9")
        self.assertIn("M.tla: Bound", self.judged())

    def test_what_the_steps_use_too_is_the_model(self) -> None:
        self.swap(FORMAL + "M.tla", "Safe == x < 3\n", "Bound == 3\nSafe == x < Bound\n")
        self.swap(FORMAL + "M.tla", "x' = x", "x' = Bound")
        self.commit("docs: X\n\n- y")
        self.swap(FORMAL + "M.tla", "Bound == 3", "Bound == 9")
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_comment_of_a_property_is_no_change(self) -> None:
        self.swap(FORMAL + "M.tla", "what is promised", "what a user relies on")
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_promise_changed_with_anything_is_refused(self) -> None:
        self.swap(FORMAL + "S.tla", "x = 0", "TRUE")
        self.put("code.rs", "fn f() {}")
        self.assertIn("S.tla: a promise", self.judged(self.REASONED))

    def test_a_new_promise_beside_its_model_is_clean(self) -> None:
        self.put(FORMAL + "T.tla", self.PROMISE)
        self.put(FORMAL + "T.cfg", self.CONFIG)
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_check_taken_out_of_a_configuration_is_seen(self) -> None:
        self.swap(FORMAL + "M.cfg", "    Safe\n", "")
        self.assertIn("no longer checks Safe", self.judged())

    def test_a_check_added_is_no_change(self) -> None:
        self.swap(FORMAL + "M.cfg", "    Safe\n", "    Safe\n    Other\n")
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_configuration_narrowed_with_a_property_is_refused(self) -> None:
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.swap(FORMAL + "M.cfg", "N = 2", "N = 1")
        self.assertIn("what it explores", self.judged(self.REASONED))

    def test_a_configuration_taken_out_is_seen(self) -> None:
        (self.root / FORMAL / "M.cfg").unlink()
        self.assertIn("M.cfg: taken out", self.judged())

    def test_what_a_run_must_answer_is_a_check(self) -> None:
        self.swap(MODELS, '"holds"', '"violates"')
        self.assertIn("M.cfg: expect", self.judged())

    def test_a_count_moved_alone_is_no_change_of_a_check(self) -> None:
        self.swap(MODELS, "7", "9")
        self.swap(FORMAL + "M.tla", "x' = x", "x' = x + 1")
        self.assertEqual(self.judged(), "")

    def test_a_count_moved_with_a_property_is_refused(self) -> None:
        self.swap(MODELS, "7", "9")
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.assertIn("the states it must find", self.judged(self.REASONED))

    def test_a_part_and_a_progress_taken_out_are_seen(self) -> None:
        self.swap(MODELS, '{"P": "TRUE"}', "{}")
        self.swap(MODELS, '["Ends"]', "[]")
        found = self.judged()
        self.assertIn("part P", found)
        self.assertIn("progress Ends", found)

    def test_a_contract_changed_with_its_code_is_refused(self) -> None:
        self.swap("rules.rs", "r <= a", "true")
        self.swap("rules.rs", "{ a }", "{ a + 1 }")
        found = self.judged(self.REASONED)
        self.assertIn("its contracts", found)
        self.assertIn("its code", found)

    def test_a_contract_changed_by_itself_for_a_reason_is_clean(self) -> None:
        self.swap("rules.rs", "r <= a", "r < a")
        self.assertEqual(self.judged(self.REASONED), "")

    def test_a_checked_caller_is_a_contract(self) -> None:
        self.swap("rules.rs", "proof fn l() ensures true {}", "")
        self.assertIn("its contracts", self.judged())

    def test_code_of_a_proved_file_changes_by_itself(self) -> None:
        self.swap("rules.rs", "{ a }", "{ a + 0 }")
        self.put("code.rs", "fn f() {}")
        self.assertEqual(self.judged(), "")

    def test_a_mutant_taken_out_is_seen(self) -> None:
        self.swap(PROOFS, '[{"breaks": "b", "from": "a", "to": "c"}]', "[]")
        self.assertIn("no longer refuses: b", self.judged())

    def test_a_mutant_that_follows_the_code_is_no_change(self) -> None:
        self.swap(PROOFS, '"from": "a"', '"from": "a + 0"')
        self.swap("rules.rs", "{ a }", "{ a + 0 }")
        self.assertEqual(self.judged(), "")

    def test_fewer_functions_verified_is_seen(self) -> None:
        self.swap(PROOFS, '"verified": 2', '"verified": 1')
        self.assertIn("fewer functions", self.judged())

    def test_what_comes_before_the_rule_is_not_judged(self) -> None:
        (self.root / SELF).unlink()
        self.commit("chore: without")
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.swap(FORMAL + "M.tla", "x' = x", "x' = x + 1")
        self.assertEqual(self.judged(), "")

    def test_taking_the_rule_out_with_a_check_is_refused(self) -> None:
        (self.root / SELF).unlink()
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.assertIn("a judge", self.judged(self.REASONED))

    def test_a_range_names_each_commit(self) -> None:
        base = self.repository.git("rev-parse", "HEAD").strip()
        self.swap(FORMAL + "M.tla", "x < 3", "x < 9")
        self.commit("feat: First")
        self.swap(FORMAL + "M.tla", "x < 9", "x < 11")
        self.commit("feat: Second\n\n- Spec: a reason")
        found = "\n".join(judged_range(self.repository, f"{base}..HEAD"))
        self.assertIn("feat: First", found)
        self.assertNotIn("feat: Second", found)


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return 0 if unittest.main(argv=[sys.argv[0]], exit=False).result.wasSuccessful() else 1
    if len(sys.argv) != 2 or ".." not in sys.argv[1]:
        print("usage: spec_isolation.py <base>..<head> | --self-test", file=sys.stderr)
        return 2
    try:
        problems = judged_range(Repository(Path(".")), sys.argv[1])
    except Unreadable as reason:
        print(f"spec-isolation: could not read the range: {reason}", file=sys.stderr)
        return 3
    for line in problems:
        print(line)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
