"""A division drawn by a model: what it is asked, and how its answer is taken (ADR 0040, ADR 0036).

This is the kind of work "divide a document", whole and without a model in it, as `conversion_by_model` is for
converting. What is here is a pure function of its inputs: the recipe a model is told, the prompt made of an
outline and the destinations the owner has, and the reading of what the model answered. What is not here is any
way of reaching a model: that is a runtime's, and a runtime knows nothing of this kind, since it is given a
prompt and returns text.

**The model is asked where each part goes, and it may decline.** It is given the parts of the outline (each with
its id, title, size and its first words, and never the whole text) and the destinations the owner has so far,
and answers for each part it is sure of either a destination by its key or a new one by a title. A part it does
not answer is *unplaced*, and that is an answer: the owner places it. It is never offered *left out*, since
whether a part is a requirement is the owner's to say, and an answer that tried to say so is refused as such.

**What it says is a claim, and nothing is placed on its authority.** A draft is suggestions: a part it names
that the outline has not, a destination nobody offered, a part placed twice, or a new title that is blank or too
long is a refused item, named with a typed reason, and the others are kept, so one wrong item costs one item.
The owner sees the draft and confirms or moves each part, and what they confirm is counted by SCE's check of a
ledger as it is for a division they drew themselves; a wrong suggestion costs a proposal that is reviewed
paragraph by paragraph and written to no spec's text until accepted.

**The unit is the whole outline, or nothing.** The parts are listed by their first words and not their text, so
an outline of a few hundred parts is a prompt of a few thousand words, and a document with more parts than that
is refused with a typed reason (`too-many-parts`) instead of being cut in a way that has not been measured: a
batch would have to be told the destinations earlier batches invented. Whether and how to cut it is decided by
the number measured on documents (`eval/divide_draft.py`), not before.

The text a model is given and the form it answers in are `pre-release`, as the conversion's form is.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any

from .conversion import plain_of_block
from .errors import IngestError
from .extraction import Extraction
from .scope import Section, outline

KIND = "divide-document"
RECIPE_VERSION = "1"
SCHEMA_VERSION = 1

# How many characters of a part's own words are shown to the model: enough to tell what it is about.
FIRST_WORDS = 160

# The most characters a prompt may hold, and so the most parts it may list.
PROMPT_CHARACTERS = 40_000

MAX_TITLE = 200
MAX_DESTINATIONS = 40
_KEY = re.compile(r"[A-Za-z0-9_-]{1,32}")

NO_ANSWER = "no-answer"
NOT_JSON = "not-json"
EMPTY = "empty"
BAD_SHAPE = "bad-shape"
UNKNOWN_PART = "unknown-part"
UNKNOWN_DESTINATION = "unknown-destination"
PLACED_TWICE = "placed-twice"
BAD_TITLE = "bad-title"
LEFT_OUT = "left-out"
TOO_MANY_PARTS = "too-many-parts"


def _refuse(what: str) -> IngestError:
    return IngestError(f"a division draft {what}")


@dataclass(frozen=True)
class Offered:
    """A destination the owner has, as the model is told of it: a key to answer by, and what it is called."""

    key: str
    kind: str  # "spec" (a spec that exists) or "new" (a spec to be made)
    title: str

    def __post_init__(self) -> None:
        if not _KEY.fullmatch(self.key):
            raise _refuse("offers a destination whose key is not 1 to 32 letters, digits, `_` or `-`")
        if self.kind not in ("spec", "new"):
            raise _refuse(f"offers a destination of kind {self.kind!r}, which is not `spec` or `new`")
        if not self.title.strip() or len(self.title) > MAX_TITLE:
            raise _refuse(f"offers a destination with no title, or one of more than {MAX_TITLE} characters")


def offered_from(raw: Any) -> tuple[Offered, ...]:
    """The destinations of a file `{"v": 1, "destinations": [{"key", "kind", "title"}, ...]}`."""
    if not isinstance(raw, dict) or set(raw) - {"v", "destinations"}:
        raise _refuse("is given destinations that are not an object of `v` and `destinations`")
    if raw.get("v") != SCHEMA_VERSION:
        raise _refuse(f"is given destinations of a version this tool does not read (it reads {SCHEMA_VERSION})")
    items = raw.get("destinations")
    if not isinstance(items, list) or not items:
        raise _refuse("is given no destination, so there is nothing to divide among")
    if len(items) > MAX_DESTINATIONS:
        raise _refuse(f"is given more than {MAX_DESTINATIONS} destinations")
    found: list[Offered] = []
    for item in items:
        if not isinstance(item, dict) or set(item) != {"key", "kind", "title"}:
            raise _refuse("is given a destination that is not an object of `key`, `kind` and `title`")
        if not all(isinstance(item[name], str) for name in ("key", "kind", "title")):
            raise _refuse("is given a destination whose key, kind or title is not text")
        found.append(Offered(item["key"], item["kind"], item["title"]))
    if len({d.key for d in found}) != len(found):
        raise _refuse("is given two destinations with one key")
    return tuple(found)


RECIPE = f"""You are helping to divide one document among several specifications. (recipe {KIND} {RECIPE_VERSION})

