"""Which recorded guess a failure actually rests on, found by changing it.

`verify` blames a failure on every recorded guess the failing position's value
flows from (`assumption_keys_behind`). Where several decide one position
together, each is only `implicated`: at least one of them is wrong, and the
run cannot say which. This asks the question directly. For each binding guess
a failure implicates, every decision its rule makes that has a known set of
alternatives is replaced by each alternative in turn, the cases are run again,
and what changed is compared:

    witness    an alternative turns failing cases into passes and breaks no
               case that passed -- the value the specification should state
    cleared    no alternative changes any failure the guess was implicated
               in, and every alternative was tried -- the guess cannot be
               what those failures rest on
    moves      alternatives change the failures but none repairs them cleanly
    unmoved    no alternative tried changed them, but the alternatives were a
               sample (a number's), so the guess is not cleared
    untried    the rule makes no decision with a known set of alternatives,
               or the run budget ran out before any was tried

⚠ WHY A RUN AND NOT A READING. Whether a guess took part in a failing round
looks answerable from the record -- a guess that a missing number reads as 0
needs the number to be missing -- and in a document that keeps nothing it is.
Measured 2026-09-27 over thirteen components: every binding guess implicated
by a failure was of that kind, and in NO failing case was its input missing
-- but in every one of those documents the input was missing in an earlier
round of the same run, and each of those documents keeps values from one
round to the next. What an early round did may still be held; only running
the alternative says whether it still matters.

⚠ `cleared` is claimed only when the alternatives were EXHAUSTED: every other
symbol of a value space, both truth values. A number has no such list, so the
values tried are a sample -- "no change for these" is reported as such and
never clears the guess. A document's own `sce:assumed` has no alternatives
anybody wrote down and is not changed here; it is what is left when the
binding's guesses beside it are cleared, and the report says it is the only
recorded guess still standing (`sole`), which is elimination among recorded
guesses and not proof -- an unrecorded decision may be wrong as well.
"""

from __future__ import annotations

import copy
import pathlib
import tempfile
from dataclasses import dataclass, field

import yaml

# Runs one report may spend. Each alternative is a whole run of the cases, and
# a value space can be long; past this, what was not tried is said, not
# guessed at.
MAX_RUNS = 64


@dataclass
class Flip:
    """One alternative for one decision of one rule."""

    decision: str            # the rule key it replaces, e.g. "when_absent", "also.ID"
    value: object
    fixed: list = field(default_factory=list)    # failing cases that now pass
    broken: list = field(default_factory=list)   # passing cases that now fail
    moved: list = field(default_factory=list)    # failing cases that fail differently


@dataclass
class Counterfactual:
    key: str
    verdict: str
    exhaustive: bool = False
    flips: list = field(default_factory=list)
    # Decisions of the rule that were not changed, with why.
    untried: list = field(default_factory=list)
    # The failing cases this guess was blamed in -- what a witness repairs.
    failing: list = field(default_factory=list)

    def as_dict(self) -> dict:
        return {"verdict": self.verdict, "exhaustive": self.exhaustive,
                "failing": list(self.failing),
                "tried": [{"decision": f.decision, "value": f.value,
                           "fixed": f.fixed, "broken": f.broken, "moved": f.moved}
                          for f in self.flips],
                "untried": list(self.untried)}


def alternatives(rule: dict, kind: str, model) -> tuple[list, list, bool]:
    """([(decision, path, value)], [untried reason], exhaustive) for one rule.

    `path` is where in the rule the value sits, as a tuple of keys.
    """
    out, untried, exhaustive = [], [], True
    address = rule.get("address") or ""

    def symbols_of(key: str):
        field_ = model.field_at(key) if key else None
        return (field_.values or {}) if field_ is not None else {}

    def other_values(current, space: dict, where: str):
        nonlocal exhaustive
        if isinstance(current, bool):
            return [not current]
        if space:
            if isinstance(current, str) and current in space:
                return [s for s in space if s != current]
            if isinstance(current, (int, float)) and current in space.values():
                return [n for n in dict.fromkeys(space.values()) if n != current]
        if isinstance(current, (int, float)):
            # ⚠ A sample, and said to be one: no list of numbers is complete.
            exhaustive = False
            return [v for v in (current - 1, current + 1) if v != current]
        untried.append(f"{where}: {current!r} has no set of alternatives")
        exhaustive = False
        return []

    if kind == "input":
        space = symbols_of(address)
        for decision in ("when_absent", "equals"):
            if decision in rule:
                for v in other_values(rule[decision], space, decision):
                    out.append((decision, (decision,), v))
    else:
        field_name = rule.get("field")
        main_space = symbols_of(f"{address}.{field_name}" if field_name else address)
        for f, v in (rule.get("also") or {}).items():
            for alt in other_values(v, symbols_of(f"{address}.{f}"), f"also.{f}"):
                out.append((f"also.{f}", ("also", f), alt))
        for when_value, fields in (rule.get("when") or {}).items():
            for f, v in fields.items():
                for alt in other_values(v, symbols_of(f"{address}.{f}"),
                                        f"when[{when_value}].{f}"):
                    out.append((f"when[{when_value}].{f}", ("when", when_value, f), alt))
        for source, v in (rule.get("map") or {}).items():
            for alt in other_values(v, main_space, f"map[{source}]"):
                out.append((f"map[{source}]", ("map", source), alt))
    # Decisions a rule can make that have no alternatives worked out here: the
    # guess may rest on them, so the rule's alternatives are not exhausted.
    unexplored = (("equals_any", "not_equals", "becomes", "protocol", "range")
                  if kind == "input" else ("passthrough", "when_nothing_sent"))
    for key in unexplored:
        if key in rule:
            untried.append(f"{key}: not changed here")
            exhaustive = False
    return out, untried, exhaustive


