"""What could not be carried of a document, said once, in two languages (ADR 0026, 0039).

An extractor that could not carry something says WHAT happened as a code and the facts that go with it
(`xlsx-pictures`, a count of pictures and of cells), and never as a sentence: a sentence is how a language
says it, and an extraction is named by its hash, so a stored extraction that held the sentences of the
language its reader happened to use would be a different extraction for each reader. The sentences are
written here, beside each other, one per code per language, and a code that has one and not the other does
not load. This is the rule `sce-build/src/open_matters.rs` holds for what a design leaves open: the sentences
have one author, and a screen that shows them writes none of its own, because translating them there would
be a second author, and the one consulted less often is the one that rots.

English is also what an extraction stores beside the code (`NotCarried.what`, `why`): it is what a program
reads that knows no code, and what is shown when the reader's language has no sentence for a code this
version does not know (an extraction made by a newer tool). A stored sentence is never the source of a
translation; the code and the facts are.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass
from typing import Any

LANGUAGES = ("en", "ko")
DEFAULT_LANGUAGE = "en"

# The enclosed file a note is about, when it is about one: every note of an enclosed file may carry it, and
# its sentence then begins with the file's name.
IN_FILE = "in_file"

# The most clause numbers or page numbers a sentence lists before it says how many more there are.
MOST_LISTED = 12


class NoteError(ValueError):
    """A code that is not one, or facts that are not the ones a code takes: the extractor's defect."""


@dataclass(frozen=True)
class Sentence:
    """What a note says: what could not be carried, and why, when a reason can be given."""

    what: str
    why: str = ""


Facts = Mapping[str, Any]
Writer = Callable[[Facts], Sentence]

# A fact is one of these, by its name below. `ints` and `strs` are lists.
_KINDS = ("int", "str", "bool", "ints", "strs")

# code -> fact name -> kind. A fact absent from a note is a defect of whoever wrote it, except `in_file`.
FACTS: dict[str, dict[str, str]] = {}
_WRITERS: dict[str, dict[str, Writer]] = {language: {} for language in LANGUAGES}


def _code(code: str, facts: dict[str, str]):
    """Declare a code and the facts it takes; the writers that follow are its sentences."""
    if code in FACTS:
        raise NoteError(f"the code {code!r} is declared twice")
    for name, kind in facts.items():
        if kind not in _KINDS:
            raise NoteError(f"the code {code!r} takes the fact {name!r} of the unknown kind {kind!r}")
    FACTS[code] = facts

    def writer(language: str):
        def register(function: Writer) -> Writer:
            _WRITERS[language][code] = function
            return function

        return register

    return writer


def kind_of(value: Any) -> str | None:
    """Which kind of fact `value` is (`int`, `str`, `bool`, `ints`, `strs`), or `None` when it is none."""
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        return "int"
    if isinstance(value, str):
        return "str"
    if isinstance(value, list) and all(isinstance(v, str) for v in value):
        return "strs"
    if isinstance(value, list) and all(isinstance(v, int) and not isinstance(v, bool) for v in value):
        return "ints"
    return None


def check_facts(code: str, facts: Facts) -> None:
    """Refuse a note whose code is not declared or whose facts are not exactly what the code takes."""
    if code not in FACTS:
        raise NoteError(f"{code!r} is not a code of what could not be carried")
    declared = FACTS[code]
    for name in facts:
        if name != IN_FILE and name not in declared:
            raise NoteError(f"the code {code!r} takes no fact {name!r}")
    for name, kind in declared.items():
        if name not in facts:
            raise NoteError(f"the code {code!r} needs the fact {name!r}")
        got = kind_of(facts[name])
        # An empty list is both kinds of list.
        if got != kind and not (facts[name] == [] and kind in ("ints", "strs")):
            raise NoteError(f"the fact {name!r} of {code!r} is {got or 'not a fact'}, and the code takes {kind}")
    if IN_FILE in facts and not isinstance(facts[IN_FILE], str):
        raise NoteError(f"the fact {IN_FILE!r} of {code!r} is not a name")


def sentence(code: str, facts: Facts, language: str = DEFAULT_LANGUAGE) -> Sentence:
    """The sentence `code` says in `language`, with the facts it was given.

    Raises `NoteError` for a code that is not declared, a language with no sentences, or facts that are not the
    ones the code takes.
    """
    if language not in _WRITERS:
        raise NoteError(f"{language!r} is not a language of these sentences ({', '.join(LANGUAGES)})")
    check_facts(code, facts)
    said = _WRITERS[language][code](facts)
    if IN_FILE in facts:
        said = Sentence(f"{facts[IN_FILE]}: {said.what}", said.why)
    return said


