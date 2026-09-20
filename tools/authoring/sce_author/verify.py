"""Run the pack's examples against the document somebody wrote.

This closes the loop. Until it existed the tool could say what a specification
failed to answer and whether a document's names were real, and nothing could
say whether the document BEHAVED. An author -- or a model writing on their
behalf -- wrote a document and got no verdict, so the only judge of correctness
was a subject-matter-specific harness living outside this package. A workflow
whose last step is "and hope" is not a workflow.

The materials were already here:

    the examples carry values, so they are a test suite
    the binding says which address feeds which input and receives which
        output, so marshalling needs no subject knowledge
    the product generates runnable code from the document

So verification is those three composed, and it stays domain-free for the same
reason every other check here does: the binding is the only thing that knows
what anything means, and it is data the caller supplied.

⚠ What this REFUSES to do is as important as what it does. A document whose
kind it cannot drive, an input rule it cannot evaluate, a case that needs a
previous round -- each is reported as a thing it could not judge, never as a
pass. A verifier that silently skips what it does not understand reports a
clean run for a document it never executed.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field

from .check import read_binding
from .errors import AuthoringError
from .pack import Pack

# The kinds whose generated shape is one function per output, taking the
# document's inputs by name. Everything else -- a statechart, an observer that
# emits events, a procedure -- has a different calling convention, and guessing
# at it would produce a verdict about something that was never run.
CALLABLE_KINDS = frozenset({"transform", "lookup", "condition", "interpolation"})

# The one backend this can DRIVE, which is a different list from the ones the
# product can EMIT.
#
# ⚠ Python is here because its generated form is importable into this process
# and its runtime is a package this process can reach: the document becomes an
# object, and driving it is calling methods. Every other backend the product
# emits is a build and a separate process, so driving one needs three things
# that do not exist yet -- a build step per language, a host program that
# stands up the engine and registers what a verifier registers, and a wire
# carrying each case's inputs in and each reading out.
#
# ⚠⚠ What that costs is stated rather than hidden: a pass here is a pass for
# the PYTHON lowering, and most of this product ships as C++. The backend
# parity suite compares what the six emitters WRITE, byte for byte, which is a
# different claim from how they BEHAVE under these cases. Nothing in this tree
# makes the second claim, so the verdict carries the backend it is about.
DRIVEN_BACKENDS = frozenset({"python"})

SCE_NS = "{http://sce.dev/ext}"


class VerifyError(AuthoringError):
    """Verification could not be performed. Never a verdict about behaviour."""


def _snake(name: str) -> str:
    """The generator's own spelling: `shownRpm` becomes `shown_rpm`."""
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).lower()


@dataclass
class CaseResult:
    name: str
    failures: list[tuple[str, object, object]] = field(default_factory=list)
    unchecked: list[str] = field(default_factory=list)
    # ⚠ Why THIS case could not be driven, when the rest could. It is kept per
    # case rather than ending the run: one case whose inputs are incomplete
    # used to abort everything, turning "four of forty-two cannot be judged"
    # into "nothing can be judged". A refusal belongs to the smallest thing it
    # is actually about.
    refusal: str = ""

    @property
    def judged(self) -> bool:
        return not self.refusal

    @property
    def passed(self) -> bool:
        return self.judged and not self.failures


@dataclass
class Verification:
    """What running the examples said, or why they could not be run."""

    refusal: str = ""
    # Which lowering of the document was actually run. ⚠ Carried on the
    # verdict rather than assumed by its reader: "every case passed" is a
    # statement about one backend, and the one driven here is not usually the
    # one a product ships. A reader told only the result supplies the rest of
    # the sentence themselves, and supplies it wrongly.
    backend: str = ""
    results: list[CaseResult] = field(default_factory=list)
    # Addresses the examples expect that the binding never writes. Reported
    # separately from a failure: nothing was computed wrongly, the document
    # simply has nothing to say there.
    unbound: list[str] = field(default_factory=list)
    # ⚠ THE OTHER HALF OF `unbound`, AND THE ONE A PASS NEEDS. Positions the
    # document writes that NO case ever expects. Nothing failed there because
    # nothing looked: "every case passed" is a statement about the cases, and
    # without this figure a reader supplies the rest of the sentence
    # themselves. A run that judged two of nine outputs and a run that judged
    # nine of nine print the same line otherwise, and the difference between
    # them is the whole value of running anything.
    unasserted: list[str] = field(default_factory=list)
    # ⚠ Failing addresses whose value the DOCUMENT marks `sce:assumed`, to the
    # reason its author wrote. An assumption compiles, so nothing downstream
    # ever mentioned it again and a case refuting one read as "your document
    # is wrong" -- when the author had already written down that this exact
    # value was a guess. Naming it turns the report into the one sentence
    # worth having: the guess you recorded is the thing that failed.
    refuted: dict = field(default_factory=dict)

    @property
    def ran(self) -> bool:
        return not self.refusal

    @property
    def passed(self) -> int:
        return sum(1 for r in self.results if r.passed)

    @property
    def failed(self) -> int:
        return sum(1 for r in self.results if r.judged and not r.passed)

    @property
    def unjudged(self) -> int:
        return sum(1 for r in self.results if not r.judged)


# ------------------------------------------------------------------ the code


@dataclass
class Build:
    """What the product said when it built the document.

    ⚠ The manifest used to be discarded on success, and it is the only place
    that answers two questions a driver cannot answer for itself: which
    `<send type>` the document executes, and whether it needs a scheduler to
    reach its own delayed acts. Reading them off the binding instead would
    ask the dictionary about the document.
    """

    refusal: str = ""
    manifest: dict = field(default_factory=dict)
    # The types this build was TOLD the host serves. ⚠ Kept rather than
    # re-derived, because the manifest stops naming them once it is told: the
    # declaration is exactly what makes the cause disappear. Deriving the
    # registration list from the manifest of the build that can be run
    # therefore registers nothing at all.
    declared: tuple = ()

    @property
    def unreachable(self) -> tuple[str, ...]:
        """Processor types the document sends to and this build cannot serve.

        ⚠ W3C SCXML 6.2.5: such a send compiles to a runtime
        `error.execution`, so a machine driven here takes its transitions and
        reaches nobody. Empty is the only state worth running.
        """
        seen = {}
        for cause in self.manifest.get("host_processor_causes") or ():
            kind = cause.get("processor_type")
            if kind:
                seen[kind] = True
        return tuple(seen)

    @property
    def needs_event_scheduler(self) -> bool:
        return bool(self.manifest.get("needs_event_scheduler"))


