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

**A picture an owner asked a model to read is carried as what it was read as, and asked about.** A figure
block holds only its `[picture: NAME]` mark, which the converter by rule writes as it is; a model that read
the picture adds Markdown blocks after it (`reading.py`). Each says which picture it was read from
(`picture`, citing the figure block) and whether the model was unsure of it, and the conversion keeps one
`Reading` per picture: what was handed over, by its hash, which runtime and model read it with which recipe,
and what it could not read. Words read from a picture have nothing to be compared with, so the check queues
every such block as *read from a picture*, for the owner to keep, correct or drop; a block that says it was
read from a picture and cites anything but that figure is *check*. A conversion with no reading writes
neither field, so its text and revision are what they were.

The wire form is a `pre-release` surface: `to_dict` and `from_dict` are its only producer and reader.
"""

from __future__ import annotations

import hashlib
import json
import re
import unicodedata
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

from .errors import IngestError
from .extraction import Block, Extraction, NotCarried

if TYPE_CHECKING:
    from .scope import Scope

SCHEMA_VERSION = 1
SCHEMA_STATUS = "pre-release"

CONVERTER_KINDS = ("rules", "model")

MATCHED = "matched"
CHECK = "check"
NOT_IN_ORIGINAL = "not-in-original"
READ_FROM_PICTURE = "read-from-picture"


def _refuse(what: str) -> IngestError:
    return IngestError(f"a conversion {what}")


@dataclass(frozen=True)
class MarkdownBlock:
    """One block of the Markdown, and the blocks of the extraction it says it was made from."""

    id: str
    markdown: str
    cites: tuple[str, ...] = ()
    # The NAME of the picture this block was read from by a model, when it was; it cites the figure block
    # that marks the picture. `None` for a block written from the words of the original.
    picture: str | None = None
    # The model said it was not sure of what it read here.
    unsure: bool = False

    def __post_init__(self) -> None:
        if not self.id:
            raise _refuse("has a block with no id")
        if len(set(self.cites)) != len(self.cites):
            raise _refuse(f"has block {self.id!r} citing one block twice")
        if any(not cite for cite in self.cites):
            raise _refuse(f"has block {self.id!r} citing a block with no id")
        if self.picture is not None and not self.picture:
            raise _refuse(f"has block {self.id!r} read from a picture with no name")
        if self.unsure and self.picture is None:
            raise _refuse(f"has block {self.id!r} unsure of a reading it is not")

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"id": self.id, "markdown": self.markdown, "cites": list(self.cites)}
        if self.picture is not None:
            out["picture"] = self.picture
        if self.unsure:
            out["unsure"] = True
        return out

    @classmethod
    def from_dict(cls, raw: Any) -> "MarkdownBlock":
        data = _object(raw, "a block", {"id", "markdown", "cites", "picture", "unsure"})
        cites = data.get("cites", [])
        if not isinstance(cites, list) or not all(isinstance(c, str) for c in cites):
            raise _refuse("has block cites that are not a list of block ids")
        unsure = data.get("unsure", False)
        if not isinstance(unsure, bool):
            raise _refuse("has a block whose `unsure` is not true or false")
        picture = data.get("picture")
        return cls(
            id=_text(data.get("id"), "a block's id"),
            markdown=_text(data.get("markdown"), "a block's markdown", allow_empty=True),
            cites=tuple(cites),
            picture=None if picture is None else _text(picture, "the picture a block was read from"),
            unsure=unsure,
        )


@dataclass(frozen=True)
class Seen:
    """A picture as it was handed over to be read: its name in the document, its hash and its format."""

    name: str
    sha256: str
    media_type: str

    def __post_init__(self) -> None:
        if not self.name or not self.media_type:
            raise _refuse("has a reading of a picture with no name or no format")
        if not re.fullmatch(r"[0-9a-f]{64}", self.sha256):
            raise _refuse("has a reading of a picture not named by its hash (64 hexadecimal digits)")

    def to_dict(self) -> dict[str, str]:
        return {"name": self.name, "sha256": self.sha256, "media_type": self.media_type}

    @classmethod
    def from_dict(cls, raw: Any) -> "Seen":
        data = _object(raw, "a picture read", {"name", "sha256", "media_type"})
        return cls(
            name=_text(data.get("name"), "a picture's name"),
            sha256=_text(data.get("sha256"), "a picture's hash"),
            media_type=_text(data.get("media_type"), "a picture's format"),
        )


@dataclass(frozen=True)
class Reading:
    """One picture a model read: which figure block marks it, what was handed over, who read it with which
    recipe, and what it said it could not read. Its blocks are the conversion's blocks that name it."""

    block: str  # the figure block that marks the picture
    picture: Seen
    recipe: str
    runtime: str
    runtime_version: str
    model: str
    unreadable: tuple[str, ...] = ()
    nothing_to_read: bool = False

    def __post_init__(self) -> None:
        if not self.block or not self.recipe or not self.runtime:
            raise _refuse("has a reading that does not say which block, recipe and runtime it is of")
        if any(not where for where in self.unreadable):
            raise _refuse("has a reading that says it could not read a place it does not name")

    def to_dict(self) -> dict[str, Any]:
        return {
            "block": self.block,
            "picture": self.picture.to_dict(),
            "recipe": self.recipe,
            "runtime": self.runtime,
            "runtime_version": self.runtime_version,
            "model": self.model,
            "unreadable": list(self.unreadable),
            "nothing_to_read": self.nothing_to_read,
        }

    @classmethod
    def from_dict(cls, raw: Any) -> "Reading":
        data = _object(
            raw,
            "a reading",
            {"block", "picture", "recipe", "runtime", "runtime_version", "model", "unreadable", "nothing_to_read"},
        )
        unreadable = data.get("unreadable", [])
        if not isinstance(unreadable, list) or not all(isinstance(u, str) for u in unreadable):
            raise _refuse("has a reading whose unreadable places are not a list of words")
        nothing = data.get("nothing_to_read", False)
        if not isinstance(nothing, bool):
            raise _refuse("has a reading whose `nothing_to_read` is not true or false")
        return cls(
            block=_text(data.get("block"), "the block a reading is of"),
            picture=Seen.from_dict(data.get("picture")),
            recipe=_text(data.get("recipe"), "a reading's recipe"),
            runtime=_text(data.get("runtime"), "a reading's runtime"),
            runtime_version=_text(data.get("runtime_version", ""), "a runtime's version", allow_empty=True),
            model=_text(data.get("model", ""), "a reading's model", allow_empty=True),
            unreadable=tuple(unreadable),
            nothing_to_read=nothing,
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
    # The pictures a model read, one for each (figure block, picture name), in the order they were read.
    readings: tuple[Reading, ...] = ()

    def __post_init__(self) -> None:
        if not re.fullmatch(r"[0-9a-f]{64}", self.extraction):
            raise _refuse("does not name the extraction it is of by its revision (64 hexadecimal digits)")
        seen: set[str] = set()
        for block in self.blocks:
            if block.id in seen:
                raise _refuse(f"has two blocks with the id {block.id!r}")
            seen.add(block.id)
        read = [(reading.block, reading.picture.name) for reading in self.readings]
        if len(set(read)) != len(read):
            raise _refuse("has two readings of one picture")
        names = {name for _, name in read}
        unrecorded = [b.id for b in self.blocks if b.picture is not None and b.picture not in names]
        if unrecorded:
            raise _refuse(f"has block(s) {', '.join(unrecorded)} read from a picture it keeps no reading of")

    def reading_of(self, block: MarkdownBlock) -> Reading | None:
        """The reading `block` was made by: the one of its picture, of the figure block it cites."""
        if block.picture is None:
            return None
        return next(
            (r for r in self.readings if r.picture.name == block.picture and (r.block,) == block.cites),
            None,
        )

    def markdown(self) -> str:
        """The Markdown a reader reads: the blocks in order, one to a paragraph."""
        return "\n\n".join(block.markdown for block in self.blocks) + "\n"

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "v": SCHEMA_VERSION,
            "converter": self.converter.to_dict(),
            "extraction": self.extraction,
            "blocks": [block.to_dict() for block in self.blocks],
        }
        if self.readings:
            out["readings"] = [reading.to_dict() for reading in self.readings]
        return out

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
        data = _object(raw, "a conversion", {"v", "converter", "extraction", "blocks", "readings"})
        if data.get("v") != SCHEMA_VERSION:
            raise _refuse(f"is of version {data.get('v')!r}; this tool reads version {SCHEMA_VERSION}")
        blocks = data.get("blocks")
        if not isinstance(blocks, list):
            raise _refuse("has no list of blocks")
        readings = data.get("readings", [])
        if not isinstance(readings, list):
            raise _refuse("has readings that are not a list")
        return cls(
            converter=Converter.from_dict(data.get("converter")),
            extraction=_text(data.get("extraction"), "the extraction it is of"),
            blocks=tuple(MarkdownBlock.from_dict(block) for block in blocks),
            readings=tuple(Reading.from_dict(reading) for reading in readings),
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
        heading = _HEADING.match(stripped)
        if heading:
            # A heading's words are inline text: the number of `# 2. Scope` is a word of it, not a list marker.
            stripped = stripped[heading.end():]
        else:
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
    # The scope the count was made under, by revision ("" for the whole extraction), and how many blocks with
    # words lay outside it: they are not asked about, and a count that hid them would read as a whole one.
    scope: str = ""
    out_of_scope: int = 0
    # What the extractor could not carry of the part counted: a note about blocks outside it is not the
    # part's (ADR 0039). The report is kept with the proposal, so a review read later says the same.
    not_carried: tuple[NotCarried, ...] = field(default_factory=tuple)

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
        confirm, what was recognised from an image (never exact), what a model read from a picture,
        what the conversion invented, and what it left out."""
        asked = [("check", id_) for id_ in self.ids(CHECK)]
        asked += [("recognised", id_) for id_ in self.recognised]
        asked += [(READ_FROM_PICTURE, id_) for id_ in self.ids(READ_FROM_PICTURE)]
        asked += [("not-in-original", id_) for id_ in self.ids(NOT_IN_ORIGINAL)]
        asked += [("missing", id_) for id_ in self.missing]
        return asked

    def to_dict(self) -> dict[str, Any]:
        return {
            "extraction": self.extraction,
            "conversion": self.conversion,
            "scope": self.scope,
            "out_of_scope": self.out_of_scope,
            "complete": self.complete,
            "matched": self.matched,
            "check": self.ids(CHECK),
            "not_in_original": self.ids(NOT_IN_ORIGINAL),
            "missing": list(self.missing),
            "recognised": list(self.recognised),
            "read_from_picture": self.ids(READ_FROM_PICTURE),
            "queue": [{"why": why, "id": id_} for why, id_ in self.queue()],
            "not_carried": [entry.to_dict() for entry in self.not_carried],
        }


def in_scope(extraction: Extraction, scope: "Scope | None") -> "Scope | None":
    """The scope a count is made under: `None` for the whole extraction, which a scope of every block is.

    One input has one spelling, so that a proposal made for everything is the same input whether the owner
    chose everything or chose nothing. A scope of another extraction is refused, as a conversion is.
    """
    if scope is None:
        return None
    scope.held_by(extraction)
    return None if len(scope.blocks) == len(extraction.blocks) else scope


def check_conversion(extraction: Extraction, conversion: Conversion, scope: "Scope | None" = None) -> Report:
    """Hold `conversion` to `extraction` block by block, or to the part of it `scope` names.

    It is refused, and no report is made, when the conversion is of another extraction: a count of what
    matched against the wrong original says nothing, and reads like a count. The scope is the owner's and is
    given here, never read from the conversion, whose maker would then decide what counts as missing
    (ADR 0026, 0039). A block outside it is not missing; it is counted as out of scope. A conversion block
    that cites a block outside it is to check, since what was not given to its maker cannot have been read.
    """
    if conversion.extraction != extraction.revision():
        raise IngestError(
            f"the conversion is of the extraction {conversion.extraction[:12]}, not of this one "
            f"({extraction.revision()[:12]}): nothing is counted against another original"
        )
    scope = in_scope(extraction, scope)
    by_id = {block.id: block for block in extraction.blocks}
    chosen = by_id if scope is None else {block_id: by_id[block_id] for block_id in scope.blocks}
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
        outside = [cite for cite in block.cites if cite not in chosen]
        if outside:
            verdicts.append(
                BlockVerdict(
                    block.id,
                    CHECK,
                    f"it cites {', '.join(outside)}, which is outside the part of the original chosen for this import",
                )
            )
            continue
        cited.update(block.cites)
        if block.picture is not None:
            # Words read from a picture have nothing in the original to be compared with: they are asked
            # about, always. What can be held is that the block says which picture, and cites only it.
            if len(block.cites) != 1 or by_id[block.cites[0]].kind != "figure":
                verdicts.append(
                    BlockVerdict(block.id, CHECK, "it says it was read from a picture and does not cite the "
                                 "one figure block that marks it")
                )
            elif conversion.reading_of(block) is None:
                verdicts.append(
                    BlockVerdict(block.id, CHECK, f"it says it was read from {block.picture}, and the conversion "
                                 f"keeps no reading of that picture at {block.cites[0]}")
                )
            else:
                verdicts.append(BlockVerdict(block.id, READ_FROM_PICTURE))
            continue
        wanted = _normal(" ".join(plain_of_block(by_id[cite]) for cite in block.cites))
        said = plain_of_markdown(block.markdown)
        if said == wanted:
            verdicts.append(BlockVerdict(block.id, MATCHED))
        else:
            verdicts.append(BlockVerdict(block.id, CHECK, "its words are not the words of the blocks it cites"))
    # A block with no words (an empty cell row) has nothing to leave out.
    missing = tuple(block.id for block in chosen.values() if block.id not in cited and plain_of_block(block))
    recognised = tuple(
        block_id for block_id, block in chosen.items() if block_id in cited and block.how_read != "text-layer"
    )
    return Report(
        tuple(verdicts),
        missing,
        recognised,
        carried_text=sum(1 for b in chosen.values() if plain_of_block(b)),
        extraction=extraction.revision(),
        conversion=conversion.revision(),
        scope="" if scope is None else scope.revision(),
        out_of_scope=sum(1 for b in extraction.blocks if b.id not in chosen and plain_of_block(b)),
        not_carried=extraction.not_carried_of(None if scope is None else scope.blocks),
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


def convert_by_rules(extraction: Extraction, scope: "Scope | None" = None) -> Conversion:
    """The conversion of an original that carries its text, or of the part of it `scope` names: each block
    written as Markdown, and cited.

    A block with no words is not written (an empty cell row has nothing to say), and so is not cited and
    is not missing either. Every other block is, one for one, which is why the check finds this
    conversion complete.
    """
    scope = in_scope(extraction, scope)
    blocks: list[MarkdownBlock] = []
    for block in extraction.blocks:
        if not plain_of_block(block) or (scope is not None and not scope.covers(block.id)):
            continue
        blocks.append(
            MarkdownBlock(
                id=f"m{len(blocks) + 1:04d}",
                markdown=markdown_of_block(block),
                cites=(block.id,),
            )
        )
    if not blocks:
        # Not a conversion that says nothing and is found complete: a document with no words in it (an
        # empty file, or pages that are pictures) has nothing to convert by rule, and a proposal made of
        # it would be an empty spec waiting to be accepted. Reading pictures is a reader's work.
        raise IngestError(
            f"the extraction has no block with words in it, so there is nothing to convert by rule "
            f"({len(extraction.blocks)} block(s), {len(extraction.not_carried)} thing(s) not carried)"
        )
    return Conversion(Converter("rules", RULES_NAME, RULES_VERSION), extraction.revision(), tuple(blocks))
