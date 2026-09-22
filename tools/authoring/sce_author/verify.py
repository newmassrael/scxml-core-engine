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
import itertools
import json
import pathlib
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field

from . import delivery, landing
from .check import (STATECHART_KINDS, activation_unsaid, driving_refusals,
                    imports_of, read_binding)
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
    # ⚠ Positions whose value depends on an input the binding leaves
    # UNRESOLVED. The case was run under every value that input could take,
    # and these came out different -- so no answer is claimed for them. They
    # are neither a pass nor a failure: the document may be right, and what it
    # is right ABOUT is still being asked of whoever knows the address.
    undetermined: list[str] = field(default_factory=list)

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
    # Inputs whose value this run REMEMBERED on the host's behalf, by the
    # binding's own `previous_of` / `state_of`.
    #
    # ⚠ THE RUN IS NOT THE PRODUCT, AND THIS IS WHERE THE TWO DIFFER. A
    # document asking for a previous round is asking something OUTSIDE it to
    # keep one: the generated code takes this round's inputs and nothing else,
    # so in production the caller has to hold these between calls. Here the
    # verifier holds them, quietly and for free.
    #
    # ⚠⚠ Quietly is the problem. A pass with this list non-empty says the
    # document behaves GIVEN a memory nobody has yet agreed to keep, and a
    # reader not told about it reads an unconditional pass. That is the same
    # shape this package already refuses elsewhere -- a document declaring
    # itself a pure computation while something else remembers for it -- and
    # the verifier was doing it to its own reader.
    host_memory: list[str] = field(default_factory=list)
    # Preconditions the PACK reads by assumption, to the reading and the
    # reason it gives: {phrase: {"expression": ..., "reason": ...}}.
    #
    # ⚠ A pass is conditional on every one of these, and nothing a case does
    # can move them: a condition read as a constant is not in the document at
    # all, so a right reading and a wrong one pass identically. That is the
    # one kind of wrong a run cannot catch, which is why the run has to say
    # it rests on it.
    #
    # ⚠ Every assumption the pack makes, not only the ones this document
    # relies on. Which phrases the prose actually writes is `questions`'
    # answer -- only the prose says, and this run is not handed it. Listing
    # fewer than the pack holds would be a guess about a text nobody read.
    assumed_preconditions: dict = field(default_factory=dict)
    # ⚠ Failing addresses whose value the DOCUMENT marks `sce:assumed`, to the
    # reason its author wrote. An assumption compiles, so nothing downstream
    # ever mentioned it again and a case refuting one read as "your document
    # is wrong" -- when the author had already written down that this exact
    # value was a guess. Naming it turns the report into the one sentence
    # worth having: the guess you recorded is the thing that failed.
    refuted: dict = field(default_factory=dict)
    # Inputs the binding declares UNRESOLVED, to the reason its author gave.
    #
    # ⚠ They used to stop the whole run, and that made honesty the expensive
    # choice: writing "nobody has told me which address this is" cost every
    # case of the component, while writing a plausible address cost nothing
    # and passed. Measured 2026-09-22 across thirty documents written from
    # prose, the unresolved key was used zero times -- and a plausible wrong
    # address is invisible, which is the whole reason the key exists.
    #
    # So the run no longer invents a value and no longer refuses. It asks the
    # only question that needs no value: does the answer change with it? Each
    # such input is boolean, so every case is run under both, and only the
    # positions that come out different are withheld. Printed whenever
    # non-empty, with how much it blocked -- a declared unknown that blocks
    # everything should be as visible as one that blocks nothing.
    #
    # A statechart asks the same question another way, because its cases share
    # one run and two answers to one case would be two machines for every case
    # after it: an unresolved input there is an EVENT nobody can say was sent,
    # and the run is judged for as long as no active state could act on it.
    # See `verify_statechart`.
    unresolved: dict = field(default_factory=dict)
    # Outputs the DOCUMENT leaves `sce:unresolved`, and every output that reads
    # one, to why. They are built with a placeholder so the rest can run, and
    # no position any of them writes is judged. The document itself is
    # untouched and its shipping build still refuses.
    unresolved_outputs: dict = field(default_factory=dict)
    # Outputs the BINDING leaves `unresolved` -- nobody has said which address
    # they reach -- to the reason. Not the same question as the one above:
    # there the document has not decided a VALUE, here the binding has not
    # placed one. Every position the examples read that no named rule writes
    # may be where one of these lands, and none of those is judged.
    unaddressed_outputs: dict = field(default_factory=dict)

    @property
    def undetermined(self) -> int:
        return sum(len(r.undetermined) for r in self.results)

    @property
    def still_open(self) -> int:
        """How many values the binding or the document leaves undecided.

        ⚠ The one count a caller gates on. Summing the fields at each call
        site is how an open output came to exit green: the status read two of
        the three and nothing said a third existed.
        """
        return (len(self.unresolved) + len(self.unresolved_outputs)
                + len(self.unaddressed_outputs))

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

    @property
    def holder(self) -> dict | None:
        """The object the document keeps its `previous()` values in, by the
        names the product gave it -- or None, when the document is pure
        functions and each output is called on its own.

        ⚠ Read from the manifest, not from the document. Whether a transform
        keeps state is the product's decision, taken while it rendered, and
        the manifest reports that decision with the names to call. Scanning
        the XML for `previous(` here would be a second answer to the same
        question, free to disagree with the first.
        """
        return self.manifest.get("holder") or None


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


# The value a VERIFICATION build gives an output its author left
# `sce:unresolved`, by the start of its declared type. It is never read: every
# position such an output writes, and every output whose expression mentions
# it, is withheld from the verdict. It exists only so the rest can be built.
# A type not listed here keeps the product's own refusal -- a placeholder
# nobody is sure compiles would turn a clear refusal into a confusing one.
_PLACEHOLDERS = (("bool", "false"), ("int", "0"), ("uint", "0"),
                 ("float", "0"), ("double", "0"))


