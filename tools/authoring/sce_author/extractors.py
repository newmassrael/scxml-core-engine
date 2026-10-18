"""Extractors: one per format, chosen by what a file is and what is installed.

ADR 0036 makes a format a seam. An extractor reads one format into an `Extraction` (`extraction.py`)
and declares, as data, what it reads and what it needs; this module is the registry that picks one.
Adding a format is adding an extractor and its declaration: no tool, screen or job kind changes.

**A suffix is a claim and the bytes decide.** What a file is called says what its owner meant. A format
with a signature (a PDF, a zip) is checked against it, and a file whose name and bytes disagree is
refused as such and not handed to the wrong reader, which would read it wrongly and succeed. A file with
no suffix of a known format is sniffed, so a PDF called `spec.dat` is still a PDF.

**A reason a file is not read is typed and is not an error.** `format-unread` (nobody reads this
format), `pack-missing` (an extractor does, and what it needs is not installed: the reason names the
pack) and `sniff-mismatch` are answers a caller can show and act on. `extract` turns one into the
tool's usual refusal for a caller that wants one; `try_extract` hands it back.

**What an extractor needs is a pack.** A pack is a set of importable modules. The baseline readers need
none; a reader that wants a library declares it, and without it is skipped in favour of the next
extractor that reads the format, or reported as `pack-missing`.
"""

from __future__ import annotations

import pathlib
import pkgutil
import re
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from typing import Protocol

from . import ingest as _ingest
from .errors import READ_ERRORS, IngestError, describe_path
from .extraction import Block, Extraction, NotCarried, Place

# How many leading bytes are read to check a signature.
SNIFF_BYTES = 16

FORMAT_UNREAD = "format-unread"
PACK_MISSING = "pack-missing"
SNIFF_MISMATCH = "sniff-mismatch"


@dataclass(frozen=True)
class Declaration:
    """What an extractor says of itself."""

    name: str
    version: str
    # Suffixes it reads, lower case with the dot.
    suffixes: tuple[str, ...] = ()
    # Signatures a file of this format begins with. Empty for a format with none (text).
    magic: tuple[bytes, ...] = ()
    # Whether the signature is this format's alone. False for one that other formats share.
    magic_identifies: bool = True
    # The ways of reading it gives (`text-layer`, `ocr`, `vision`).
    ways: tuple[str, ...] = ("text-layer",)
    # The unit a mixed document is chosen by (`page`), or None for a format read whole.
    unit: str | None = None
    # Importable modules it needs, and the name of the pack that carries them.
    needs: tuple[str, ...] = ()
    pack: str | None = None

    def __post_init__(self) -> None:
        if not self.name or not self.version:
            raise ValueError("an extractor declares its name and its version")
        if not self.suffixes and not self.magic:
            raise ValueError(f"extractor {self.name!r} declares neither a suffix nor a signature")
        for suffix in self.suffixes:
            if not suffix.startswith(".") or suffix != suffix.lower():
                raise ValueError(f"extractor {self.name!r} declares the suffix {suffix!r}: lower case, with the dot")
        if self.needs and not self.pack:
            raise ValueError(f"extractor {self.name!r} needs modules and does not name the pack that carries them")

    def available(self) -> bool:
        """Whether what it needs is installed."""
        return not self.missing()

    def missing(self) -> list[str]:
        """The modules it needs that are not installed.

        Looked up by listing what the interpreter could import and never by importing: this module is
        loaded by the server process, which does not import code by name (`test_core_is_domain_free`),
        and a pack that is only looked for must not run its import-time code.
        """
        if not self.needs:
            return []
        installed = {found.name for found in pkgutil.iter_modules()}
        return [module for module in self.needs if module not in installed]

    def has_suffix(self, suffix: str) -> bool:
        return suffix in self.suffixes

    def begins_as_mine(self, head: bytes) -> bool:
        """Whether the first bytes are a signature of this format."""
        return any(head.startswith(sign) for sign in self.magic)

    def is_identified_by_signature(self, head: bytes) -> bool:
        """Whether the first bytes alone say a file is of this format (a PDF's `%PDF-`).

        A signature that other formats share (a zip's, which a word document, a workbook and an
        archive all begin with) checks a file that claims the format and never finds one that does not.
        """
        return self.magic_identifies and self.begins_as_mine(head)