def generate(document: pathlib.Path, codegen: pathlib.Path,
             into: pathlib.Path, backend: str = "python") -> Build:
    """Build the document, declaring whatever host processors it sends to.

    ⚠ TWO PASSES, and it is the product's own handshake rather than a way
    around one. A `<send type="x">` compiles to a runtime `error.execution`
    until the build is told the host serves `x`; with `--host-processor x` the
    same site compiles to a dispatch, and the manifest's cause for it
    disappears. So the first pass is how the types become known and the second
    is how they become reachable. Driven without it, a machine took its
    transitions, sent nothing anybody could receive, and every case read the
    resting value -- a full run, judged, and about a document nobody could
    hear.
    """
    build = _emit(document, codegen, into, (), backend)
    if build.refusal or not build.unreachable:
        return build
    declared = build.unreachable
    build = _emit(document, codegen, into, declared, backend)
    if build.refusal:
        return build
    if build.unreachable:
        # The declaration is what makes a cause disappear, so one that
        # survives it names a type this pass did not reach.
        return Build(refusal=(
            f"{document.name} still sends to "
            f"{', '.join(build.unreachable)} after the build was told about "
            f"{', '.join(declared)}. Running it would drive a machine whose "
            f"sends raise error.execution instead of reaching anybody."))
    build.declared = declared
    return build


def _emit(document: pathlib.Path, codegen: pathlib.Path, into: pathlib.Path,
          host_processors, backend: str) -> Build:
    """One run of the generator. Its refusal, or its manifest.

    ⚠ The product's own words are passed through untouched. A document with an
    open decision in it is refused here BY THE PRODUCT, and that refusal names
    the placeholder and the reason the author wrote for it -- which is exactly
    the feedback the author needs. Re-phrasing it would lose the reason.
    """
    if not codegen.exists():
        raise VerifyError(
            f"{codegen}: the code generator is not there, so no document can "
            f"be run. Build it, or name another with --codegen.")
    argv = [str(codegen), "generate", str(document), "-o", str(into),
            "-l", backend]
    for kind in host_processors:
        argv += ["--host-processor", kind]
    run = subprocess.run(argv, capture_output=True, text=True)
    if run.returncode != 0:
        return Build(refusal=(run.stderr.strip() or run.stdout.strip()
                              or f"the code generator refused with status "
                                 f"{run.returncode}"))
    # ⚠ One JSON object on one line is what the generator promises, and a
    # driver that could not parse it used to carry on with an empty manifest
    # -- registering no processor and reporting the error run that followed as
    # the document's behaviour. It is a refusal instead.
    for line in reversed(run.stdout.strip().splitlines()):
        try:
            return Build(manifest=json.loads(line))
        except json.JSONDecodeError:
            continue
    return Build(refusal=(
        f"the code generator built {document.name} and printed no manifest, "
        f"so nothing says which host processors the document sends to"))


def pseudo_page(document: pathlib.Path, codegen: pathlib.Path | None,
                deploy: pathlib.Path | None) -> tuple[str, str]:
    """One run of the generator's review surface. Returns `(page, refusal)`.

    ⚠ It lives HERE, in the module that already spawns, rather than beside
    the rest of the rendering seam in `pseudo.py`. Exactly one module in this
    core may run another program -- `test_only_one_module_may_run_another_
    program` holds that -- and the way to add a second caller is to give it a
    function here, not to widen the allowed list. The rule is about how many
    places can reach outside the pack, and composing keeps that at one.

    ⚠ The product's own words come back untouched, the same as `_emit` does
    with a build refusal. A document the surface will not abbreviate is
    refused BY THE PRODUCT, and that refusal names the construct -- which is
    what the author needs to read.
    """
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    if not codegen.exists():
        raise VerifyError(
            f"{codegen}: the code generator is not there, so no document can "
            f"be shown. Build it, or name another with --codegen.")
    argv = [str(codegen), "pseudo", str(document)]
    if deploy is not None:
        argv += ["--deploy", str(deploy)]
    run = subprocess.run(argv, capture_output=True, text=True)
    if run.returncode != 0:
        return "", (run.stderr.strip() or run.stdout.strip()
                    or f"the code generator refused with status "
                       f"{run.returncode}")
    # ⚠ Byte for byte, not stripped. The surface's contract is that a value
    # reaches the page as the author spelled it, and a caller that trims the
    # page is the first thing to break it.
    return run.stdout, ""


def load(into: pathlib.Path, document: pathlib.Path):
    """Import what was generated, as a package so its own imports resolve."""
    # ⚠ A generated statechart imports the product's own runtime; a generated
    # pure computation does not, which is why nothing needed this until one
    # was driven. Without it the import died with `No module named
    # 'sce_runtime'` -- a traceback out of a verifier, about the verifier's
    # environment rather than about the document it was asked to judge.
    runtime = _default_runtime()
    if runtime.is_dir() and str(runtime) not in sys.path:
        sys.path.insert(0, str(runtime))
    stem = document.stem.lower().replace("_", "")
    emitted = [p for p in into.glob("*.py") if p.name != "__init__.py"]
    if not emitted:
        raise VerifyError(f"{document}: the generator wrote no python")
    main = next((p for p in emitted if p.stem.lower().replace("_", "") == stem),
                emitted[0])
    (into / "__init__.py").write_text("", encoding="utf-8")
    sys.path.insert(0, str(into.parent))
    parent_spec = importlib.util.spec_from_file_location(
        into.name, into / "__init__.py", submodule_search_locations=[str(into)])
    parent = importlib.util.module_from_spec(parent_spec)
    sys.modules[into.name] = parent
    parent_spec.loader.exec_module(parent)
    spec = importlib.util.spec_from_file_location(
        f"{into.name}.{main.stem}", main, submodule_search_locations=[str(into)])
    module = importlib.util.module_from_spec(spec)
    sys.modules[f"{into.name}.{main.stem}"] = module
    spec.loader.exec_module(module)
    return module


