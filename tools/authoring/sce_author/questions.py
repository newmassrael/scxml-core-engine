"""What the prose does not say.

This is the deliverable an author reads. Every class below is about the
relation between a text and a model, never about what the text is about, so
the same five run unchanged against any pack.

Precision is the whole value. A warning that is usually wrong trains its reader
to skip the list, and a list nobody reads is worse than no list, because it
also reports that the work was done. So each class below carries the reason it
cannot fire on something already answered, and the CLI reports the count per
class so a class that swamps the others is visible rather than averaged away.

⚠⚠ BEFORE ADDING A CLASS, READ `schema/conventions.v1.schema.json` THROUGH.

The single commonest defect in this package has been a check that gave a wrong
answer because it never consulted something the pack had already declared. It
happened four times in one sitting:

    absence_tokens   unread, so a source a record explicitly marked as not
                     reporting counted as present -- twelve telltales dark
    infrastructure   unread by the kind guard, so the platform's own supply
                     ladder was reported as the author's memory, on 16 of 16
    protocols.latch  the note said "the rungs are cumulative" and only the
                     first clause was built
    gate_off         the cascade already answered a question being asked again

⚠ There is no test for this, and three attempts to build one all failed: a
field is "mentioned" by the loader whether or not any check uses it, the
loader legitimately consumes some fields itself, and others are renamed onto
the dataclass. The defect has no mechanical signature. It has a habit instead,
and this is where the habit is written down.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass, asdict
from functools import lru_cache

from .pack import Conventions, Model, gate_off_value
from .prose import Prose


# How loudly each class speaks. This is a judgement about ACTIONABILITY -- an
# error is something that cannot be right, a warning is something that has to
# be decided, a note is something to confirm -- and it lives here rather than
# in each caller. A caller that had to derive it would derive it differently,
# and the one that got `source-not-fully-read` wrong would silently trust a
# reading of an incomplete text.
SEVERITY = {
    "source-not-fully-read": "error",
    "value-not-in-space": "error",
    "unknown-name": "error",
    # The platform HAS this, several times over, numbered. An error because
    # the document cannot be written until somebody says which member an
    # index picks -- and neither side's document says.
    "name-is-indexed": "error",
    # Grounded silences: something actually exercises this, so the gap is
    # load-bearing rather than theoretical. Louder than the ungrounded pair
    # below for that reason alone.
    "example-drives-unnamed-signal": "error",
    "example-drives-undeclared-address": "error",
    "example-expects-undecided-output": "error",
    # The loudest thing the examples can say: the component is not a function
    # of its inputs. Nothing else in this list can notice it.
    "example-shows-memory": "error",
    "ambiguous-name": "warning",
    "no-decision-logic": "warning",
    "no-time-input": "warning",
    "no-examples": "warning",
    # A check that declined to run. It says so rather than returning nothing,
    # because nothing is indistinguishable from a clean answer.
    "cases-carry-no-values": "note",
    "cases-not-independent": "note",
    "no-comparable-cases": "note",
    "memory-check-partial": "note",
    # A note rather than a warning because the pack's convention does supply
    # an answer. ⚠ Whether that answer is ever witnessed cannot be decided
    # here; `gate_off_unstated` carries the measurement that says so.
    "gate-off-unstated": "note",
    "value-cased-differently": "note",
}


# An identifier read as the words it is made of. A specification may name an
# output by its identifier (`OUT_RoadSignal`) or in words ("the road signal"),
# and both are the same mention. Reading only the first called four outputs
# undecided on a specification that decides all four -- seven false findings
# out of eleven, on the first subject matter that writes in paragraphs.
_WORDS = re.compile(r"[A-Z]+(?![a-z])|[A-Z][a-z0-9]*|[a-z0-9]+")

# Identifiers are removed from a line before its words are read. Without that,
# `IN_TrainApproach` in a sentence lends the word "train" to it, and an output
# called `OUT_TrainSignal` looks mentioned by a line that never mentions it.
_IDENTIFIER = re.compile(r"\b[A-Za-z][A-Za-z0-9]*_[A-Za-z0-9_]+\b")


@lru_cache(maxsize=None)
def _token(symbol: str) -> re.Pattern:
    """This symbol WRITTEN, rather than these letters appearing somewhere.

    `\\b` is not enough: the boundary between `_` and a letter is not a word
    boundary, so `\\bOFF\\b` still matches inside `DISPLAY_OFF`. The guards
    below treat an underscore as part of the name, which is what a symbol is.
    """
    return re.compile(r"(?<![A-Za-z0-9_])" + re.escape(str(symbol))
                      + r"(?![A-Za-z0-9_])")


def _name_words(name: str) -> list[str]:
    core = name.split("_", 1)[-1] if "_" in name else name
    return [w.lower() for w in _WORDS.findall(core) if len(w) >= 3]


def mentions(prose: Prose, names) -> bool:
    """Does this text name this thing, by identifier or in words?"""
    body = prose.text
    if any(n in body for n in names):
        return True
    plain = [_IDENTIFIER.sub(" ", line).lower() for line in body.splitlines()]
    for name in names:
        words = _name_words(name)
        if not words:
            continue
        if len(words) >= 2:
            if any(all(re.search(rf"\b{re.escape(w)}\b", line) for w in words)
                   for line in plain):
                return True
        elif len(words[0]) >= 4:
            if any(re.search(rf"\b{re.escape(words[0])}\b", line) for line in plain):
                return True
    return False


@dataclass
class Question:
    kind: str
    subject: str
    detail: str
    file: str = ""
    line: int = 0

    @property
    def severity(self) -> str:
        return SEVERITY.get(self.kind, "warning")

    @property
    def id(self) -> str:
        """Stable across runs, so a caller can tell one question from the same
        question asked again. Without it nothing can be dismissed, tracked or
        deduplicated -- every run is a fresh list of strangers."""
        seed = f"{self.kind}|{self.subject}|{self.file}".encode("utf-8")
        return hashlib.sha256(seed).hexdigest()[:12]

    def as_dict(self) -> dict:
        out = asdict(self)
        out["id"] = self.id
        out["severity"] = self.severity
        # ⚠ No location is not location zero. Line 0 is a valid-looking number
        # and an empty path is a valid-looking path, so a caller that draws a
        # marker would draw it at the top of a file, next to a sentence that
        # has nothing to do with the question. One class of question is about
        # something the prose NEVER writes and therefore has no line by its
        # nature -- 317 of 2,334 measured. The keys are absent instead.
        if not out["line"]:
            out.pop("line")
        if not out["file"]:
            out.pop("file")
        return out


def _is_a_numbered_member(address: str, prose: Prose, conv: Conventions,
                          model: Model, examples) -> bool:
    """Is this address one member of a family `name-is-indexed` already names?

    Only when the PROSE writes the family's name: otherwise the address is
    undeclared for an unrelated reason and its trailing digits mean nothing.
    """
    leaf = address.rsplit(".", 1)[-1]
    stem = leaf.rstrip("0123456789")
    if stem == leaf:
        return False
    return (stem in prose.names_of(conv)
            and len(_numbered(stem, model, examples)) > 1)


def _numbered(name: str, model: Model, examples) -> list[str]:
    """Members of a numbered family for this name, from BOTH witnesses."""
    leaves = set(model.numbered(name))
    for case in getattr(examples, "cases", ()) or ():
        for address in list(case.given) + list(case.expect):
            leaf = address.rsplit(".", 1)[-1]
            if leaf.startswith(name) and leaf[len(name):].isdigit():
                leaves.add(leaf)
    return [leaf for _n, leaf in sorted((int(leaf[len(name):]), leaf)
                                        for leaf in leaves)]


def unknown_names(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """A name the prose must receive from outside, that the model does not have.

    Only the classes the conventions call `supplied` can raise this. A name the
    document defines itself has no reason to be in an interface model, and
    reporting one asks the author to request their own variable from the
    platform team.
    """
    out = []
    for name, role in sorted(prose.names_of(conv).items()):
        if role != "supplied" or model.resolve(name):
            continue
        where = prose.locate(name)
        # ⚠ "The platform has not got this" is the wrong sentence when the
        # platform has FIFTEEN of it. A specification writes one subscripted
        # name where the platform publishes a numbered family, and telling the
        # author their name does not exist sends them to ask for something
        # they already have -- when the real question is which member an index
        # picks, which no document on either side answers.
        # ⚠ Both sources. A numbered family often reaches the pack through the
        # EXAMPLES rather than the interface model -- the model is built from
        # what the prose names, and the prose names the subscripted form, so
        # the members never get in. The cases place them, which is the second
        # witness to what the platform actually has.
        numbered = _numbered(name, model, examples)
        if len(numbered) > 1:
            out.append(
                Question(
                    kind="name-is-indexed",
                    subject=name,
                    detail=(
                        f"the interface model has no {name}, and it has "
                        f"{len(numbered)} addresses named {numbered[0]} "
                        f"through {numbered[-1]}. Nothing on either side says "
                        f"which one an index picks"
                    ),
                    file=str(where[0]) if where else "",
                    line=where[1] if where else 0,
                )
            )
            continue
        out.append(
            Question(
                kind="unknown-name",
                subject=name,
                detail="named as an input but no address in the interface model",
                file=str(where[0]) if where else "",
                line=where[1] if where else 0,
            )
        )
    return out


def ambiguous_names(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """A name that reaches two addresses. The pack has to settle it, not the reader."""
    out = []
    for name in sorted(prose.names_of(conv)):
        hits = model.resolve(name)
        if len(hits) > 1:
            where = prose.locate(name)
            out.append(
                Question(
                    kind="ambiguous-name",
                    subject=name,
                    detail="reaches " + ", ".join(e.address for e in hits),
                    file=str(where[0]) if where else "",
                    line=where[1] if where else 0,
                )
            )
    return out


def _looks_like_a_symbol(token: str, space: dict) -> bool:
    """Does this token have the shape the symbols of THIS space have?

    Derived from the pack's own data rather than from a list of words to
    ignore. Prose sits on both sides of a comparison operator -- a
    specification writes `X == Y, excluding Z` and a naive reader takes
    `excluding` for a value -- and a word list to exclude it would be a
    guess about a language, which is exactly what a general tool cannot hold.

    The space answers instead: if every symbol in it is upper case, then a
    token that is not upper case is prose. Seven of eighteen findings were
    this, measured 2026-09-19.
    """
    shapes = [
        (re.compile(r"^[A-Z][A-Z0-9_]*$"), lambda s: bool(re.fullmatch(r"[A-Z][A-Z0-9_]*", s))),
        (re.compile(r"^[a-z][a-z0-9_]*$"), lambda s: bool(re.fullmatch(r"[a-z][a-z0-9_]*", s))),
    ]
    for pattern, admits in shapes:
        if all(pattern.match(s) for s in space):
            return admits(token)
    return True


def values_outside_space(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """The prose compares a name against a symbol that name cannot take.

    This is the check that catches a specification drifting from the platform,
    and it is only sound when the name resolves to exactly one address with an
    enumerated space; anything else is skipped rather than guessed at.

    Three things are deliberately NOT reported here:

      a token the pack calls an absence    absence is not a member of any value
                                           space and putting it in one would
                                           invent a value the platform has not
                                           got
      a token shaped unlike the space      it is prose that happened to follow
                                           an operator
      a token that differs only in case    reported separately and more
                                           softly, because the author wrote a
                                           symbol that exists and spelled it
                                           their own way, which is a different
                                           conversation from writing one that
                                           does not exist
    """
    out = []
    seen = set()
    for name, op, token in prose.comparisons(conv):
        hits = model.resolve(name)
        if len(hits) != 1:
            continue
        entry = hits[0]
        scalar = entry.field("")
        if scalar is None or scalar.values is None:
            continue
        space = scalar.values
        if token in space or token.isdigit() or token in conv.absence_tokens:
            continue
        # ⚠ The case-folded lookup comes BEFORE the shape gate, and the order
        # is the whole point: a token that matches a symbol apart from its case
        # IS that symbol, whatever shape it has. Gating on shape first dropped
        # `Valid` against a space of `VALID`/`INVALID` as though it were prose.
        folded = {s.lower(): s for s in space}
        if token.lower() not in folded and not _looks_like_a_symbol(token, space):
            continue
        key = (name, token)
        if key in seen:
            continue
        seen.add(key)
        where = prose.locate(token)
        if token.lower() in folded:
            out.append(
                Question(
                    kind="value-cased-differently",
                    subject=f"{name} {op} {token}",
                    detail=f"{entry.address} spells it {folded[token.lower()]}",
                    file=str(where[0]) if where else "",
                    line=where[1] if where else 0,
                )
            )
            continue
        out.append(
            Question(
                kind="value-not-in-space",
                subject=f"{name} {op} {token}",
                detail=(
                    f"{entry.address} admits "
                    + ", ".join(sorted(space))
                    + f" — not {token}"
                ),
                file=str(where[0]) if where else "",
                line=where[1] if where else 0,
            )
        )
    return out


def outputs_without_logic(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """An output exists and nothing in the prose decides it.

    The test is whether the prose writes any of the names the model gives that
    address. An earlier form of this check asked instead whether a table's
    output column fitted the address's value space, and was wrong 41 times out
    of 42: a table that emits an identifier can never fit a status enumeration,
    so the check was measuring the shape of the table rather than the silence
    of the document.
    """
    out = []
    for entry in model.outputs():
        if mentions(prose, entry.names):
            continue
        out.append(
            Question(
                kind="no-decision-logic",
                subject=entry.address,
                detail=(
                    "the specification never writes "
                    + (" or ".join(entry.names) if entry.names else "any name for this address")
                ),
            )
        )
    return out


def gate_off_unstated(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """The prose says what an output shows, and not what it shows otherwise.

    Reported only where the pack's cascade has an answer, so the question is
    always actionable: it names the value the platform would use and asks the
    author to confirm it rather than leaving the reader to discover that a
    value had to be chosen at all.
    """
    if not conv.gate_off:
        return []
    out = []
    body = prose.text
    universe = {e.address: e.names for e in model.outputs() if e.names}
    blocks = prose.blocks(universe)
    for entry in model.outputs():
        if not mentions(prose, entry.names):
            continue  # already reported as having no logic at all
        # ⚠ The off value has to be looked for in what the prose says about
        # THIS output, and a table puts the other case on a continuation row
        # that never repeats the name. Both narrower readings were wrong; see
        # `Prose.blocks`.
        said = blocks.get(entry.address, "")
        for fld in entry.fields:
            off = gate_off_value(conv, fld.values)
            # ⚠ As a TOKEN, not a substring. `off in said` read
            # `DISPLAY_OFF`, `ACT_PRESSURE_OFF` and even an image identifier
            # holding `HEV` as the document having stated the off value, and
            # went quiet on eleven positions where it had not. Found by
            # deleting the closing case from documents that state it and
            # checking that the class then fires: 38 of 49 did, and every one
            # of the eleven that did not was this.
            if off is None or _token(off).search(said):
                continue
            where = prose.locate(entry.names[0]) if entry.names else None
            # ⚠ This class stays UNGROUNDED, and the attempt to ground it is
            # written down here so it is not made again.
            #
            # It is the loudest class by count and the weakest by evidence: it
            # is true by construction -- the prose really does not state the
            # other case -- but what makes it worth raising is whether the
            # convention's answer is ever WITNESSED. So a split was tried:
            # louder when `examples.expected` reads the position back, a note
            # when nothing does.
            #
            # Measured on one corpus of 129 subject packs before keeping it,
            # and it separates nothing: 3272 of 3600 output fields are asserted
            # by some case, 101 of the 129 packs assert every one of their own,
            # and the split moved 732 of 757 findings into the loud half.
            #
            # The reason is that address-level reading is the wrong predicate.
            # The question worth asking is whether any case exercises the
            # precondition-FALSE branch, and that cannot be decided here: the
            # gate is written over the document's own identifiers, and what
            # maps an identifier to an address is the BINDING, which does not
            # exist yet when questions are asked. Grounding this would need
            # case values and a binding, so it belongs downstream of both.
            out.append(
                Question(
                    kind="gate-off-unstated",
                    subject=entry.address + (f".{fld.name}" if fld.name else ""),
                    detail=(
                        f"the specification never states the value when the "
                        f"precondition is false; the platform convention gives {off}"
                    ),
                    file=str(where[0]) if where else "",
                    line=where[1] if where else 0,
                )
            )
    return out


def durations_without_a_clock(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """The prose states a duration and no input can observe time passing.

    A conversion that ignores the duration still passes a test suite that never
    advances a clock, so this class exists to say what a green result cannot.
    """
    if conv.duration_pattern is None or conv.time_inputs:
        return []
    out = []
    seen = set()
    for src in prose.sources:
        for lineno, line in enumerate(src.text.splitlines(), 1):
            m = conv.duration_pattern.search(line)
            if not m or m.group(0) in seen:
                continue
            seen.add(m.group(0))
            out.append(
                Question(
                    kind="no-time-input",
                    subject=m.group(0),
                    detail="a duration is stated and the pack declares no input that observes time",
                    file=str(src.path),
                    line=lineno,
                )
            )
    return out


def source_not_fully_read(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    """The reader could not carry part of a source document.

    This class is first among equals: every other question below asks what the
    text does not say, and all of them are wrong if the text is not all there.
    A specification whose tables live in an embedded object reads as a
    specification that decides nothing, and the difference is invisible unless
    the reader says so here.
    """
    return [
        Question(
            kind="source-not-fully-read",
            subject=str(src.path),
            detail=note,
            file=str(src.path),
        )
        for src in prose.sources
        for note in src.notes
    ]


def examples_drive_unnamed(prose: Prose, model: Model, conv: Conventions,
                           examples=None) -> list[Question]:
    """Something exercises an address and the specification never names it.

    This is the class the other seven cannot reach. All of them compare a text
    against a model, and a model can only say what EXISTS -- so a specification
    that simply fails to mention a signal the product uses reads as a complete
    specification. Nothing is missing until something expects it.

    Measured against one corpus: every conversion that stalled did so here.
    Two of them were a signal the examples drive and the prose never writes;
    the others were behaviour no static reading could have produced.
    """
    if examples is None or not examples.present:
        return []
    body = prose.text
    origin = f" (examples: {examples.origin})" if examples.origin else ""
    out = []
    for address in sorted(examples.driven):
        if address.startswith(conv.infrastructure):
            continue
        # An address may name a field of a record, and the field may itself be
        # nested. `Model.owning` follows it up; stripping a single segment
        # left four declared addresses looking undeclared.
        entry = model.owning(address)
        if entry is None and _is_a_numbered_member(address, prose, conv,
                                                   model, examples):
            # ⚠ Already said ONCE, and better, by `name-is-indexed`. A
            # specification that writes `Setting_X[i]` against a platform
            # holding `Setting_X1..X15` produced FIFTEEN copies of it, and the
            # 25 copies on one component buried the four addresses that were
            # the real fact -- that its main input is another unit's output.
            # One fact reported once is what makes the other visible.
            continue
        if entry is None:
            # ⚠ NOT in the model at all, and this is the sharper half. An
            # address the examples drive that this pack does not even declare
            # is something the subject matter reaches for from OUTSIDE itself
            # -- another component's output, a shared setting. A specification
            # that is silent about that reads as self-contained when it is not,
            # and no comparison against this pack's own model can say so.
            out.append(
                Question(
                    kind="example-drives-undeclared-address",
                    subject=address,
                    detail=(
                        "the examples drive this and the interface model does "
                        "not declare it; the specification is reaching outside "
                        "what it declares" + origin
                    ),
                )
            )
            continue
        if entry.role == "output" or mentions(prose, entry.names):
            continue
        out.append(
            Question(
                kind="example-drives-unnamed-signal",
                subject=address,
                detail="the examples drive this and the specification never names it" + origin,
            )
        )
    return out


def examples_expect_undecided(prose: Prose, model: Model, conv: Conventions,
                              examples=None) -> list[Question]:
    """An address the examples read back that the prose never decides.

    `no-decision-logic` asks the same thing of every output in the model, and
    most of those nothing ever reads. This one is GROUNDED: something actually
    requires a value here, so the silence is load-bearing rather than
    theoretical.
    """
    if examples is None or not examples.present:
        return []
    body = prose.text
    out = []
    for address in sorted(examples.expected):
        entry = model.owning(address)
        if entry is None:
            continue
        if mentions(prose, entry.names):
            continue
        out.append(
            Question(
                kind="example-expects-undecided-output",
                subject=address,
                detail="the examples read this back and the specification never decides it",
            )
        )
    return out


def examples_need_memory(prose: Prose, model: Model, conv: Conventions,
                         examples=None) -> list[Question]:
    """Two cases agree on every input and disagree on an output.

    Then the answer is not a function of the inputs, and something is being
    remembered. This is worth its own class because NOTHING ELSE CAN SEE IT.
    An interface model says what exists, so it cannot object; the prose is
    free to describe each case correctly and never mention that the two are
    distinguished by history; and a document can declare itself a pure
    computation and be believed, because it syntactically is one -- the memory
    ends up in whatever drives it.

    ⚠ Measured on one corpus: the two components that did not convert cleanly
    failed for exactly this reason, and in both the logic was written as a
    pure computation. One watched a quantity against a threshold that turns on
    higher than it turns off; the other reported which of many events had just
    CHANGED, which needs a bit of history per event. Neither is unusual, and
    neither was stated in the prose.

    Two conditions have to hold before this can be asked at all, and when
    either fails the answer is that the check did not run, never silence:

      the cases carry values      addresses alone cannot be compared
      the cases are independent   a case that is a delta on the one before it
                                  was not run under the inputs it lists
    """
    if examples is None or not examples.present:
        return []
    if not examples.valued:
        return [Question(
            kind="cases-carry-no-values",
            subject="examples",
            detail=("the cases name the addresses they touch but not the "
                    "values, so no check that compares two cases can run"),
        )]
    if not examples.independent:
        return [Question(
            kind="cases-not-independent",
            subject="examples",
            detail=("the examples do not declare `independent_cases`, so a "
                    "case may be a delta on the one before it and no check "
                    "that compares two cases can run"),
        )]
    # ⚠ Compare on the inputs THIS component has, not on everything a case
    # happens to set. A case may carry a whole installation's signals, and
    # comparing whole `given` maps found nothing at all on a corpus of 18
    # components -- with tens of keys accumulated, two cases are never equal,
    # so the check could not fire and looked clean.
    #
    # What the component is a function OF is what the interface model declares
    # as its inputs, and an address outside that which a case drives is
    # already `example-drives-undeclared-address`. The classes compose; this
    # one does not need to repeat that.
    inputs = tuple(sorted(e.address for e in model.entries if e.role != "output"))
    seen: dict[str, dict[str, object]] = {}
    found: dict[str, tuple[str, str]] = {}
    judged: set[str] = set()
    expected: set[str] = set()
    comparable = False
    for case in examples.cases:
        key = _stable({a: case.given.get(a) for a in inputs})
        if key in seen:
            comparable = True
        expected |= set(case.expect)
        for address, value in case.expect.items():
            earlier = seen.setdefault(key, {})
            if address in earlier:
                # Read back twice under the same inputs: this address is one
                # the check can actually speak about.
                judged.add(address)
                if earlier[address] != value and address not in found:
                    found[address] = (earlier[address], value)
            earlier.setdefault(address, value)
        seen.setdefault(key, {})
    # ⚠⚠ The third way this check can fail to speak, and the one that looks
    # exactly like a clean answer: no two cases ever drive the same inputs, so
    # there was never a comparison to make.
    #
    # This is not a corner. Measured over 127 packs, 62 of them -- 49% -- have
    # no two comparable cases at all. And the component that prompted the
    # class is one of them: its own document holds the previous answer, so it
    # demonstrably remembers, and the check returned nothing. Reading that
    # nothing as "no memory here" is the mistake the whole module is built to
    # make impossible.
    if not comparable:
        return [Question(
            kind="no-comparable-cases",
            subject="examples",
            detail=("no two cases drive the same inputs, so whether anything "
                    "is remembered cannot be decided from these examples"),
        )]
    out = []
    # ⚠⚠ Comparability is per ADDRESS, and saying it only per pack hides most
    # of the truth. Measured over the 65 packs that have any comparable pair
    # at all: of 2702 addresses the cases read back, only 410 -- 15% -- were
    # ever read back TWICE under the same inputs. The other 85% got no answer
    # and no notice, which reads exactly like a clean one.
    #
    # The case that showed it: on one component the check spoke about one
    # slot and was silent about the neighbouring slot that actually fails, and
    # the silence was "never compared", not "nothing remembered".
    #
    # One decline per unjudged address would be 2292 notes and would drown the
    # list, so the coverage is reported as a number instead. A reader who sees
    # 1 of 5 knows what the silence about the other four is worth.
    if len(judged) < len(expected):
        out.append(Question(
            kind="memory-check-partial",
            subject="examples",
            detail=(f"only {len(judged)} of {len(expected)} addresses the cases "
                    f"read back were ever read back twice under the same "
                    f"inputs; whether the rest remember anything is undecided"),
        ))
    for address in sorted(found):
        first, second = found[address]
        entry = model.owning(address)
        out.append(
            Question(
                kind="example-shows-memory",
                subject=address,
                detail=(
                    f"two cases drive the same inputs and require {first!r} "
                    f"and {second!r} here, so this depends on something other "
                    f"than the inputs; the specification has to say what is "
                    f"remembered"
                ),
                file=str(prose.locate(entry.names[0])[0])
                if entry and entry.names and prose.locate(entry.names[0]) else "",
                line=prose.locate(entry.names[0])[1]
                if entry and entry.names and prose.locate(entry.names[0]) else 0,
            )
        )
    return out


def _stable(mapping: dict) -> str:
    """A comparable key for a case's inputs, whatever the values are made of."""
    return repr(sorted((str(k), repr(v)) for k, v in mapping.items()))