class Extractor(Protocol):
    declaration: Declaration

    def extract(self, path: pathlib.Path) -> Extraction: ...


@dataclass(frozen=True)
class Unreadable:
    """Why a file is not read, as an answer."""

    code: str
    why: str
    pack: str | None = None

    def words(self, path: pathlib.Path) -> str:
        return f"{path}: {self.why}"


class Registry:
    """The extractors of a process, in the order they are preferred."""

    def __init__(self, extractors: Iterable[Extractor] = ()) -> None:
        self._extractors: list[Extractor] = []
        for extractor in extractors:
            self.register(extractor)

    def register(self, extractor: Extractor) -> None:
        name = extractor.declaration.name
        if any(held.declaration.name == name for held in self._extractors):
            raise ValueError(f"an extractor named {name!r} is already registered")
        self._extractors.append(extractor)

    @property
    def extractors(self) -> tuple[Extractor, ...]:
        return tuple(self._extractors)

    def choose(self, path: pathlib.Path) -> Extractor | Unreadable:
        """The extractor for a file, or why none will read it."""
        path = pathlib.Path(path)
        if not path.is_file():
            raise IngestError(describe_path(path))
        try:
            with path.open("rb") as held:
                head = held.read(SNIFF_BYTES)
        except READ_ERRORS as exc:
            raise IngestError(f"{path}: could not be opened to see what it is ({exc})") from exc
        suffix = path.suffix.lower()
        claimed = [e for e in self._extractors if e.declaration.has_suffix(suffix)]
        # A format with a signature is held to it, whatever the file is called.
        for extractor in claimed:
            declaration = extractor.declaration
            if declaration.magic and not declaration.begins_as_mine(head):
                return Unreadable(
                    SNIFF_MISMATCH,
                    f"is called {suffix} and does not begin as a {declaration.name} file does, so it is not "
                    "handed to its reader: a file with the wrong name is read wrongly and reads clean",
                )
        if not claimed and suffix not in _ingest.ELSEWHERE:
            # A name that says nothing known: the bytes may.
            claimed = [e for e in self._extractors if e.declaration.is_identified_by_signature(head)]
        if not claimed:
            return Unreadable(
                FORMAT_UNREAD,
                f"no extractor reads {suffix or 'a file with no suffix'}"
                + (f" ({_ingest.ELSEWHERE[suffix]})" if suffix in _ingest.ELSEWHERE else ""),
            )
        for extractor in claimed:
            if extractor.declaration.available():
                return extractor
        first = claimed[0].declaration
        missing = first.missing()
        return Unreadable(
            PACK_MISSING,
            f"its extractor {first.name!r} needs the pack {first.pack!r} (missing: {', '.join(missing)}), "
            "which is not installed",
            first.pack,
        )

    def try_extract(self, path: pathlib.Path) -> Extraction | Unreadable:
        chosen = self.choose(path)
        if isinstance(chosen, Unreadable):
            return chosen
        return chosen.extract(pathlib.Path(path))

    def extract(self, path: pathlib.Path) -> Extraction:
        done = self.try_extract(path)
        if isinstance(done, Unreadable):
            raise IngestError(done.words(pathlib.Path(path)))
        return done


# ----------------------------------------------------------- text, from the readers of `ingest`

_HEADING = re.compile(r"^(#{1,6})\s+(\S.*)$")
_LIST_ITEM = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+\S")
_ONLY_MARKS = re.compile(r"^(?:\[picture(?:: [^\]]+)?\]|\[\d+ pictures\])(?:\s+(?:\[picture(?:: [^\]]+)?\]))*$")
_ENCLOSED = re.compile(r"^\[enclosed object: [^\]]+\]$")


def _row_cells(line: str) -> list[str]:
    inner = line.strip()
    inner = inner[1:] if inner.startswith("|") else inner
    inner = inner[:-1] if inner.endswith("|") else inner
    return [cell.strip() for cell in inner.split(" | ")] if " | " in inner else [inner.strip()]