# ------------------------------------------------------------ the marshalling


def _given(case, address: str):
    return case.given.get(address)


class Latches:
    """The protocol state carried from one case to the next.

    ⚠ A protocol is not a function of its parameters' CURRENT values. The one
    this was measured against answers by which parameter changed most recently,
    so the same values mean different things depending on what came before --
    which is why it is a protocol and not a comparison. Evaluating it therefore
    needs the cases in the order they happened, and a pack that does not say
    they are ordered gets a refusal instead of a guess.

    Verified against all five cases of one component: two counters, the lamp
    dark until the on-counter first ticks, and dark again on the round the
    off-counter ticks -- including the round where BOTH ticked, which is why
    the tie-break is declared rather than assumed.
    """

    def __init__(self, conventions):
        self.protocols = getattr(conventions, "protocols", None) or {}
        self.previous: dict[str, object] = {}
        self.state: dict[str, bool] = {}
        self._ladder_case = None
        self._ladder_moved: list = []
        self._ladder_reached = None

    def defines(self, protocol: str) -> bool:
        return bool((self.protocols.get(protocol) or {}).get("latch"))

    def _ladder_changes(self, rungs, case) -> list:
        """Which rungs of a shared ladder moved this round.

        Watched once per case rather than per input: every input drawing on
        the ladder asks about the same readings, and each keeping its own idea
        of what changed would let one input's evaluation consume the change
        another input needed.
        """
        if self._ladder_case is not case:
            self._ladder_case = case
            moved = []
            for rung in rungs:
                if case.drove:
                    if rung in case.drove:
                        moved.append(rung)
                    continue
                now = case.given.get(rung)
                seen = self.previous.get(f"\x00ladder\x00{rung}", _UNSEEN)
                if seen is not _UNSEEN and now != seen:
                    moved.append(rung)
                self.previous[f"\x00ladder\x00{rung}"] = now
            # ⚠ A rung the record does not mention this round leaves the
            # ladder where it was; the reading persists until another rung is
            # asserted. So an empty list means "unchanged", never "off".
            if moved:
                self._ladder_reached = max(moved, key=rungs.index)
            self._ladder_moved = ([self._ladder_reached]
                                  if self._ladder_reached else [])
        return self._ladder_moved

    def value(self, key: str, protocol: str, parameters: dict, case) -> bool:
        latch = self.protocols[protocol]["latch"]
        held = self.state.get(key, latch.get("initial", "clear") == "set")
        changed = set()
        for param, address in parameters.items():
            if case.drove:
                # ⚠ The record says what it set. Preferred over a subtraction
                # because a counter RESTATED at the same reading is still a
                # statement that the thing happened again -- and subtracting
                # reads that as nothing happening. On one corpus a ladder
                # stood at identical readings for dozens of consecutive cases
                # while the record went on asserting them.
                if address in case.drove:
                    changed.add(param)
                continue
            now = case.given.get(address)
            seen = self.previous.get(f"{key}\x00{param}", _UNSEEN)
            if seen is not _UNSEEN and now != seen:
                changed.add(param)
            self.previous[f"{key}\x00{param}"] = now
        sets = latch["set_when_changed"] in changed
        # ⚠ WHICH address did the setting, kept because the tie-break needs it
        # and it is not always this input's own counter: a cumulative ladder
        # can set this input from a rung it never names. Assuming it was the
        # input's own address raised on four components.
        set_by = parameters.get(latch["set_when_changed"]) if sets else None
        # ⚠ Reaching a later rung INCLUDES every earlier one. Without this an
        # input asking "has it been on at all" goes false the moment a longer
        # reading passes -- which is precisely the state that input exists to
        # describe. The ladder is watched globally rather than per input,
        # because a rung this input does not name is still a rung it includes.
        rungs = latch.get("cumulative")
        if rungs and not sets:
            mine = parameters.get(latch["set_when_changed"])
            if mine in rungs:
                for reached in self._ladder_changes(rungs, case):
                    if rungs.index(reached) >= rungs.index(mine):
                        sets, set_by = True, reached
                        break
        clears = latch["clear_when_changed"] in changed
        if sets and clears:
            both = latch.get("both", "clear")
            order = list(case.drove)
            cleared_by = parameters.get(latch["clear_when_changed"])
            if both == "last" and set_by in order and cleared_by in order:
                # Whichever the record listed later is the more recent, which
                # is what this idiom is named for. No arbitrary winner.
                held = order.index(set_by) > order.index(cleared_by)
            else:
                # Either the pack asked for a fixed winner, or the record did
                # not place both -- in which case there is no "later" to read
                # and a declared default is the honest answer.
                held = both == "set"
        elif sets:
            held = True
        elif clears:
            held = False
        self.state[key] = held
        return held


_UNSEEN = object()


class History:
    """What the round before produced, for the rules that ask about it.

    A specification writes transitions -- "when it becomes X" -- and a
    transition is not a fact about now. So the binding can say `previous_of`
    (what another input read last round) and `state_of` (what this document
    produced last round), and both need the cases in the order they happened.

    ⚠ The FIRST round has no round before it, and the two are not alike there:

      previous_of   may fall back to the input's current value, which is the
                    assumption that nothing had changed before observation
                    began. That is what edge detection means in practice, and
                    it is stated rather than hidden.
      state_of      has nothing to fall back on. An output fed back before it
                    has ever been produced is a number nobody computed, so
                    without a declared `initial` the first round is a refusal.
    """

    def __init__(self):
        self.inputs: dict[str, object] = {}
        self.outputs: dict[str, object] = {}
        self.started = False

    def earlier(self, name, rule, case, latches, model):
        if rule.get("previous_of"):
            target = rule["previous_of"]
            if self.started and target in self.inputs:
                return self.inputs[target]
            if "initial" in rule:
                return rule["initial"]
            return _UNSEEN          # the caller resolves this to "as it is now"
        target = rule["state_of"]
        if self.started and target in self.outputs:
            return self.outputs[target]
        if "initial" in rule:
            return rule["initial"]
        raise VerifyError(
            f"input {name!r}: feeds back the output {target!r} before the "
            f"document has ever produced it, and the binding declares no "
            f"`initial`. The first round would read a number nobody computed.")


