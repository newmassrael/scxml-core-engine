"""One page for whoever writes the document.

The shape below was observed rather than designed. Over twenty conversions of
one subject matter, the same six things were looked up in the same order every
time, and every conversion that came out wrong had gone wrong on a platform
convention rather than on a misreading of the prose. Assembling those six is
therefore not a convenience: it removes the class of error that actually
occurred.

The briefing carries specification text, so it is written to a file and the
caller is told only its size. A pack may be confidential; a general tool has no
business deciding otherwise on its behalf.
"""

from __future__ import annotations

import pathlib

from .pack import Pack, gate_off_value, rule_text
from .prose import Prose
from .questions import ask


def _value_line(fld) -> str:
    if fld.values:
        return ", ".join(f"{s}={n}" for s, n in sorted(fld.values.items(), key=lambda kv: kv[1]))
    return f"({fld.type or 'no declared value space'})"


HEADER = [
    "# Conversion brief",
    "",
    "Assembled by a machine. The decision logic is in section 1 and nowhere",
    "else; the rest is what section 1 needs in order to reach a platform.",
    "",
]


# The brief's own section headings, as `_parts` writes them. Sections are
# found by these ELEMENTS, never by searching the joined text: a specification
# carries headings of its own (`## Road signal`), and splitting the text on
# "## " cut section 1 into pieces.
HEADINGS = (
    "## 1. Specification",
    "## 2. Addresses this specification touches",
    "## 3. Outputs this specification is expected to decide",
    "## 4. Preconditions",
    "## 5. What this specification does not answer",
    "## 6. When you have to decide anyway",
    "## 7. When the answer depends on what happened before",
    "## 8. What the host does with the document",
)


def sections(prose: Prose, pack: Pack) -> list[tuple[str, list[str]]]:
    """The brief as (heading, lines) pairs, in order. `assemble` joins them.

    ⚠ Split so a caller can take them one at a time. For the largest
    specification in one corpus the whole brief was 227,761 characters --
    72% of it section 1, the specification itself -- and a tool result that
    size is refused by the client that asked for it and spilled into a file
    (measured 2026-09-26; a writer read it back in pieces, a smaller model
    could not). `index` is what a caller gets instead.
    """
    out: list[tuple[str, list[str]]] = []
    for element in _parts(prose, pack)[len(HEADER):]:
        if element in HEADINGS:
            out.append((element[len("## "):], []))
        else:
            out[-1][1].append(element)
    return out


def _join(chosen: list[tuple[str, list[str]]]) -> str:
    lines = list(HEADER)
    for heading, body in chosen:
        lines += [f"## {heading}", *body]
    return "\n".join(lines) + "\n"


def index(prose: Prose, pack: Pack, limit: int) -> str:
    """What a caller is told when the whole brief exceeds `limit` characters."""
    lines = HEADER[:1] + [
        "",
        f"The whole brief is too large to return at once (over {limit:,} "
        f"characters). Ask for its sections by number with `sections`, one or "
        f"a few at a time:",
        "",
    ]
    for heading, body in sections(prose, pack):
        number = heading.split(".", 1)[0]
        note = ""
        if number == "1":
            note = (" -- the specification itself, exactly the prose file(s) you "
                    "passed; read those files directly, in parts, instead")
        lines.append(f"- {heading} ({len(chr(10).join(body)):,} characters){note}")
    return "\n".join(lines) + "\n"


def pick(prose: Prose, pack: Pack, wanted: list[int]) -> str:
    """Only the numbered sections asked for, in the order asked."""
    found = {h.split(".", 1)[0]: (h, b) for h, b in sections(prose, pack)}
    missing = [n for n in wanted if str(n) not in found]
    if missing:
        raise ValueError(f"the brief has no section {missing[0]}; it has "
                         f"{', '.join(sorted(found, key=int))}")
    return _join([found[str(n)] for n in wanted])


def assemble(prose: Prose, pack: Pack) -> str:
    return _join(sections(prose, pack))