class Note(str):
    """A sentence a reader said of what it could not carry, that also says what it is.

    It is the English sentence, as a string, so that everything that reads a note as text reads it; and it
    keeps its code and facts, which are what an extraction stores and what another language is written from.
    A note has no reason of its own (a reader's note is one sentence); a code that gives one is a
    `NotCarried`.
    """

    code: str
    facts: dict[str, Any]

    def __new__(cls, code: str, **facts: Any) -> "Note":
        said = sentence(code, facts)
        if said.why:
            raise NoteError(f"the code {code!r} gives a reason, so it is a NotCarried and not a Note")
        made = super().__new__(cls, said.what)
        made.code = code
        made.facts = dict(facts)
        return made


def listed(items: list[str], most: int = MOST_LISTED) -> str:
    """Clause numbers for an English sentence: all of them, or the first few and how many were left."""
    if len(items) <= most:
        return ", ".join(items)
    return f"{', '.join(items[:most])} and {len(items) - most} more"


def listed_ko(items: list[str], most: int = MOST_LISTED) -> str:
    if len(items) <= most:
        return ", ".join(items)
    return f"{', '.join(items[:most])} 외 {len(items) - most}개"


def paged(numbers: list[int], most: int = MOST_LISTED) -> str:
    """Page numbers for a sentence: the first few, and a mark that there are more."""
    shown = ", ".join(str(n) for n in numbers[:most])
    return shown + (", ..." if len(numbers) > most else "")


# ----------------------------------------------------------------------------------------- a workbook

xlsx_merged = _code("xlsx-merged-ranges", {"count": "int", "sheet": "str"})


@xlsx_merged("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} merged cell range(s) in "
        f"sheet \"{f['sheet']}\""
        " were left as the file stores them -- the value"
        " sits in the first cell and the rest are empty"
    )


@xlsx_merged("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"시트 \"{f['sheet']}\"의 병합된 셀 범위 {f['count']}개를 파일에 저장된 그대로 두었어요."
        " 값은 첫 번째 셀에 있고 나머지는 비어 있어요"
    )


xlsx_pictures = _code("xlsx-pictures", {"pictures": "int", "cells": "int"})


@xlsx_pictures("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['pictures']} picture(s) sit on {f['cells']} cell(s) of this "
        f"workbook (anchored on the cell, or held in it) and are marked "
        f"[picture] there. Which cell holds one is carried; what a picture "
        f"shows is not read, and in a grid a picture can be the whole content "
        f"of its cell"
    )


@xlsx_pictures("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"이 통합 문서의 셀 {f['cells']}개에 그림 {f['pictures']}개가 있어요"
        "(셀에 고정되어 있거나 셀 안에 들어 있어요). 그 자리에 [picture]로 표시했어요."
        " 어느 셀에 그림이 있는지는 옮겼지만 그림이 무엇을 보여 주는지는 읽지 않았어요."
        " 표에서는 그림이 셀의 내용 전부일 수 있어요"
    )


xlsx_rich = _code("xlsx-rich-values", {"count": "int"})


@xlsx_rich("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} cell(s) in this workbook hold a rich value that is not "
        f"a picture (a data type or an object) and keep only the text the "
        f"file stores for them"
    )


@xlsx_rich("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"이 통합 문서의 셀 {f['count']}개에는 그림이 아닌 특수한 값(데이터 형식이나 개체)이 있어요."
        " 파일이 그 셀에 저장한 글만 남겼어요"
    )


xlsx_loose = _code("xlsx-loose-pictures", {"count": "int"})


@xlsx_loose("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} picture(s) in this workbook are not anchored on a cell "
        f"(floating, or inside a group) and were not placed"
    )


@xlsx_loose("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"이 통합 문서의 그림 {f['count']}개는 셀에 고정되어 있지 않아서(떠 있거나 그룹 안에 있어요)"
        " 자리를 정하지 못했어요"
    )


xlsx_other = _code("xlsx-other-objects", {"count": "int"})


@xlsx_other("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} drawn object(s) in this workbook other than pictures "
        f"(shapes, charts) were not read"
    )