def verification_source(document: pathlib.Path, declared,
                        scratch: pathlib.Path) -> pathlib.Path:
    """The document to BUILD for a verification run.

    ⚠ The product refuses to build a document holding an open decision, and
    for SHIPPING that is exactly right -- a value nobody has decided cannot be
    shipped. But the refusal used to end the verification too, which
    made declaring an unknown cost every case: measured 2026-09-22, five
    documents written from prose left the same one value open, and all five
    lost verification entirely although their decision logic, checked by hand,
    was as right as the documents that had guessed. That is the incentive the
    binding's `unresolved` had until the same day, one layer down.

    So verification builds a COPY in which each open output carries a
    placeholder, and then withholds everything that placeholder could reach.
    The author's document is not changed and the shipping build still refuses.

    Returns the document itself -- to be refused by the product in its own
    words -- whenever the copy cannot be made safely: an open decision that is
    not a datamodel output, or one of a type this has no placeholder for.
    """
    if not declared.unresolved:
        return document
    text = document.read_text(encoding="utf-8")
    placed = 0

    def placeholder(match: re.Match) -> str:
        nonlocal placed
        tag = match.group(0)
        ident = re.search(r'\bid="([^"]+)"', tag)
        if not ident or ident.group(1) not in declared.unresolved:
            return tag
        if re.search(r'\sexpr="', tag):
            placed += 1
            return tag
        kind = (re.search(r'\bsce:type="([^"]+)"', tag) or [None, ""])[1]
        value = next((v for prefix, v in _PLACEHOLDERS if kind.startswith(prefix)),
                     None)
        if value is None:
            return tag
        placed += 1
        return tag.replace("<data", f'<data expr="{value}"', 1)

    copy = re.sub(r"<data\b[^>]*>", placeholder, text, flags=re.S)
    # ⚠ Counted against EVERY open marker in the text, not only the declared
    # outputs: a marker on a state or a transition is an open decision this
    # copy cannot withhold, and building around it would claim a verdict
    # about a machine nobody has finished.
    # Counted with comments stripped: an author explaining a marker in a
    # comment is not a second marker.
    markers = re.sub(r"<!--.*?-->", "", copy, flags=re.S).count('sce:unresolved="')
    if placed != len(declared.unresolved) or placed != markers:
        return document
    # The copy lives elsewhere, so every import it names is made absolute
    # against the ORIGINAL's directory -- where the generator would have
    # resolved it.
    base = document.resolve().parent

    def absolute(match: re.Match) -> str:
        return re.sub(r'\bsrc="([^"]+)"',
                      lambda m: f'src="{(base / m.group(1)).resolve()}"',
                      match.group(0))

    copy = re.sub(r"<sce:import\b[^>]*>", absolute, copy, flags=re.S)
    target = scratch / document.name
    target.write_text(copy, encoding="utf-8")
    return target


def assumption_behind(writer: str | None, declared, binding: dict) -> str:
    """What a failure at a position `writer` writes rests on, if its author
    wrote it down as a guess: the document's `sce:assumed` first, then the
    binding's `assumed` on that output's rule, then on any input it reads.

    ⚠ The binding half is new. A decision made in the binding -- reading a
    number's absence as 0, choosing a symbol for a platform default -- could
    only be written in `note`, which nothing reads, so a case refuting it read
    as "the document is wrong". Walked over the same `reads` graph the
    document's own assumptions are, because the input a guess sits on is
    usually a step upstream of the value that failed.
    """
    if not writer:
        return ""
    found = declared.rests_on_an_assumption(writer)
    if found:
        return found
    inputs = binding.get("inputs") or {}
    rule = (binding.get("outputs") or {}).get(writer) or {}
    if rule.get("assumed"):
        return f"the binding's rule for {writer}: {rule['assumed']}"
    seen, stack = set(), [writer]
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        held = inputs.get(current) or {}
        if held.get("assumed"):
            return f"the binding's rule for {current}: {held['assumed']}"
        stack.extend(declared.reads.get(current, ()))
    return ""


def withheld_outputs(declared) -> dict:
    """Every output a verdict may not rest on: {output: why}.

    The ones the author left open, and -- transitively -- every output whose
    expression mentions one of them, since its value was computed from the
    placeholder.
    """
    why = {name: reason for name, reason in declared.unresolved.items()}
    grew = True
    while grew:
        grew = False
        for name, mentions in declared.reads.items():
            if name in why:
                continue
            upstream = sorted(mentions & set(why))
            if upstream:
                why[name] = f"reads {', '.join(upstream)}, which is unresolved"
                grew = True
    return why