def input_value(name: str, rule: dict, case, latches: Latches | None = None,
                model=None, history: History | None = None, conv=None):
    """One of the document's inputs, from what the case drove.

    Every branch here is a key the binding schema publishes. A rule using a
    key this cannot evaluate raises rather than defaulting: an input silently
    read as False is a document that was run on inputs nobody supplied, and
    the verdict would be about that run rather than about the document.
    """
    if rule.get("internal"):
        raise VerifyError(f"input {name!r}: 'internal' is an output key")
    if "variant_is" in rule:
        # ⚠ Read off the CASE. A record tagged with a build and never setting
        # any configuration signal is ordinary -- the tag IS the statement --
        # and a case that does not carry one answers false rather than
        # refusing, because "this record is not from that build" is a true
        # answer and an untagged record is from no particular build.
        return case.variant in set(rule["variant_is"])
    if rule.get("clock"):
        # ⚠ Time is read off the CASE, not off an address. A record that did
        # not say when it was taken cannot drive a document that watches the
        # clock, and reading zero instead would make every duration answer as
        # though no time had passed -- true of no round that ever happened.
        if case.elapsed_ms is None:
            # Records often note the time only on the cases where it mattered.
            # What the others mean is the binding's to say, for the same
            # reason a missing number is: there is no safe silent answer.
            if "when_absent" in rule:
                return rule["when_absent"]
            raise VerifyError(
                f"input {name!r} is the clock and this case records no "
                f"`elapsed_ms`. Say what an unrecorded time reads as with "
                f"`when_absent`.")
        return case.elapsed_ms
    if rule.get("protocol"):
        protocol = rule["protocol"]
        if latches is None:
            raise VerifyError(
                f"input {name!r}: reads {protocol!r}, whose answer depends on "
                f"which parameter changed most recently, and the examples do "
                f"not declare themselves `ordered`. Without an order there is "
                f"no previous round to have changed from.")
        if not latches.defines(protocol):
            raise VerifyError(
                f"input {name!r}: the pack names the protocol {protocol!r} and "
                f"never defines it, so this verifier has nothing to evaluate. "
                f"Give it a `latch` in the conventions, or verify a document "
                f"that does not use it.")
        parameters = rule.get("parameters") or {}
        missing = [p for p in latches.protocols[protocol]["parameters"]
                   if p not in parameters]
        if missing:
            raise VerifyError(f"input {name!r}: {protocol!r} needs parameter(s) "
                              f"{', '.join(missing)}")
        return latches.value(name, protocol, parameters, case)
    # ⚠ Both of these need the round before, so both need an ORDER. Reading a
    # set of cases in file order would be reading a promise nobody made.
    if rule.get("previous_of") or rule.get("state_of"):
        if history is None:
            which = "previous_of" if rule.get("previous_of") else "state_of"
            raise VerifyError(
                f"input {name!r}: reads {which}, which needs the round before "
                f"it, and the examples do not declare themselves `ordered`.")
        return history.earlier(name, rule, case, latches, model)
    address = rule.get("address")
    if not address:
        raise VerifyError(f"input {name!r}: the binding gives it no address")
    value = _given(case, address)
    if rule.get("absent"):
        # ⚠ Absence is not always a missing key. A record commonly writes a
        # TOKEN in the value's place -- the conventions already declare which,
        # and the schema for them already says "a binding expresses them with
        # `absent: true`". The core simply never read the list, so a source
        # the record explicitly marked as not reporting counted as present and
        # twelve telltales stayed dark across four components.
        if address not in case.given:
            return True
        tokens = {str(t).strip().lower()
                  for t in (getattr(conv, "absence_tokens", None) or ())}
        return str(value).strip().lower() in tokens
    # ⚠ The comparison goes through the value space, on this side too. A case
    # records `1` where the binding names `ON`, because a record and a
    # specification are kept by different people. Comparing the spellings
    # instead made every symbol comparison false, every input arrive as False,
    # and the document answer OFF everywhere -- which reads as a broken
    # document rather than as a verifier that never resolved the symbol.
    field_ = _field_at(model, address) if model is not None else None
    if "equals" in rule:
        return _same(value, rule["equals"], field_)
    if "equals_any" in rule:
        return any(_same(value, v, field_) for v in rule["equals_any"])
    if "not_equals" in rule:
        return not _same(value, rule["not_equals"], field_)
    if rule.get("number") or "range" in rule:
        # ⚠ A number has no "not any symbol" to fall back on the way a symbol
        # comparison does, so absence has to be DECLARED. Folding it into zero
        # made seven cases on one corpus look as though the specification had
        # been misread, when nothing had been reporting at all.
        has_absent = "when_absent" in rule
        if value is None:
            if has_absent:
                return rule["when_absent"]
            raise VerifyError(
                f"input {name!r}: the case drives no value at {address}, and a "
                f"number was wanted. Say what absence reads as with "
                f"`when_absent`; there is no safe silent answer.")
        try:
            number = float(value)
        except (TypeError, ValueError) as exc:
            if has_absent:
                return rule["when_absent"]
            raise VerifyError(f"input {name!r}: {value!r} at {address} is not "
                              f"a number") from exc
        low, high = rule.get("range", (None, None))
        if low is not None and not (low <= number <= high):
            # A reading the platform cannot represent is not a reading.
            if has_absent:
                return rule["when_absent"]
            raise VerifyError(
                f"input {name!r}: {number} at {address} is outside the "
                f"declared range {low}..{high}, which is a different fact from "
                f"any value in it")
        return int(number) if number.is_integer() else number
    raise VerifyError(f"input {name!r}: the binding says nothing about how to "
                      f"read {address}")


