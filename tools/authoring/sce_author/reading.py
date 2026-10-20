"""A picture read by a model: what it is asked, what it is handed, and how its answer joins a conversion.

This is the kind of work "read a picture" (ADR 0036), whole and without a model in it. A figure block of an
extraction holds only the `[picture: NAME]` mark of a picture the reader could not read, and the converter by
rule writes the mark as it is. When the owner asks for a picture to be read, a model that can see is handed
the picture and nothing else, and what it read is added to the conversion after the mark. Everything that is
the kind's is here and is a pure function of its inputs: which pictures can be read and why not, the recipe a
model is told, the form it answers in, the reading of that answer, and its place in the conversion. How a
model is reached, and how a picture travels to it, is a runtime's.

**The unit is one picture.** A figure block may mark more than one picture (two set side by side), so a unit
is a figure block and one NAME it marks. A picture is handed over as it is in the document, by its hash, or
refused by name: a format a model cannot be shown, one heavier than a client can hold, a mark with no name,
a figure outside the part the owner chose, a block that is not a figure.

**The model is given the picture alone.** No surrounding text is in the prompt, so nothing it writes can be
completed from the words beside the picture; it is told to copy what is legible exactly, to set a table as a
table and a drawing as what connects to what, and to say where it could not read rather than guess.

**What it read is a claim the owner decides.** The answer is read, not trusted: a reading that cannot be
read is a unit that failed, named with a typed reason, and the mark stays as it was. Every block that was
read is queued by the check as *read from a picture* (`conversion.READ_FROM_PICTURE`), for the owner to
keep, correct or drop, and the conversion keeps what was handed over and who read it (`conversion.Reading`).

The recipe, the answer's form and `units.json` are `pre-release`, as the conversion's form is.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
from dataclasses import dataclass
from typing import Any

from .conversion import Conversion, MarkdownBlock, Reading, Seen, in_scope
from .errors import IngestError, PictureTooLarge
from .extraction import Extraction
from .ingest import MAX_PICTURE_BYTES, read_pictures
from .scope import Scope

KIND = "read-picture"
RECIPE_VERSION = "1"
RECIPE_NAME = f"{KIND} {RECIPE_VERSION}"

# Why a picture is not handed over to be read.
NO_SUCH_BLOCK = "no-such-block"
NOT_A_FIGURE = "not-a-figure"
OUTSIDE_SCOPE = "outside-scope"
PICTURES_UNNAMED = "pictures-unnamed"
PICTURE_NOT_FOUND = "picture-not-found"
PICTURE_NOT_SHOWABLE = "picture-not-showable"
PICTURE_TOO_LARGE = "picture-too-large"
OTHER_PICTURE = "other-picture"

# Why an answer is not taken.
NO_ANSWER = "no-answer"
NOT_JSON = "not-json"
BAD_SHAPE = "bad-shape"
EMPTY = "empty"

_MARK = re.compile(r"\[picture: ([^\]]+)\]")
_UNNAMED = re.compile(r"\[picture\]|\[\d+ pictures\]")

BLOCK_KINDS = ("paragraph", "heading", "list", "table", "diagram")

# The form a model answers in: what a runtime that can be held to a schema is held to, and what
# `parse_answer` reads. Owned by this kind; a runtime only carries it.
ANSWER_SCHEMA: dict[str, Any] = {
    "type": "object",
    "additionalProperties": False,
    "required": ["blocks", "unreadable", "nothing_to_read"],
    "properties": {
        "blocks": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["kind", "markdown", "unsure"],
                "properties": {
                    "kind": {"type": "string", "enum": list(BLOCK_KINDS)},
                    "markdown": {"type": "string"},
                    "unsure": {"type": "boolean"},
                },
            },
        },
        "unreadable": {"type": "array", "items": {"type": "string"}},
        "nothing_to_read": {"type": "boolean"},
    },
}

RECIPE = f"""You read one picture from a specification and write what it says as Markdown. (recipe {RECIPE_NAME})

The picture is attached. You are given nothing else from the document, on purpose: write only what the
picture shows.

Rules:
1. Copy every legible word, number, unit and name exactly as it is drawn, in the language it is written in.
   Do not summarise, correct, translate, complete or explain anything.
2. A table is a Markdown table with a `| --- |` row under its first row. Text is a paragraph, a list is a
   list, a heading is a heading.
3. A drawing of boxes, states or parts and the lines between them is a list of what connects to what, each
   line naming both ends and the label drawn on the connection, followed by what each box holds.
4. Where you cannot read something, do not guess: name the place in `unreadable` and leave it out of the
   blocks. Mark a block `unsure` when you read it but are not certain of it.
5. A picture with nothing to read in it (a photograph, a logo) answers `nothing_to_read` with no blocks.

