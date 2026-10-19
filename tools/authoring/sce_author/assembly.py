"""The text a review ends in: a conversion with the owner's decisions applied (ADR 0037).

A review does not change the conversion. The conversion, the check of it and every decision are kept as
they were made (ADR 0025), and the text the owner accepts is made from them here, once, when they accept:
the same three inputs always give the same text. The caller says how many decisions it assembled from, and
the specs folder refuses the text when more were taken since, so a text is never saved as if it answered a
log it did not read.

**What each decision does.** The latest decision on a block stands; the earlier ones are history.

- `keep`: the block is written as the conversion has it.
- `correct`: the owner's words stand where the block was.
- `drop`: the block is left out. Of a block of the original that the conversion left out, it says the
  owner leaves it out, which writes nothing and answers the question that was asked about it.
- `add`: a block of the original that the conversion left out is written as the rules writer would write
  it, after the block that carries the nearest earlier block of the original (at the start when none
  does). Two added blocks keep the order they have in the original.

A block nobody decided about is written as the conversion has it. Whether every question the check asked
has a decision is the specs folder's to judge at accept, not this module's: it holds the questions.

**What the text changes.** Against the spec's current text the assembly says what a reader of the spec
will see change, as lines added and removed in hunks, and not how the conversion compares with the
original: the owner is shown what the spec becomes (ADR 0025).

The text is not marked or annotated: it is the Markdown a reader reads, one block to a paragraph, exactly
as `Conversion.markdown` writes a conversion. The wire form of the changes is a `pre-release` surface.
"""

from __future__ import annotations

import difflib
import hashlib
import json
from dataclasses import dataclass
from typing import Any

from .conversion import Conversion, markdown_of_block, plain_of_block
from .errors import IngestError
from .extraction import Extraction

DECISION_KINDS = ("keep", "correct", "drop", "add")
CONTEXT_LINES = 2


def _refuse(what: str) -> IngestError:
    return IngestError(f"an assembly {what}")


@dataclass(frozen=True)
class Decision:
    """One line of a proposal's log, as the specs folder gives it."""

    n: int
    block: str
    what: str
    text: str | None = None

    @classmethod
    def from_dict(cls, raw: Any) -> "Decision":
        if not isinstance(raw, dict):
            raise _refuse("has a decision that is not an object")
        n, block, what, text = raw.get("n"), raw.get("block"), raw.get("what"), raw.get("text")
        if not isinstance(n, int) or isinstance(n, bool) or n < 1:
            raise _refuse("has a decision with no place in the log")
        if not isinstance(block, str) or not block:
            raise _refuse(f"has decision {n} naming no block")
        if what not in DECISION_KINDS:
            raise _refuse(f"has decision {n} of a kind it does not know ({what!r})")
        if what == "correct":
            if not isinstance(text, str) or not text.strip():
                raise _refuse(f"has decision {n} correcting block {block!r} with no words")
        elif text is not None:
            raise _refuse(f"has decision {n} carrying words, which only a correction does")
        return cls(n, block, what, text)


def decisions_from_json(text: str) -> list[Decision]:
    """The log as `read_proposal` gives it: a JSON array of decisions, in the order they were taken."""
    try:
        raw = json.loads(text)
    except json.JSONDecodeError as exc:
        raise _refuse(f"was given decisions that are not JSON ({exc})") from exc
    if not isinstance(raw, list):
        raise _refuse("was given decisions that are not a list")
    return [Decision.from_dict(item) for item in raw]


def standing(decisions: list[Decision]) -> list[Decision]:
    """The latest decision on each block, in the order the blocks were first decided."""
    latest: dict[str, Decision] = {}
    for decision in decisions:
        latest.pop(decision.block, None)
        latest[decision.block] = decision
    first_seen: list[str] = []
    for decision in decisions:
        if decision.block not in first_seen:
            first_seen.append(decision.block)
    return [latest[block] for block in first_seen]


@dataclass(frozen=True)
class Assembly:
    """The text, and what was done to make it."""

    text: str
    decisions: int
    kept: tuple[str, ...]
    corrected: tuple[str, ...]
    dropped: tuple[str, ...]
    added: tuple[str, ...]

    def revision(self) -> str:
        """The name the specs folder keeps this text under."""
        return hashlib.sha256(self.text.encode("utf-8")).hexdigest()


