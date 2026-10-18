"""A conversion: Markdown that cites the blocks of an extraction it was made from, and SCE's check of it.

ADR 0026 holds a conversion to its original block by block, and does not take its maker's word for how
much of the original it carries. This module is the form of a conversion, the check that is SCE's and
not the maker's, and the converter by rule for an original that carries its text.

**A conversion cites; SCE checks the citation.** Each Markdown block names the blocks of the extraction
it was made from (`cites`). The check compares the normalised text of the Markdown block with the
normalised text of the blocks it cites, in the order it cites them. A block whose text the citation does
not support is *check*, whatever its maker said; a block that cites nothing is *not in the original*; a
block of the original that no Markdown block cites is *missing*. The counts are the check's, and the
form has no field in which a conversion could say it is complete: `from_dict` refuses a field it does not
know, so a maker that adds `"complete": true` is refused and not believed.

**What is compared is the words, not the markup.** Heading marks, list markers (bullets of a PDF
included), table pipes and the separator row, and backslash escapes are taken off both sides. A table is
compared by its cells. Nothing else is forgiven: a letter, a syllable or a word different is a block to
check, which is the failure of a recogniser that replaces a character by a look-alike.

**The converter by rule is the check's control.** An original that carries its text (Markdown, HTML, a
word file, a PDF with a text layer) needs no model: `convert_by_rules` writes each block as Markdown and
cites it. A conversion it makes must match every block, and a test holds that it does; so a check that
cannot tell the converter's work from a damaged copy of it is a check that does nothing.

**Reading by recognition is queued for a person first.** A block read from an image is never exact
(ADR 0026), so the check lists the cited blocks that were read by `ocr` or `vision`, and the review queue
asks about those whatever the text says.

The wire form is a `pre-release` surface: `to_dict` and `from_dict` are its only producer and reader.
"""

from __future__ import annotations

import hashlib
import json
import re
import unicodedata
from dataclasses import dataclass, field
from typing import Any

from .errors import IngestError
from .extraction import Block, Extraction

SCHEMA_VERSION = 1
SCHEMA_STATUS = "pre-release"

CONVERTER_KINDS = ("rules", "model")

MATCHED = "matched"
CHECK = "check"
NOT_IN_ORIGINAL = "not-in-original"


def _refuse(what: str) -> IngestError:
    return IngestError(f"a conversion {what}")


@dataclass(frozen=True)
class MarkdownBlock:
    """One block of the Markdown, and the blocks of the extraction it says it was made from."""

    id: str
    markdown: str
    cites: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        if not self.id:
            raise _refuse("has a block with no id")
        if len(set(self.cites)) != len(self.cites):
            raise _refuse(f"has block {self.id!r} citing one block twice")
        if any(not cite for cite in self.cites):
            raise _refuse(f"has block {self.id!r} citing a block with no id")

    def to_dict(self) -> dict[str, Any]:
        return {"id": self.id, "markdown": self.markdown, "cites": list(self.cites)}

    @classmethod
    def from_dict(cls, raw: Any) -> "MarkdownBlock":
        data = _object(raw, "a block", {"id", "markdown", "cites"})
        cites = data.get("cites", [])
        if not isinstance(cites, list) or not all(isinstance(c, str) for c in cites):
            raise _refuse("has block cites that are not a list of block ids")
        return cls(
            id=_text(data.get("id"), "a block's id"),
            markdown=_text(data.get("markdown"), "a block's markdown", allow_empty=True),
            cites=tuple(cites),
        )


@dataclass(frozen=True)
class Converter:
    """Who made the conversion: a rule of this tool or a model, by the name it reports."""

    kind: str
    name: str
    version: str

    def __post_init__(self) -> None:
        if self.kind not in CONVERTER_KINDS:
            raise _refuse(f"names a kind of converter {self.kind!r}, which is not one of {CONVERTER_KINDS}")
        if not self.name or not self.version:
            raise _refuse("does not say which converter made it, and which version")

    def to_dict(self) -> dict[str, str]:
        return {"kind": self.kind, "name": self.name, "version": self.version}

    @classmethod
    def from_dict(cls, raw: Any) -> "Converter":
        data = _object(raw, "the converter", {"kind", "name", "version"})
        return cls(
            kind=_text(data.get("kind"), "the converter's kind"),
            name=_text(data.get("name"), "the converter's name"),
            version=_text(data.get("version"), "the converter's version"),
        )


