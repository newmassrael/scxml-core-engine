"""Which recorded guess a failure actually rests on, found by changing it.

`verify` blames a failure on every recorded guess the failing position's value
flows from (`assumption_keys_behind`). Where several decide one position
together, each is only `implicated`: at least one of them is wrong, and the
run cannot say which. This asks the question directly. For each recorded
guess a failure implicates, every decision it makes that has a known set of
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
symbol of a value space, both truth values -- and for a number, one value per
interval the document's own thresholds cut, which is every behaviour a
document has that only COMPARES the number with constants (`number_uses`),
inside the range the interface model gives it. A number the document does
anything else with (arithmetic, `previous()`) has no complete list: the
values tried are a sample, reported as such, and never clear the guess.

A document's own `sce:assumed` is changed to each of its
`sce:assumed-candidates`, the values its author weighed with the chosen one
among them (`candidate_site` fixes what that attribute means; the product
accepts it and reads nothing from it). Exhaustive over what the author
weighed, which the report says. Without candidates it is not changed; it is
what is left when the guesses beside it are cleared, and the report says it
is the only recorded guess still standing (`sole`) -- elimination among
RECORDED guesses, not proof: an unrecorded decision may be wrong as well.
"""

from __future__ import annotations

import copy
import pathlib
import re
import tempfile
import xml.etree.ElementTree as ET
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


_OPS = r"(<=|>=|===|!==|==|!=|<|>)"
_NUMBER = r"(-?\d+(?:\.\d+)?)"


def _token(text: str) -> str:
    """`text` as a whole token: not part of a longer name or number."""
    return rf"(?<![\w.]){re.escape(text)}(?![\w.])"


def candidate_site(expr: str, candidates) -> tuple[str, str]:
    """(the chosen candidate, '') -- or ('', why the site is not known).

    ⚠ The product accepts `sce:assumed-candidates` and reads nothing from it;
    what it MEANS is fixed here, where it is first used: the values the
    author weighed, the chosen one AMONG them, and the chosen one is where
    the others go. So the expression must hold exactly one of them, once --
    or be one of them whole. Anything else would be this tool deciding which
    part of the author's expression the list was about.
    """
    if expr.strip() in candidates:
        return expr.strip(), ""
    found = [(c, len(re.findall(_token(c), expr))) for c in candidates]
    present = [(c, n) for c, n in found if n]
    if len(present) == 1 and present[0][1] == 1:
        return present[0][0], ""
    if not present:
        return "", (f"none of its candidates ({' '.join(candidates)}) appears in "
                    f"its expression, so which part of it they replace is not "
                    f"known -- list the value it chose among them")
    return "", (f"its expression holds {', '.join(f'{c} ×{n}' for c, n in present)} "
                f"of its candidates, so which one it chose is not known -- the "
                f"chosen value must appear once")


def with_candidate(expr: str, chosen: str, other: str) -> str:
    """The expression with `other` where `chosen` is (`candidate_site`)."""
    if expr.strip() == chosen:
        return other
    return re.sub(_token(chosen), lambda _m: other, expr, count=1)


def number_uses(name: str, texts) -> tuple[list, bool]:
    """(the thresholds `name` is compared against, whether that is ALL it is
    used for) across every expression and condition of a document.

    Where every use is a comparison with a literal, the document cannot tell
    two values apart that fall on the same side of every threshold, so one
    value per interval is every behaviour it has -- the alternatives are
    EXHAUSTIVE. Any other use (arithmetic, `previous()`, a comparison with
    another variable) and they are a sample.
    """
    thresholds, uses, compared = set(), 0, 0
    left = re.compile(_token(name) + rf"\s*{_OPS}\s*{_NUMBER}")
    right = re.compile(rf"{_NUMBER}\s*{_OPS}\s*" + _token(name))
    for text in texts:
        text = re.sub(r"'[^']*'|\"[^\"]*\"", "''", text)
        uses += len(re.findall(_token(name), text))
        for match in left.finditer(text):
            thresholds.add(float(match.group(2)))
            compared += 1
        for match in right.finditer(text):
            thresholds.add(float(match.group(1)))
            compared += 1
    return sorted(thresholds), uses == compared