@xlsx_other("ko")
def _(f: Facts) -> Sentence:
    return Sentence(f"이 통합 문서에는 그림이 아닌 그려진 개체(도형, 차트) {f['count']}개가 있어요. 읽지 않았어요")


xlsx_drawings = _code("xlsx-unreadable-drawings", {"count": "int", "sheets": "strs"})


@xlsx_drawings("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} drawing(s) referred to by sheet(s) {', '.join(f['sheets'])} of "
        f"this workbook could not be read (the drawing is missing from the file, cannot "
        f"be parsed, or the sheet's links to it cannot be read). Pictures on those "
        f"sheets were not seen, and a cell that reads as empty there may have held one"
    )


@xlsx_drawings("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"이 통합 문서의 시트 {', '.join(f['sheets'])}가 가리키는 그리기 {f['count']}개를 읽지 못했어요"
        "(파일에 그리기가 없거나, 해석할 수 없거나, 시트에서 그리기로 가는 연결을 읽을 수 없어요)."
        " 그 시트의 그림은 보지 못했고, 거기서 비어 보이는 셀에 그림이 있었을 수 있어요"
    )


# ------------------------------------------------------------------------------------ enclosed files

enclosed_workbook = _code("enclosed-workbook-unopened", {"reason": "str"})


@enclosed_workbook("en")
def _(f: Facts) -> Sentence:
    return Sentence(f"an enclosed workbook could not be opened ({f['reason']})")


@enclosed_workbook("ko")
def _(f: Facts) -> Sentence:
    return Sentence(f"첨부된 통합 문서를 열지 못했어요({f['reason']})")


enclosed_deck = _code("enclosed-deck-unopened", {"reason": "str"})


@enclosed_deck("en")
def _(f: Facts) -> Sentence:
    return Sentence(f"an enclosed presentation could not be opened ({f['reason']})")


@enclosed_deck("ko")
def _(f: Facts) -> Sentence:
    return Sentence(f"첨부된 프레젠테이션을 열지 못했어요({f['reason']})")


enclosed_member = _code("enclosed-file-unopened", {"reason": "str"})


@enclosed_member("en")
def _(f: Facts) -> Sentence:
    return Sentence(f"could not be opened ({f['reason']})")


@enclosed_member("ko")
def _(f: Facts) -> Sentence:
    return Sentence(f"열지 못했어요({f['reason']})")


enclosed_objects = _code("enclosed-objects-unopened", {"names": "strs"})


@enclosed_objects("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        "enclosed objects that could not be opened: "
        + ", ".join(f["names"])
        + " -- when a document hands a requirement to an attachment, the "
        "decision logic is in there and not in the text. This is the "
        "loss that looks most like success."
    )


@enclosed_objects("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        "열지 못한 첨부 개체: "
        + ", ".join(f["names"])
        + " -- 문서가 요구사항을 첨부 파일에 넘겼다면 판단 규칙은 본문이 아니라 거기에 있어요."
        " 성공처럼 보이는 가장 위험한 누락이에요."
    )


enclosed_opened = _code("enclosed-files-opened", {"count": "int"})


@enclosed_opened("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['count']} enclosed file(s) WERE opened and their contents "
        f"carried; a merged cell keeps the file's own layout, so a table "
        f"that looked merged on screen reads wider here"
    )


@enclosed_opened("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"첨부 파일 {f['count']}개를 열어서 내용을 옮겼어요. 병합된 셀은 파일 자신의 배치를 그대로 두므로,"
        " 화면에서 병합되어 보이던 표가 여기서는 더 넓게 나와요"
    )


# --------------------------------------------------------------------------------------- a word file

docx_pictures = _code(
    "docx-pictures-unread",
    {
        "pictures": "int",  # every picture the document holds
        "in_body": "int",  # those the body marks where they sit
        "numbered": "bool",  # the document numbers its clauses
        "silent": "strs",  # numbered clauses that state nothing in text and show a picture
        "speaking": "int",  # numbered clauses that state something in text and also show one
        "drawn": "int",  # pictures drawn in the body
        "attributed": "int",  # of those, the ones a numbered clause holds
    },
)


