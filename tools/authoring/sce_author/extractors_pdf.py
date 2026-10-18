"""The extractor of a page-layout document that carries its own text (a PDF with a text layer).

ADR 0035 splits reading by what an original carries. A PDF whose pages carry their text is read here, by
a library and no model: every line comes with the box it sits in, so a block is placed by its page and a
box that is exact, because it is where the document put the text. A page that carries no text (a scan, a
picture) is not read here and is said not to be: reading pixels is a reader's work, and reporting it as an
empty page would read as a page that says nothing.

The library is PDFium through `pypdfium2` (BSD-3 or Apache-2.0, the binary bundled with it). It is a pack
and not a dependency of the package: without it the registry answers `pack-missing` and names it.

**What this reads, and what it says it did not.** It reads lines and gathers them into blocks by their
own geometry: lines that follow each other closely at one margin and one size are a paragraph, a line
set clearly larger than the page's body text is a heading, and a line that begins as a list item is one.
The order is the one the document's text is in. It does not recover a table's cells (a table comes out as
its lines, and a decision table read as lines loses which value goes with which condition, which is why
the extraction always says so), does not place pictures and drawings (it counts them), and does not
untangle columns. Each of those is an entry in what was not carried.

**Limits are refused before the file is read.** A file or a page count past the limits below, and a page
larger than any sheet that is printed, is refused with its numbers. ADR 0035 runs reading in a jail with a
memory and a time limit; these are the checks that need no jail to make.
"""

from __future__ import annotations

import pathlib
import re
from dataclasses import dataclass

from .errors import READ_ERRORS, IngestError
from .extraction import Block, Extraction, NotCarried, Place
from .extractor_contract import Declaration

NAME = "pdf-text"
VERSION = "1"
PACK = "pdf-text"
MODULES = ("pypdfium2",)

DECLARATION = Declaration(
    NAME,
    VERSION,
    (".pdf",),
    magic=(b"%PDF-",),
    ways=("text-layer",),
    unit="page",
    needs=MODULES,
    pack=PACK,
)

# The largest file and the most pages read. A specification is not a library of scans.
MAX_BYTES = 200 * 1024 * 1024
MAX_PAGES = 5000
# The longest side of a page, in points: the PDF format's own ceiling is 14400.
MAX_PAGE_POINTS = 14400

# A line closer than this many line heights to the one before belongs to the same paragraph.
PARAGRAPH_GAP = 0.8
# A line this many times taller than the page's body text is a heading.
HEADING_RATIO = 1.3
# How far, in line heights, a line may start from the margin of the one before and still continue it.
MARGIN_SHIFT = 1.5
# How different two lines' heights may be, as a share of the smaller, and still be one paragraph.
SIZE_SHIFT = 0.25
MOST_LISTED = 12

_LIST_ITEM = re.compile(r"^(?:[-*+•·▪◦–]|\d+[.)])\s+\S")


@dataclass
class Line:
    """One line of a page, from its top-left corner, in points."""

    left: float
    top: float
    right: float
    bottom: float
    text: str
    # The type size the line is set in, which is what the document says and not what its letters
    # happen to reach: the box of a line of capitals is taller than the box of one without ascenders.
    size: float


def _listed(numbers: list[int]) -> str:
    shown = ", ".join(str(n) for n in numbers[:MOST_LISTED])
    return shown + (", ..." if len(numbers) > MOST_LISTED else "")


def _page_lines(textpage, page_height: float, raw) -> list[Line]:
    lines: list[Line] = []
    for index in range(textpage.count_rects()):
        left, bottom, right, top = textpage.get_rect(index)
        text = " ".join(textpage.get_text_bounded(left, bottom, right, top).split())
        if not text:
            continue
        first = textpage.get_index(left + 1.0, (bottom + top) / 2, 2.0, 2.0)
        size = raw.FPDFText_GetFontSize(textpage.raw, first) if first is not None else 0.0
        # A line whose first letter cannot be found is measured by its box, which is only near.
        size = size if size > 0 else top - bottom
        # PDF measures up from the bottom of the page; a reader's box is measured down from the top.
        lines.append(Line(left, page_height - top, right, page_height - bottom, text, size))
    return lines


def _group(lines: list[Line]) -> list[list[Line]]:
    """The lines of a page gathered into paragraphs, in the order they came."""
    groups: list[list[Line]] = []
    for line in lines:
        if groups:
            first = groups[-1][0]
            previous = groups[-1][-1]
            smaller = min(line.size, previous.size)
            close = (
                0 <= line.top - previous.bottom <= PARAGRAPH_GAP * smaller
                and abs(line.size - previous.size) <= SIZE_SHIFT * smaller
                and not _LIST_ITEM.match(line.text)
            )
            if _LIST_ITEM.match(first.text):
                # An item holds the lines that wrap under it, at its margin or further in.
                continues = close and line.left >= first.left - 1.0
            else:
                continues = close and abs(line.left - previous.left) <= MARGIN_SHIFT * smaller
            if continues:
                groups[-1].append(line)
                continue
        groups.append([line])
    return groups


