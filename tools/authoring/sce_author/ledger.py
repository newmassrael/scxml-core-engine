"""The ledger of a divided document: where each paragraph of it went (ADR 0040).

A document that holds several specs is not refused and is not imported in one piece: it is divided, and
the division is written down as a ledger. A ledger is a list of allotments, each a destination (a spec
that exists, a spec to be made, or *left out*) and the blocks of the extraction that go there. It is the
owner's, as a scope is (ADR 0039), and it is held the way a conversion is (ADR 0026): the one who made the
allotments does not decide that nothing was missed. `check_ledger` counts, by id, which paragraphs went to
no place, which went both into a spec and out of the document, and which went to several specs, and the
ledger is complete only when none is unplaced and none contradicts.

**The ledger is made from parts, and the tools turn them into blocks.** A screen shows the outline of the
extraction (`scope.outline`) and sends, for each destination, the ids of the parts it chose; `make_ledger`
expands them to the blocks they are, so that no host works out a block range.

**What each spec is given is the allotment as a scope.** `scopes_of` gives, for each spec allotment, the
scope (ADR 0039) its proposal is made under, or none when the allotment is the whole extraction, which is no
scope. It refuses a ledger that is not complete: a confirm made of it would leave paragraphs behind.

The ledger's form, the plan it is made from and the report of its check are `pre-release`: `to_dict` and
`from_dict` are their only producers and readers.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass, field
from typing import Any

from .conversion import plain_of_block
from .errors import IngestError
from .extraction import Extraction
from .scope import Scope, scope_of

SCHEMA_VERSION = 1
SCHEMA_STATUS = "pre-release"

SPEC = "spec"
NEW = "new"
LEFT_OUT = "left-out"
KINDS = (SPEC, NEW, LEFT_OUT)

# The id of a spec the specs folder keeps: what `SpecId::parse` of the store accepts.
_SPEC_ID = re.compile(r"[a-z0-9][a-z0-9-]{0,79}")
MAX_TITLE = 200
MAX_REASON = 1000


def _refuse(what: str) -> IngestError:
    return IngestError(f"a ledger {what}")


def _object(raw: Any, what: str, known: set[str]) -> dict[str, Any]:
    if not isinstance(raw, dict):
        raise _refuse(f"{what} is not a JSON object")
    unknown = sorted(set(raw) - known)
    if unknown:
        raise _refuse(f"{what} has a field nobody defined ({', '.join(unknown)})")
    return raw


@dataclass(frozen=True)
class Destination:
    """Where an allotment goes: a spec that exists, a spec to be made, or out of the document."""

    kind: str
    spec: str | None = None  # the id of a spec that exists (kind `spec`)
    title: str | None = None  # the title of a spec to be made (kind `new`)

    def __post_init__(self) -> None:
        if self.kind not in KINDS:
            raise _refuse(f"has a destination of kind {self.kind!r}, which is not one of {', '.join(KINDS)}")
        if self.kind == SPEC:
            if self.title is not None or not isinstance(self.spec, str) or not _SPEC_ID.fullmatch(self.spec):
                raise _refuse("names a spec by something that is not a spec's id (lower case letters, digits and hyphens)")
        elif self.kind == NEW:
            if self.spec is not None or not isinstance(self.title, str) or not self.title.strip():
                raise _refuse("makes a spec with no title")
            if len(self.title) > MAX_TITLE:
                raise _refuse(f"gives a spec to be made a title of more than {MAX_TITLE} characters")
        elif self.spec is not None or self.title is not None:
            raise _refuse("names a spec for what is left out")

    def to_dict(self) -> dict[str, Any]:
        if self.kind == SPEC:
            return {"kind": SPEC, "spec": self.spec}
        if self.kind == NEW:
            return {"kind": NEW, "title": self.title}
        return {"kind": LEFT_OUT}

    @classmethod
    def from_dict(cls, raw: Any) -> "Destination":
        data = _object(raw, "a destination", {"kind", "spec", "title"})
        kind = data.get("kind")
        if not isinstance(kind, str):
            raise _refuse("has a destination with no kind")
        return cls(kind, data.get("spec"), data.get("title"))


@dataclass(frozen=True)
class Allotment:
    """The blocks that go to one destination, in the extraction's order."""

    to: Destination
    blocks: tuple[str, ...]
    # Why a part was left out. Asked for, and allowed to be empty: leaving a part out is an act, and an empty
    # reason is still that act. Only what is left out has one.
    reason: str = ""

    def __post_init__(self) -> None:
        if not self.blocks:
            raise _refuse("has an allotment of no block: a destination with nothing for it is not one")
        if any(not isinstance(b, str) or not b for b in self.blocks):
            raise _refuse("names a block with no id")
        if len(set(self.blocks)) != len(self.blocks):
            raise _refuse("names a block twice in one allotment")
        if not isinstance(self.reason, str):
            raise _refuse("gives a reason that is not text")
        if self.reason and self.to.kind != LEFT_OUT:
            raise _refuse("gives a reason for a part that is not left out")
        if len(self.reason) > MAX_REASON:
            raise _refuse(f"gives a reason of more than {MAX_REASON} characters")

    @property
    def identity(self) -> tuple[str, str]:
        """What makes two allotments the same destination: a spec by its id, a new one by its title, and what is
        left out by its reason (parts left out for different reasons are different allotments)."""
        if self.to.kind == SPEC:
            return (SPEC, self.to.spec or "")
        if self.to.kind == NEW:
            return (NEW, self.to.title or "")
        return (LEFT_OUT, self.reason)

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"to": self.to.to_dict(), "blocks": list(self.blocks)}
        if self.to.kind == LEFT_OUT:
            out["reason"] = self.reason
        return out

    @classmethod
    def from_dict(cls, raw: Any) -> "Allotment":
        data = _object(raw, "an allotment", {"to", "reason", "blocks"})
        blocks = data.get("blocks")
        if not isinstance(blocks, list) or not all(isinstance(b, str) for b in blocks):
            raise _refuse("does not hold an allotment's blocks as a list of ids")
        return cls(Destination.from_dict(data.get("to")), tuple(blocks), data.get("reason", ""))