Answer only with the JSON object the answer's schema describes:
{{"blocks": [{{"kind": "paragraph", "markdown": "...", "unsure": false}}], "unreadable": [], "nothing_to_read": false}}
"""


@dataclass(frozen=True)
class Unit:
    """One picture to read: the figure block that marks it, and the picture as it is handed over, by hash."""

    n: int
    block: str
    picture: Seen

    def file_name(self) -> str:
        suffix = pathlib.PurePosixPath(self.picture.name).suffix.lower() or ".bin"
        return f"unit-{self.n:04d}{suffix}"

    def prompt_name(self) -> str:
        return f"unit-{self.n:04d}.prompt.txt"

    def answer_name(self) -> str:
        return f"unit-{self.n:04d}.answer.json"

    def to_dict(self) -> dict[str, Any]:
        return {
            "n": self.n,
            "block": self.block,
            "picture": self.picture.to_dict(),
            "prompt": self.prompt_name(),
            "file": self.file_name(),
            "answer": self.answer_name(),
        }

    @classmethod
    def from_dict(cls, raw: Any) -> "Unit":
        if not isinstance(raw, dict):
            raise IngestError("a unit of a reading is not an object")
        try:
            return cls(n=int(raw["n"]), block=str(raw["block"]), picture=Seen.from_dict(raw["picture"]))
        except (KeyError, TypeError, ValueError) as exc:
            raise IngestError(f"a unit of a reading does not say which picture it is ({exc})") from exc


@dataclass(frozen=True)
class Handed:
    """A unit and the picture's bytes, as they are taken out of the document to be handed over."""

    unit: Unit
    data: bytes


@dataclass(frozen=True)
class Refused:
    """A picture that is not handed over, or an answer that is not taken: which, with a typed reason."""

    block: str
    code: str
    why: str
    n: int | None = None
    picture: str | None = None

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"block": self.block, "code": self.code, "why": self.why}
        if self.n is not None:
            out["n"] = self.n
        if self.picture is not None:
            out["picture"] = self.picture
        return out


@dataclass(frozen=True)
class ReadBlock:
    kind: str
    markdown: str
    unsure: bool


@dataclass(frozen=True)
class Answer:
    """What a model said it read of one picture."""

    blocks: tuple[ReadBlock, ...]
    unreadable: tuple[str, ...]
    nothing_to_read: bool


def units_for(
    extraction: Extraction, block_ids: list[str], document: pathlib.Path, scope: Scope | None = None,
    limit: int = MAX_PICTURE_BYTES,
) -> tuple[list[Handed], list[Refused]]:
    """The pictures the figure blocks `block_ids` mark, each as a unit, and the ones that cannot be read.

    `document` is the original's file the extraction was read from: the pictures are taken from it as they
    are. A block outside `scope` is refused, since what the owner did not choose is never handed to anything
    (ADR 0039). A block named twice is read once. A document that cannot be read is refused whole, as it is
    by `read_pictures`; a picture too heavy is refused alone.
    """
    scope = in_scope(extraction, scope)
    by_id = {block.id: block for block in extraction.blocks}
    units: list[Handed] = []
    refused: list[Refused] = []
    for block_id in dict.fromkeys(block_ids):
        block = by_id.get(block_id)
        if block is None:
            refused.append(Refused(block_id, NO_SUCH_BLOCK, "the extraction has no block by this id"))
            continue
        if block.kind != "figure":
            refused.append(Refused(block_id, NOT_A_FIGURE, f"it is a {block.kind}, not a figure that marks a picture"))
            continue
        if scope is not None and not scope.covers(block_id):
            refused.append(Refused(block_id, OUTSIDE_SCOPE, "it is outside the part of the document chosen"))
            continue
        if _UNNAMED.search(block.text):
            refused.append(Refused(block_id, PICTURES_UNNAMED, "it marks a picture the reader could not name"))
            continue
        for name in _MARK.findall(block.text):
            try:
                found = read_pictures([document], name, limit)
            except PictureTooLarge as exc:
                refused.append(Refused(block_id, PICTURE_TOO_LARGE, str(exc), picture=name))
                continue
            if not found:
                refused.append(Refused(block_id, PICTURE_NOT_FOUND, "the document holds no picture by this name",
                                       picture=name))
                continue
            picture = found[0]
            if picture.mime is None:
                refused.append(Refused(block_id, PICTURE_NOT_SHOWABLE, "its format cannot be shown to a model",
                                       picture=name))
                continue
            seen = Seen(name, hashlib.sha256(picture.data).hexdigest(), picture.mime)
            if block.picture is not None and block.picture != seen.sha256:
                # The extraction names the picture it read by hash (ADR 0032): a document whose picture of
                # that name is another one is not the document the extraction was read from.
                refused.append(Refused(block_id, OTHER_PICTURE, "the document's picture by this name is not the "
                                       "one the extraction names", picture=name))
                continue
            units.append(Handed(Unit(n=len(units) + 1, block=block_id, picture=seen), picture.data))
    return units, refused


def prompt_for(unit: Unit) -> str:
    """The whole text a model is given with the picture: the recipe and the picture's label, nothing more."""
    return f"{RECIPE}\nThe picture's name in the document: {unit.picture.name}\n"