@docx_pictures("en")
def _(f: Facts) -> Sentence:
    if f["in_body"]:
        placed = (
            f" {f['in_body']} of them sit in the body and are named where they sit, as"
            " [picture: NAME]; look at one with the `picture` tool."
        )
        if f["pictures"] > f["in_body"]:
            placed += (
                f" {f['pictures'] - f['in_body']} more are not in the body"
                " (a header, a footer) and are not placed."
            )
    else:
        placed = " None could be named where it sits."
    if not f["numbered"]:
        where = " This document has no numbered clauses, so where they sit could not be answered."
    else:
        where = ""
        if f["silent"]:
            where += (
                f" {len(f['silent'])} numbered clause(s) state nothing in text"
                f" and show only a picture: {listed(f['silent'])}."
                " There, reading the text is not reading the clause."
            )
        if f["speaking"]:
            where += (
                f" {f['speaking']} numbered clause(s) state something in text"
                " and also show a picture."
                " The text does not settle that the picture adds nothing: a clause"
                " can state a condition in text and leave the rule that decides it"
                " to the picture beside it (an example it refers to, a timing, a"
                " range). Look at each picture before relying on the text around it."
            )
    if f["drawn"] and f["attributed"] < f["drawn"]:
        where += f" {f['drawn'] - f['attributed']} of {f['drawn']} sit outside the numbering and were not placed."
    return Sentence(
        f"{f['pictures']} picture(s) were not read"
        " -- this reader makes no model calls, so it cannot say what a picture"
        " shows; it says where each one is." + placed + where
    )


@docx_pictures("ko")
def _(f: Facts) -> Sentence:
    if f["in_body"]:
        placed = (
            f" 그중 {f['in_body']}개는 본문에 있고, 있는 자리에 [picture: NAME]으로 이름을 붙였어요."
            " `picture` 도구로 볼 수 있어요."
        )
        if f["pictures"] > f["in_body"]:
            placed += (
                f" {f['pictures'] - f['in_body']}개는 본문이 아니라(머리글, 바닥글)"
                " 자리를 정하지 못했어요."
            )
    else:
        placed = " 자리에 이름을 붙일 수 있는 그림이 없어요."
    if not f["numbered"]:
        where = " 번호가 붙은 조항이 없는 문서라서 그림이 어느 조항에 있는지는 답할 수 없었어요."
    else:
        where = ""
        if f["silent"]:
            where += (
                f" 번호가 붙은 조항 {len(f['silent'])}개는 글로는 아무것도 말하지 않고 그림만 보여요:"
                f" {listed_ko(f['silent'])}. 그 조항은 글만 읽으면 조항을 읽은 것이 아니에요."
            )
        if f["speaking"]:
            where += (
                f" 번호가 붙은 조항 {f['speaking']}개는 글로도 말하고 그림도 보여요."
                " 글이 그림에 더할 것이 없다고 정하지는 않아요. 조항이 조건은 글로 쓰고 그 조건을"
                " 정하는 규칙은 옆의 그림(참조하는 예, 타이밍, 범위)에 맡길 수 있어요."
                " 글만 믿기 전에 각 그림을 보세요."
            )
    if f["drawn"] and f["attributed"] < f["drawn"]:
        where += f" {f['drawn']}개 중 {f['drawn'] - f['attributed']}개는 번호 체계 밖에 있어서 자리를 정하지 못했어요."
    return Sentence(
        f"그림 {f['pictures']}개를 읽지 않았어요."
        " 이 읽기 도구는 모델을 부르지 않아서 그림이 무엇을 보여 주는지는 말할 수 없고,"
        " 각 그림이 어디에 있는지만 말해요." + placed + where
    )


short_rows = _code("short-rows", {})


@short_rows("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        "rows shorter than their table",
        "the reader wrote them with fewer cells; empty cells were added so the grid is whole",
    )


@short_rows("ko")
def _(f: Facts) -> Sentence:
    return Sentence("표보다 짧은 행", "읽기 도구가 셀을 더 적게 썼어요. 표가 온전하도록 빈 셀을 채웠어요")


# ----------------------------------------------------------------------------------------------- a PDF

pdf_blank = _code("pdf-pages-without-text", {"blank": "ints", "pages": "int"})


@pdf_blank("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{len(f['blank'])} of {f['pages']} page(s) carry no text: {paged(f['blank'])}",
        "each is a picture or empty, and reading pixels is a reader's work, not this extractor's; "
        "an image on it is a figure, and a drawing of lines and shapes is not placed",
    )