_RANK = {SPEC: 0, NEW: 1, LEFT_OUT: 2}


@dataclass(frozen=True)
class Ledger:
    """Where every paragraph of one extraction went. One allotment to a destination, in a fixed order, so that
    the same division has the one revision however the screen listed it."""

    extraction: str
    allotments: tuple[Allotment, ...] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        if not isinstance(self.extraction, str) or not re.fullmatch(r"[0-9a-f]{64}", self.extraction):
            raise _refuse("does not name the extraction it is of by its revision (64 hexadecimal digits)")
        ordered = tuple(sorted(self.allotments, key=lambda a: (_RANK[a.to.kind], a.identity[1])))
        seen: set[tuple[str, str]] = set()
        for allotment in ordered:
            if allotment.identity in seen:
                raise _refuse(f"has two allotments to the same destination ({allotment.identity[1] or 'left out'})")
            seen.add(allotment.identity)
        object.__setattr__(self, "allotments", ordered)

    def to_dict(self) -> dict[str, Any]:
        return {
            "v": SCHEMA_VERSION,
            "extraction": self.extraction,
            "allotments": [a.to_dict() for a in self.allotments],
        }

    def to_json(self) -> str:
        """The canonical text a ledger is named by: sorted keys, no spaces, UTF-8."""
        return json.dumps(self.to_dict(), sort_keys=True, separators=(",", ":"), ensure_ascii=False)

    def revision(self) -> str:
        return hashlib.sha256(self.to_json().encode("utf-8")).hexdigest()

    @classmethod
    def from_dict(cls, raw: Any) -> "Ledger":
        data = _object(raw, "a ledger", {"v", "extraction", "allotments"})
        if data.get("v") != SCHEMA_VERSION:
            raise _refuse(f"is of a version this tool does not read (it reads {SCHEMA_VERSION})")
        allotments = data.get("allotments")
        if not isinstance(allotments, list):
            raise _refuse("does not hold its allotments as a list")
        return cls(data.get("extraction"), tuple(Allotment.from_dict(a) for a in allotments))  # type: ignore[arg-type]

    @classmethod
    def from_json(cls, text: str) -> "Ledger":
        try:
            raw = json.loads(text)
        except json.JSONDecodeError as exc:
            raise _refuse(f"is not JSON ({exc})") from exc
        return cls.from_dict(raw)

    def held_by(self, extraction: Extraction) -> "Ledger":
        """This ledger, once it is known to be of `extraction`: its blocks are there, in its order."""
        if self.extraction != extraction.revision():
            raise IngestError(
                f"the ledger is of the extraction {self.extraction[:12]}, not of this one "
                f"({extraction.revision()[:12]}): a division is not carried to another document"
            )
        order = {block.id: index for index, block in enumerate(extraction.blocks)}
        for allotment in self.allotments:
            missing = [b for b in allotment.blocks if b not in order]
            if missing:
                raise IngestError(f"the ledger names {', '.join(missing[:5])}, which the extraction does not have")
            places = [order[b] for b in allotment.blocks]
            if places != sorted(places):
                raise IngestError("the ledger's blocks are not in the order the extraction has them")
        return self


# ----------------------------------------------------------------------------------------- the check


@dataclass(frozen=True)
class DestinationCount:
    to: Destination
    reason: str
    blocks: int
    with_words: int

    def to_dict(self) -> dict[str, Any]:
        out: dict[str, Any] = {"to": self.to.to_dict(), "blocks": self.blocks, "with_words": self.with_words}
        if self.to.kind == LEFT_OUT:
            out["reason"] = self.reason
        return out