def output_values(name: str, rule: dict, computed) -> dict:
    """Where one of the document's outputs lands, as address.field -> value."""
    if rule.get("internal"):
        return {}
    address = rule.get("address")
    if not address:
        raise VerifyError(f"output {name!r}: the binding gives it no address")
    out = {}
    key = address + (f".{rule['field']}" if rule.get("field") else "")
    if "map" in rule:
        table = {str(k): v for k, v in rule["map"].items()}
        # ⚠ A document output declared `bool` arrives as True/False, and a map
        # of a two-valued field is naturally written 0/1. Those are the same
        # two cases, and comparing the spellings made every boolean output
        # unmappable -- reported as "the binding's map has no entry", which
        # sends a reader to edit a binding that was right.
        wanted = str(int(computed)) if isinstance(computed, bool) else str(computed)
        if wanted not in table:
            raise VerifyError(
                f"output {name!r}: the document produced {computed!r} and the "
                f"binding's map has no entry for it")
        out[key] = table[wanted]
    elif rule.get("passthrough"):
        out[key] = computed
    else:
        raise VerifyError(f"output {name!r}: the binding says neither 'map' "
                          f"nor 'passthrough', so where its value goes is "
                          f"undefined")
    # `when` first and `also` after, which is the order the schema states: a
    # default in `also` fills what `when` left alone rather than replacing it.
    for value, fields in (rule.get("when") or {}).items():
        if str(value) == str(computed):
            for fname, fvalue in fields.items():
                out[f"{address}.{fname}"] = fvalue
    for fname, fvalue in (rule.get("also") or {}).items():
        out.setdefault(f"{address}.{fname}", fvalue)
    return out


def _field_at(model, key: str):
    """The model's field for an `address` or `address.field` key.

    ⚠ The named field is tried FIRST. Looking for the unnamed one first
    returned nothing for every address that has named fields, so the value
    space was never found and a document computing the right answer everywhere
    was reported as failing every case.

    ⚠⚠ And the split is tried at EVERY dot, not just the last one. A field
    name can itself contain a dot -- `LinkedSound.Type` is one field of an
    event record, not a field `Type` of an address ending in `LinkedSound` --
    and splitting only at the last dot looked for a pair that does not exist.
    That single wrong split accounted for 121 of 259 failures on one corpus:
    the document had produced `REPEAT_COUNT` where the record said `1`, which
    are the same value, and the report blamed the document.
    """
    parts = key.split(".")
    for cut in range(len(parts) - 1, 0, -1):
        address, wanted = ".".join(parts[:cut]), ".".join(parts[cut:])
        entry = model.owning(address)
        if entry is None:
            continue
        field_ = entry.field(wanted)
        if field_ is not None:
            return field_
    entry = model.owning(key)
    if entry is not None:
        for field_ in entry.fields:
            if not field_.name:
                return field_
    return None


def _same(want, got, field_=None) -> bool:
    """Equal as the position means it, not as Python spells it.

    ⚠ Two spellings of one value is the normal case, not an edge. A record kept
    by one team writes `2` where another writes `ON`, and both are that field's
    value. The MODEL is the authority on which symbol a code is, so the
    comparison goes through the value space rather than through `str`.
    Without this, a document computing every answer correctly failed every
    case, and the report blamed the document.
    """
    if want == got:
        return True
    if str(want).strip() == str(got).strip():
        return True
    # ⚠ A number written two ways is one number. A record keeps `0.0` where a
    # document computes `0`, and comparing the SPELLINGS called six cases
    # failures across three components -- the document had the value right and
    # the report blamed it, which is the same defect as comparing a symbol
    # against its own code.
    try:
        if float(str(want).strip()) == float(str(got).strip()):
            return True
    except (TypeError, ValueError):
        pass
    values = getattr(field_, "values", None)
    if not values:
        return False

    def symbol(value):
        text = str(value).strip()
        if text in values:
            return text
        for name, code in values.items():
            if str(code) == text:
                return name
        return None

    left, right = symbol(want), symbol(got)
    return left is not None and left == right


# ------------------------------------------------------------------ the whole


# --------------------------------------------------------- driving a machine


# A statechart is not READ, it is DRIVEN: events go in, and what it produces
# for anything outside leaves as a `<send>` (W3C SCXML 6.2). Nothing about the
# calling convention above survives that, which is why this is its own path
# rather than a branch inside the one below.
STATECHART_KINDS = frozenset({"statechart"})


class SendRecorder:
    """Every host-served send the machine made, in the order it made them."""

    def __init__(self) -> None:
        self.sends: list = []

    def __call__(self, request):
        self.sends.append(request)
        # W3C SCXML 6.2.5 lets a handler answer with the events the act
        # produced. A verifier answers with none: inventing a reply would
        # drive the machine on words no record contains, and the verdict
        # would be about a run the examples did not describe.
        return []

    def take(self) -> list:
        """The sends since the last call, and reset. One case, one reading."""
        taken, self.sends = self.sends, []
        return taken


def sent_value(name: str, rule: dict, requests):
    """What one case's sends say this output became."""
    sent = rule.get("sent") or {}
    wanted = sent.get("processor")
    matching = [r for r in requests
                if wanted is None or r.processor_type == wanted]
    if not matching:
        # ⚠ Not an absence to skip over. A machine that should have signalled
        # and did not is the failure most worth catching, and an output that
        # simply fails to appear is reported as a position nobody looked at.
        return rule["when_nothing_sent"]
    # ⚠ The LAST. A machine crossing two states in one case sends twice, and
    # what the position HOLDS when the case ends is what `expect` describes.
    # The first would be a moment inside the case that no record claimed.
    request = matching[-1]
    if sent.get("param"):
        values = request.params.get(sent["param"]) or []
        if len(values) != 1:
            raise VerifyError(
                f"output {name!r}: reads the param {sent['param']!r}, which "
                f"this send carried {len(values)} time(s). `<param>` may "
                f"repeat under one name, and choosing among them would be the "
                f"driver deciding something the document said more than once")
        return values[0]
    if sent.get("content"):
        return request.content
    return request.event_name