def _parts(prose: Prose, pack: Pack) -> list[str]:
    model, conv = pack.model, pack.conventions
    names = prose.names_of(conv)
    questions = ask(prose, model, conv, pack.examples)

    # ⚠ The test is "does the prose WRITE this name", against the text.
    # Testing membership of `names` instead asks a narrower question -- that
    # set only holds names matching the pack's name_classes, which are the
    # ones a document RECEIVES. Every output was therefore reported as
    # untouched while the question class, which reads the text, correctly
    # said otherwise. A brief that contradicts the questions beside it is
    # worse than one that says less.
    body = prose.text
    touched = [e for e in model.entries if any(n in body for n in e.names)]

    parts: list[str] = HEADER + [
        "## 1. Specification",
        "",
    ]
    for src in prose.sources:
        parts += [f"<!-- {src.path} (read as {src.reader}) -->", ""]
        for note in src.notes:
            # Beside the text it is missing from, not in a log somewhere.
            parts += [f"> NOT CARRIED FROM THIS FILE: {note}", ""]
        parts += [src.text.rstrip(), ""]

    parts += ["## 2. Addresses this specification touches", ""]
    if not touched:
        parts.append("(none — no name in the prose resolves to the interface model)")
    for entry in touched:
        parts.append(f"- `{entry.address}` ({entry.role}) — written as " + ", ".join(entry.names))
        for fld in entry.fields:
            label = f".{fld.name}" if fld.name else ""
            parts.append(f"  - `{label or '(scalar)'}` {_value_line(fld)}")
            # ⚠ Two things this line got wrong, and both made a reader
            # believe something the core does not know.
            #
            # It was printed for INPUTS as well, where the question has no
            # meaning at all -- a two-valued input read as "when the
            # precondition is false it is WARN", which is not a sentence
            # about anything.
            #
            # And it was stated as a FACT. It is a proposal from the pack's
            # cascade, which is a procedure with a measured hit rate, not an
            # oracle. A proposal that reads like a fact is the shape a tool
            # takes when it makes its reader wrong quietly.
            if entry.role != "output":
                continue
            off = gate_off_value(conv, fld.values)
            if off is not None:
                note = f" ({conv.gate_off_note})" if conv.gate_off_note else ""
                parts.append(
                    f"    - the pack's convention PROPOSES `{off}` when the "
                    f"precondition is false{note}. The specification does not "
                    f"say; confirm it against the platform."
                )

    parts += ["", "## 3. Outputs this specification is expected to decide", ""]
    for entry in model.outputs():
        mark = "written" if entry in touched else "NOT WRITTEN BY THE PROSE"
        parts.append(f"- `{entry.address}` — {mark}")

    parts += ["", "## 4. Preconditions", ""]
    if conv.precondition_inputs:
        parts.append("Inputs the document declares, and what each observes:")
        for name, note in sorted(conv.precondition_inputs.items()):
            rule = conv.precondition_rules.get(name)
            # The rule is printed as the binding writes it: `check` holds a
            # binding reading this input to exactly this.
            parts.append(f"- `{name}` — {note}"
                         + (f"; bind it as `{rule_text(rule)}`" if rule else ""))
    if conv.precondition_phrases:
        parts.append("")
        parts.append("Phrase the prose writes, and the expression it becomes:")
        for phrase, expr in sorted(conv.precondition_phrases.items()):
            # ⚠ An assumed reading is printed AS one. This line used to be
            # the only place the table surfaced at all, and an assumption
            # printed like any other reading is a fact to whoever writes the
            # document -- the author then drops the condition with nothing to
            # say they were told why.
            reason = conv.precondition_assumed.get(phrase)
            parts.append(f"- `{phrase}` -> `{expr}`"
                         + (f" — ASSUMED by the pack: {reason}" if reason else ""))
    if conv.protocols:
        parts.append("")
        parts.append("Ways of reading an address that a binding may name:")
        for proto, spec in sorted(conv.protocols.items()):
            params = ", ".join(spec["parameters"])
            parts.append(f"- `{proto}` ({params})" + (f" — {spec['note']}" if spec.get("note") else ""))

    parts += ["", "## 5. What this specification does not answer", ""]
    if not questions:
        parts.append("(nothing found)")
    for q in questions:
        at = f" [{q.file}:{q.line}]" if q.file else ""
        parts.append(f"- **{q.kind}** `{q.subject}`{at} — {q.detail}")

    parts += ["", "## 6. When you have to decide anyway", "", _DECIDING]
    parts += ["", "## 7. When the answer depends on what happened before",
              "", _REMEMBERING]
    parts += ["", "## 8. What the host does with the document", "",
              *host_lines(conv.host)]
    return parts


