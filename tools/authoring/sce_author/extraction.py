"""The form between an original document and its conversion: blocks, and what was not carried.

ADR 0026 holds a conversion to the original block by block, and ADR 0036 makes a format one extractor
among many. This module is the one thing every extractor writes and everything after it reads: an
`Extraction`. Only an extractor knows a format; the comparison, the review and the coverage count read
blocks and nothing else, so a new format is one extractor and no new screen.

Two rules are held here and not left to each extractor.

**What was not carried is part of the answer.** An extraction is its blocks and the list of what the
extractor could not carry (a table it could not recover, a page with no text, a picture it could not
place). The list is kept with the blocks, shown, and never dropped: a document that reads as though it
said nothing is the founding hazard of every reader in this package.

**Reading by recognition is never exact.** A block read from an image (`ocr`, `vision`) carries the
recogniser's confidence as a number, and a block read from the document's own text (`text-layer`)
carries none, because it is exact. An extraction that says otherwise is refused when it is made and when
it is read back, so a reader cannot report a guess as a fact.

The wire form is a `pre-release` surface (`SCE_WIRE_CONTRACTS.md`): `to_dict` and `from_dict` are its
only producer and reader. `from_dict` refuses a field it does not know, so a form written by a newer
tool is never half read.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from typing import Any

from .errors import IngestError

# The version of the wire form. A change that an old reader would misread bumps it.
SCHEMA_VERSION = 1

# Whether the wire form is settled. `SCE_WIRE_CONTRACTS.md` lists the same word for this surface.
SCHEMA_STATUS = "pre-release"

KINDS = ("heading", "paragraph", "list_item", "table", "figure", "attachment")
HOW_READ = ("text-layer", "ocr", "vision")
PRECISIONS = ("exact", "approximate", "none")


def _refuse(what: str) -> IngestError:
    return IngestError(f"an extraction {what}")


@dataclass(frozen=True)
class Place:
    """Where a block came from in its member of the original.

    A word-processor file or a page-less format is placed by `part` and `ordinal`; a page-layout
    document is placed by its `page` and, when the extractor knows one, a `box` on it. The page is
    required of a page-layout block and the box is not (ADR 0035): a reader chosen for how well it
    reads cannot also be required to say where. A box says how exact it is, and is `(left, top,
    right, bottom)` in points measured from the top-left corner of the page, the way a picture of
    the page is laid out, so that a reviewer's overlay needs no flip.
    """

    member: str
    part: str | None = None
    ordinal: int | None = None
    page: int | None = None
    box: tuple[float, float, float, float] | None = None
    precision: str = "none"

    def __post_init__(self) -> None:
        if not self.member:
            raise _refuse("has a place with no member")
        if self.precision not in PRECISIONS:
            raise _refuse(f"has a place whose precision {self.precision!r} is not one of {PRECISIONS}")
        if self.box is not None and self.precision == "none":
            raise _refuse("has a box that does not say how exact it is")
        if self.box is None and self.precision != "none":
            raise _refuse(f"says its box is {self.precision} and has none")
        if self.page is not None and self.page < 1:
            raise _refuse("has a page before the first")

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"member": self.member, "precision": self.precision}
        for key in ("part", "ordinal", "page"):
            value = getattr(self, key)
            if value is not None:
                out[key] = value
        if self.box is not None:
            out["box"] = list(self.box)
        return out

    @classmethod
    def from_dict(cls, raw: Any) -> "Place":
        data = _object(raw, "a place", {"member", "part", "ordinal", "page", "box", "precision"})
        box = data.get("box")
        if box is not None:
            if not isinstance(box, list) or len(box) != 4 or not all(_number(v) for v in box):
                raise _refuse("has a box that is not four numbers")
            box = tuple(float(v) for v in box)
        return cls(
            member=_text(data.get("member"), "a place's member"),
            part=_optional_text(data.get("part"), "a place's part"),
            ordinal=_optional_int(data.get("ordinal"), "a place's ordinal"),
            page=_optional_int(data.get("page"), "a place's page"),
            box=box,
            precision=data.get("precision", "none"),
        )


@dataclass(frozen=True)
class Block:
    """One piece of an original, in the order it sits.

    `text` is what a reader sees. A table also keeps its grid in `cells`, one row per tuple, every
    row as wide as the table, so a merged cell is written in each column it covers. A figure names its
    picture by `picture` (the hash of the member that holds it, ADR 0032) and the text is the label a
    person reads. `confidence` is `None` for what the document carries as text and a number between 0 and 1
    for what was recognised.
    """

    id: str
    kind: str
    text: str
    place: Place
    how_read: str = "text-layer"
    confidence: float | None = None
    level: int | None = None
    cells: tuple[tuple[str, ...], ...] | None = None
    picture: str | None = None

    def __post_init__(self) -> None:
        if not self.id:
            raise _refuse("has a block with no id")
        if self.kind not in KINDS:
            raise _refuse(f"has block {self.id!r} of kind {self.kind!r}, which is not one of {KINDS}")
        if self.how_read not in HOW_READ:
            raise _refuse(f"has block {self.id!r} read by {self.how_read!r}, which is not one of {HOW_READ}")
        if self.how_read == "text-layer":
            if self.confidence is not None:
                raise _refuse(f"gives block {self.id!r} a confidence, though its text is the document's own")
        else:
            if self.confidence is None:
                raise _refuse(f"reads block {self.id!r} by {self.how_read} without saying how sure it is")
            if not 0.0 <= self.confidence <= 1.0:
                raise _refuse(f"gives block {self.id!r} a confidence outside 0 to 1")
        if self.level is not None and self.kind != "heading":
            raise _refuse(f"gives block {self.id!r} a level, and only a heading has one")
        if self.cells is not None:
            if self.kind != "table":
                raise _refuse(f"gives block {self.id!r} a grid, and only a table has one")
            widths = {len(row) for row in self.cells}
            if len(widths) > 1:
                raise _refuse(f"has table {self.id!r} whose rows are not all as wide as the table")
        if self.picture is not None and self.kind != "figure":
            raise _refuse(f"gives block {self.id!r} a picture, and only a figure has one")

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "id": self.id,
            "kind": self.kind,
            "text": self.text,
            "place": self.place.to_dict(),
            "how_read": self.how_read,
        }
        if self.confidence is not None:
            out["confidence"] = self.confidence
        if self.level is not None:
            out["level"] = self.level
        if self.cells is not None:
            out["cells"] = [list(row) for row in self.cells]
        if self.picture is not None:
            out["picture"] = self.picture
        return out

    @classmethod
    def from_dict(cls, raw: Any) -> "Block":
        data = _object(
            raw, "a block", {"id", "kind", "text", "place", "how_read", "confidence", "level", "cells", "picture"}
        )
        cells = data.get("cells")
        if cells is not None:
            if not isinstance(cells, list) or not all(
                isinstance(row, list) and all(isinstance(cell, str) for cell in row) for row in cells
            ):
                raise _refuse("has a grid that is not rows of text")
            cells = tuple(tuple(row) for row in cells)
        confidence = data.get("confidence")
        if confidence is not None and not _number(confidence):
            raise _refuse("has a confidence that is not a number")
        return cls(
            id=_text(data.get("id"), "a block's id"),
            kind=_text(data.get("kind"), "a block's kind"),
            text=_text(data.get("text"), "a block's text", allow_empty=True),
            place=Place.from_dict(data.get("place")),
            how_read=data.get("how_read", "text-layer"),
            confidence=None if confidence is None else float(confidence),
            level=_optional_int(data.get("level"), "a block's level"),
            cells=cells,
            picture=_optional_text(data.get("picture"), "a block's picture"),
        )


@dataclass(frozen=True)
class NotCarried:
    """Something an extractor could not carry, said in a sentence a person can act on."""

    what: str
    why: str = ""
    count: int | None = None

    def __post_init__(self) -> None:
        if not self.what:
            raise _refuse("lists something not carried without saying what")
        if self.count is not None and self.count < 0:
            raise _refuse("counts a negative number of things not carried")

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"what": self.what}
        if self.why:
            out["why"] = self.why
        if self.count is not None:
            out["count"] = self.count
        return out

    @classmethod
    def from_dict(cls, raw: Any) -> "NotCarried":
        data = _object(raw, "an entry of what was not carried", {"what", "why", "count"})
        return cls(
            what=_text(data.get("what"), "what was not carried"),
            why=_text(data.get("why", ""), "why it was not carried", allow_empty=True),
            count=_optional_int(data.get("count"), "a count of what was not carried"),
        )


@dataclass(frozen=True)
class Extraction:
    """What one extractor made of one member of an original."""

    extractor: str
    extractor_version: str
    member: str
    blocks: tuple[Block, ...]
    not_carried: tuple[NotCarried, ...] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        if not self.extractor or not self.extractor_version:
            raise _refuse("does not say which extractor made it, and which version")
        if not self.member:
            raise _refuse("does not say which member of the original it is of")
        seen: set[str] = set()
        for block in self.blocks:
            if block.id in seen:
                raise _refuse(f"has two blocks with the id {block.id!r}")
            seen.add(block.id)

    @property
    def clean(self) -> bool:
        """Nothing was left behind: the one answer a caller may not assume."""
        return not self.not_carried

    def text(self) -> str:
        """The blocks as the text a reader of the original would read, one block to a paragraph."""
        return "\n".join(block.text for block in self.blocks if block.text) + "\n"

    def to_dict(self) -> dict[str, Any]:
        return {
            "v": SCHEMA_VERSION,
            "extractor": self.extractor,
            "extractor_version": self.extractor_version,
            "member": self.member,
            "blocks": [block.to_dict() for block in self.blocks],
            "not_carried": [entry.to_dict() for entry in self.not_carried],
        }

    def to_json(self) -> str:
        """The canonical text the extraction is named by: sorted keys, no spaces, UTF-8."""
        return json.dumps(self.to_dict(), sort_keys=True, separators=(",", ":"), ensure_ascii=False)

    def revision(self) -> str:
        """The hash an extraction is stored and cited by (ADR 0026)."""
        return hashlib.sha256(self.to_json().encode("utf-8")).hexdigest()

    @classmethod
    def from_json(cls, text: str) -> "Extraction":
        try:
            raw = json.loads(text)
        except ValueError as exc:
            raise _refuse(f"is not JSON ({exc})") from exc
        return cls.from_dict(raw)

    @classmethod
    def from_dict(cls, raw: Any) -> "Extraction":
        data = _object(raw, "an extraction", {"v", "extractor", "extractor_version", "member", "blocks", "not_carried"})
        if data.get("v") != SCHEMA_VERSION:
            raise _refuse(f"is of version {data.get('v')!r}; this tool reads version {SCHEMA_VERSION}")
        blocks = data.get("blocks")
        if not isinstance(blocks, list):
            raise _refuse("has no list of blocks")
        carried = data.get("not_carried", [])
        if not isinstance(carried, list):
            raise _refuse("has a list of what was not carried that is not a list")
        return cls(
            extractor=_text(data.get("extractor"), "the extractor's name"),
            extractor_version=_text(data.get("extractor_version"), "the extractor's version"),
            member=_text(data.get("member"), "the member"),
            blocks=tuple(Block.from_dict(block) for block in blocks),
            not_carried=tuple(NotCarried.from_dict(entry) for entry in carried),
        )


def _object(raw: Any, what: str, known: set[str]) -> dict[str, Any]:
    if not isinstance(raw, dict):
        raise _refuse(f"has {what} that is not an object")
    unknown = sorted(set(raw) - known)
    if unknown:
        raise _refuse(f"has {what} with a field this tool does not know: {', '.join(unknown)}")
    return raw


def _text(value: Any, what: str, allow_empty: bool = False) -> str:
    if not isinstance(value, str) or (not value and not allow_empty):
        raise _refuse(f"has {what} that is not {'text' if allow_empty else 'non-empty text'}")
    return value


def _optional_text(value: Any, what: str) -> str | None:
    return None if value is None else _text(value, what)


def _optional_int(value: Any, what: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise _refuse(f"has {what} that is not a whole number")
    return value


def _number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)