def number_alternatives(current, thresholds, whole: bool, bounds) -> list:
    """One value per interval the thresholds cut, and each threshold itself,
    inside the platform's range; `current` left out."""
    low, high = bounds or (None, None)
    points = set()
    if not thresholds:
        points.add(current + 1)
    for t in thresholds:
        points.add(t)
        if whole:
            points.update((t - 1, t + 1))
    if not whole:
        points.update((thresholds[0] - 1, thresholds[-1] + 1) if thresholds else ())
        points.update((a + b) / 2 for a, b in zip(thresholds, thresholds[1:]))
    keep = sorted(p for p in points
                  if p != current
                  and (low is None or p >= low) and (high is None or p <= high))
    return [int(p) if whole and float(p).is_integer() else p for p in keep]


def document_texts(document: pathlib.Path) -> list:
    """Every expression and condition a document evaluates."""
    root = ET.parse(document).getroot()
    return [value for element in root.iter()
            for attribute, value in element.attrib.items()
            if attribute in ("expr", "cond")]


def alternatives(rule: dict, kind: str, model, number_space=None) -> tuple[list, list, bool]:
    """([(decision, path, value)], [untried reason], exhaustive) for one rule.

    `path` is where in the rule the value sits, as a tuple of keys.
    `number_space`, for an input, is `(thresholds, only compared, whole
    numbers)` of the document input it feeds (`number_uses`).
    """
    out, untried, exhaustive = [], [], True
    address = rule.get("address") or ""

    def symbols_of(key: str):
        field_ = model.field_at(key) if key else None
        return (field_.values or {}) if field_ is not None else {}

    def other_values(current, space: dict, where: str, partition=None):
        nonlocal exhaustive
        if isinstance(current, bool):
            return [not current]
        if space:
            if isinstance(current, str) and current in space:
                return [s for s in space if s != current]
            if isinstance(current, (int, float)) and current in space.values():
                return [n for n in dict.fromkeys(space.values()) if n != current]
        if isinstance(current, (int, float)):
            field_ = model.field_at(address) if address else None
            bounds = field_.range if field_ is not None else None
            if partition is not None:
                thresholds, only_compared, whole = partition
                if not only_compared:
                    # ⚠ A sample, and said to be one: the document does more
                    # with the number than compare it, so no list is complete.
                    exhaustive = False
                    untried.append(f"{where}: the document uses it beyond comparing "
                                   f"it with a constant, so the values tried "
                                   f"({len(thresholds)} threshold(s) and their "
                                   f"neighbours) are a sample")
                return number_alternatives(current, thresholds, whole, bounds)
            exhaustive = False
            return [v for v in (current - 1, current + 1) if v != current]
        untried.append(f"{where}: {current!r} has no set of alternatives")
        exhaustive = False
        return []

    if kind == "input":
        space = symbols_of(address)
        for decision in ("when_absent", "equals"):
            if decision in rule:
                # The number an absence reads as IS the input's value in that
                # round, so the document's own thresholds cut its alternatives.
                partition = number_space if decision == "when_absent" else None
                for v in other_values(rule[decision], space, decision, partition):
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


_DATA = re.compile(r"<data\b((?:\s+[\w:.-]+\s*=\s*(?:\"[^\"]*\"|'[^']*'))*)(\s*/?>)")
_ATTRIBUTE = re.compile(r"(\s+)([\w:.-]+)(\s*=\s*)(\"[^\"]*\"|'[^']*')")