Below are the specifications the parts may go to, and then the parts of the document, each with its id, its
title, its size and its first words.

Rules:
1. For each part, say which specification it belongs to, by its key, or propose a new specification with a
   short title. A part belongs to a specification only when what it says is about that specification's work.
2. If you are not sure where a part goes, do not answer for it. Leaving a part out of your answer is the right
   answer when you are unsure; the owner will place it.
3. Do not decide that a part is unwanted, and do not summarise, rewrite or judge any of the words. Every part
   you do not place is decided by the owner.
4. A part goes to one specification. When you propose a new one, use exactly the same title for every part that
   goes to it.

Answer with a JSON array and nothing else, one object for each part you place, naming a specification by its
key with `spec`, or a new one by its title with `new`:
[{{"part": "s2", "spec": "d0"}}, {{"part": "s5", "new": "Door lock"}}]
"""


@dataclass(frozen=True)
class Prepared:
    """What a model is given: the text, and the ids of the parts it lists, in the outline's order."""

    prompt: str
    parts: tuple[str, ...]


def _first_words(extraction: Extraction, section: Section) -> str:
    """The first words of a part's own paragraphs, after its heading, as one line."""
    words: list[str] = []
    size = 0
    for block in extraction.blocks[section.first:section.end]:
        if block.kind == "heading" and block.text.strip() == section.title.strip():
            continue
        plain = " ".join(plain_of_block(block).split())
        if not plain:
            continue
        words.append(plain)
        size += len(plain) + 1
        if size >= FIRST_WORDS:
            break
    line = " ".join(words)
    return line if len(line) <= FIRST_WORDS else line[: FIRST_WORDS - 1].rstrip() + "…"


def _line(extraction: Extraction, section: Section) -> str:
    title = section.title.strip() or f"({section.kind})"
    head = f"[{section.id}] level {section.level} \"{title}\" ({section.blocks} paragraphs, {section.characters} characters)"
    words = _first_words(extraction, section)
    # A part that is only its heading has no words after it to show.
    return f"{head}: {words}" if words else head


def prepare(extraction: Extraction, destinations: tuple[Offered, ...], by: str | None = None) -> Prepared:
    """The prompt for dividing `extraction` among `destinations`, and the parts it lists.

    Every part with a paragraph that has words is listed, a heading counting as one: it is a paragraph the check
    of a ledger asks a place for. Refused when fewer than two parts are listed (there is nothing to divide) and
    when the outline is too long for one prompt, since a cut that has not been measured would be a guess.
    """
    if not destinations:
        raise _refuse("is given no destination, so there is nothing to divide among")
    _, sections = outline(extraction, by)
    listed = [s for s in sections if s.blocks > 0]
    if len(listed) < 2:
        raise _refuse("is asked of a document with fewer than two parts that hold words, so there is nothing to divide")
    destination_lines = "\n".join(
        f"[{d.key}] {d.title} ({'a specification that exists' if d.kind == 'spec' else 'a specification to be made'})"
        for d in destinations
    )
    part_lines = "\n".join(_line(extraction, s) for s in listed)
    prompt = f"{RECIPE}\nSpecifications:\n{destination_lines}\n\nParts:\n{part_lines}\n"
    if len(prompt) > PROMPT_CHARACTERS:
        raise IngestError(
            f"[{TOO_MANY_PARTS}] the outline has {len(listed)} parts, more than one prompt of "
            f"{PROMPT_CHARACTERS} characters holds; place the parts by hand"
        )
    return Prepared(prompt, tuple(s.id for s in listed))


@dataclass(frozen=True)
class Suggestion:
    """A part the model placed: at a destination that was offered (`key`), or at a new one (`title`)."""

    part: str
    key: str | None = None
    title: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {"part": self.part, "key": self.key, "title": self.title}


@dataclass(frozen=True)
class Refused:
    """One item of an answer that was not taken, and why, as an answer and not an error."""

    item: int  # which item of the answer, from 1
    code: str
    why: str

    def to_dict(self) -> dict[str, Any]:
        return {"item": self.item, "code": self.code, "why": self.why}