@pdf_blank("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['pages']}쪽 중 {len(f['blank'])}쪽에 글이 없어요: {paged(f['blank'])}",
        "각 쪽은 그림이거나 비어 있어요. 픽셀을 읽는 일은 읽기 도구의 몫이고 이 추출기의 몫이 아니에요."
        " 쪽에 있는 이미지는 그림 블록이 되고, 선과 도형으로 그린 그림은 자리를 정하지 않아요",
    )


pdf_unplaced = _code("pdf-pictures-unplaced", {"pictures": "int", "limit": "int", "pages": "ints"})


@pdf_unplaced("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['pictures']} picture(s) past the {f['limit']} a page places, on page(s) {paged(f['pages'])}",
        "counted and not placed as figures; a page of that many images is a drawing cut into strips "
        "and not that many pictures a person can decide about",
    )


@pdf_unplaced("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"한 쪽에 자리를 정하는 상한 {f['limit']}개를 넘은 그림 {f['pictures']}개, 해당 쪽: {paged(f['pages'])}",
        "그림 블록으로 자리를 정하지 않고 수만 세었어요. 이미지가 그렇게 많은 쪽은 조각으로 잘린 그림이고,"
        " 사람이 하나씩 판단할 수 있는 그림의 수가 아니에요",
    )


pdf_unidentified = _code("pdf-pictures-unidentified", {"pictures": "int"})


@pdf_unidentified("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['pictures']} picture(s) whose stored bytes could not be read",
        "each is a figure that names no picture by hash, so a reading cannot tell it from another",
    )


@pdf_unidentified("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"저장된 바이트를 읽지 못한 그림 {f['pictures']}개",
        "각각은 해시로 그림을 가리키지 못하는 그림 블록이라서, 읽는 쪽이 다른 그림과 구별하지 못해요",
    )


pdf_layout = _code("pdf-layout", {})


@pdf_layout("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        "the layout of the pages: tables, columns and the order of what sits side by side",
        "lines are read in the order the document's text is in and gathered by their own spacing; "
        "a table comes out as its lines and its cells are not recovered, so a value may be "
        "separated from the condition it goes with",
    )


@pdf_layout("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        "쪽의 배치: 표, 단, 나란히 놓인 것의 순서",
        "줄은 문서가 가진 글의 순서대로 읽고 줄 사이 간격으로 묶어요."
        " 표는 줄로 나오고 칸을 복원하지 않아서, 값이 그 값에 해당하는 조건과 떨어질 수 있어요",
    )


pdf_drawings = _code("pdf-drawings", {})


@pdf_drawings("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        "drawings of lines and shapes: a diagram that is not an image",
        "only an image the page holds is placed as a figure; a drawing made of strokes is not an "
        "image object, so a page can show one that this extraction does not mention",
    )


@pdf_drawings("ko")
def _(f: Facts) -> Sentence:
    return Sentence(
        "선과 도형으로 그린 그림: 이미지가 아닌 도표",
        "쪽이 가진 이미지만 그림 블록이 돼요. 획으로 그린 그림은 이미지 개체가 아니라서,"
        " 쪽에는 보여도 이 추출에는 나오지 않을 수 있어요",
    )


# ------------------------------------------------------------------------------------ a picture read

picture_incomplete = _code("picture-read-incomplete", {"picture": "str", "runtime": "str", "unreadable": "strs"})


@picture_incomplete("en")
def _(f: Facts) -> Sentence:
    return Sentence(
        f"{f['picture']} was read by {f['runtime']}, which could not read all of it",
        "; ".join(f["unreadable"]),
    )


@picture_incomplete("ko")
def _(f: Facts) -> Sentence:
    # What the model said it could not read is its own words, and is not translated.
    return Sentence(
        f"{f['picture']}: 읽은 도구 {f['runtime']}, 그림 전부를 읽지는 못했어요",
        "; ".join(f["unreadable"]),
    )


def codes() -> tuple[str, ...]:
    """Every code, in the order they were declared."""
    return tuple(FACTS)


def _held() -> None:
    """A code that has a sentence in one language and not in the other does not load."""
    for language in LANGUAGES:
        missing = sorted(set(FACTS) - set(_WRITERS[language]))
        if missing:
            raise NoteError(f"no sentence in {language!r} for the code(s) {', '.join(missing)}")
        extra = sorted(set(_WRITERS[language]) - set(FACTS))
        if extra:
            raise NoteError(f"a sentence in {language!r} for the undeclared code(s) {', '.join(extra)}")


_held()