def pseudo_page(document: pathlib.Path, codegen: pathlib.Path | None,
                deploy: pathlib.Path | None, shape: str | None = None,
                lexicon: str | None = None) -> tuple[str, str]:
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

    ⚠ `shape` and `lexicon` are passed through WITHOUT being checked here,
    and nothing in this tool lists what they may be. The product's registry
    is what those names mean; a copy of it here would be a second list that
    goes stale the day a shape is registered, and it would refuse a name the
    product accepts while sounding authoritative about it. An unknown name
    comes back as the product's own refusal, which names the real set.

    ⚠ Omitted rather than defaulted when nobody asked. The generator's own
    defaults are `indent` and `en`, and passing them explicitly would make
    this tool the second place that decides what the default page is.
    """
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    if not codegen.exists():
        raise VerifyError(
            f"{codegen}: the code generator is not there, so no document can "
            f"be shown. Build it, or name another with --codegen.")
    argv = [str(codegen), "pseudo", str(document)]
    if deploy is not None:
        argv += ["--deploy", str(deploy)]
    if shape is not None:
        argv += ["--shape", shape]
    if lexicon is not None:
        argv += ["--lexicon", lexicon]
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


def host_memory_of(inputs: dict, conventions) -> list[str]:
    """The inputs whose previous round something OUTSIDE the document keeps.

    ⚠ All three shapes count, because all three are answered from a round
    that has already gone: `previous_of` (another input, last round),
    `state_of` (this document's own output, last round) and a protocol whose
    pack definition carries a `latch` -- a latch answers by which parameter
    moved most recently, which is a fact about the round before.

    ⚠⚠ Counting only the first two would under-report, and under-reporting is
    exactly the defect this figure exists to remove: the run would hold a
    remembered value and the report would not mention it.
    """
    protocols = getattr(conventions, "protocols", None) or {}
    held = []
    for name, rule in inputs.items():
        if rule.get("previous_of") or rule.get("state_of"):
            held.append(name)
        elif rule.get("protocol"):
            if (protocols.get(rule["protocol"]) or {}).get("latch"):
                held.append(name)
    return sorted(held)


def assumed_preconditions_of(conventions) -> dict:
    """The pack's assumed readings, as a verdict carries them."""
    phrases = getattr(conventions, "precondition_phrases", None) or {}
    assumed = getattr(conventions, "precondition_assumed", None) or {}
    return {phrase: {"expression": phrases[phrase], "reason": reason}
            for phrase, reason in sorted(assumed.items())}


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
        # ⚠ Read the ladder on EVERY round, not only on the rounds where this
        # input has not already been set. Where the ladder stands is a global
        # observation, not this input's conclusion, and skipping it whenever
        # the latch happens to set leaves the reading stale: measured
        # 2026-09-22, an input was still told the ladder sat at the bottom
        # rung several rounds after the record had stepped it up, because
        # every round that moved it up had also set the latch and so never
        # looked. The guard below used to wrap this call, which is what made
        # the reading depend on the asker.
        reached = self._ladder_changes(rungs, case) if rungs else []
        # ⚠ A rung BELOW this input's own is the ladder being asserted again
        # from lower down, and that is not silence -- it is a statement that
        # the longer reading has stopped holding. Without this the latch only
        # ever rose: an input asking "has the longer reading passed" stayed
        # true after the record put the ladder back at the bottom, because
        # nothing that round named its own counter and a held latch looked
        # like the honest answer. Measured 2026-09-22 on an output whose
        # record steps the ladder down between cases: it stayed on for a round
        # the record calls off, and the DOCUMENT was blamed for it.
        # ⚠⚠ The oracle this pack was derived from reads the ladder the same
        # way -- `rung(last asserted) >= rung(wanted) > 0`, recomputed every
        # round rather than latched upward. Only the upward half was carried
        # over, and the half that was dropped is the one that says "no".
        lowered = False
        if rungs and not sets:
            mine = parameters.get(latch["set_when_changed"])
            if mine in rungs:
                for rung in reached:
                    if rungs.index(rung) >= rungs.index(mine):
                        sets, set_by = True, rung
                        break
                    lowered = True
        clears = latch["clear_when_changed"] in changed or lowered
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
# An output whose value depended on an unresolved input on the round it was
# remembered. A later round that reads it back cannot be judged either.
_UNDETERMINED = object()
# How many unresolved inputs a run will enumerate. Each doubles every case.
MAX_OPEN_INPUTS = 6
# A `hold_last` output with nothing held yet: the slot is not written.
_NOT_WRITTEN = object()


def _held(name: str, rule: dict, value, history) -> object:
    """The value a `hold_last` rule writes this round.

    ⚠ Carried over from the oracle the first packs were measured with, which
    kept it in the binding on purpose: "the event slot says WHAT is turning
    off, so its identifier keeps its last value when every condition is false
    -- that is the slot's structure, not the specification's, so the binding
    holds it". The first authoring core never took it over; three components
    written against the oracle then had every case unjudged, because the
    document's "no event" value had no entry in the map.
    """
    why = landing.form_refusal(rule)
    if why:
        raise VerifyError(f"output {name!r}: {why}")
    if landing.mapped(rule["map"], value)[0]:
        return value
    return history.held.get(name, _NOT_WRITTEN)


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
        # Output name -> the last value a `hold_last` rule WROTE. A slot the
        # component does not write keeps what it held; this is that slot.
        self.held: dict[str, object] = {}
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
                model=None, history: History | None = None, conv=None,
                sce_type: str | None = None, variants: dict | None = None):
    """One of the document's inputs, from what the case drove.

    Every branch here is a key the binding schema publishes. A rule using a
    key this cannot evaluate raises rather than defaulting: an input silently
    read as False is a document that was run on inputs nobody supplied, and
    the verdict would be about that run rather than about the document.

    `sce_type` is what the document declares this input to be, and `variants`
    the enumeration it names, if it names one. A rule naming an address and
    nothing else is read as that type -- see `delivery`.
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
    # ⚠ Nothing left but the address itself, so it hands the document its own
    # value, read as the document's declared type. This used to raise "the
    # binding says nothing about how to read" unless the rule restated the
    # type as `number: true` -- and a text input had no key at all.
    try:
        return delivery.read_as_is(
            name, rule, value, address in case.given, sce_type, field_,
            getattr(conv, "absence_tokens", None) or (), variants)
    except delivery.DeliveryError as exc:
        raise VerifyError(str(exc)) from exc


def output_values(name: str, rule: dict, computed) -> dict:
    """Where one of the document's outputs lands, as address.field -> value."""
    if rule.get("internal"):
        return {}
    address = rule.get("address")
    if not address:
        raise VerifyError(f"output {name!r}: the binding gives it no address")
    # The judgement `check` makes of this rule before any case runs.
    why = landing.form_refusal(rule)
    if why:
        raise VerifyError(f"output {name!r}: {why}")
    out = {}
    key = address + (f".{rule['field']}" if rule.get("field") else "")
    if "map" in rule:
        # ⚠ A document output declared `bool` arrives as True/False, and a map
        # of a two-valued field is written 0/1, true/false, or as YAML's own
        # booleans. Those are the same two cases, and comparing spellings made
        # a boolean output unmappable -- reported as "the binding's map has no
        # entry", which sends a reader to edit a binding that was right. One
        # reading for every site, `landing.names_value`.
        found, written = landing.mapped(rule["map"], computed)
        if not found:
            raise VerifyError(
                f"output {name!r}: the document produced {computed!r} and the "
                f"binding's map has no entry for it")
        out[key] = written
    else:
        out[key] = computed
    # `when` first and `also` after, which is the order the schema states: a
    # default in `also` fills what `when` left alone rather than replacing it.
    for value, fields in (rule.get("when") or {}).items():
        if landing.names_value(value, computed):
            for fname, fvalue in fields.items():
                out[f"{address}.{fname}"] = fvalue
    for fname, fvalue in (rule.get("also") or {}).items():
        out.setdefault(f"{address}.{fname}", fvalue)
    return out