def document_with(text: str, ident: str, expr: str) -> str:
    """The document's text with `<data id=ident>`'s `expr` replaced.

    ⚠ On the TEXT, not a re-serialised tree: a parser keeps no record of
    namespace prefixes, and the product reads `sce:` by prefix as much as by
    namespace (`read_document` says why), so a rewritten tree is a different
    document. Each attribute value is delimited by its own quote, so a `>`
    inside an expression -- legal XML -- does not end the element early."""
    escaped = (expr.replace("&", "&amp;").replace("<", "&lt;")
               .replace(">", "&gt;").replace('"', "&quot;"))

    def element(match: re.Match) -> str:
        attributes = match.group(1)
        if not re.search(rf"\sid\s*=\s*([\"']){re.escape(ident)}\1", attributes):
            return match.group(0)

        def one(a: re.Match) -> str:
            if a.group(2) != "expr":
                return a.group(0)
            return f'{a.group(1)}expr{a.group(3)}"{escaped}"'
        return "<data" + _ATTRIBUTE.sub(one, attributes) + match.group(2)

    changed = _DATA.sub(element, text)
    if changed == text:
        raise ValueError(f"no <data id={ident!r}> with an expr in the document")
    return changed


def explore(pack, binding_path: pathlib.Path, base, run, max_runs: int = MAX_RUNS) -> dict:
    """{assumption key: Counterfactual} for every recorded guess a failure
    implicates or refutes: a binding rule's decisions over their alternatives,
    and a document's `sce:assumed` over the candidates its author wrote.
    `run(binding_path)` is a verify run of the same pack and codegen; `base`
    is the run the report is about."""
    from .check import read_binding, read_document
    from .verify import imports_made_absolute

    binding = read_binding(binding_path)
    document = (binding_path.parent / binding["document"]).resolve()
    declared = read_document(document)
    texts = document_texts(document)
    source = imports_made_absolute(document.read_text(encoding="utf-8"), document)
    before = _outcomes(base)
    failing_of: dict = {}
    for key, a in base.assumptions.items():
        cases = {e[0] for e in a.refuted_by + a.implicated_by}
        if cases:
            failing_of[key] = cases
    found: dict = {}
    runs = 0
    with tempfile.TemporaryDirectory() as tmp:
        for key in sorted(failing_of):
            source_kind, _, subject = key.partition(":")
            if source_kind == "document":
                options, untried, exhaustive = document_options(subject, declared)
            else:
                kind, _, name = subject.partition(":")
                rule = binding[f"{kind}s"][name]
                space = None
                if kind == "input" and isinstance(rule.get("when_absent"), (int, float)) \
                        and not isinstance(rule.get("when_absent"), bool):
                    thresholds, only = number_uses(name, texts)
                    whole = str(declared.types.get(name, "")).startswith(("int", "uint"))
                    space = (thresholds, only, whole)
                options, untried, exhaustive = alternatives(rule, kind, pack.model, space)
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
                if source_kind == "document":
                    # The document changes; it goes beside the variant binding
                    # under its own name, its imports pointing home.
                    home = pathlib.Path(tmp) / f"variant-{runs}"
                    home.mkdir()
                    changed = home / document.name
                    expr, chosen = path
                    changed.write_text(
                        document_with(source, subject, with_candidate(expr, chosen, value)),
                        encoding="utf-8")
                    variant["document"] = str(changed)
                else:
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


def document_options(ident: str, declared) -> tuple[list, list, bool]:
    """What a document's `sce:assumed` can be changed to: its expression with
    each other candidate its author weighed where the chosen one is. The
    candidates the author listed are all there are, so trying each is
    exhaustive -- over what the author weighed, which is what the report
    says it is."""
    listed = declared.assumed_candidates.get(ident)
    if not listed:
        return [], [("no `sce:assumed-candidates` on it: nobody wrote down what "
                     "else it could have been")], False
    candidates, expr = listed
    chosen, why = candidate_site(expr, candidates)
    if why:
        return [], [why], False
    return ([("candidate", (expr, chosen), other)
             for other in candidates if other != chosen], [], True)


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
                              " -- but not every alternative of this guess was "
                              "tried (below), so it is not cleared"))
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
