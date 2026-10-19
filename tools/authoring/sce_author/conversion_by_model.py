"""A conversion made by a model: what it is asked, in what pieces, and how its answer is taken (ADR 0036).

This is the kind of work "convert blocks", whole and without a model in it. A document that carries its
text is converted by rule (`conversion.convert_by_rules`) and needs none; a model is for what a rule
cannot read, and the first thing it has to do is be measurable. So everything that is the kind's is here
and is a pure function of its inputs: the unit of work, the recipe a model is told, the reading of what
it answered, and the putting back together. What is not here is any way of reaching a model. That is a
runtime's (a client, a server on the person's network), and a runtime knows nothing of this kind: it is
given a prompt and returns text.

**The unit is a run of blocks, never part of a block.** A whole document in one turn runs past a model's
time and context, and a retry should cost a piece, so the blocks are cut into units: at a page, when the
extraction knows pages, and when a unit has as many characters as it should hold. A block with no words
is not given to a model, as the converter by rule does not write one. A block is never cut: one longer
than a unit is a unit of its own.

**The model's answer is a claim, and the check is SCE's.** A model is told to write the same words and to
say which blocks each paragraph was made from; whether it did is `check_conversion`'s to say, as it is of
any conversion, and a model that summarised or corrected is found by the same words-for-words comparison
that holds the converter by rule to its own work. What is checked here is only that the answer can be
read: a unit whose answer cannot is a unit that failed, named by its number and a typed reason, and the
others are kept, so a failure costs one unit. The blocks of a unit that failed are then *missing* to the
check, which is what they are.

The text a model is given and the form it answers in are `pre-release`, as the conversion's form is.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from .conversion import Conversion, Converter, MarkdownBlock, plain_of_block
from .errors import IngestError
from .extraction import Block, Extraction

KIND = "convert-blocks"
RECIPE_VERSION = "1"

# How many characters of a document's words one unit holds, unless it is told another number. A model with
# a small context is told a smaller one.
UNIT_CHARACTERS = 6000

NO_ANSWER = "no-answer"
NOT_JSON = "not-json"
BAD_SHAPE = "bad-shape"
EMPTY = "empty"


@dataclass(frozen=True)
class Unit:
    """One piece of work: the blocks of an extraction a model is given at once."""

    n: int
    blocks: tuple[str, ...]


@dataclass(frozen=True)
class UnitRefused:
    """Why the answer to a unit could not be read, as an answer and not an error."""

    n: int
    code: str
    why: str


def _size(block: Block) -> int:
    return len(block.text) + sum(len(cell) for row in (block.cells or ()) for cell in row)


def split_units(extraction: Extraction, characters: int = UNIT_CHARACTERS) -> list[Unit]:
    """The extraction's blocks with words, in order, cut into units at a page and at a size."""
    if characters < 1:
        raise IngestError("a unit has to hold at least one character")
    units: list[Unit] = []
    held: list[str] = []
    held_size = 0
    page: int | None = None

    def close() -> None:
        nonlocal held, held_size
        if held:
            units.append(Unit(len(units) + 1, tuple(held)))
        held, held_size = [], 0

    for block in extraction.blocks:
        if not plain_of_block(block):
            continue
        block_page = block.place.page
        if held and block_page is not None and page is not None and block_page != page:
            close()
        size = _size(block)
        if held and held_size + size > characters:
            close()
        held.append(block.id)
        held_size += size
        if block_page is not None:
            page = block_page
    close()
    return units


RECIPE = f"""You convert the blocks of a document into Markdown. (recipe {KIND} {RECIPE_VERSION})

You are given blocks, each on its own line as `[id] kind: words`. A table is given as its rows, each row on
a line of its own beginning with `|`. Write the Markdown of these blocks, in their order.

Rules:
1. Say exactly the words of the blocks. Do not summarise, correct, translate, reorder, complete or add
   anything, and keep numbers, units and names as they are written.
2. Write one Markdown paragraph for each block. Join blocks only when they are the lines of one sentence.
3. A heading keeps its level with `#` marks. A list item begins with `- `. A table is a Markdown table made
   of its rows, with a `| --- |` row under the first. A mark such as `[picture: NAME]` is written as it is.
4. For each paragraph, say which blocks it was made from, by their ids.

Answer with a JSON array and nothing else, one object for each paragraph:
[{{"markdown": "# Title", "cites": ["b0001"]}}, {{"markdown": "Words of the next block.", "cites": ["b0002"]}}]
"""