def written_positions(outputs: dict) -> tuple[set, dict]:
    """Every position the binding writes, and which output writes it.

    ⚠ Counted the way `output_values` WRITES, fields `also` and `when` carry
    included. Both run paths used to count only `address.field`, so a binding
    that wrote an event's ID beside its status was reported as never writing
    the ID -- "the examples read N address(es) the binding never writes" --
    while the very same run produced and judged it. A report that contradicts
    the run beside it is read as the run being wrong. And the count lived in
    two copies, so the two paths could not even disagree consistently.
    """
    bound: set = set()
    writes: dict = {}
    for name, rule in outputs.items():
        if rule.get("unresolved") or rule.get("internal") or not rule.get("address"):
            continue
        address = rule["address"]
        keys = [address + (f".{rule['field']}" if rule.get("field") else "")]
        for fields in (rule.get("when") or {}).values():
            keys += [f"{address}.{fname}" for fname in fields]
        keys += [f"{address}.{fname}" for fname in (rule.get("also") or {})]
        for key in keys:
            bound.add(key)
            writes.setdefault(key, name)
    return bound, writes


def _field_at(model, key: str):
    """The model's field for an `address` or `address.field` key.

    The lookup is the model's (`Model.field_at`), so `check` asks the same
    question with the same answer.
    """
    return model.field_at(key)


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
# rather than a branch inside the one below. Which kinds those are is
# `check.STATECHART_KINDS`, because `check` judges their bindings by the same
# rule (`check.driving_refusals`).


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


def open_frame(pack: Pack, inputs: dict, outputs: dict,
               backend: str = "") -> tuple[Verification, set, dict]:
    """What either path reports before a case runs, from the binding alone.

    One copy for both paths. The statechart path was written by copying this
    block from the computation path, and a field added to one later would
    have been missing from the other with nothing to say so.
    """
    bound, writes = written_positions(outputs)
    verification = Verification(backend=backend)
    expected = {a for case in pack.examples.cases for a in case.expect}
    verification.unbound = sorted(expected - bound)
    verification.unasserted = sorted(bound - expected)
    verification.host_memory = host_memory_of(inputs, pack.conventions)
    verification.assumed_preconditions = assumed_preconditions_of(pack.conventions)
    verification.unresolved = {n: r["unresolved"] for n, r in sorted(inputs.items())
                               if r.get("unresolved")}
    verification.unaddressed_outputs = {
        n: r["unresolved"] for n, r in sorted(outputs.items())
        if r.get("unresolved") and not r.get("internal")}
    return verification, bound, writes


def _suffixes_written(rule: dict) -> set:
    """The tail of every position an output rule writes, address left off.

    `written_positions` builds each position as the address followed by
    these, so an output with no address yet can only have written a position
    that ends in one of them. An empty tail is a rule writing the bare
    address, which any position could be.
    """
    tails = {f".{rule['field']}" if rule.get("field") else ""}
    for fields in (rule.get("when") or {}).values():
        tails |= {f".{fname}" for fname in fields}
    tails |= {f".{fname}" for fname in (rule.get("also") or {})}
    return tails