def examples_are_absent(prose: Prose, model: Model, conv: Conventions,
                        examples=None) -> list[Question]:
    """Say so when there is no second opinion.

    ⚠ Without examples, two classes above cannot fire, and a caller reading a
    short list would take it for a clean specification. An empty result from a
    check that never ran is the shape this whole tool exists to refuse.
    """
    if examples is not None and examples.present:
        return []
    return [
        Question(
            kind="no-examples",
            subject="(this pack)",
            detail=(
                "the pack supplies no examples, so nothing here can notice a "
                "signal the specification simply fails to mention. Every other "
                "answer compares the text against what EXISTS, which cannot "
                "reveal what is missing."
            ),
        )
    ]


CLASSES = (
    source_not_fully_read,
    examples_are_absent,
    examples_drive_unnamed,
    examples_expect_undecided,
    examples_need_memory,
    unknown_names,
    ambiguous_names,
    values_outside_space,
    outputs_without_logic,
    gate_off_unstated,
    durations_without_a_clock,
)


def ask(prose: Prose, model: Model, conv: Conventions, examples=None) -> list[Question]:
    # Every class takes the same four arguments. Dispatching on whether a call
    # raises TypeError was tried and removed: it swallows a real TypeError
    # raised INSIDE a class, and a check that fails silently is the shape this
    # whole tool exists to refuse.
    out: list[Question] = []
    for fn in CLASSES:
        out += fn(prose, model, conv, examples)
    return out