def assemble(conversion: Conversion, extraction: Extraction, decisions: list[Decision]) -> Assembly:
    """The Markdown of `conversion` with `decisions` applied, and nothing of the original added unasked."""
    if conversion.extraction != extraction.revision():
        raise _refuse(
            f"was given a conversion of the extraction {conversion.extraction[:12]}, not of this one "
            f"({extraction.revision()[:12]})"
        )
    in_conversion = {block.id: block for block in conversion.blocks}
    in_original = {block.id: block for block in extraction.blocks}
    carried_by: dict[str, str] = {}
    for block in conversion.blocks:
        for cite in block.cites:
            carried_by[cite] = block.id

    decided = standing(decisions)
    for decision in decided:
        if decision.what == "add":
            if decision.block in in_conversion or decision.block not in in_original:
                raise _refuse(f"cannot add {decision.block!r}: it is not a block of the original the conversion left out")
            if decision.block in carried_by:
                raise _refuse(
                    f"cannot add {decision.block!r}: the conversion already carries it in {carried_by[decision.block]!r}"
                )
            if not plain_of_block(in_original[decision.block]):
                raise _refuse(f"cannot add {decision.block!r}: it has no words to add")
        elif decision.block in in_conversion:
            continue
        elif decision.what == "drop" and decision.block in carried_by:
            raise _refuse(
                f"has decision {decision.n} (drop) on {decision.block!r}, which the conversion carries in "
                f"{carried_by[decision.block]!r}: decide on that block"
            )
        elif decision.what == "drop" and decision.block in in_original:
            # The owner leaves a block of the original out that the conversion already left out: there is
            # nothing to write, and the decision is the answer to the question that was asked about it.
            continue
        else:
            raise _refuse(
                f"has decision {decision.n} ({decision.what}) on {decision.block!r}, which is no block of the "
                f"conversion"
                + (" (only an add or a drop says what becomes of a block of the original)" if decision.block in in_original else "")
            )
    by_block = {decision.block: decision for decision in decided}

    # Where each added block goes: after the conversion block that carries the nearest earlier block of
    # the original, or first when none does. They are placed in the order the original has them.
    order = {block.id: index for index, block in enumerate(extraction.blocks)}
    after: dict[str | None, list[str]] = {}
    for decision in sorted((d for d in decided if d.what == "add"), key=lambda d: order[d.block]):
        anchor: str | None = None
        for earlier in reversed(extraction.blocks[: order[decision.block]]):
            if earlier.id in carried_by:
                anchor = carried_by[earlier.id]
                break
        after.setdefault(anchor, []).append(decision.block)

    paragraphs: list[str] = []
    kept: list[str] = []
    corrected: list[str] = []
    dropped: list[str] = []
    added: list[str] = []

    def put_added(anchor: str | None) -> None:
        for original_id in after.get(anchor, []):
            paragraphs.append(markdown_of_block(in_original[original_id]))
            added.append(original_id)

    put_added(None)
    for block in conversion.blocks:
        decision = by_block.get(block.id)
        if decision is not None and decision.what == "drop":
            dropped.append(block.id)
        elif decision is not None and decision.what == "correct":
            assert decision.text is not None
            paragraphs.append(decision.text.strip())
            corrected.append(block.id)
        else:
            paragraphs.append(block.markdown)
            kept.append(block.id)
        put_added(block.id)
    # What the owner left out of a block of the original the conversion had left out is dropped too, and
    # said so, since the question that was asked about it was answered.
    dropped.extend(d.block for d in decided if d.what == "drop" and d.block not in in_conversion)
    if not paragraphs:
        raise _refuse("would leave nothing: every block of the conversion was dropped")
    return Assembly(
        text="\n\n".join(paragraphs) + "\n",
        decisions=len(decisions),
        kept=tuple(kept),
        corrected=tuple(corrected),
        dropped=tuple(dropped),
        added=tuple(added),
    )


def changes(current: str | None, text: str) -> dict[str, Any]:
    """What `text` changes in `current` (the spec's text now; `None` for a spec with no text), by line."""
    old = (current or "").splitlines()
    new = text.splitlines()
    matcher = difflib.SequenceMatcher(a=old, b=new, autojunk=False)
    hunks: list[dict[str, Any]] = []
    added = removed = 0
    for group in matcher.get_grouped_opcodes(CONTEXT_LINES):
        lines: list[dict[str, str]] = []
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                lines.extend({"op": "keep", "text": line} for line in old[i1:i2])
                continue
            lines.extend({"op": "remove", "text": line} for line in old[i1:i2])
            lines.extend({"op": "add", "text": line} for line in new[j1:j2])
            removed += i2 - i1
            added += j2 - j1
        hunks.append({"old_start": group[0][1] + 1, "new_start": group[0][3] + 1, "lines": lines})
    return {
        "had_text": current is not None,
        "unchanged": added == 0 and removed == 0,
        "added": added,
        "removed": removed,
        "hunks": hunks,
    }


def report_of(made: Assembly, current: str | None) -> dict[str, Any]:
    """What the caller needs beside the text: its name, the count it was made from, and what it changes.

    The text itself is not in it: it is written to a file, and a report is ids and counts.
    """
    return {
        "revision": made.revision(),
        "decisions": made.decisions,
        "kept": list(made.kept),
        "corrected": list(made.corrected),
        "dropped": list(made.dropped),
        "added": list(made.added),
        "changes": changes(current, made.text),
    }