class CaseJudge:
    """What a case's expectations say about what a run produced.

    ⚠ ONE copy, for both paths. Each had its own, and the rule that a case
    with a withheld position is not a pass lived in only one of them.
    """

    def __init__(self, verification: Verification, pack: Pack, binding: dict,
                 declared, outputs: dict, bound: set, writes: dict):
        self.verification, self.model = verification, pack.model
        self.binding, self.declared = binding, declared
        self.bound, self.writes = bound, writes
        # ⚠ An output with no address yet still writes SOMEWHERE -- a place
        # nobody has named. Every position the examples read that no named
        # rule writes may be that place. These used to be counted `unchecked`,
        # as positions no rule of this binding claims, and an unchecked
        # position does not stop a pass: measured 2026-09-22, a binding that
        # failed one case of five passed all five, exit 0, once the failing
        # output was written `unresolved`. That is the loophole the input
        # side had already closed -- write `unresolved` wherever it is hard
        # and watch the pass count hold.
        self.unaddressed = {name: _suffixes_written(outputs[name])
                            for name in verification.unaddressed_outputs}

    def landing_here(self, position: str) -> list:
        """The unaddressed outputs this position could turn out to be."""
        if position in self.bound:
            return []
        return [name for name, tails in self.unaddressed.items()
                if "" in tails or any(position.endswith(t) for t in tails if t)]

    def judge(self, result: CaseResult, case, produced: dict,
              undetermined=frozenset(), open_values=()) -> None:
        """Compare every position the case expects, and close the verdict.

        `undetermined` is what the run itself could not settle; `open_values`
        names what it rested on, for the refusal.
        """
        resting_on: set = set()
        for address, want in sorted(case.expect.items()):
            maybe = self.landing_here(address)
            if address in undetermined or maybe:
                result.undetermined.append(address)
                resting_on.update(maybe if maybe else open_values)
                continue
            if address not in produced:
                result.unchecked.append(address)
                continue
            if not _same(want, produced[address], _field_at(self.model, address)):
                result.failures.append((address, want, produced[address]))
                rests_on = assumption_behind(self.writes.get(address),
                                             self.declared, self.binding)
                if rests_on:
                    self.verification.refuted.setdefault(address, rests_on)
        # ⚠ A case with ANY withheld position is not a pass. The first version
        # only refused one whose EVERY position was withheld, and it never
        # fired: a case that expects an event also expects the identifier and
        # sound the binding writes as constants, those come out identical
        # under any input, and so a case whose whole point -- on or off --
        # depended on the unknown still counted as passed on the constants
        # alone. Measured on the first probe: 4 passed, when 3 of the 4 had
        # their status withheld. A wrong answer on a position that WAS settled
        # is still a failure: withholding the unknown does not excuse what was
        # known.
        if result.undetermined and not result.failures:
            result.refusal = (
                f"{len(result.undetermined)} position(s) this case expects "
                f"depend on unresolved value(s) {', '.join(sorted(resting_on))}; "
                f"the rest agreed, but a case is not passed on part of what it "
                f"asserts")


def rounds_of(cases):
    """Every round the examples drive, in order, and whether it is judged.

    A case's `before` steps come first and are driven like any round -- the
    machine, the latches and the remembered values all move -- but nothing is
    judged on them. They are the setup the record states, not a claim about
    the result.
    """
    for case in cases:
        for step in case.before:
            yield step, case, False
        yield case, case, True