def blocks_of_text(text: str, member: str, line_is_block: bool) -> tuple[list[Block], list[NotCarried]]:
    """The blocks of a reader's text, and what putting them in blocks could not keep exact.

    The readers of `ingest` write a document as text (a table as one line per row beginning with `|`,
    a picture as a `[picture: NAME]` mark) and say what they could not carry. This gives that text the
    form of ADR 0026 without changing what it says. A block is placed by its ordinal in the reader's
    text, because a reader that flattened a document cannot say where in the file each line sat.

    `line_is_block` is for a reader that writes one block to a line (a word document, HTML); a text
    reader's lines are wrapped, so a run of lines is one paragraph.
    """
    blocks: list[Block] = []
    notes: list[NotCarried] = []
    short_rows = 0

    def add(kind: str, body: str, **extra) -> None:
        ordinal = len(blocks) + 1
        blocks.append(
            Block(
                id=f"b{ordinal:04d}",
                kind=kind,
                text=body,
                place=Place(member=member, ordinal=ordinal),
                **extra,
            )
        )

    run: list[str] = []
    table: list[str] = []

    def flush_run() -> None:
        if run:
            add("paragraph", " ".join(part.strip() for part in run))
            run.clear()

    def flush_table() -> None:
        nonlocal short_rows
        if not table:
            return
        rows = [_row_cells(line) for line in table]
        width = max(len(row) for row in rows)
        short_rows += sum(1 for row in rows if len(row) < width)
        grid = tuple(tuple(row + [""] * (width - len(row))) for row in rows)
        add("table", "\n".join(table), cells=grid)
        table.clear()

    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("|"):
            flush_run()
            table.append(stripped)
            continue
        flush_table()
        if not stripped:
            flush_run()
            continue
        heading = _HEADING.match(stripped)
        if heading:
            flush_run()
            add("heading", heading.group(2).strip(), level=len(heading.group(1)))
        elif _ENCLOSED.match(stripped):
            flush_run()
            add("attachment", stripped)
        elif _ONLY_MARKS.match(stripped):
            flush_run()
            add("figure", stripped)
        elif _LIST_ITEM.match(stripped):
            flush_run()
            add("list_item", stripped)
        elif line_is_block:
            add("paragraph", stripped)
        else:
            run.append(stripped)
    flush_run()
    flush_table()
    if short_rows:
        notes.append(
            NotCarried(
                "rows shorter than their table",
                "the reader wrote them with fewer cells; empty cells were added so the grid is whole",
                short_rows,
            )
        )
    return blocks, notes


@dataclass(frozen=True)
class ReaderExtractor:
    """An extractor made of a reader of `ingest`, so that what it says is what that reader says."""

    declaration: Declaration
    read: Callable[[pathlib.Path], _ingest.Ingested]
    line_is_block: bool

    def extract(self, path: pathlib.Path) -> Extraction:
        ingested = self.read(path)
        blocks, notes = blocks_of_text(ingested.text, path.name, self.line_is_block)
        carried = [NotCarried(note) for note in ingested.notes] + notes
        return Extraction(
            extractor=self.declaration.name,
            extractor_version=self.declaration.version,
            member=path.name,
            blocks=tuple(blocks),
            not_carried=tuple(carried),
        )


DOCX_MAGIC = (b"PK\x03\x04",)


def builtin() -> Registry:
    """The extractors every install has: the readers of `ingest`, which need no pack."""
    return Registry(
        [
            ReaderExtractor(
                Declaration("plain", "1", (".md", ".txt", ".text")), _ingest.READERS[".md"], line_is_block=False
            ),
            ReaderExtractor(Declaration("html", "1", (".html", ".htm")), _ingest.READERS[".html"], line_is_block=True),
            ReaderExtractor(
                Declaration("docx", "1", (".docx",), magic=DOCX_MAGIC, magic_identifies=False),
                _ingest.READERS[".docx"],
                line_is_block=True,
            ),
        ]
    )


# The registry a process uses unless it is given another one.
DEFAULT = builtin()