def _listed(block: Block) -> str:
    if block.cells:
        rows = "\n".join("| " + " | ".join(row) + " |" for row in block.cells)
        return f"[{block.id}] table:\n{rows}"
    if block.kind == "heading":
        return f"[{block.id}] heading level {block.level or 1}: {block.text}"
    return f"[{block.id}] {block.kind}: {block.text}"


def prompt_for(extraction: Extraction, unit: Unit) -> str:
    """The whole text a model is given for `unit`: the recipe and the blocks."""
    by_id = {block.id: block for block in extraction.blocks}
    try:
        listed = "\n".join(_listed(by_id[block_id]) for block_id in unit.blocks)
    except KeyError as exc:
        raise IngestError(f"unit {unit.n} names block {exc.args[0]!r}, which this extraction does not have") from exc
    return f"{RECIPE}\nBlocks:\n{listed}\n"


def _first_array(text: str) -> Any:
    """The first JSON array in `text`, whatever surrounds it: a fence, words before it, words after it."""
    decoder = json.JSONDecoder()
    start = text.find("[")
    while start != -1:
        try:
            value, _ = decoder.raw_decode(text, start)
        except json.JSONDecodeError:
            start = text.find("[", start + 1)
            continue
        if isinstance(value, list):
            return value
        start = text.find("[", start + 1)
    raise ValueError("no JSON array")


def parse_answer(n: int, text: str | None) -> list[tuple[str, tuple[str, ...]]] | UnitRefused:
    """The paragraphs of a model's answer to unit `n`, as (Markdown, cited ids), or why they cannot be read."""
    if text is None or not text.strip():
        return UnitRefused(n, NO_ANSWER, "there is no answer to this unit")
    try:
        items = _first_array(text)
    except ValueError:
        return UnitRefused(n, NOT_JSON, "the answer holds no JSON array")
    if not items:
        return UnitRefused(n, EMPTY, "the answer is an empty list, and the unit has words")
    paragraphs: list[tuple[str, tuple[str, ...]]] = []
    for index, item in enumerate(items, start=1):
        if not isinstance(item, dict) or set(item) != {"markdown", "cites"}:
            return UnitRefused(n, BAD_SHAPE, f"item {index} is not an object with `markdown` and `cites`")
        markdown, cites = item["markdown"], item["cites"]
        if not isinstance(markdown, str) or not markdown.strip():
            return UnitRefused(n, BAD_SHAPE, f"item {index} has no words in `markdown`")
        if not isinstance(cites, list) or not all(isinstance(c, str) and c for c in cites):
            return UnitRefused(n, BAD_SHAPE, f"item {index} has `cites` that is not a list of block ids")
        if len(set(cites)) != len(cites):
            return UnitRefused(n, BAD_SHAPE, f"item {index} cites a block twice")
        paragraphs.append((markdown.strip("\n"), tuple(cites)))
    return paragraphs


def assemble_units(
    extraction: Extraction,
    answers: dict[int, list[tuple[str, tuple[str, ...]]] | UnitRefused],
    converter: Converter,
) -> tuple[Conversion, list[UnitRefused]]:
    """The conversion of the units that were answered, in order, and the units that were not.

    The blocks of a unit that failed are not in the conversion, and the check says so as *missing*. When no
    unit was answered there is nothing to convert, and that is refused as a document with nothing in it is.
    """
    blocks: list[MarkdownBlock] = []
    failed: list[UnitRefused] = []
    for n in sorted(answers):
        answer = answers[n]
        if isinstance(answer, UnitRefused):
            failed.append(answer)
            continue
        for markdown, cites in answer:
            blocks.append(MarkdownBlock(id=f"m{len(blocks) + 1:04d}", markdown=markdown, cites=cites))
    if not blocks:
        raise IngestError(
            f"no unit of the extraction was answered in a way that can be read ({len(failed)} failed), "
            "so there is nothing to convert"
        )
    return Conversion(converter, extraction.revision(), tuple(blocks)), failed