# ⚠ What the host does was a page written by hand for each corpus and handed
# to the author beside the pack, while `verify` read the pack's `host` -- two
# statements of one platform fact, one of them invisible to every tool. The
# first time they parted, the page said "the component writes its outputs at
# the end of every round", an author wrote every output every round, and an
# event the specification delays by two seconds announced its old value at
# 2 ms, failing the platform's own test (measured 2026-09-27). So the author is
# told here, from the same keys `verify` models.
_ACTIVATIONS = {
    "on-change": (
        "The host runs the document once each time one of its inputs CHANGES "
        "value. A write of the value an input already holds reaches the "
        "document not at all, so an input's event means it changed. Each such "
        "run is one round."),
    "periodic": (
        "The host runs the document once per period, whatever changed. Each "
        "such run is one round."),
}

_WRITES = {
    "every-round": """\
At the end of every round the host writes each output its rule writes that
round, changed or not -- and every write is seen: whatever waits on an output,
a test included, is told of each one and reads the first after the input it
caused. So a round that writes an output announces its value then.

- A transform's outputs, and a statechart output whose rule gives a
  `when_nothing_sent` its `map` names, are written every round.
- A statechart output bound `hold_last: true` with a `when_nothing_sent` its
  `map` leaves out is written only in a round the document sends it a mapped
  value; in every other round it keeps what it held and nothing is announced.

Where the specification says an output takes its value only at a certain
moment -- after a delay, on a transition, when a timer expires -- write it then
and not before: rewriting its old value in the meantime announces that old
value first. Bind such an output the second way and send it only at that moment
(a delayed `<send>` for a delay). Everything else, the first way.""",
}


def host_lines(host: dict) -> list[str]:
    """Section 8: the pack's `host`, in the words an author acts on."""
    if not host:
        return ["The pack does not say what the host does. When the document "
                "runs and which outputs a round writes are facts about the "
                "deployment: ask whoever hosts it, and state them in the pack's "
                "`host` so every tool reads the same answer."]
    lines: list[str] = []
    activation = host.get("activation")
    if activation:
        lines += [f"**When it runs** (`activation: {activation}`, for every "
                  f"binding on this platform -- a binding leaves it out and "
                  f"takes this one). " + _ACTIVATIONS[activation], ""]
    else:
        lines += ["**When it runs** is not stated for the platform: a binding "
                  "whose document keeps values says `activation` itself.", ""]
    writes = host.get("writes")
    if writes:
        lines += [f"**What a round writes** (`writes: {writes}`).", "",
                  _WRITES[writes]]
    else:
        lines += ["**What a round writes** is not stated for the platform, so "
                  "no case can be read as the first thing a round announced."]
    return lines


# ⚠ A specification very often answers from history -- "when A becomes B",
# "keep the last value while nothing is reported" -- and until a transform
# could say so, the only way to write it was for the BINDING to feed the value
# back in. The document then claimed a purity it did not have, and the memory
# was left to whoever hosts the generated code. Measured on one corpus, eleven
# of twenty-nine documents were written that way. The move is taught here, at
# the point of writing, because a refusal after the fact is the more
# expensive way to learn it.
_REMEMBERING = """\
A transform can keep a value from one activation to the next, and says so
itself. `previous(x)` is the value field `x` of this document held at the end
of the previous activation; `x` is one of its inputs or outputs:

    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="reported > 0 ? reported : previous(shown)"/>

- A field read through `previous()` must declare `sce:initial`: the value it
  holds before the first activation. There is no silent default.
- A read through `previous()` is not a dependency, so an output may read its
  own previous value. `shown = shown + 1` is a cycle; `previous(shown) + 1` is
  not.
- When the host runs the document -- `activation: on-change` (once each time
  its inputs change) or `activation: periodic` (once per period) -- is the
  pack's `host` (section 8) or, where the pack does not say, the binding's.
  What "previous" means depends on it, and `check` and `verify` refuse the
  binding of a document that reads `previous()` until one of them says.

Do not make the BINDING remember instead (`previous_of`, `state_of`). Memory
the binding keeps is memory the document does not declare, so the document
alone is no longer the component; `check` refuses it on a transform and names
the `previous()` that replaces it.
"""


