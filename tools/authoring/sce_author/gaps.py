"""What the specification does not say, as the run found it.

A document written from a specification records every place its author had to
decide something the text did not: `sce:assumed` in the document, `assumed` on
a binding rule, `sce:unresolved` / `unresolved` where no answer could even be
guessed, and the pack's own readings of preconditions. `verify` then runs the
product's cases. Put together, those two say, for each gap, which of three
very different things is true:

    refuted    the product's tests answer it, and the answer is not the guess
    implicated a case fails where it and other guesses decide together --
               at least one of them is wrong, the cases do not say which
    held       the tests agree with the guess -- the text should still say so
    untested   nothing compares it -- neither the text nor the tests decide it

⚠ WHY THIS IS A REPORT OF ITS OWN. `verify` already named a refuted guess
beside the failure it caused. What it did not say is the other two, and they
are the larger part: a guess no case ever compared reads, in a verdict of "all
passed", exactly like a guess that was checked. A reader asking "where is the
specification incomplete, and what should it say" had to reconstruct that
from a failure list, a document's attributes and a binding by hand -- on
2026-09-27 one component's thirty gaps were listed that way, and the list
could not say which of them no test had looked at.

⚠ Nothing here quotes the specification or the cases beyond the handful of
values a refutation is ABOUT. Where a gap sits in the text is given as file
and line, so the report can travel further than the specification may.
"""

from __future__ import annotations

from dataclasses import dataclass, field

# Most urgent first. A refutation is a known wrong answer; an untested guess
# is an unknown one, which is worse than an open question only because nobody
# is asking it.
ORDER = ("refuted", "implicated", "untested", "open", "question", "untestable",
         "held")

FIX = {
    "refuted": ("The product's tests answer this, and not as guessed. The "
                "specification should state what they expect."),
    "implicated": ("A case fails at a position this guess decides together "
                   "with the others named; at least one of them is wrong, and "
                   "the cases do not say which. The specification should "
                   "settle each."),
    "untested": ("No case compares a position resting on this guess. Neither "
                 "the specification nor the tests decide it; both should."),
    "open": ("No answer could even be guessed. The specification, or the "
             "interface it is written against, must name it."),
    "question": "Answer it in the specification.",
    "untestable": ("Read as a constant, which no case can move. The "
                   "specification should state what this condition means."),
    "held": ("The tests agree with the guess. The specification should still "
             "say it, so the next reader does not have to guess again."),
}


@dataclass
class Gap:
    kind: str
    subject: str
    reason: str
    marker: str = ""
    positions: list = field(default_factory=list)
    # (file, line, the name found there): where the text touches it.
    where: list = field(default_factory=list)
    # refuted: (case, address, expected, got); implicated: the same plus the
    # other guesses; held: case names.
    evidence: list = field(default_factory=list)
    # Cases in which positions resting on this guess agreed. Carried for
    # every status: an implicated guess that also agreed in forty cases is a
    # weaker suspect than one that agreed in none, and the reader ranks them.
    agreed: int = 0

    @property
    def fix(self) -> str:
        return FIX[self.kind]

    def as_dict(self) -> dict:
        return {"kind": self.kind, "subject": self.subject, "marker": self.marker,
                "reason": self.reason, "positions": list(self.positions),
                "where": [{"file": str(f), "line": n, "name": name}
                          for f, n, name in self.where],
                "evidence": [list(e) if isinstance(e, tuple) else e
                             for e in self.evidence],
                "agreed": self.agreed,
                "fix": self.fix}


def _where(positions, model, prose) -> list:
    """Where the specification first names each position's address."""
    if prose is None:
        return []
    found = []
    for position in positions:
        entry = model.owning(position)
        for name in (entry.names if entry else ()):
            hit = prose.locate(name)
            if hit and (hit[0], hit[1], name) not in found:
                found.append((hit[0], hit[1], name))
                break
    return found


def report(verification, pack, prose=None, questions=()) -> list[Gap]:
    """Every gap the run and the prose expose, most urgent first.

    `questions` are `questions.ask`'s answers for the same prose, when the
    caller asked for them: what the text leaves open before anything is run.
    """
    gaps = []
    for a in verification.assumptions.values():
        gaps.append(Gap(
            kind=a.status, subject=a.subject, reason=a.reason, marker=a.marker,
            positions=list(a.positions),
            where=_where(a.positions, pack.model, prose),
            evidence={"refuted": a.refuted_by,
                      "implicated": a.implicated_by}.get(a.status, a.held_in)[:],
            agreed=len(a.held_in)))
    for name, why in sorted(verification.unresolved.items()):
        gaps.append(Gap(kind="open", subject=f"input {name}", reason=why))
    for name, why in sorted(verification.unresolved_outputs.items()):
        gaps.append(Gap(kind="open", subject=f"output {name}", reason=why))
    for name, why in sorted(verification.unaddressed_outputs.items()):
        gaps.append(Gap(kind="open", subject=f"output {name}", reason=why))
    for phrase, held in sorted(verification.assumed_preconditions.items()):
        gaps.append(Gap(kind="untestable", subject=f"precondition {phrase!r}",
                        reason=f"read as {held['expression']!r}: {held['reason']}"))
    for q in questions:
        gaps.append(Gap(kind="question", subject=f"{q.kind}: {q.subject}",
                        reason=q.detail,
                        where=[(q.file, q.line, q.subject)] if q.file else []))
    return sorted(gaps, key=lambda g: ORDER.index(g.kind))