class StatechartRun:
    """One engine, driven through the cases in the order they happened.

    ⚠ The engine is NOT rebuilt per case. An examples file is ONE run: what a
    case observes is partly the result of the cases before it, which is the
    whole reason the document has states. Restarting between cases would
    verify a machine that forgets, and that is a different document.
    """

    def __init__(self, module, build: Build):
        self.recorder = SendRecorder()
        self.engine = module.create_engine()
        for processor in build.declared:
            self.engine.register_event_processor(processor, self.recorder)
        self.engine.initialize()
        # Whatever the machine did on its way into the initial configuration
        # belongs to no case, because no record drove it.
        self.recorder.take()

    def event(self, name: str):
        found = self.engine.policy.get_event_from_name(name)
        if found is None:
            raise VerifyError(
                f"the document declares no event named {name!r}. Sending it "
                f"would leave the machine where it was, and every reading "
                f"afterwards would be of a machine nobody drove")
        return found

    def drive(self, rule: dict, case) -> bool:
        """Send this input's event if the case drove it. True when sent."""
        address = rule.get("address")
        if address is None or address not in (case.drove or ()):
            return False
        becomes = rule.get("becomes")
        if becomes is not None and case.given.get(address) != becomes:
            return False
        self.engine.send_event(self.event(rule["event"]))
        return True

    def declared_value(self, name: str):
        """What one of the document's own declared outputs is holding now.

        ⚠ This is NOT reaching into the machine. `sce:direction="out"` is the
        document declaring that variable part of its outward surface -- the
        generator emits a host-facing accessor for every one of them, and
        `check` refuses a binding that leaves one uncovered. Renaming it is an
        interface change, and a verification that breaks on one is right to.
        The active CONFIGURATION is the opposite case and stays unreadable: no
        state is declared an output anywhere, so asserting on one would break
        on a rename that changed nothing about what the document does.

        ⚠⚠ Through the ACCESSOR, never the session. Reaching past it into the
        engine's private session id would be this package helping itself to
        the runtime's insides to read a value the runtime already offers.
        """
        reader = getattr(self.engine.policy, _snake(name), None)
        if reader is None:
            raise VerifyError(
                f"output {name!r}: the document declares it an output and the "
                f"generated machine offers no accessor for it. One is emitted "
                f"for a variable whose type the document settles and whose id "
                f"is a legal identifier, so this one is neither")
        value = reader()
        if value is None:
            raise VerifyError(
                f"output {name!r}: the machine cannot say what it is holding. "
                f"That answer covers a session that never initialised, a "
                f"variable assigned a value of another type, and an engine "
                f"that refused -- and none of them is a value to judge a case "
                f"on")
        return value

    def observe(self, case, restated: bool = False) -> None:
        """Move virtual time to the moment this case was observed.

        ⚠ NOTHING IS SUBTRACTED FROM ANYTHING. `elapsed_ms` is the age of
        the SITUATION, and the situation is what this case just drove, so
        the observation sits exactly that far after the drive. Reading it
        as a delta between two cases would be wrong twice over: the field
        restarts whenever the situation does, and its own schema says a
        record where it goes backwards is ordinary rather than broken.

        ⚠⚠ One advance, however large. The engine pops due entries one
        macrostep apart, so a long step does not step over a deadline the
        document distinguishes -- and choosing a step SIZE is the move its
        runtime warns against, because the host owns this clock outright.
        """
        if case.elapsed_ms is None:
            if self.engine.time_until_next_scheduled_ms() is not None:
                raise VerifyError(
                    "the machine is waiting on a delayed act and this case "
                    "records no `elapsed_ms`, so nothing says whether the "
                    "wait was over when it was observed. Reading it here "
                    "would date the reading to a moment no record names")
            return
        if case.elapsed_ms < 0:
            raise VerifyError(
                f"records `elapsed_ms` of {case.elapsed_ms}, and an age "
                f"cannot be negative. A LATER case reading lower than an "
                f"earlier one is ordinary -- the situation restarted -- but "
                f"a single one below zero says the record means something "
                f"else by the field")
        if restated:
            # ⚠ The one place the anchor is genuinely ambiguous, and it is
            # refused rather than chosen. This case drove the same addresses
            # to the same values as the one before it, which the schema says
            # is a real assertion and not nothing happening -- but nothing
            # says whether the age is measured from THIS assertion or from
            # the earlier one that began the situation. The two put the
            # reading at different moments, and a delayed act sits between
            # them, so picking one would silently date every such reading.
            raise VerifyError(
                f"drove the same values as the case before it and records an "
                f"`elapsed_ms` of {case.elapsed_ms}, and nothing says whether "
                f"that age runs from this assertion or from the one that "
                f"started the situation. The two are different moments, and "
                f"a delayed act can fall between them")
        self.engine.advance_time(int(case.elapsed_ms))