@dataclass(frozen=True)
class LedgerReport:
    """SCE's count of a ledger. A ledger cannot make this for itself."""

    extraction: str
    ledger: str
    words: int  # blocks with words in the extraction: what has to have a place
    in_specs: int  # of them, those in at least one spec's allotment
    left_out: int  # of them, those that are left out and in no spec
    unplaced: tuple[str, ...]
    contradicting: tuple[str, ...]
    shared: tuple[str, ...]
    empty: tuple[int, ...]  # indexes (in the ledger's order) of spec destinations with no paragraph with words
    destinations: tuple[DestinationCount, ...]

    @property
    def complete(self) -> bool:
        """Every paragraph with words has a place, none is both used and left out, and no spec is given nothing."""
        return not self.unplaced and not self.contradicting and not self.empty

    def to_dict(self) -> dict[str, Any]:
        return {
            "extraction": self.extraction,
            "ledger": self.ledger,
            "complete": self.complete,
            "words": self.words,
            "in_specs": self.in_specs,
            "left_out": self.left_out,
            "unplaced": list(self.unplaced),
            "contradicting": list(self.contradicting),
            "shared": list(self.shared),
            "empty": list(self.empty),
            "destinations": [d.to_dict() for d in self.destinations],
        }


def check_ledger(extraction: Extraction, ledger: Ledger) -> LedgerReport:
    """Count where the paragraphs of `extraction` went under `ledger`.

    It is refused, and no report is made, when the ledger is of another extraction or names a block it does not
    have: a count against the wrong document says nothing, and reads like a count. A block with no words (an
    empty cell row) has nothing to place, so it is not unplaced, though an allotment may hold it.
    """
    ledger.held_by(extraction)
    wordy = {block.id for block in extraction.blocks if plain_of_block(block)}
    specs: dict[str, int] = {}
    out: set[str] = set()
    counts: list[DestinationCount] = []
    empty: list[int] = []
    for index, allotment in enumerate(ledger.allotments):
        with_words = [b for b in allotment.blocks if b in wordy]
        counts.append(DestinationCount(allotment.to, allotment.reason, len(allotment.blocks), len(with_words)))
        if allotment.to.kind == LEFT_OUT:
            out.update(allotment.blocks)
            continue
        if not with_words:
            empty.append(index)
        for block_id in allotment.blocks:
            specs[block_id] = specs.get(block_id, 0) + 1
    order = [block.id for block in extraction.blocks]
    placed = {b for b in order if b in wordy and (b in specs or b in out)}
    return LedgerReport(
        extraction=extraction.revision(),
        ledger=ledger.revision(),
        words=len(wordy),
        in_specs=sum(1 for b in order if b in wordy and b in specs),
        left_out=sum(1 for b in order if b in wordy and b in out and b not in specs),
        unplaced=tuple(b for b in order if b in wordy and b not in placed),
        contradicting=tuple(b for b in order if b in specs and b in out),
        shared=tuple(b for b in order if specs.get(b, 0) > 1),
        empty=tuple(empty),
        destinations=tuple(counts),
    )


# ---------------------------------------------------------------------------- from parts, and to scopes


def make_ledger(extraction: Extraction, plan: Any, by: str | None = None) -> Ledger:
    """The ledger of a plan: for each destination, the parts of the outline it was given.

    The plan is `{"v": 1, "allotments": [{"to": ..., "reason": ..., "sections": [...], "trees": [...]}]}`:
    `sections` are the ids of parts taken as their own blocks and `trees` the ids of parts taken with every part
    under them (ADR 0039), as `make-scope` takes them. An allotment of no part is refused.
    """
    data = _object(plan, "a plan", {"v", "allotments"})
    if data.get("v") != SCHEMA_VERSION:
        raise _refuse(f"has a plan of a version this tool does not read (it reads {SCHEMA_VERSION})")
    entries = data.get("allotments")
    if not isinstance(entries, list):
        raise _refuse("has a plan whose allotments are not a list")
    allotments: list[Allotment] = []
    for raw in entries:
        entry = _object(raw, "a planned allotment", {"to", "reason", "sections", "trees"})
        sections, trees = entry.get("sections", []), entry.get("trees", [])
        for name, ids in (("sections", sections), ("trees", trees)):
            if not isinstance(ids, list) or not all(isinstance(i, str) for i in ids):
                raise _refuse(f"has a planned allotment whose {name} are not a list of part ids")
        if not sections and not trees:
            raise _refuse("has a planned allotment that names no part")
        blocks = scope_of(extraction, list(sections), by, list(trees)).blocks
        allotments.append(Allotment(Destination.from_dict(entry.get("to")), blocks, entry.get("reason", "")))
    return Ledger(extraction.revision(), tuple(allotments))


def scopes_of(extraction: Extraction, ledger: Ledger) -> list[tuple[Allotment, Scope | None]]:
    """The scope each spec allotment is imported under, in the ledger's order; `None` where the allotment is
    every block of the extraction, which is no scope (ADR 0039). What is left out has none.

    Refused for a ledger that is not complete.
    """
    report = check_ledger(extraction, ledger)
    if not report.complete:
        raise IngestError(
            f"the ledger is not complete ({len(report.unplaced)} unplaced, {len(report.contradicting)} "
            f"contradicting, {len(report.empty)} spec(s) with nothing for them): nothing is made of it"
        )
    made: list[tuple[Allotment, Scope | None]] = []
    for allotment in ledger.allotments:
        if allotment.to.kind == LEFT_OUT:
            continue
        whole = len(allotment.blocks) == len(extraction.blocks)
        made.append((allotment, None if whole else Scope(extraction.revision(), allotment.blocks)))
    return made