def parse_answer(n: int, block: str, text: str | None) -> Answer | Refused:
    """What a model answered for unit `n`, or why it cannot be taken."""
    if text is None or not text.strip():
        return Refused(block, NO_ANSWER, "there is no answer to this picture", n=n)
    try:
        raw = json.loads(text)
    except ValueError:
        return Refused(block, NOT_JSON, "the answer is not a JSON object", n=n)
    if not isinstance(raw, dict) or set(raw) != {"blocks", "unreadable", "nothing_to_read"}:
        return Refused(block, BAD_SHAPE, "the answer is not an object of `blocks`, `unreadable`, `nothing_to_read`",
                       n=n)
    blocks, unreadable, nothing = raw["blocks"], raw["unreadable"], raw["nothing_to_read"]
    if not isinstance(nothing, bool):
        return Refused(block, BAD_SHAPE, "`nothing_to_read` is not true or false", n=n)
    if not isinstance(unreadable, list) or not all(isinstance(u, str) and u.strip() for u in unreadable):
        return Refused(block, BAD_SHAPE, "`unreadable` is not a list of places", n=n)
    if not isinstance(blocks, list):
        return Refused(block, BAD_SHAPE, "`blocks` is not a list", n=n)
    read: list[ReadBlock] = []
    for index, item in enumerate(blocks, start=1):
        if not isinstance(item, dict) or set(item) != {"kind", "markdown", "unsure"}:
            return Refused(block, BAD_SHAPE, f"block {index} is not an object of `kind`, `markdown`, `unsure`", n=n)
        if item["kind"] not in BLOCK_KINDS:
            return Refused(block, BAD_SHAPE, f"block {index} is of a kind the answer does not have", n=n)
        if not isinstance(item["markdown"], str) or not item["markdown"].strip():
            return Refused(block, BAD_SHAPE, f"block {index} has no words", n=n)
        if not isinstance(item["unsure"], bool):
            return Refused(block, BAD_SHAPE, f"block {index} has an `unsure` that is not true or false", n=n)
        read.append(ReadBlock(item["kind"], item["markdown"].strip("\n"), item["unsure"]))
    if nothing and read:
        return Refused(block, BAD_SHAPE, "the answer says there is nothing to read and reads something", n=n)
    if not nothing and not read and not unreadable:
        return Refused(block, EMPTY, "the answer reads nothing, and does not say there is nothing to read", n=n)
    return Answer(tuple(read), tuple(u.strip() for u in unreadable), nothing)


@dataclass(frozen=True)
class Runtime:
    """Who read: the runtime a host ran, its version, and the model it reported."""

    name: str
    version: str
    model: str


def with_readings(
    conversion: Conversion, units: list[Unit], answers: dict[int, Answer], runtime: Runtime,
) -> Conversion:
    """`conversion` with what was read of each picture added after the block that writes its mark.

    A reading block is named after its figure block (`b0012-r1`, ...), so the ids the converter by rule gave
    stay what they were and a decision taken of one of them is a decision of the same block. A unit with no
    answer adds nothing: its mark stays as it was. A picture that was read is refused when no block of the
    conversion writes its figure, since a reading of something the conversion does not carry has no place.
    """
    by_n = {unit.n: unit for unit in units}
    added: dict[str, list[MarkdownBlock]] = {}
    readings = list(conversion.readings)
    counts: dict[str, int] = {}
    for n in sorted(answers):
        unit = by_n.get(n)
        if unit is None:
            raise IngestError(f"there is an answer to unit {n}, and no unit {n} was prepared")
        answer = answers[n]
        readings.append(
            Reading(
                block=unit.block,
                picture=unit.picture,
                recipe=RECIPE_NAME,
                runtime=runtime.name,
                runtime_version=runtime.version,
                model=runtime.model,
                unreadable=answer.unreadable,
                nothing_to_read=answer.nothing_to_read,
            )
        )
        for read in answer.blocks:
            counts[unit.block] = counts.get(unit.block, 0) + 1
            added.setdefault(unit.block, []).append(
                MarkdownBlock(
                    id=f"{unit.block}-r{counts[unit.block]}",
                    markdown=read.markdown,
                    cites=(unit.block,),
                    picture=unit.picture.name,
                    unsure=read.unsure,
                )
            )
    last_writing: dict[str, int] = {}
    for index, block in enumerate(conversion.blocks):
        for cite in block.cites:
            last_writing[cite] = index
    homeless = sorted(figure for figure in added if figure not in last_writing)
    if homeless:
        raise IngestError(
            f"no block of the conversion writes {', '.join(homeless)}, so what was read of it has no place"
        )
    blocks: list[MarkdownBlock] = []
    for index, block in enumerate(conversion.blocks):
        blocks.append(block)
        for figure, reads in added.items():
            if last_writing[figure] == index:
                blocks.extend(reads)
    return Conversion(conversion.converter, conversion.extraction, tuple(blocks), tuple(readings))