# ⚠ Section 5 tells an author what is missing and stops there, which leaves
# them exactly where they were: something has to go in the document, the
# specification does not say what, and nothing tells them how to write that
# down. Both markers below already exist in the product -- they are the
# blocking and non-blocking halves of one pair -- and neither was mentioned to
# the person who needs them.
#
# Measured on one corpus, twice over: a document that used `sce:unresolved`
# had its refusal carried all the way back to the author WITH the reason they
# wrote, and a document that used `sce:assumed` compiled, ran, and had its
# assumption refuted by a case years later -- which `verify` now reports as
# "the guess you recorded is the thing that failed" rather than as a defect.
# A guess with no marker gets neither.
_DECIDING = """\
A question in section 5 is not a reason to stop. It is a reason not to guess
SILENTLY. Two markers say which kind of decision you made, and the difference
between them is whether the build should proceed:

- `sce:unresolved="SOME_ID" sce:unresolved-reason="..."` — nobody has decided
  and the value is load-bearing. The code generator REFUSES a document that
  carries one, and the refusal quotes your reason, so it reaches whoever can
  answer instead of being lost. Use it when a wrong value would be worse than
  no build.

- `sce:assumed="SOME_ID" sce:assumed-reason="..."` — you had to choose and you
  chose. The build proceeds. `verify` remembers it: if a case ever contradicts
  a value that rests on the assumption, the report names the assumption and
  quotes your reason rather than reporting that the document is wrong.

Write the reason for whoever has to answer it, not for yourself: what the
source does and does not say, what you tried, and what would settle it. A
marker with no reason is a guess with a label.

Every `sce:assumed` also lists the values the decision could take under the
specification, the current one included: `sce:assumed-candidates="OFF ON"`.
One value alone says the decision has no other. `check` refuses a guess
without the list. A failing case can then be run with each other value in its
place (`gaps --counterfactual`), and the one that repairs it is the answer to
send back.

Give each decided value its own `<data>` -- a decision variable -- whose
`expr` is that value alone, mark IT `sce:assumed`, and have the logic read it:

    <data id="offRedZone" sce:type="int32" sce:direction="out"
          sce:assumed="REDZONE_OFF" sce:assumed-reason="..."
          sce:assumed-candidates="1 2" expr="1"/>
    <data id="redZone" sce:type="int32" sce:direction="out"
          expr="ign ? computed : offRedZone"/>

In a transform every field is `in` or `out`, so the decision variable is an
`out` the binding lands nowhere: `offRedZone: {internal: true}`. Then the list
has exactly one place to go. A value written inline, once, in
a larger expression is accepted too; a value the expression writes more than
once is not, because which of them the decision is cannot be told apart. A
value held before the first round can be the decision as well
(`sce:initial`).

⚠ THE SAME APPLIES TO ADDRESSES, AND YOU MAY NOT HAVE THEM YET.

The document is already independent of the platform: it uses its own
identifiers and the BINDING is the dictionary that says which address each one
is. So the decision logic can be written in full before anybody has produced
the platform's list of addresses -- section 2 of this page is that list, and if
it is thin or absent, that is the situation you are in.

Do not invent an address. Write the rule with the reason instead:

    inputs:
      supplyOn:
        unresolved: "the source calls this the supply signal and the
                     platform list is not available yet"

`check` then reports it as an address still missing rather than as a name that
does not exist, and `verify` says it cannot run rather than running on a value
nobody supplied. When the list arrives, only the binding changes; the document
you wrote does not.

A plausible wrong address is invisible. A declared missing one is a question
with an owner.
"""


def write(prose: Prose, pack: Pack, out: pathlib.Path) -> int:
    text = assemble(prose, pack)
    out.write_text(text, encoding="utf-8")
    return len(text.encode("utf-8"))