def verify_statechart(pack: Pack, binding: dict, module, build: Build,
                      declared) -> Verification:
    """Replay the pack's cases through the document as one run."""
    examples = pack.examples
    if not examples.ordered:
        return Verification(refusal=(
            "the cases do not declare themselves `ordered`, and a statechart "
            "is replayed rather than recomputed: what a case observes is "
            "partly the result of the cases before it. Reading the file's "
            "line order as a timeline it was never promised would produce a "
            "verdict about an order nobody recorded."))
    if build.needs_event_scheduler and not any(
            case.elapsed_ms is not None for case in examples.cases):
        return Verification(refusal=(
            "the document has delayed acts, so it has to be driven through "
            "time, and no case records an `elapsed_ms`. Virtual time would "
            "never move, and every delayed act would be reported as one that "
            "never fired -- a full run, judged, against a document that was "
            "never given the chance to do half of what it does."))

    inputs = dict(binding.get("inputs") or {})
    outputs = dict(binding.get("outputs") or {})
    driving = {n: r for n, r in inputs.items() if r.get("event")}
    if not driving:
        return Verification(refusal=(
            "no input rule names an `event`, so no case can drive the "
            "machine. Every case would be judged against a document sitting "
            "in its initial configuration, which is a verdict about nothing."))

    bound, writes = set(), {}
    for name, rule in outputs.items():
        if rule.get("unresolved") or rule.get("internal"):
            continue
        if rule.get("address"):
            key = rule["address"] + (f".{rule['field']}" if rule.get("field") else "")
            bound.add(key)
            writes[key] = name

    verification = Verification()
    expected = {a for case in examples.cases for a in case.expect}
    verification.unbound = sorted(expected - bound)
    verification.unasserted = sorted(bound - expected)

    try:
        run = StatechartRun(module, build)
    except VerifyError as exc:
        return Verification(refusal=str(exc))

    previous: tuple = ()
    for case in examples.cases:
        result = CaseResult(name=case.name)
        # What this case ASSERTED, as the record states it: the addresses it
        # drove, at the values it drove them to. ⚠ Read rather than derived.
        # Comparing one case's whole `given` with the next one's would call a
        # restatement nothing happening, which is the error `drove` exists to
        # stop.
        signature = tuple(sorted((a, case.given.get(a))
                                 for a in (case.drove or ())))
        restated, previous = bool(signature) and signature == previous, signature
        try:
            if not any(run.drive(rule, case) for rule in driving.values()):
                # ⚠ A case that drove nothing this document listens for is
                # NOT a pass. Reading the machine afterwards would report
                # whatever the previous case left, attributed to this one.
                raise VerifyError(
                    f"drove {list(case.drove) or 'nothing'}, and no input "
                    f"rule turns any of that into an event this document "
                    f"receives")
            # ⚠ AFTER the drive and BEFORE the reading. The drive is what
            # starts the situation whose age `elapsed_ms` states, and the
            # reading is what that age dates -- so a delayed act reaches the
            # machine in between, which is the whole point of it having one.
            run.observe(case, restated)
            requests = run.recorder.take()
            produced: dict = {}
            for name, rule in outputs.items():
                if rule.get("unresolved") or rule.get("internal"):
                    continue
                # A send is the channel to prefer and is asked first. Failing
                # that, the document's own declared output variable -- and
                # failing THAT, nothing, because the only places left are ones
                # the document never said anybody could read.
                if rule.get("sent"):
                    value = sent_value(name, rule, requests)
                elif name in declared.outputs:
                    value = run.declared_value(name)
                else:
                    raise VerifyError(
                        f"output {name!r} is bound to no `sent`, and the "
                        f"document does not declare it `sce:direction=\"out\"` "
                        f"either. What is left is the active configuration, "
                        f"and a state is not an output: asserting on one would "
                        f"break when a state is renamed or split while the "
                        f"document went on doing the same thing")
                produced.update(output_values(name, rule, value))
        except VerifyError as exc:
            result.refusal = str(exc)
            verification.results.append(result)
            continue
        for address, want in sorted(case.expect.items()):
            if address not in produced:
                result.unchecked.append(address)
                continue
            if not _same(want, produced[address], _field_at(pack.model, address)):
                result.failures.append((address, want, produced[address]))
                writer = writes.get(address)
                rests_on = (declared.rests_on_an_assumption(writer)
                            if writer else "")
                if rests_on:
                    verification.refuted.setdefault(address, rests_on)
        verification.results.append(result)
    return verification