def _set(rule: dict, path: tuple, value) -> dict:
    changed = copy.deepcopy(rule)
    target = changed
    for key in path[:-1]:
        target = target[key]
    target[path[-1]] = value
    return changed


def _outcomes(verification) -> list:
    """Each judged case's verdict as (name, failures), in run order."""
    return [(r.name, tuple(sorted((a, repr(w), repr(g)) for a, w, g in r.failures)))
            if r.judged else (r.name, None)
            for r in verification.results]


def explore(pack, binding_path: pathlib.Path, base, run, max_runs: int = MAX_RUNS) -> dict:
    """{assumption key: Counterfactual} for every binding guess a failure
    implicates or refutes. `run(binding_path)` is a verify run of the same
    pack and codegen; `base` is the run the report is about."""
    from .check import read_binding

    binding = read_binding(binding_path)
    document = (binding_path.parent / binding["document"]).resolve()
    before = _outcomes(base)
    failing_of: dict = {}
    for key, a in base.assumptions.items():
        cases = {e[0] for e in a.refuted_by + a.implicated_by}
        if a.source == "binding" and cases:
            failing_of[key] = cases
    found: dict = {}
    runs = 0
    with tempfile.TemporaryDirectory() as tmp:
        for key in sorted(failing_of):
            _, kind, name = key.split(":", 2)
            rule = binding[f"{kind}s"][name]
            options, untried, exhaustive = alternatives(rule, kind, pack.model)
            cf = Counterfactual(key=key, verdict="untried", exhaustive=exhaustive,
                                untried=untried, failing=sorted(failing_of[key]))
            found[key] = cf
            for decision, path, value in options:
                if runs >= max_runs:
                    cf.exhaustive = False
                    cf.untried.append(f"{decision} = {value!r}: the run budget "
                                      f"({max_runs}) was spent")
                    continue
                variant = copy.deepcopy(binding)
                variant[f"{kind}s"][name] = _set(rule, path, value)
                variant["document"] = str(document)
                path_ = pathlib.Path(tmp) / f"variant-{runs}.yaml"
                path_.write_text(yaml.safe_dump(variant, allow_unicode=True),
                                 encoding="utf-8")
                runs += 1
                after = _outcomes(run(path_))
                flip = Flip(decision=decision, value=value)
                for (name_, was), (_, now) in zip(before, after):
                    if was is None or now is None:
                        continue
                    if was and not now and name_ in failing_of[key]:
                        flip.fixed.append(name_)
                    elif not was and now:
                        flip.broken.append(name_)
                    elif was and now != was and name_ in failing_of[key]:
                        flip.moved.append(name_)
                cf.flips.append(flip)
            cf.verdict = _verdict(cf)
    return found


def repairs_all(flip: Flip, cf: Counterfactual) -> bool:
    """Whether one alternative repairs EVERY failure the guess was blamed in
    and breaks nothing.

    ⚠ Every, not some. Measured 2026-09-27: one output's guess had two
    failures, and one alternative repaired the first while another repaired
    the second -- each "a value the tests expect", reported twice, when what
    the pair says is that no single value is right: the document hands one
    value to two situations the tests tell apart."""
    return set(flip.fixed) == set(cf.failing) and not flip.broken


def _verdict(cf: Counterfactual) -> str:
    if not cf.flips:
        return "untried"
    if any(repairs_all(f, cf) for f in cf.flips):
        return "witness"
    if any(f.fixed or f.moved for f in cf.flips):
        return "moves"
    return "cleared" if cf.exhaustive else "unmoved"


def lines(gap) -> list[str]:
    """What changing a gap's guess did, for a reader: one line per finding."""
    out = []
    cf = gap.counterfactual
    if cf:
        failing = set(cf["failing"])
        # Alternatives with one outcome are one line: a value space can be
        # dozens long, and forty lines saying "breaks 1" say it once.
        grouped: dict = {}
        for f in cf["tried"]:
            outcome = (f["decision"], len(f["fixed"]), len(f["moved"]), len(f["broken"]),
                       set(f["fixed"]) == failing and not f["broken"])
            grouped.setdefault(outcome, []).append(f["value"])
        for (decision, fixed, moved, broken, repairs), values in grouped.items():
            shown = ", ".join(repr(v) for v in values[:6]) + (
                f" and {len(values) - 6} more" if len(values) > 6 else "")
            if repairs:
                out.append(f"{decision} = {shown}: repairs every failure it was "
                           f"blamed in ({fixed}), breaks none -- what the tests expect")
            elif fixed or moved or broken:
                out.append(f"{decision} = {shown}: repairs {fixed} of "
                           f"{len(failing)}, moves {moved}, breaks {broken}")
            else:
                out.append(f"{decision} = {shown}: changes nothing"
                           + ("" if cf["exhaustive"] else
                              " -- a sample, not every value it could take"))
        out.extend(f"not tried: {u}" for u in cf["untried"])
    for case, address in gap.sole:
        out.append(f"case {case!r}: {address} -- every other recorded guess "
                   f"beside this one was cleared; it is the only one left")
    return out


def sole_suspects(base, found: dict) -> dict:
    """{assumption key: [(case, address)]} -- failures at which every other
    recorded guess implicated beside this one was cleared."""
    at: dict = {}
    for key, a in base.assumptions.items():
        for e in a.implicated_by:
            at.setdefault((e[0], e[1]), set()).add(key)
    out: dict = {}
    for spot, keys in at.items():
        standing = {k for k in keys if (found.get(k) and found[k].verdict) != "cleared"}
        if len(standing) == 1 and len(keys) > 1:
            out.setdefault(next(iter(standing)), []).append(spot)
    return out
