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

import ctypes
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
# Two characters are on one line when their baselines are no further apart than this share of the larger type
# size (a raised or lowered glyph is set a part of a size off), and when they are no further apart along it
# than this many sizes (a word's space is about a third of one; a column gutter is several), or no more
# overlapping than KERN of one (kerning and a letter set over its neighbour).
SAME_BASELINE = 0.4
GLUE = 0.8
# After a space the document drew, whitespace of its own making is allowed to be wider: several spaces in a
# run are one space character to the library, with the rest of the width left as a gap.
SPACED = 4.0
KERN = 0.5

_LIST_ITEM = re.compile(r"^(?:[-*+•·▪◦–]|\d+[.)])\s+\S")
# A clause number (`3.1`, `3.1.2`, `A.1`) and the title that follows it, which begins with a letter or an
# opening bracket. Only numbers made of digits count: a lettered list item is not a section.
# The Hangul syllables (U+AC00 to U+D7A3) are written as escapes, so this source stays ASCII.
_CLAUSE = re.compile(r"^(\d+(?:\.\d+)*)\.?\s+[A-Za-z<(\[\"'가-힣]")
# The longest line that is a title and not a sentence that begins with a number.
MAX_TITLE = 100


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


@dataclass
class _Run:
    """The line being gathered from the characters of a page, in PDF's own measure (up from the bottom)."""

    left: float
    bottom: float
    right: float
    top: float
    size: float
    baseline: float
    text: list[str]
    space: bool = False
    # A space the document drew (it has a width) was the last thing on the line: whitespace the producer put
    # in the run itself, which is not a gap between two pieces of drawing.
    spaced: bool = False

    def continued_by(self, left: float, baseline: float, size: float) -> bool:
        """Whether a character set at `baseline` in this type size is the next of this line: it is on the same
        baseline, reads on in the direction of the text, and has no more before it than a word's space.

        A character is on a line by where it is set and not by how much of its box overlaps the others': a
        comma's box lies below the line, a quotation mark's above it, and neither is on another line. A raised
        or lowered glyph (a power, an index) is set a part of a type size off, and is still the line's.
        """
        if abs(baseline - self.baseline) > SAME_BASELINE * max(size, self.size):
            return False
        # A character that starts further back than a letter's kerning begins another row or a column to the
        # left, and one further on than a space is another column: both are another line.
        return -KERN * size <= left - self.right <= (SPACED if self.spaced else GLUE) * size


def _unicode(textpage, raw, index: int) -> str:
    """The character at `index`: PDFium answers in UTF-16 code units, and a pair is put back together."""
    unit = raw.FPDFText_GetUnicode(textpage.raw, index)
    return chr(unit) if not 0xD800 <= unit <= 0xDFFF else "�"


def _page_lines(textpage, page_height: float, raw) -> list[Line]:
    """The lines of a page, read from its characters in the order the document has them.

    The library's rectangles are not lines: a producer that sets every glyph on its own (a browser's
    print-to-PDF does) gets a rectangle for each, and they overlap and repeat. A character is the unit that is
    always one thing, with its own box and type size, so a line is the characters that follow each other on
    one baseline with no more between them than a word's space, which is how a reader sees it. A gap wider
    than that is a column, and stays a line of its own: the extraction says it does not untangle columns.
    """
    lines: list[Line] = []
    run: _Run | None = None

    def close() -> None:
        nonlocal run
        if run is not None:
            text = " ".join("".join(run.text).split())
            if text:
                # PDF measures up from the bottom of the page; a reader's box is measured down from the top.
                lines.append(Line(run.left, page_height - run.top, run.right, page_height - run.bottom, text, run.size))
        run = None

    for index in range(textpage.count_chars()):
        character = _unicode(textpage, raw, index)
        if character in "\r\n":
            # The library puts a break where it has seen the drawing move; the geometry decides what a line is.
            continue
        if character.isspace():
            if run is not None:
                run.space = True
                # A space the document drew has a width, and the line reaches to the end of it: several of
                # them in one run are a gap the producer made on purpose and is still one run's.
                left, bottom, right, top = textpage.get_charbox(index)
                if right > left and bottom < run.top and top > run.bottom:
                    run.right = max(run.right, right)
                    run.spaced = True
            continue
        left, bottom, right, top = textpage.get_charbox(index)
        size = raw.FPDFText_GetFontSize(textpage.raw, index)
        # A character whose size cannot be read is measured by its box, which is only near.
        size = size if size > 0 else top - bottom
        origin_x, origin_y = ctypes.c_double(), ctypes.c_double()
        raw.FPDFText_GetCharOrigin(textpage.raw, index, ctypes.byref(origin_x), ctypes.byref(origin_y))
        if run is not None and not run.continued_by(left, origin_y.value, size):
            close()
        if run is None:
            run = _Run(left, bottom, right, top, size, origin_y.value, [character])
            continue
        # A word's space is the library's to say: it puts a space character where the drawing leaves a gap, and
        # a threshold of this reader's own would put one after every narrow glyph (the full stop of `3.1.1`).
        if run.space:
            run.text.append(" ")
        run.space = False
        run.spaced = False
        run.text.append(character)
        run.left, run.bottom = min(run.left, left), min(run.bottom, bottom)
        run.right, run.top = max(run.right, right), max(run.top, top)
    close()
    return lines


def _section_depth(text: str) -> int:
    """How many numbers a clause number has (`3.1.2 Title` is 3, `3 Title` is 1), or 0 for a line that is
    not a title: a clause number, a title that begins a word, and no more than a title's length, which does not
    end as a sentence does."""
    if len(text) > MAX_TITLE:
        return 0
    found = _CLAUSE.match(text)
    if found is None or text.rstrip().endswith((".", ";", ":", ",")):
        return 0
    return found.group(1).count(".") + 1


def _numbering_base(pages, body: float, larger: list[float]) -> int:
    """The level the document set its own top-level numbers at (`3 Core Constructs`), which is where a
    numbered line of two or more numbers goes one level below. Without any, the first level."""
    levels: dict[int, int] = {}
    for _, groups, _ in pages:
        for group in groups:
            if len(group) == 1 and group[0].size >= HEADING_RATIO * body and _section_depth(group[0].text) == 1:
                level = min(6, 1 + larger.index(round(group[0].size * 2) / 2))
                levels[level] = levels.get(level, 0) + 1
    return max(levels, key=lambda level: (levels[level], -level)) if levels else 1


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
        larger = [s for s in sizes if s >= HEADING_RATIO * body]
        base = _numbering_base(pages, body, larger)
        for number, groups, lines in pages:
            for group in groups:
                text = " ".join(l.text for l in group)
                tall = group[0].size >= HEADING_RATIO * body and len(group) == 1
                kind, extra = "paragraph", {}
                depth = _section_depth(text) if len(group) == 1 else 0
                if tall:
                    # Level by rank among the sizes taller than the body text, tallest first.
                    kind = "heading"
                    extra["level"] = min(6, 1 + larger.index(round(group[0].size * 2) / 2))
                elif depth >= 2:
                    # A clause number is how a specification marks its sections, and a line of it that stands
                    # alone at the body's size is a heading the document did not set larger. Its level is how
                    # deep its number goes, from the level the document's own top-level numbers were set at.
                    kind = "heading"
                    extra["level"] = min(6, base + depth - 1)
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