@dataclass(frozen=True)
class Conversion:
    """Markdown made of an extraction, block by block."""

    converter: Converter
    extraction: str  # the revision of the extraction it is of
    blocks: tuple[MarkdownBlock, ...]

    def __post_init__(self) -> None:
        if not re.fullmatch(r"[0-9a-f]{64}", self.extraction):
            raise _refuse("does not name the extraction it is of by its revision (64 hexadecimal digits)")
        seen: set[str] = set()
        for block in self.blocks:
            if block.id in seen:
                raise _refuse(f"has two blocks with the id {block.id!r}")
            seen.add(block.id)

    def markdown(self) -> str:
        """The Markdown a reader reads: the blocks in order, one to a paragraph."""
        return "\n\n".join(block.markdown for block in self.blocks) + "\n"

    def to_dict(self) -> dict[str, Any]:
        return {
            "v": SCHEMA_VERSION,
            "converter": self.converter.to_dict(),
            "extraction": self.extraction,
            "blocks": [block.to_dict() for block in self.blocks],
        }

    def to_json(self) -> str:
        """The canonical text a conversion is named by: sorted keys, no spaces, UTF-8."""
        return json.dumps(self.to_dict(), sort_keys=True, separators=(",", ":"), ensure_ascii=False)

    def revision(self) -> str:
        return hashlib.sha256(self.to_json().encode("utf-8")).hexdigest()

    @classmethod
    def from_json(cls, text: str) -> "Conversion":
        try:
            raw = json.loads(text)
        except ValueError as exc:
            raise _refuse(f"is not JSON ({exc})") from exc
        return cls.from_dict(raw)

    @classmethod
    def from_dict(cls, raw: Any) -> "Conversion":
        data = _object(raw, "a conversion", {"v", "converter", "extraction", "blocks"})
        if data.get("v") != SCHEMA_VERSION:
            raise _refuse(f"is of version {data.get('v')!r}; this tool reads version {SCHEMA_VERSION}")
        blocks = data.get("blocks")
        if not isinstance(blocks, list):
            raise _refuse("has no list of blocks")
        return cls(
            converter=Converter.from_dict(data.get("converter")),
            extraction=_text(data.get("extraction"), "the extraction it is of"),
            blocks=tuple(MarkdownBlock.from_dict(block) for block in blocks),
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


# ------------------------------------------------------------------------------ what is compared

_BULLETS = "-*+\u2022\u00b7\u25aa\u25e6\u2013"
_MARKER = re.compile(rf"^\s*(?:[{re.escape(_BULLETS)}]|\d+[.)])\s+")
_HEADING = re.compile(r"^\s{0,3}#{1,6}\s+")
_SEPARATOR = re.compile(r"^\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)*\|?$")
_ESCAPED = re.compile(r"\\([\\`*_{}\[\]()#+\-.!|>~])")
_PIPE = "\u0000"


def _normal(text: str) -> str:
    return " ".join(unicodedata.normalize("NFKC", text).split())


def _unescape(text: str) -> str:
    return _ESCAPED.sub(r"\1", text)


def _row_cells(line: str) -> list[str]:
    """The cells of a pipe row, a pipe written `\\|` staying in its cell."""
    inner = line.strip().replace("\\|", _PIPE)
    inner = inner[1:] if inner.startswith("|") else inner
    inner = inner[:-1] if inner.endswith("|") else inner
    return [_unescape(cell.replace(_PIPE, "|")).strip() for cell in inner.split("|")]


def plain_of_markdown(markdown: str) -> str:
    """The words of a Markdown block, with its markup taken off."""
    words: list[str] = []
    # Where the line stands in a run of pipe rows: the delimiter row of a table is its second row and
    # nowhere else, so a row of dashes that is a value, or a rule between paragraphs, is a value or words.
    place = 0
    for line in markdown.splitlines():
        stripped = line.strip()
        if not stripped:
            place = 0
            continue
        if stripped.startswith("|"):
            if place == 1 and _SEPARATOR.match(stripped):
                place += 1
                continue
            words.extend(_row_cells(stripped))
            place += 1
            continue
        place = 0
        stripped = _HEADING.sub("", stripped)
        stripped = _MARKER.sub("", stripped)
        words.append(_unescape(stripped))
    return _normal(" ".join(words))


def plain_of_block(block: Block) -> str:
    """The words of an extracted block, in the same form."""
    if block.kind == "table":
        if block.cells is not None:
            return _normal(" ".join(cell for row in block.cells for cell in row))
        return plain_of_markdown(block.text)
    if block.kind == "list_item":
        return _normal(_MARKER.sub("", block.text.strip()))
    return _normal(block.text)


# -------------------------------------------------------------------------------------- the check


@dataclass(frozen=True)
class BlockVerdict:
    """What the check says of one Markdown block."""

    id: str
    status: str
    why: str = ""


@dataclass(frozen=True)
class Report:
    """SCE's count of a conversion. A conversion cannot make this for itself."""

    verdicts: tuple[BlockVerdict, ...]
    missing: tuple[str, ...]
    recognised: tuple[str, ...]
    carried_text: int = 0
    notes: tuple[str, ...] = field(default_factory=tuple)
    # What the count was made of, each by revision: a proposal is kept only when its parts name each other.
    extraction: str = ""
    conversion: str = ""

    def ids(self, status: str) -> list[str]:
        return [verdict.id for verdict in self.verdicts if verdict.status == status]

    @property
    def matched(self) -> int:
        return len(self.ids(MATCHED))

    @property
    def complete(self) -> bool:
        """Every Markdown block is supported, none is invented, and nothing of the original is left out."""
        return not self.ids(CHECK) and not self.ids(NOT_IN_ORIGINAL) and not self.missing

    def queue(self) -> list[tuple[str, str]]:
        """What a person is asked about, in the order they are asked: what the check could not
        confirm, what was recognised from an image (never exact), what the conversion invented, and
        what it left out."""
        asked = [("check", id_) for id_ in self.ids(CHECK)]
        asked += [("recognised", id_) for id_ in self.recognised]
        asked += [("not-in-original", id_) for id_ in self.ids(NOT_IN_ORIGINAL)]
        asked += [("missing", id_) for id_ in self.missing]
        return asked

    def to_dict(self) -> dict[str, Any]:
        return {
            "extraction": self.extraction,
            "conversion": self.conversion,
            "complete": self.complete,
            "matched": self.matched,
            "check": self.ids(CHECK),
            "not_in_original": self.ids(NOT_IN_ORIGINAL),
            "missing": list(self.missing),
            "recognised": list(self.recognised),
            "queue": [{"why": why, "id": id_} for why, id_ in self.queue()],
        }


def check_conversion(extraction: Extraction, conversion: Conversion) -> Report:
    """Hold `conversion` to `extraction` block by block.

    It is refused, and no report is made, when the conversion is of another extraction: a count of what
    matched against the wrong original says nothing, and reads like a count.
    """
    if conversion.extraction != extraction.revision():
        raise IngestError(
            f"the conversion is of the extraction {conversion.extraction[:12]}, not of this one "
            f"({extraction.revision()[:12]}): nothing is counted against another original"
        )
    by_id = {block.id: block for block in extraction.blocks}
    cited: set[str] = set()
    verdicts: list[BlockVerdict] = []
    for block in conversion.blocks:
        if not block.cites:
            verdicts.append(BlockVerdict(block.id, NOT_IN_ORIGINAL, "it cites no block of the original"))
            continue
        unknown = [cite for cite in block.cites if cite not in by_id]
        if unknown:
            verdicts.append(
                BlockVerdict(block.id, CHECK, f"it cites {', '.join(unknown)}, which the original does not have")
            )
            continue
        cited.update(block.cites)
        wanted = _normal(" ".join(plain_of_block(by_id[cite]) for cite in block.cites))
        said = plain_of_markdown(block.markdown)
        if said == wanted:
            verdicts.append(BlockVerdict(block.id, MATCHED))
        else:
            verdicts.append(BlockVerdict(block.id, CHECK, "its words are not the words of the blocks it cites"))
    # A block with no words (an empty cell row) has nothing to leave out.
    missing = tuple(
        block.id for block in extraction.blocks if block.id not in cited and plain_of_block(block)
    )
    recognised = tuple(block.id for block in extraction.blocks if block.id in cited and block.how_read != "text-layer")
    return Report(
        tuple(verdicts),
        missing,
        recognised,
        carried_text=sum(1 for b in extraction.blocks if plain_of_block(b)),
        extraction=extraction.revision(),
        conversion=conversion.revision(),
    )


# ------------------------------------------------------------------------- the converter by rule

_STARTS_AS_MARKUP = re.compile(r"^(?:#{1,6}\s|>|[-*+]\s|\d+[.)]\s|\||```|~~~)")


def _escape_paragraph(text: str) -> str:
    """A paragraph that begins as Markdown would read as something else is written with its first
    character escaped, so that what a renderer shows is the words and not a heading or a list."""
    if not _STARTS_AS_MARKUP.match(text):
        return text
    ordered = re.match(r"^(\d+)([.)])", text)
    if ordered:
        # An ordered marker is escaped at its delimiter (`1\.`): a backslash before a digit is no escape.
        return text[: ordered.end(1)] + "\\" + text[ordered.end(1):]
    return "\\" + text


def _cell(text: str) -> str:
    return text.replace("\\", "\\\\").replace("|", "\\|").replace("\n", " ")


def _table(block: Block) -> str:
    if block.cells is None:
        raise IngestError(
            f"the table {block.id} has no grid of cells: it is written by rule only from the cells it was read with"
        )
    rows = [list(row) for row in block.cells]
    if not rows or not rows[0]:
        return ""
    width = len(rows[0])
    lines = ["| " + " | ".join(_cell(cell) for cell in rows[0]) + " |"]
    lines.append("| " + " | ".join("---" for _ in range(width)) + " |")
    lines += ["| " + " | ".join(_cell(cell) for cell in row) + " |" for row in rows[1:]]
    return "\n".join(lines)


def markdown_of_block(block: Block) -> str:
    """One block of an extraction as the Markdown that says the same words."""
    if block.kind == "heading":
        return "#" * (block.level or 1) + " " + block.text
    if block.kind == "table":
        return _table(block)
    if block.kind == "list_item":
        # A bullet that Markdown does not know (a PDF's) is written as the one it does.
        text = block.text.strip()
        marker = _MARKER.match(text)
        if marker:
            body = text[marker.end():]
            return ("- " if marker.group(0).strip()[0] in _BULLETS else marker.group(0).strip() + " ") + body
        return "- " + text
    if block.kind == "paragraph":
        return _escape_paragraph(block.text)
    return block.text


RULES_NAME = "blocks-to-markdown"
RULES_VERSION = "1"


def convert_by_rules(extraction: Extraction) -> Conversion:
    """The conversion of an original that carries its text: each block written as Markdown, and cited.

    A block with no words is not written (an empty cell row has nothing to say), and so is not cited and
    is not missing either. Every other block is, one for one, which is why the check finds this
    conversion complete.
    """
    blocks: list[MarkdownBlock] = []
    for block in extraction.blocks:
        if not plain_of_block(block):
            continue
        blocks.append(
            MarkdownBlock(
                id=f"m{len(blocks) + 1:04d}",
                markdown=markdown_of_block(block),
                cites=(block.id,),
            )
        )
    return Conversion(Converter("rules", RULES_NAME, RULES_VERSION), extraction.revision(), tuple(blocks))