class StatechartRun:
    """One engine, driven through the cases in the order they happened.

    ⚠ The engine is NOT rebuilt per case. An examples file is ONE run: what a
    case observes is partly the result of the cases before it, which is the
    whole reason the document has states. Restarting between cases would
    verify a machine that forgets, and that is a different document.
    """

    def __init__(self, module, build: Build, model=None):
        # The interface model, so `becomes` is compared as the position means
        # it rather than as the record happens to spell it.
        self.model = model
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
        # ⚠ Through the value space, as `equals` already is on the computation
        # side. A record writes `1` where the binding names `LOCK`; comparing
        # the spellings sent no event for any case of a document whose inputs
        # are enumerations, and every case came back "could not be judged".
        field_ = _field_at(self.model, address) if self.model is not None else None
        if becomes is not None and not _same(case.given.get(address), becomes, field_):
            return False
        self.engine.send_event(self.event(rule["event"]))
        return True

    def acting_on(self, event_name: str, declared) -> str | None:
        """An active state that could act on this event now, if any.

        ⚠ The one place the active configuration is asked, and it is asked
        only to WITHHOLD. `declared_value` explains why a state is never
        asserted on; nothing here asserts on one either. A state named here
        makes no case pass or fail -- it stops a verdict from being claimed
        about a run whose next move nobody can say. Renaming a state changes
        which name this returns and nothing about what is judged.

        ⚠⚠ Through `is_state_active`, the predicate W3C SCXML 5.9.2 gives the
        document itself as `In()`, rather than a walk of the engine's
        configuration. A state with no id cannot be asked about, so it is
        taken to be active: the question is whether the event COULD matter,
        and not knowing is a yes.
        """
        for state in declared.states_acting_on(event_name):
            if state is None:
                return "(a state with no id)"
            if self.engine.policy.is_state_active(state, self.engine):
                return state
        return None

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
    # Never empty: `verify` refused a binding naming no event before building
    # anything (`driving_refusals`).
    driving = {n: r for n, r in inputs.items() if r.get("event")}

    verification, bound, writes = open_frame(pack, inputs, outputs)
    judge = CaseJudge(verification, pack, binding, declared, outputs, bound, writes)

    # ⚠ An unresolved input that names an event is one whose SENDING nobody
    # can say: a case may have driven the address it turns out to be, or not.
    # It used to be dropped without a word -- `drive` sends nothing for a rule
    # with no address -- so every case was judged on a machine that had simply
    # never been told, and the run exited green on a binding that cannot ship.
    #
    # The computation path answers an unknown by running both values and
    # keeping what agrees. That cannot be done here: the cases share one run,
    # so two answers to one case are two machines for every case after it,
    # doubling each round. What CAN be asked is narrower and exact -- whether
    # the event could have changed anything. W3C SCXML 3.13 selects
    # transitions from the active configuration only, so while no active state
    # acts on the event, sending it and not sending it leave the same machine
    # and the case is judged either way. From the first moment one could, the
    # run's next configuration is unknown, and every case from there -- that
    # one included -- is withheld rather than judged on a machine that may be
    # somewhere no record drove it.
    open_events = {n: r["event"] for n, r in sorted(inputs.items())
                   if r.get("unresolved") and r.get("event")}
    # Why the run stopped being knowable, once it has.
    lost = ""

    def could_have_moved(where: str) -> str:
        for name, event in open_events.items():
            state = run.acting_on(event, declared)
            if state is not None:
                return (f"input {name!r} has no address yet, and from {where} "
                        f"on its event {event!r} could have reached the "
                        f"machine: state {state!r} was active and acts on it. "
                        f"Whether it was sent depends on the address nobody "
                        f"has named, and the cases share one run, so none "
                        f"from that point is judged")
        return ""

    def withhold(result: CaseResult, case) -> None:
        result.undetermined = sorted(case.expect)
        result.refusal = lost
        verification.results.append(result)

    try:
        run = StatechartRun(module, build, pack.model)
    except VerifyError as exc:
        return Verification(refusal=str(exc))

    previous: tuple = ()
    failed: set = set()
    for case, owner, judged in rounds_of(examples.cases):
        if id(owner) in failed:
            continue
        result = CaseResult(name=owner.name)
        if lost:
            if judged:
                withhold(result, case)
            continue
        # What this case ASSERTED, as the record states it: the addresses it
        # drove, at the values it drove them to. ⚠ Read rather than derived.
        # Comparing one case's whole `given` with the next one's would call a
        # restatement nothing happening, which is the error `drove` exists to
        # stop.
        signature = tuple(sorted((a, case.given.get(a))
                                 for a in (case.drove or ())))
        restated, previous = bool(signature) and signature == previous, signature
        try:
            # ⚠ EVERY driven address, in the order the record drove them. This
            # was `any(run.drive(...) for rule in ...)`, which stops at the
            # first rule that sends -- so a case that drove two addresses told
            # the machine about one, in the order the BINDING happened to list
            # them. Every case this suite had drove a single address, so it
            # never showed; the first real document whose rule needs two
            # inputs to move together failed each such case with the second
            # event never sent.
            #
            # ⚠ An open event can only have been sent at an address this case
            # DROVE, so a case that drove nothing sent none. Where among them
            # is unknown -- so the machine is asked at every configuration it
            # passes through while being driven: before the first send and
            # after each one.
            where = f"case {owner.name or '(unnamed)'!r}"
            lost = could_have_moved(where) if case.drove else ""
            moved = False
            for address in dict.fromkeys(case.drove or ()):
                for rule in driving.values():
                    if lost:
                        break
                    if rule.get("address") == address and run.drive(rule, case):
                        moved = True
                        lost = could_have_moved(where)
            if lost:
                if judged:
                    withhold(result, case)
                continue
            if not moved:
                if not judged:
                    # A setup step that moves nothing this document listens
                    # for leaves the machine as it was. That is a fact about
                    # the setup, and nothing is claimed on it.
                    continue
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
            if not judged:
                # What the machine sent while being set up is not what the
                # case is judged on: the case's own round starts clean.
                continue
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
            # A setup step that cannot be driven leaves the case unjudgeable,
            # and says which step it was.
            result.refusal = str(exc) if judged else f"{case.name}: {exc}"
            verification.results.append(result)
            failed.add(id(owner))
            continue
        judge.judge(result, case, produced)
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
            f"is one function per output -- beside a holder when the document "
            f"keeps values -- and "
            f"{', '.join(sorted(STATECHART_KINDS))}, which it drives by events. "
            f"Another kind is another calling convention, and guessing at one "
            f"would produce a verdict about something that was never run."))
    # ⚠ Before anything is built, and in `check`'s own words. Each of these
    # is a rule no case could get past -- and on a statechart, one the run
    # used to get past SILENTLY: a value an input rule computed was dropped
    # and the event sent bare, so a guard comparing it compared what the
    # variable started as, and the verdict blamed the document.
    stopped = driving_refusals(declared, dict(binding.get("inputs") or {}))
    if stopped:
        return Verification(refusal="\n".join(f"{where}: {why}"
                                              for where, why in stopped))

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
    # A statechart's open decisions sit on states and transitions, which a
    # placeholder cannot stand in for; it is built as written and refused by
    # the product in its own words, as before.
    source = (document if declared.kind in STATECHART_KINDS else
              verification_source(document, declared,
                                  pathlib.Path(tempfile.mkdtemp(prefix="sce_verify_src_"))))
    withheld = withheld_outputs(declared) if source != document else {}
    build = generate(source, codegen, into, backend)
    if build.refusal:
        return Verification(refusal=build.refusal)
    # ⚠ The generated module imports each `<sce:import>` as a sibling, and the
    # generator builds only the document it was handed. Without this every
    # document importing an enumeration died on its own import line, before a
    # case ran -- a crash that named the generated module, not the import.
    for imported in imports_of(document):
        built = generate(imported, codegen, into, backend)
        if built.refusal:
            return Verification(refusal=(
                f"{imported.name}, which {document.name} imports, could not "
                f"be built: {built.refusal}"))
    module = load(into, document)

    if declared.kind in STATECHART_KINDS:
        run = verify_statechart(pack, binding, module, build, declared)
        run.backend = backend
        return run

    inputs = dict(binding.get("inputs") or {})
    outputs = dict(binding.get("outputs") or {})
    verification, bound, writes = open_frame(pack, inputs, outputs, backend)
    verification.unresolved_outputs = dict(sorted(withheld.items()))
    judge = CaseJudge(verification, pack, binding, declared, outputs, bound, writes)

    # A latch carries state between cases, so it exists only when the pack says
    # the cases are a timeline. Without that, `None` here is what makes the
    # refusal above fire rather than reading file order as an order.
    latches = Latches(pack.conventions) if examples.ordered else None
    history = History() if examples.ordered else None

    # ⚠ Needing the round before is a property of the BINDING against the
    # PACK, knowable before a single case runs -- and it would refuse every
    # case identically. So it is said once, up front, rather than repeated as
    # many times as there are cases.
    # ⚠ An unresolved INPUT must never be driven from a value nobody
    # supplied -- a verdict would then be about that value. It used to stop
    # the whole run for that reason, and that was right about the premise and
    # too wide in the conclusion: a value nobody supplied is not needed to ask
    # whether the answer DEPENDS on it. A boolean input has two values; every
    # case runs under both, and a position that comes out the same either way
    # is judged -- a stronger claim than a pass under one guessed address, not
    # a weaker one. Only a position that differs is withheld.
    #
    # This is the rule `CaseResult.refusal` already states, one level down:
    # a refusal belongs to the smallest thing it is actually about.
    #
    # What still refuses, and why:
    #   not a BOOL   a number or a text has no two values to try; any choice
    #                would be invented
    #   a MEMORY     (`previous_of` / `state_of` of it) would carry an unknown
    #                forward into rounds that never read it
    #   too many     every one doubles the runs; past the cap the report would
    #                be a cost nobody asked for
    #
    # ⚠ "Not a bool" is read from the DOCUMENT's declaration. It was read from
    # the binding's `number: true`, so an open input feeding a text was run as
    # though it were False and then True -- two values the document's own
    # declaration says it can never receive.
    #
    # An unresolved OUTPUT is narrower: the rest runs, and only the positions
    # it could have written are withheld (`CaseJudge`).
    open_inputs = verification.unresolved
    numbers = sorted(n for n in open_inputs
                     if delivery.document_class(declared.types.get(n)) != "bool")
    remembered = sorted(n for n, r in inputs.items()
                        if r.get("previous_of") in open_inputs
                        or r.get("state_of") in open_inputs
                        or (n in open_inputs
                            and (r.get("previous_of") or r.get("state_of"))))
    if numbers or remembered or len(open_inputs) > MAX_OPEN_INPUTS:
        why = (f"the document declares {', '.join(numbers)} as something other "
               f"than a truth value, which has no two values to try" if numbers else
               f"{', '.join(remembered)} would carry an unresolved value into "
               f"a later round" if remembered else
               f"{len(open_inputs)} unresolved inputs would need "
               f"{2 ** len(open_inputs)} runs of every case (the cap is "
               f"{MAX_OPEN_INPUTS})")
        return Verification(refusal=(
            f"the binding does not say which address feeds "
            f"{', '.join(sorted(open_inputs))}, and {why}. Nothing can be run "
            f"until it does:\n  "
            + "\n  ".join(f"{n}: {w}" for n, w in sorted(open_inputs.items()))))
    unknown = sorted(open_inputs)
    # Every value the unresolved inputs could jointly take. With none it is a
    # single empty assignment, and the run below is exactly what it was.
    assignments = [dict(zip(unknown, bits)) for bits in
                   itertools.product((False, True), repeat=len(unknown))]

    # ⚠ There is deliberately NO monotonicity check on the clock. One was
    # written, on the reading that `elapsed_ms` is a moment on a timeline, and
    # the data refused it: the field is how long the SITUATION had held, so it
    # restarts whenever the situation does and going backwards is ordinary.
    # The check would have refused a whole component's records as broken.

    # ⚠ A document that keeps values between activations is driven through
    # its HOLDER -- one per run, one `update` per round, setup steps included
    # -- because only the holder keeps what the next round reads. Calling each
    # output's function on its own would hand it no kept value at all.
    holder = build.holder
    if holder:
        activation = binding.get("activation")
        if activation is None:
            # An incomplete binding, which `check` refuses in these words.
            return Verification(refusal=activation_unsaid(document.name))
        if activation != "on-change":
            # A complete binding this verifier cannot replay: its own refusal.
            return Verification(refusal=(
                f"{document.name} keeps values from one activation to the next "
                f"(it reads previous()), and the binding says it runs "
                f"{activation!r}. What `previous(x)` means is the value one "
                f"ACTIVATION ago, and these records can replay only one kind: "
                f"one activation per recorded case, which is `activation: "
                f"on-change`. Under `periodic` the records cannot stand in for "
                f"the activations that happened between them."))
        if not examples.ordered:
            return Verification(refusal=(
                f"{document.name} keeps values from one activation to the next "
                f"(it reads previous()), and the examples do not declare "
                f"themselves `ordered`. Without an order there is no activation "
                f"before this one, and reading the file's order as a timeline "
                f"would be reading a promise nobody made."))

    if not examples.ordered:
        needs_history = sorted(
            [n for n, r in inputs.items()
             if r.get("previous_of") or r.get("state_of") or r.get("protocol")]
            + [n for n, r in outputs.items() if r.get("hold_last")])
        if needs_history:
            return Verification(refusal=(
                f"rule(s) {', '.join(needs_history)} need the round before "
                f"them, and the examples do not declare themselves `ordered`. "
                f"Without an order there is no previous round, and reading the "
                f"file's order as a timeline would be reading a promise nobody "
                f"made."))

    failed: set = set()
    # One holder per joint value of the unresolved inputs -- one when there
    # are none -- each driven through every round under its own value.
    #
    # ⚠ That is sound only while the holders AGREE on what they keep. Driving
    # a constant value per holder tries two timelines of an unknown input, not
    # every one; but if every holder ends a round keeping the same values, the
    # round's kept values did not depend on the unknown at all, so no timeline
    # could have reached a different state -- and the next round starts from
    # one known state again. The first round after which they disagree is
    # where the kept values stop being known, and every later round is refused
    # rather than judged on a state that only one of the guesses reached.
    worlds = ([getattr(module, holder["new"])() for _ in assignments]
              if holder else None)
    # Why what the document kept is no longer known, once it is not.
    lost = ""
    for case, owner, judged in rounds_of(examples.cases):
        if id(owner) in failed:
            continue
        result = CaseResult(name=owner.name or "(unnamed)")
        # Two passes, because a rule about the previous round names ANOTHER
        # input, and on the first round it may have to fall back to what that
        # input reads now. One pass would have to evaluate them in an order
        # nothing declares.
        try:
            plain = {n: r for n, r in inputs.items()
                     if not (r.get("previous_of") or r.get("state_of"))
                     and n not in open_inputs}
            values = {n: input_value(n, r, case, latches, pack.model, history,
                                     pack.conventions, declared.types.get(n),
                                     declared.variants_of(n))
                      for n, r in plain.items()}
            for n, r in inputs.items():
                if n in values or n in open_inputs:
                    continue
                got = input_value(n, r, case, latches, pack.model, history,
                                  pack.conventions, declared.types.get(n),
                                  declared.variants_of(n))
                if got is _UNSEEN:
                    target = r["previous_of"]
                    if target not in values:
                        return Verification(refusal=(
                            f"input {n!r}: reads the previous value of "
                            f"{target!r}, which this binding does not declare"))
                    got = values[target]
                values[n] = got
            stale = sorted(n for n, v in values.items() if v is _UNDETERMINED)
            if stale:
                raise VerifyError(
                    f"input(s) {', '.join(stale)} read back an earlier output "
                    f"that depended on unresolved input(s) "
                    f"{', '.join(unknown)}, so this round has no known input")
        except VerifyError as exc:
            # This case could not be driven. The next one still might. A setup
            # step that cannot be driven leaves its case unjudgeable, and says
            # which step it was.
            result.refusal = str(exc) if judged else f"{case.name}: {exc}"
            verification.results.append(result)
            failed.add(id(owner))
            if worlds is not None and not lost:
                # ⚠ The next one might NOT, when the document keeps values:
                # the activation that did not happen is one the kept values
                # never saw, and every later round would start from a state
                # the host could not have been in.
                lost = (f"round {case.name!r} could not be driven, so what "
                        f"the document kept from then on is not known")
            continue
        kwargs = {_snake(n): v for n, v in values.items()}
        # With a holder, ONE activation per round computes every output, and
        # each output is read from the record it returns.
        records = None
        if worlds is not None:
            if lost:
                result.refusal = lost if judged else f"{case.name}: {lost}"
                verification.results.append(result)
                failed.add(id(owner))
                continue
            try:
                records = [getattr(w, holder["update"])(
                               **kwargs, **{_snake(k): v for k, v in a.items()})
                           for w, a in zip(worlds, assignments)]
            except TypeError as exc:
                return Verification(refusal=(
                    f"the document's holder would not take the bound inputs "
                    f"({exc})"))
            if any(vars(w) != vars(worlds[0]) for w in worlds[1:]):
                lost = (f"after round {case.name!r} what the document kept "
                        f"depends on unresolved input(s) {', '.join(unknown)}, "
                        f"so no later round starts from a known state")
        produced: dict = {}
        undetermined: set = set()
        for name, rule in outputs.items():
            if rule.get("unresolved"):
                # It has nowhere to land yet. The document still computes it,
                # and saying so would be reporting a gap the author declared.
                continue
            if name in withheld:
                # ⚠ Built from a placeholder so the rest could run. Its value
                # is not the document's answer and is never compared -- every
                # position it writes is withheld, and a later round reading it
                # back is withheld too.
                if history is not None:
                    history.outputs[name] = _UNDETERMINED
                undetermined.update(written_positions({name: rule})[0])
                continue
            fn = (None if records is not None else
                  getattr(module, "compute_" + _snake(name), None))
            produces = (hasattr(records[0], _snake(name)) if records is not None
                        else fn is not None)
            if not produces:
                if not rule.get("internal"):
                    return Verification(refusal=(
                        f"output {name!r}: the binding names it and the "
                        f"document does not produce it"))
                continue
            try:
                # One run per joint value of the unresolved inputs -- a single
                # run when there are none, which is the run as it always was.
                runs = ([getattr(r, _snake(name)) for r in records]
                        if records is not None else
                        [fn(**kwargs, **{_snake(k): v for k, v in a.items()})
                         for a in assignments])
                computed = runs[0]
                settled = all(r == computed for r in runs[1:])
                if history is not None:
                    # ⚠ Remembered RAW, before the binding maps it. What a
                    # later round feeds back is the document's own value, not
                    # the platform symbol it lands as -- mapping first would
                    # hand the document a word it never produced.
                    history.outputs[name] = computed if settled else _UNDETERMINED
                if rule.get("hold_last"):
                    # ⚠ The slot, not the document, holds. A component that
                    # does not write a slot leaves what it held, so a value
                    # with no entry in `map` writes the last one this rule
                    # wrote -- and, before there is one, writes nothing.
                    runs = [_held(name, rule, r, history) for r in runs]
                written = [{} if r is _NOT_WRITTEN else output_values(name, rule, r)
                           for r in runs]
                if (rule.get("hold_last") and runs[0] is not _NOT_WRITTEN
                        and all(r == runs[0] for r in runs[1:])):
                    history.held[name] = runs[0]
                # ⚠ Per POSITION, not per output. An output that differs in
                # its status field can still write the same identifier in
                # every run, and that identifier is a thing the document got
                # right whatever the address turns out to be.
                for address in set().union(*written):
                    seen = [w.get(address, _UNSEEN) for w in written]
                    if seen[0] is not _UNSEEN and all(v == seen[0] for v in seen):
                        produced[address] = seen[0]
                    else:
                        undetermined.add(address)
            except VerifyError as exc:
                result.refusal = str(exc) if judged else f"{case.name}: {exc}"
                break
            except TypeError as exc:
                return Verification(refusal=(
                    f"output {name!r}: the document's function would not take "
                    f"the bound inputs ({exc})"))
        if result.refusal:
            verification.results.append(result)
            failed.add(id(owner))
            continue
        judge.judge(result, case, produced, undetermined,
                    open_values=unknown + sorted(withheld))
        if history is not None:
            history.inputs = dict(values)
            history.started = True
        # A setup step moved what it moves -- the remembered values above --
        # and nothing is claimed on it.
        if judged:
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