@dataclass(frozen=True)
class Draft:
    """What the model's answer is, read: the suggestions, what was not taken and the parts left unplaced."""

    extraction: str
    suggestions: tuple[Suggestion, ...]
    refused: tuple[Refused, ...]
    unplaced: tuple[str, ...]  # the parts listed to the model that it did not place, in the outline's order
    # Set when the whole answer could not be read: no suggestion, and the reason.
    unreadable: Refused | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "v": SCHEMA_VERSION,
            "kind": KIND,
            "recipe": RECIPE_VERSION,
            "extraction": self.extraction,
            "suggestions": [s.to_dict() for s in self.suggestions],
            "refused": [r.to_dict() for r in self.refused],
            "unplaced": list(self.unplaced),
            "unreadable": None if self.unreadable is None else self.unreadable.to_dict(),
        }


# Leaving a part out is the owner's act and not a place the model may choose. What a model writes to say it, as
# a key of an item or as the destination or title it names, is refused as that, so that the draft never decides
# it by another name.
_OUT_WORDS = frozenset({
    "out", "left out", "left-out", "left_out", "leave out", "leave-out", "leave_out", "leave", "omit", "skip",
    "ignore", "exclude", "unwanted", "drop", "not a requirement",
})
LEFT_OUT_WHY = "leaving a part out is the owner's to decide, not the draft's"


def _says_out(word: Any) -> bool:
    return isinstance(word, str) and word.strip().casefold() in _OUT_WORDS


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


def read_answer(
    extraction: Extraction,
    prepared: Prepared,
    destinations: tuple[Offered, ...],
    text: str | None,
) -> Draft:
    """A model's answer, taken item by item against what it was told.

    The first mention of a part is kept; a later one is refused. A title that is a destination's own title (any
    case) is that destination, so a model that proposes "new" what the owner already has places it where it
    belongs; two new titles that are one title in another case are one new destination.
    """
    revision = extraction.revision()
    listed = set(prepared.parts)
    keys = {d.key for d in destinations}
    by_title = {d.title.strip().casefold(): d.key for d in destinations}

    def unreadable(code: str, why: str) -> Draft:
        return Draft(revision, (), (), prepared.parts, Refused(0, code, why))

    if text is None or not text.strip():
        return unreadable(NO_ANSWER, "there is no answer")
    try:
        items = _first_array(text)
    except ValueError:
        return unreadable(NOT_JSON, "the answer holds no JSON array")
    if not items:
        return unreadable(EMPTY, "the answer is an empty list, so nothing was placed")

    suggestions: list[Suggestion] = []
    refused: list[Refused] = []
    seen: set[str] = set()
    new_titles: dict[str, str] = {}  # casefolded title to the title as first written
    for index, item in enumerate(items, start=1):
        if not isinstance(item, dict) or "part" not in item or not isinstance(item["part"], str):
            refused.append(Refused(index, BAD_SHAPE, "the item is not an object with a `part`"))
            continue
        extra = set(item) - {"part", "spec", "new"}
        if any(_says_out(name) for name in extra):
            refused.append(Refused(index, LEFT_OUT, LEFT_OUT_WHY))
            continue
        if extra or (("spec" in item) == ("new" in item)):
            refused.append(Refused(index, BAD_SHAPE, "the item names neither or both of `spec` and `new`, or says something else"))
            continue
        part = item["part"]
        if part not in listed:
            refused.append(Refused(index, UNKNOWN_PART, f"the outline listed no part {part!r}"))
            continue
        if part in seen:
            refused.append(Refused(index, PLACED_TWICE, f"part {part} is placed a second time"))
            continue
        if "spec" in item:
            key = item["spec"]
            if isinstance(key, str) and key in keys:
                seen.add(part)
                suggestions.append(Suggestion(part, key=key))
            elif _says_out(key):
                refused.append(Refused(index, LEFT_OUT, LEFT_OUT_WHY))
            else:
                refused.append(Refused(index, UNKNOWN_DESTINATION, "the destination was not one of those offered"))
            continue
        title = item["new"]
        if _says_out(title):
            refused.append(Refused(index, LEFT_OUT, LEFT_OUT_WHY))
            continue
        if not isinstance(title, str) or not title.strip() or len(title) > MAX_TITLE:
            refused.append(Refused(index, BAD_TITLE, "a new specification has no title, or one that is too long"))
            continue
        folded = title.strip().casefold()
        seen.add(part)
        if folded in by_title:
            suggestions.append(Suggestion(part, key=by_title[folded]))
        else:
            suggestions.append(Suggestion(part, title=new_titles.setdefault(folded, title.strip())))
    placed = {s.part for s in suggestions}
    unplaced = tuple(p for p in prepared.parts if p not in placed)
    return Draft(revision, tuple(suggestions), tuple(refused), unplaced)