def verify(pack: Pack, binding_path: pathlib.Path,
           codegen: pathlib.Path | None = None,
           backend: str = "python") -> Verification:
    """Run every example case against the bound document."""
    from .check import read_document  # local: only verification needs it

    if backend not in DRIVEN_BACKENDS:
        return Verification(refusal=(
            f"{backend!r} is a backend the product EMITS and this verifier "
            f"cannot DRIVE. It drives {', '.join(sorted(DRIVEN_BACKENDS))}, "
            f"whose generated form imports into this process and whose "
            f"runtime is reachable from it -- the document becomes an object "
            f"and driving it is calling methods. Another backend is a build "
            f"and a separate process, so it needs three things nothing here "
            f"has: a build step for that language, a host program that stands "
            f"the engine up and registers what a verifier registers, and a "
            f"wire carrying each case's inputs in and each reading out. Until "
            f"those exist, running it would report a verdict about a program "
            f"nobody started."))
    binding = read_binding(binding_path)
    document = (binding_path.parent / binding["document"]).resolve()
    declared = read_document(document)
    if declared.kind not in (CALLABLE_KINDS | STATECHART_KINDS):
        return Verification(refusal=(
            f"{document.name} declares kind {declared.kind!r}. This verifier "
            f"drives {', '.join(sorted(CALLABLE_KINDS))}, whose generated shape "
            f"is one function per output, and "
            f"{', '.join(sorted(STATECHART_KINDS))}, which it drives by events. "
            f"Another kind is another calling convention, and guessing at one "
            f"would produce a verdict about something that was never run."))

    examples = pack.examples
    if not (examples and examples.present and examples.cases):
        return Verification(refusal=(
            "the pack has no examples, so there is nothing to run the document "
            "against. A document that passes no cases has not been shown to "
            "work; it has been shown to compile."))
    if not examples.valued:
        return Verification(refusal=(
            "the cases name the addresses they touch but not the values, so "
            "the document cannot be driven by them."))

    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    into = pathlib.Path(tempfile.mkdtemp(prefix="sce_verify_"))
    build = generate(document, codegen, into, backend)
    if build.refusal:
        return Verification(refusal=build.refusal)
    module = load(into, document)

    if declared.kind in STATECHART_KINDS:
        run = verify_statechart(pack, binding, module, build, declared)
        run.backend = backend
        return run

    inputs = dict(binding.get("inputs") or {})
    outputs = dict(binding.get("outputs") or {})
    bound = set()
    writes = {}
    for name, rule in outputs.items():
        if rule.get("unresolved"):
            continue
        if not rule.get("internal") and rule.get("address"):
            key = rule["address"] + (f".{rule['field']}" if rule.get("field") else "")
            bound.add(key)
            writes[key] = name

    verification = Verification(backend=backend)
    expected = {a for case in examples.cases for a in case.expect}
    verification.unbound = sorted(expected - bound)
    verification.unasserted = sorted(bound - expected)

    # A latch carries state between cases, so it exists only when the pack says
    # the cases are a timeline. Without that, `None` here is what makes the
    # refusal above fire rather than reading file order as an order.
    latches = Latches(pack.conventions) if examples.ordered else None
    history = History() if examples.ordered else None

    # ⚠ Needing the round before is a property of the BINDING against the
    # PACK, knowable before a single case runs -- and it would refuse every
    # case identically. So it is said once, up front, rather than repeated as
    # many times as there are cases.
    # ⚠ A binding with an unresolved INPUT cannot be run at all: the document
    # would be driven from a value nobody supplied, and the verdict would be
    # about that. Said once, naming every gap and the reason its author gave,
    # because the next step is to take those reasons to whoever can answer
    # them -- not to fix the document.
    #
    # An unresolved OUTPUT is narrower and does NOT stop the run: the rest
    # still executes and only the positions that output would have written go
    # unchecked, which the per-case report already says.
    open_inputs = sorted((n, r["unresolved"]) for n, r in inputs.items()
                         if r.get("unresolved"))
    if open_inputs:
        return Verification(refusal=(
            "the binding does not say which address feeds "
            + ", ".join(n for n, _ in open_inputs)
            + ". Nothing can be run until it does:\n  "
            + "\n  ".join(f"{n}: {why}" for n, why in open_inputs)))

    # ⚠ There is deliberately NO monotonicity check on the clock. One was
    # written, on the reading that `elapsed_ms` is a moment on a timeline, and
    # the data refused it: the field is how long the SITUATION had held, so it
    # restarts whenever the situation does and going backwards is ordinary.
    # The check would have refused a whole component's records as broken.

    if not examples.ordered:
        needs_history = sorted(
            n for n, r in inputs.items()
            if r.get("previous_of") or r.get("state_of") or r.get("protocol"))
        if needs_history:
            return Verification(refusal=(
                f"input(s) {', '.join(needs_history)} need the round before "
                f"them, and the examples do not declare themselves `ordered`. "
                f"Without an order there is no previous round, and reading the "
                f"file's order as a timeline would be reading a promise nobody "
                f"made."))

    for case in examples.cases:
        result = CaseResult(name=case.name or "(unnamed)")
        # Two passes, because a rule about the previous round names ANOTHER
        # input, and on the first round it may have to fall back to what that
        # input reads now. One pass would have to evaluate them in an order
        # nothing declares.
        try:
            plain = {n: r for n, r in inputs.items()
                     if not (r.get("previous_of") or r.get("state_of"))}
            values = {n: input_value(n, r, case, latches, pack.model, history,
                                     pack.conventions)
                      for n, r in plain.items()}
            for n, r in inputs.items():
                if n in values:
                    continue
                got = input_value(n, r, case, latches, pack.model, history,
                                  pack.conventions)
                if got is _UNSEEN:
                    target = r["previous_of"]
                    if target not in values:
                        return Verification(refusal=(
                            f"input {n!r}: reads the previous value of "
                            f"{target!r}, which this binding does not declare"))
                    got = values[target]
                values[n] = got
        except VerifyError as exc:
            # This case could not be driven. The next one still might.
            result.refusal = str(exc)
            verification.results.append(result)
            continue
        kwargs = {_snake(n): v for n, v in values.items()}
        produced: dict = {}
        for name, rule in outputs.items():
            if rule.get("unresolved"):
                # It has nowhere to land yet. The document still computes it,
                # and saying so would be reporting a gap the author declared.
                continue
            fn = getattr(module, "compute_" + _snake(name), None)
            if fn is None:
                if not rule.get("internal"):
                    return Verification(refusal=(
                        f"output {name!r}: the binding names it and the "
                        f"document does not produce it"))
                continue
            try:
                computed = fn(**kwargs)
                if history is not None:
                    # ⚠ Remembered RAW, before the binding maps it. What a
                    # later round feeds back is the document's own value, not
                    # the platform symbol it lands as -- mapping first would
                    # hand the document a word it never produced.
                    history.outputs[name] = computed
                produced.update(output_values(name, rule, computed))
            except VerifyError as exc:
                result.refusal = str(exc)
                break
            except TypeError as exc:
                return Verification(refusal=(
                    f"output {name!r}: the document's function would not take "
                    f"the bound inputs ({exc})"))
        if result.refusal:
            verification.results.append(result)
            continue
        for address, want in sorted(case.expect.items()):
            if address not in produced:
                result.unchecked.append(address)
                continue
            if not _same(want, produced[address], _field_at(pack.model, address)):
                result.failures.append((address, want, produced[address]))
                writer = writes.get(address)
                rests_on = (declared.rests_on_an_assumption(writer)
                            if writer else "")
                if rests_on:
                    verification.refuted.setdefault(address, rests_on)
        if history is not None:
            history.inputs = dict(values)
            history.started = True
        verification.results.append(result)
    return verification


def _default_codegen() -> pathlib.Path:
    """Where the product's generator sits in this tree, by default."""
    root = pathlib.Path(__file__).resolve().parents[3]
    return root / "target" / "debug" / "sce-codegen"


def _default_runtime() -> pathlib.Path:
    """Where the product's Python runtime sits in this tree, by default.

    Derived the way the generator's location is, and for the same reason: a
    constant would be a home this package is not entitled to have.
    """
    root = pathlib.Path(__file__).resolve().parents[3]
    return root / "backends" / "python" / "runtime"