def _box(group: list[Line]) -> tuple[float, float, float, float]:
    return (
        round(min(l.left for l in group), 2),
        round(min(l.top for l in group), 2),
        round(max(l.right for l in group), 2),
        round(max(l.bottom for l in group), 2),
    )


class PdfTextExtractor:
    declaration = DECLARATION

    def extract(self, path: pathlib.Path) -> Extraction:
        path = pathlib.Path(path)
        try:
            size = path.stat().st_size
        except READ_ERRORS as exc:
            raise IngestError(f"{path}: could not be measured before it was read ({exc})") from exc
        if size > MAX_BYTES:
            raise IngestError(
                f"{path}: {size} bytes is past the {MAX_BYTES} this extractor reads; it is refused and not cut, "
                "because half a document is a different document"
            )
        # Imported here and not at the top: the pack may be absent, and the registry says so before this runs.
        import pypdfium2 as pdfium

        try:
            document = pdfium.PdfDocument(str(path))
        except (pdfium.PdfiumError, *READ_ERRORS) as exc:
            raise IngestError(f"{path}: not a readable PDF ({exc})") from exc
        try:
            return self._read(path, document, pdfium)
        finally:
            document.close()

    def _read(self, path: pathlib.Path, document, pdfium) -> Extraction:
        count = len(document)
        if count > MAX_PAGES:
            raise IngestError(
                f"{path}: {count} pages is past the {MAX_PAGES} this extractor reads; it is refused and not cut"
            )
        if count == 0:
            raise IngestError(f"{path}: a PDF with no pages says nothing, which is not the same as a clean read")
        pages: list[tuple[int, list[list[Line]], list[Line]]] = []
        blank: list[int] = []
        pictured: list[int] = []
        pictures = 0
        for number in range(1, count + 1):
            page = document[number - 1]
            try:
                width, height = page.get_size()
                if max(width, height) > MAX_PAGE_POINTS:
                    raise IngestError(
                        f"{path}: page {number} is {width:.0f} by {height:.0f} points, past the "
                        f"{MAX_PAGE_POINTS} of the longest sheet; it is refused"
                    )
                textpage = page.get_textpage()
                try:
                    lines = _page_lines(textpage, height, pdfium.raw)
                finally:
                    textpage.close()
                here = sum(1 for _ in page.get_objects(filter=[pdfium.raw.FPDF_PAGEOBJ_IMAGE]))
            finally:
                page.close()
            if here:
                pictured.append(number)
                pictures += here
            if lines:
                pages.append((number, _group(lines), lines))
            else:
                blank.append(number)

        sizes = sorted({round(l.size * 2) / 2 for _, _, lines in pages for l in lines}, reverse=True)
        # The body text is the size most of the document's letters are set in, so that a handful of
        # large lines (a title page) does not become what everything else is measured against.
        letters: dict[float, int] = {}
        for _, _, lines in pages:
            for l in lines:
                letters[round(l.size * 2) / 2] = letters.get(round(l.size * 2) / 2, 0) + len(l.text)
        body = max(letters, key=letters.__getitem__) if letters else 0.0
        blocks: list[Block] = []
        member = path.name
        for number, groups, lines in pages:
            for group in groups:
                text = " ".join(l.text for l in group)
                tall = group[0].size >= HEADING_RATIO * body and len(group) == 1
                kind, extra = "paragraph", {}
                if tall:
                    # Level by rank among the sizes taller than the body text, tallest first.
                    larger = [s for s in sizes if s >= HEADING_RATIO * body]
                    kind = "heading"
                    extra["level"] = min(6, 1 + larger.index(round(group[0].size * 2) / 2))
                elif _LIST_ITEM.match(group[0].text):
                    kind = "list_item"
                ordinal = len(blocks) + 1
                blocks.append(
                    Block(
                        id=f"b{ordinal:04d}",
                        kind=kind,
                        text=text,
                        place=Place(member, ordinal=ordinal, page=number, box=_box(group), precision="exact"),
                        **extra,
                    )
                )

        carried: list[NotCarried] = []
        if blank:
            carried.append(
                NotCarried(
                    f"{len(blank)} of {count} page(s) carry no text: {_listed(blank)}",
                    "each is a picture or empty, and reading pixels is a reader's work, not this extractor's",
                    len(blank),
                )
            )
        if pictures:
            carried.append(
                NotCarried(
                    f"{pictures} picture(s) on page(s) {_listed(pictured)}",
                    "counted and not placed or read; a picture is not text",
                    pictures,
                )
            )
        if pages:
            carried.append(
                NotCarried(
                    "the layout of the pages: tables, columns and the order of what sits side by side",
                    "lines are read in the order the document's text is in and gathered by their own spacing; "
                    "a table comes out as its lines and its cells are not recovered, so a value may be "
                    "separated from the condition it goes with",
                )
            )
        if not blocks and not blank:
            raise IngestError(f"{path}: read as a document and produced no text")
        return Extraction(NAME, VERSION, member, tuple(blocks), tuple(carried))
