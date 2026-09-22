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

from .pack import Pack, gate_off_value
from .prose import Prose
from .questions import ask


def _value_line(fld) -> str:
    if fld.values:
        return ", ".join(f"{s}={n}" for s, n in sorted(fld.values.items(), key=lambda kv: kv[1]))
    return f"({fld.type or 'no declared value space'})"


def assemble(prose: Prose, pack: Pack) -> str:
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

    parts: list[str] = [
        "# Conversion brief",
        "",
        "Assembled by a machine. The decision logic is in section 1 and nowhere",
        "else; the rest is what section 1 needs in order to reach a platform.",
        "",
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
            parts.append(f"- `{name}` — {note}")
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
    return "\n".join(parts) + "\n"


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
- The binding says when the host runs the document: `activation: on-change`
  (once each time its inputs change) or `activation: periodic` (once per
  period). What "previous" means depends on it, and `check` and `verify`
  refuse the binding of a document that reads `previous()` until it says.

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
