"""The scope of an import: which blocks of an extraction the owner chose (ADR 0039).

An import is of a whole document unless the owner says otherwise, and "otherwise" has to be said in a way
that the cut, the check and the record can all hold to without anyone cutting the document again. A scope
is that: a small document naming the extraction it is of and the ids of the blocks in it. It is the owner's
choice and an input to everything that reads blocks; it is never a field of a conversion, whose maker would
then decide what counts as missing (ADR 0026).

**The tools turn a choice into a scope.** `outline` lists the sections of an extraction and `scope_of`
takes the ids of sections and gives the scope, so that a host or a screen shows sections and sends ids and
never works out which blocks a section holds.

**What a section is.** The headings of the document, nested by level, with a first section for what comes
before the first heading. A document with no heading is divided into its pages, and one with neither is one
section. A section is its own blocks: the ones from its heading to the next heading of any level. The
sections under it are sections of their own and are chosen as such, so that what is chosen is exactly what
was named and a part under a chosen one can be left out. `tree` names a section with everything under it,
for a caller that wants the whole of a function without listing its parts.

The scope's form is `pre-release`: `to_dict` and `from_dict` are its only producer and reader.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from typing import Any

from .conversion import plain_of_block
from .errors import IngestError
from .extraction import Block, Extraction

SCHEMA_VERSION = 1
SCHEMA_STATUS = "pre-release"

BY_HEADINGS = "headings"
BY_PAGES = "pages"
BY_WHOLE = "whole"
OUTLINES = (BY_HEADINGS, BY_PAGES, BY_WHOLE)

# A title is the words of a heading, kept to what a list can show.
TITLE_CHARACTERS = 200


def _refuse(what: str) -> IngestError:
    return IngestError(f"a scope {what}")


@dataclass(frozen=True)
class Scope:
    """The blocks of one extraction an import is of, by id, in the order the extraction has them."""

    extraction: str
    blocks: tuple[str, ...]

    def __post_init__(self) -> None:
        if not re.fullmatch(r"[0-9a-f]{64}", self.extraction):
            raise _refuse("does not name the extraction it is of by its revision (64 hexadecimal digits)")
        if not self.blocks:
            raise _refuse("holds no block: an import of nothing is not an import")
        if any(not block for block in self.blocks):
            raise _refuse("names a block with no id")
        if len(set(self.blocks)) != len(self.blocks):
            raise _refuse("names a block twice")

    def covers(self, block_id: str) -> bool:
        return block_id in self.blocks

    def to_dict(self) -> dict[str, Any]:
        return {"v": SCHEMA_VERSION, "extraction": self.extraction, "blocks": list(self.blocks)}

    def to_json(self) -> str:
        """The canonical text a scope is named by: sorted keys, no spaces, UTF-8."""
        return json.dumps(self.to_dict(), sort_keys=True, separators=(",", ":"), ensure_ascii=False)

    def revision(self) -> str:
        return hashlib.sha256(self.to_json().encode("utf-8")).hexdigest()

    @classmethod
    def from_dict(cls, raw: Any) -> "Scope":
        if not isinstance(raw, dict):
            raise _refuse("is not a JSON object")
        unknown = sorted(set(raw) - {"v", "extraction", "blocks"})
        if unknown:
            raise _refuse(f"has a field nobody defined ({', '.join(unknown)})")
        if raw.get("v") != SCHEMA_VERSION:
            raise _refuse(f"is of a version this tool does not read (it reads {SCHEMA_VERSION})")
        blocks = raw.get("blocks")
        if not isinstance(blocks, list) or not all(isinstance(b, str) for b in blocks):
            raise _refuse("does not hold its blocks as a list of ids")
        extraction = raw.get("extraction")
        if not isinstance(extraction, str):
            raise _refuse("does not name the extraction it is of")
        return cls(extraction, tuple(blocks))

    @classmethod
    def from_json(cls, text: str) -> "Scope":
        try:
            raw = json.loads(text)
        except json.JSONDecodeError as exc:
            raise _refuse(f"is not JSON ({exc})") from exc
        return cls.from_dict(raw)

    def held_by(self, extraction: Extraction) -> "Scope":
        """This scope, once it is known to be of `extraction`: its blocks are there, in its order."""
        if self.extraction != extraction.revision():
            raise IngestError(
                f"the scope is of the extraction {self.extraction[:12]}, not of this one "
                f"({extraction.revision()[:12]}): a choice of blocks is not carried to another document"
            )
        order = {block.id: index for index, block in enumerate(extraction.blocks)}
        missing = [block for block in self.blocks if block not in order]
        if missing:
            raise IngestError(f"the scope names {', '.join(missing[:5])}, which the extraction does not have")
        places = [order[block] for block in self.blocks]
        if places != sorted(places):
            raise IngestError("the scope's blocks are not in the order the extraction has them")
        return self


def whole(extraction: Extraction) -> Scope:
    """The scope of everything: what an import is of when the owner chose nothing."""
    if not extraction.blocks:
        raise IngestError("the extraction has no block, so there is nothing to be the scope of")
    return Scope(extraction.revision(), tuple(block.id for block in extraction.blocks))


@dataclass(frozen=True)
class Section:
    """One part of a document the owner can choose, with what a person needs to see to choose it."""

    id: str
    kind: str  # "preamble" | "heading" | "page" | "whole"
    title: str
    level: int
    parent: str | None
    blocks: int  # its own blocks with words: from its heading to the next heading of any level
    characters: int
    tree_blocks: int  # the same with the sections under it
    tree_characters: int
    page_first: int | None  # the pages of its own blocks
    page_last: int | None
    first: int  # where it starts in the extraction's blocks, where its own blocks end, and where the
    end: int  # sections under it end (all exclusive)
    tree_end: int

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "kind": self.kind,
            "title": self.title,
            "level": self.level,
            "parent": self.parent,
            "blocks": self.blocks,
            "characters": self.characters,
            "tree_blocks": self.tree_blocks,
            "tree_characters": self.tree_characters,
            "page_first": self.page_first,
            "page_last": self.page_last,
        }


def _measure(extraction: Extraction, first: int, end: int) -> tuple[int, int, int | None, int | None]:
    blocks = extraction.blocks[first:end]
    worded = [b for b in blocks if plain_of_block(b)]
    pages = [b.place.page for b in blocks if b.place.page is not None]
    return (
        len(worded),
        sum(len(plain_of_block(b)) for b in worded),
        min(pages) if pages else None,
        max(pages) if pages else None,
    )


def _title(block: Block) -> str:
    words = " ".join(block.text.split())
    return words if len(words) <= TITLE_CHARACTERS else words[: TITLE_CHARACTERS - 1] + "…"


def _by_headings(extraction: Extraction) -> list[Section]:
    blocks = extraction.blocks
    heads = [i for i, b in enumerate(blocks) if b.kind == "heading"]
    starts: list[tuple[int, str, int]] = []  # (index, kind, level)
    if heads and heads[0] > 0 and any(plain_of_block(b) for b in blocks[: heads[0]]):
        starts.append((0, "preamble", 1))
    starts.extend((i, "heading", blocks[i].level or 1) for i in heads)
    sections: list[Section] = []
    stack: list[tuple[int, str]] = []  # (level, id) of the sections still open
    for position, (index, kind, level) in enumerate(starts):
        # Its own blocks end at the next heading of any level. The sections under it end where the next
        # heading of its level or a higher one begins; what comes before the first heading is under none,
        # so it ends at the first heading, of whatever level that is.
        end = starts[position + 1][0] if position + 1 < len(starts) else len(blocks)
        tree_end = len(blocks)
        for later_index, later_kind, later_level in starts[position + 1 :]:
            if later_kind == "heading" and (kind == "preamble" or later_level <= level):
                tree_end = later_index
                break
        while stack and stack[-1][0] >= level:
            stack.pop()
        identifier = f"s{len(sections) + 1}"
        parent = stack[-1][1] if stack and kind == "heading" else None
        count, characters, page_first, page_last = _measure(extraction, index, end)
        tree_count, tree_characters, _, _ = _measure(extraction, index, tree_end)
        sections.append(
            Section(
                id=identifier,
                kind=kind,
                title=_title(blocks[index]) if kind == "heading" else "",
                level=level if kind == "heading" else 1,
                parent=parent,
                blocks=count,
                characters=characters,
                tree_blocks=tree_count,
                tree_characters=tree_characters,
                page_first=page_first,
                page_last=page_last,
                first=index,
                end=end,
                tree_end=tree_end,
            )
        )
        if kind == "heading":
            stack.append((level, identifier))
    return sections


def _by_pages(extraction: Extraction) -> list[Section]:
    blocks = extraction.blocks
    runs: list[tuple[int, int, int | None]] = []  # (first, end, page)
    for index, block in enumerate(blocks):
        page = block.place.page
        if runs and (page is None or page == runs[-1][2]):
            runs[-1] = (runs[-1][0], index + 1, runs[-1][2])
        else:
            runs.append((index, index + 1, page))
    sections: list[Section] = []
    for first, end, _ in runs:
        count, characters, page_first, page_last = _measure(extraction, first, end)
        sections.append(
            Section(
                f"s{len(sections) + 1}", "page", "", 1, None,
                count, characters, count, characters, page_first, page_last, first, end, end,
            )
        )
    return sections


def _whole(extraction: Extraction) -> list[Section]:
    count, characters, page_first, page_last = _measure(extraction, 0, len(extraction.blocks))
    last = len(extraction.blocks)
    return [Section("s1", "whole", "", 1, None, count, characters, count, characters, page_first, page_last, 0, last, last)]


def chosen_outline(extraction: Extraction, by: str | None = None) -> str:
    """Which way the extraction is divided: as asked, else by its headings, else its pages, else not at all."""
    if by is not None:
        if by not in OUTLINES:
            raise IngestError(f"an outline is by one of {', '.join(OUTLINES)}, not by {by!r}")
        return by
    if any(b.kind == "heading" for b in extraction.blocks):
        return BY_HEADINGS
    if any(b.place.page is not None for b in extraction.blocks):
        return BY_PAGES
    return BY_WHOLE


def outline(extraction: Extraction, by: str | None = None) -> tuple[str, list[Section]]:
    """The sections of an extraction, in the order they begin, and the way they were found."""
    if not extraction.blocks:
        raise IngestError("the extraction has no block, so it has no sections")
    way = chosen_outline(extraction, by)
    if way == BY_HEADINGS:
        sections = _by_headings(extraction)
        if not sections:
            raise IngestError("the extraction has no heading, so it cannot be divided by its headings")
    elif way == BY_PAGES:
        if not any(b.place.page is not None for b in extraction.blocks):
            raise IngestError("the extraction has no page, so it cannot be divided by its pages")
        sections = _by_pages(extraction)
    else:
        sections = _whole(extraction)
    return way, sections


def scope_of(
    extraction: Extraction, section_ids: list[str], by: str | None = None, tree_ids: list[str] | None = None
) -> Scope:
    """The scope of the sections with these ids, each as its own blocks, in the extraction's order.

    `tree_ids` are sections taken with everything under them. A section named both ways is taken once.
    """
    tree_ids = tree_ids or []
    if not section_ids and not tree_ids:
        raise IngestError("nothing is chosen: an import of nothing is not an import")
    _, sections = outline(extraction, by)
    known = {s.id: s for s in sections}
    unknown = [i for i in [*section_ids, *tree_ids] if i not in known]
    if unknown:
        raise IngestError(f"the extraction has no section {', '.join(unknown)}")
    chosen: set[int] = set()
    for identifier in section_ids:
        chosen.update(range(known[identifier].first, known[identifier].end))
    for identifier in tree_ids:
        chosen.update(range(known[identifier].first, known[identifier].tree_end))
    ids = tuple(extraction.blocks[i].id for i in sorted(chosen))
    return Scope(extraction.revision(), ids)
