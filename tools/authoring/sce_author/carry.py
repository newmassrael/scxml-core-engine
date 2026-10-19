"""A revised document carries the ledger by what is the same, and asks about the rest (ADR 0040).

The customer sends a second version of a document that was divided among specs, and the work of the first
(which paragraph is which spec's) is lost unless it can be carried. It is carried only where that is safe:
a paragraph whose words are the same as the old one's goes where the old one went, and every other paragraph
is *asked*, so that nothing is dropped and nothing is guessed.

**Paragraphs are matched in order, by their words as the check reads them** (`plain_of_block`), so that a
sentence that occurs twice is matched by its place and not by its text alone. A matched paragraph is placed
where its twin was, in every allotment its twin was in (a paragraph shared by two specs stays shared, one left
out stays left out with its reason). What is left over is of three kinds, and each is named by id:

- *changed*: a new paragraph that stands where an old one with no twin stood, between the same two matched
  paragraphs. It is shown beside the old one and is **never carried**: a requirement that moved from "five
  seconds" to "fifty seconds" is exactly what an owner must decide, and no score of likeness says it is not.
  The pairing is by place and nothing else, so it is a pointer to look at and not a verdict.
- *fresh*: a new paragraph with no old one at its place.
- *gone*: an old paragraph that nothing in the new document stands for, with where it had been placed, so an
  owner sees which of the paragraphs they allotted the customer removed.

The draft is a ledger of the new extraction that holds the carried paragraphs only, so the check of it counts
exactly the changed and the fresh ones as unplaced. The report names blocks by id and says no words of the
document, as the other reports of the tools do.

The report is `pre-release`: `Carried.to_dict` is its only producer.
"""

from __future__ import annotations

import difflib
from dataclasses import dataclass
from typing import Any

from .conversion import plain_of_block
from .extraction import Extraction
from .ledger import Allotment, Destination, Ledger

SCHEMA_VERSION = 1


@dataclass(frozen=True)
class Place:
    """Where an old paragraph had been placed: a destination, and why when it was left out."""

    to: Destination
    reason: str = ""

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"to": self.to.to_dict()}
        if self.reason:
            out["reason"] = self.reason
        return out


@dataclass(frozen=True)
class Changed:
    """A new paragraph that stands where an old one stood, and was not carried."""

    block: str
    from_block: str
    was: tuple[Place, ...]

    def to_dict(self) -> dict[str, Any]:
        return {"block": self.block, "from_block": self.from_block, "was": [p.to_dict() for p in self.was]}


@dataclass(frozen=True)
class Gone:
    """An old paragraph that nothing in the new document stands for."""

    block: str
    was: tuple[Place, ...]

    def to_dict(self) -> dict[str, Any]:
        return {"block": self.block, "was": [p.to_dict() for p in self.was]}


@dataclass(frozen=True)
class Carried:
    """The draft ledger of a revised document, and what was asked of the owner."""

    old_extraction: str
    ledger: Ledger
    carried: int
    changed: tuple[Changed, ...]
    fresh: tuple[str, ...]
    gone: tuple[Gone, ...]
    # The destinations of the old ledger, in its order, kept even when no paragraph went to one of them: a
    # spec that received nothing from the revision is still a spec of the division.
    destinations: tuple[Destination, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "v": SCHEMA_VERSION,
            "old_extraction": self.old_extraction,
            "extraction": self.ledger.extraction,
            "ledger": self.ledger.revision(),
            "carried": self.carried,
            "changed": [c.to_dict() for c in self.changed],
            "fresh": list(self.fresh),
            "gone": [g.to_dict() for g in self.gone],
            "destinations": [d.to_dict() for d in self.destinations],
        }


def _words(extraction: Extraction) -> list[tuple[str, str]]:
    """The paragraphs that have words, in order, as (id, the words the check reads)."""
    return [(block.id, plain) for block in extraction.blocks if (plain := plain_of_block(block))]


def carry_ledger(old: Extraction, old_ledger: Ledger, new: Extraction) -> Carried:
    """The ledger of `new` that `old_ledger` of `old` gives, for what is the same, and what is asked about."""
    old_ledger.held_by(old)
    old_words = _words(old)
    new_words = _words(new)

    placed: dict[str, list[Place]] = {}
    for allotment in old_ledger.allotments:
        place = Place(allotment.to, allotment.reason)
        for block_id in allotment.blocks:
            placed.setdefault(block_id, []).append(place)

    matcher = difflib.SequenceMatcher(None, [t for _, t in old_words], [t for _, t in new_words], autojunk=False)
    twin: dict[str, str] = {}  # a new paragraph to the old one it is the same as
    pointers: dict[str, str] = {}  # a new paragraph to the old one that stood at its place, and was changed
    matched_old: set[str] = set()
    anchors = matcher.get_matching_blocks()  # the last is a sentinel of size 0 at the ends
    old_from = new_from = 0
    paired_old: set[str] = set()
    for i, j, size in anchors:
        # Between the previous anchor and this one, the old paragraphs with no twin and the new ones with none
        # stood in the same gap: they are paired by order, as a pointer to what changed.
        for (old_id, _), (new_id, _) in zip(old_words[old_from:i], new_words[new_from:j]):
            pointers[new_id] = old_id
            paired_old.add(old_id)
        for k in range(size):
            twin[new_words[j + k][0]] = old_words[i + k][0]
            matched_old.add(old_words[i + k][0])
        old_from, new_from = i + size, j + size

    allotments: list[Allotment] = []
    for allotment in old_ledger.allotments:
        kept = set(allotment.blocks)
        blocks = tuple(block.id for block in new.blocks if twin.get(block.id) in kept)
        if blocks:
            allotments.append(Allotment(allotment.to, blocks, allotment.reason))

    changed = tuple(
        Changed(new_id, pointers[new_id], tuple(placed.get(pointers[new_id], ())))
        for new_id, _ in new_words
        if new_id in pointers
    )
    fresh = tuple(new_id for new_id, _ in new_words if new_id not in twin and new_id not in pointers)
    gone = tuple(
        Gone(old_id, tuple(placed.get(old_id, ())))
        for old_id, _ in old_words
        if old_id not in matched_old and old_id not in paired_old
    )
    destinations: list[Destination] = []
    for allotment in old_ledger.allotments:
        if allotment.to not in destinations:
            destinations.append(allotment.to)
    return Carried(
        old_extraction=old.revision(),
        ledger=Ledger(new.revision(), tuple(allotments)),
        carried=len(twin),
        changed=changed,
        fresh=fresh,
        gone=gone,
        destinations=tuple(destinations),
    )
