"""Hold a revision to the reach of what changed: judge it, hand what is out of reach back to
whoever revised it, judge again, and say it is within reach only when the judgment of the design as
it stands says so (`docs/adr/0013-a-revision-is-held-to-its-reach-by-a-loop-its-caller-runs.md`).

`scxml_revision_check` is a report. Nothing in the product refuses a design for it, and the model
that revises runs in the owner's client, which SCE does not control (README, "When the same
specification is drafted more than once"): asking it to change only what the delta says moved is a
request, and a request can be ignored. What can be held is the RESULT. This is the loop a caller
runs around its client, and it never calls a revision within reach when the judgment did not.

⚠ Four ways a loop of this kind lies, each closed here:

  - success on a verdict that saw nothing. `summary.seen` 0 means no requirement was compared
    (no node cites one, or the kind of document has nowhere to), so `within-reach` says nothing
    about the design: the outcome is `not-judged`, never a pass;
  - success because the reviser said it was done. Only the judgment of the design as it stands,
    made again after every revision, decides; what the reviser reports is not read;
  - success against a baseline the reviser moved. The acceptance record is what the design is
    judged against, and a reviser that can write to it can make any design pass by replacing it
    with the record of that design. `keep_baseline` reads the record once and the judgment is of a
    private copy; `guarded` fails a round after which the record on disk is not the bytes it was;
  - going on without end or without progress. The rounds are bounded, and a revision that leaves the
    violations exactly as they were stops the loop (`stalled`): the same request would be answered
    the same way.

What it does not do: it does not say the revision is RIGHT (`within-reach` is the design's changes
being accounted for by the words', on the product's closure of what a requirement depends on), and it
cannot restrain a client that edits more than it was told to within one round. A change out of reach
is found after it is made and asked to be put back; the design the loop hands on is the one that was
judged.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import shlex
import tempfile
from dataclasses import dataclass
from typing import Callable

from . import process, revision
from .errors import AuthoringError
from .verify import acceptance_delta

WITHIN_REACH = "within-reach"
OUTSIDE_REACH = "outside-reach"
STALLED = "stalled"
NOT_JUDGED = "not-judged"
REVISER_FAILED = "reviser-failed"

DEFAULT_ROUNDS = 3
# Every round is a call to a model somebody pays for; a revision that is not within reach after
# this many is not going to be, and the owner should be told rather than charged for the tenth.
MAX_ROUNDS = 10
# How long a reviser may take for one round, by default. A client that drafts a design is minutes;
# an hour is a client that is stuck.
REVISER_TIMEOUT_S = 3600
# How much of a failing reviser's own words are kept in the sentence that says it failed.
_TAIL = 1500


class GateError(AuthoringError):
    """A gate that was asked for something it cannot do (a number of rounds that is not one)."""


class ReviserError(AuthoringError):
    """What a reviser raises when it could not do its round: the loop reports it and stops."""


@dataclass(frozen=True)
class Round:
    """One judgment of the design as it stood. Round 0 is the design as it was handed in."""
    number: int
    verdict: str
    # (requirement, kind, the places it moved at): the places are part of the identity of a
    # violation, so a revision that put back two of three places is progress and not a stall.
    violations: tuple[tuple[str, str, tuple[str, ...]], ...]
    seen: int


@dataclass(frozen=True)
class Outcome:
    """How a gate ended. `result` is the judgment of the design as it stands, or None when
    nothing could be judged; `reason` is a sentence a person is told; `revisions` is how many
    times the reviser was asked, a call that failed included (a design nobody revised after a
    judgment was refused is not a judgment short, which is why it is counted and not derived)."""
    status: str
    rounds: tuple[Round, ...]
    result: dict | None
    reason: str
    revisions: int

    @property
    def within_reach(self) -> bool:
        return self.status == WITHIN_REACH

    def as_json(self) -> dict:
        return {
            "status": self.status,
            "within_reach": self.within_reach,
            "reason": self.reason,
            "revisions": self.revisions,
            "rounds": [{"round": r.number, "verdict": r.verdict, "seen": r.seen,
                        "violations": [{"requirement": id_, "kind": kind, "moved": list(places)}
                                       for id_, kind, places in r.violations]}
                       for r in self.rounds],
            "result": self.result,
        }


def judge(delta: object, record: pathlib.Path, root: pathlib.Path,
          design: pathlib.Path | None = None, *, codegen: pathlib.Path | None = None,
          cwd: pathlib.Path | None = None) -> tuple[dict | None, str]:
    """The words delta joined with the evidence of the design now: `(judgment, refusal)`.

    The evidence is the product's own comparison of the rows the owner was shown with the design as
    it is (`sce-codegen acceptance-delta`), the words are the delta's, and the join is
    `revision.join`. A refusal is the product's sentence (a design it cannot read, a record taken
    under an older rule); a delta that is not this record's raises `revision.RevisionError`.
    `design` names the revised draft when it is not the record's own document."""
    report, refusal = acceptance_delta(record, root, codegen, design=design, cwd=cwd)
    if refusal:
        return None, refusal
    try:
        held = json.loads(record.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise revision.RevisionError(
            f"'record' cannot be read as an acceptance record: {error}") from error
    revision.belongs_to(delta, held)
    return revision.join(delta, json.loads(report)), ""


@dataclass(frozen=True)
class Baseline:
    """The acceptance record as the gate found it: what the design is judged against.

    A judgment reads the record twice (the product reads the evidence in it, and the words delta is
    held to the manifest and the sidecar it pins), so a record that is replaced between rounds is
    not one baseline but a different one each time. The bytes are read ONCE, and `copy`, a private
    read-only file, is what every judgment reads; `source` is the file the owner keeps, which a
    reviser may be told of and must not change (`moved`)."""
    source: pathlib.Path
    copy: pathlib.Path
    sha256: str

    def moved(self) -> str:
        """An empty string while the source is the bytes it was, else the sentence that says it
        is not (a record that cannot be read is a record that moved)."""
        try:
            now = hashlib.sha256(self.source.read_bytes()).hexdigest()
        except OSError as error:
            return (f"the acceptance record {self.source} cannot be read after the reviser ran "
                    f"({error}): the design is judged against the record as it was at the start, "
                    "and a reviser that touches the baseline has not revised the design")
        if now == self.sha256:
            return ""
        return (f"the reviser changed the acceptance record {self.source}: the design is judged "
                "against the record as it was at the start, and a reviser that replaces the "
                "baseline has not revised the design")


def keep_baseline(record: pathlib.Path, directory: pathlib.Path) -> Baseline:
    """Read the acceptance record once and hold a private, read-only copy of it in `directory`.

    The copy is made from the bytes that were hashed, so the hash is of what is judged. The record
    names its files relative to the root the judgment is given, not to where it lies, so a copy
    elsewhere is judged the same."""
    data = record.read_bytes()
    copy = directory / "acceptance-record.json"
    copy.write_bytes(data)
    copy.chmod(0o400)
    return Baseline(record, copy, hashlib.sha256(data).hexdigest())


def guarded(revise: Callable[[str], None], baseline: Baseline) -> Callable[[str], None]:
    """A reviser that has failed its round if, after it ran, the baseline is not the bytes it was."""
    def revise_and_check(request: str) -> None:
        revise(request)
        moved = baseline.moved()
        if moved:
            raise ReviserError(moved)

    return revise_and_check


# What a reviser is told to do about each kind of violation. The WHY of a kind is the page's
# (`revision.render`), which is handed on beside it; these are the directions the page does not give.
_DIRECTION = {
    "moved-without-reason":
        "put back what the design had at these places before the revision; this requirement's "
        "words did not change, so nothing asked it to move",
    "retired-still-cited":
        "the specification dropped this requirement: remove its id from every `sce:req` that "
        "cites it, and leave the elements themselves unless they exist for no other requirement",
}


def _violations(result: dict) -> tuple[tuple[str, str, tuple[str, ...]], ...]:
    return tuple(sorted((row["requirement"], row["kind"], tuple(sorted(row.get("moved") or ())))
                        for row in result["requirements"] if row["severity"] == revision.VIOLATION))


def repair_request(result: dict, number: int, rounds: int) -> str:
    """What the reviser is told when a revision is outside its reach: each violation with the
    places it moved at and what to do about it, then the page the owner would read.

    The page is the same one `scxml_revision_report` returns, so what the reviser reads and what the
    owner is shown cannot differ."""
    lines = [f"This revision is outside the reach of what the specification changed "
             f"(attempt {number} of {rounds}).", "",
             "Change only what is listed below, and nothing else in the design:"]
    for row in result["requirements"]:
        if row["severity"] != revision.VIOLATION:
            continue
        places = row.get("moved") or []
        where = ("moved at " + ", ".join(places)) if places else "moved at places the product could not name"
        lines.append(f"- {row['requirement']} ({row['kind']}): {where}. "
                     f"{_DIRECTION.get(row['kind'], 'put it back')}.")
    lines += ["",
              "Do not report that you are finished: the design is judged again as it stands after "
              "this attempt, and only that judgment says whether it is within reach.", "",
              revision.render(result, None)]
    return "\n".join(lines) + "\n"


def hold(judging: Callable[[], tuple[dict | None, str]], revise: Callable[[str], None], *,
         rounds: int = DEFAULT_ROUNDS) -> Outcome:
    """Judge, and while the revision is outside its reach ask `revise` to put it right, up to
    `rounds` times. `judging()` is `(judgment, refusal)` of the design as it stands; `revise(request)`
    edits that design in place, or raises `ReviserError`.

    Ends `within-reach` only on a judgment that saw evidence and found no violation, made AFTER the
    last revision. `outside-reach` is the rounds used up, `stalled` a revision that changed nothing
    the judgment counts, `not-judged` a judgment that said nothing (or was refused), and
    `reviser-failed` a reviser that could not do its round."""
    if isinstance(rounds, bool) or not isinstance(rounds, int) or not 0 <= rounds <= MAX_ROUNDS:
        raise GateError(f"the number of rounds has to be a whole number from 0 to {MAX_ROUNDS}")
    history: list[Round] = []
    # `number` is also how many times the reviser has been asked before this judgment.
    for number in range(rounds + 1):
        result, refusal = judging()
        if result is None:
            return Outcome(NOT_JUDGED, tuple(history), None, refusal, number)
        held = Round(number, result["verdict"], _violations(result), result["summary"]["seen"])
        history.append(held)
        if held.seen == 0:
            return Outcome(NOT_JUDGED, tuple(history), result,
                           "the judgment saw no evidence for any requirement (no node of the design "
                           "cites one, or its kind has nowhere to), so it compared nothing and its "
                           "verdict says nothing about the design", number)
        if held.verdict == WITHIN_REACH:
            return Outcome(WITHIN_REACH, tuple(history), result,
                           f"within reach after {number} revision(s)", number)
        if number > 0 and held.violations == history[-2].violations:
            return Outcome(STALLED, tuple(history), result,
                           f"revision {number} left the violations exactly as they were "
                           f"({_listed(held.violations)}): asking again would be answered the same way",
                           number)
        if number == rounds:
            return Outcome(OUTSIDE_REACH, tuple(history), result,
                           f"still outside the reach of what changed after {number} revision(s): "
                           f"{_listed(held.violations)}", number)
        try:
            revise(repair_request(result, number + 1, rounds))
        except ReviserError as error:
            return Outcome(REVISER_FAILED, tuple(history), result, str(error), number + 1)
    raise AssertionError("unreachable: the last round returns")  # pragma: no cover


def _listed(violations: tuple[tuple[str, str, tuple[str, ...]], ...]) -> str:
    return ", ".join(f"{id_} ({kind})" for id_, kind, _places in violations)


def command_reviser(command: str, design: pathlib.Path, *,
                    timeout_s: int = REVISER_TIMEOUT_S) -> Callable[[str], None]:
    """A reviser that is a command: any client runner that edits the design it is told of.

    The request is written to a file and the command is run (without a shell) with
    `SCE_REVISION_REQUEST` naming that file and `SCE_REVISION_DESIGN` the design to edit in place.
    A command that exits non-zero, cannot be started or runs past the timeout has failed its round.
    Its own output is kept for the sentence that says so, and is otherwise not read: whether it
    worked is the judgment's to say.

    ⚠ It runs in a session of its own, and a timeout stops everything it started, not only the
    command: a client runner starts programs, and one left running would go on editing the design
    after the gate said the round was over. What a command leaves running after it EXITS is not
    stopped (`process.run`, `own_group`)."""
    argv = shlex.split(command)
    if not argv:
        raise GateError("the reviser command is empty")

    def revise(request: str) -> None:
        with tempfile.TemporaryDirectory() as directory:
            asked = pathlib.Path(directory) / "revision-request.md"
            asked.write_text(request, encoding="utf-8")
            try:
                done = process.run(argv, timeout=timeout_s, own_group=True,
                                   env={"SCE_REVISION_REQUEST": str(asked),
                                        "SCE_REVISION_DESIGN": str(design)})
            except OSError as error:
                raise ReviserError(f"the reviser could not be started: {error}") from error
            except process.ProcessTimeout as error:
                # A stop that left a pipe open is not a clean one, and the process layer is the one that knows.
                held = (f"; its {', '.join(error.left_open)} pipe(s) were still held by what it started and "
                        "were left open" if error.left_open else "")
                raise ReviserError(
                    f"the reviser ran past {timeout_s} seconds and was stopped{held}") from error
            except ValueError as error:
                # Its output is read as UTF-8, and a client that wrote anything else has said
                # nothing this can keep; whether it revised is still the judgment's to say.
                raise ReviserError(f"the reviser's output could not be read as text: {error}") from error
        if done.returncode != 0:
            said = (done.stdout + done.stderr).strip()[-_TAIL:]
            raise ReviserError(f"the reviser exited with status {done.returncode}"
                               + (f": {said}" if said else ""))

    return revise
