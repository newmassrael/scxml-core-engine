"""The works folder, as the authoring tools reach it.

The workbench application keeps a specification as a *work*: its text, its
saved revisions, and the model an authoring client wrote from it. This module is
how the tools read a work and save a model to it, and it does so by running
`sce-work`, never by opening the folder.

⚠ One writer of the folder. A save is a compare-and-swap on the revision the
caller read, taken under a lock, and the desktop application and this server
both save. A second implementation here -- read the files, write the files --
would be a second definition of what a save is and of when two saves conflict,
without the first one's lock: the day they disagreed an owner would lose text.
`sce-work` is the application's own command layer (`app-core`), so there is one.

The refusals keep their words. `sce-work` answers a refusal as a JSON line with a
`kind` a program branches on and a sentence for a person, and `WorksError`
carries both: a client that is refused a save as a `conflict` is told what the
work is at now and reads again, instead of retrying the same bytes.
"""

from __future__ import annotations

import json
import os
import pathlib
import threading
import uuid

from . import process
from .errors import AuthoringError

# `sce-work` waits up to thirty seconds for another save of the same work and a
# draw of a model is not done here, so a call that has not answered in twice
# that is stuck rather than slow.
WORK_SECONDS = 60.0


class WorksError(AuthoringError):
    """The works folder would not do what was asked.

    `kind` is the word the command layer gave the refusal (`conflict`,
    `not-found`, `invalid-id`, ...), or one of this module's own: `unavailable`
    (no `sce-work` to run), `timeout`, `failed` (it said something that is not a
    refusal). `detail` is the facts a caller acts on, when there are any: for a
    `conflict`, the revision the caller wrote from and the one that is current.
    """

    def __init__(self, kind: str, message: str, detail: object = None):
        super().__init__(message)
        self.kind = kind
        self.detail = detail


def default_work_binary() -> pathlib.Path:
    """Where `sce-work` is, by default: `SCE_WORK` when the environment names
    one, else this tree's own build.

    The environment first, for the reason the generator's is (`verify`): an
    installed bundle has no tree, and its launcher names the binary it ships.
    Resolved to an absolute path so a call that starts in another directory
    does not read a relative name from there.
    """
    named = os.environ.get("SCE_WORK")
    if named:
        return pathlib.Path(named).resolve()
    root = pathlib.Path(__file__).resolve().parents[3]
    return root / "target" / "debug" / "sce-work"


def call_work(command: str, args: dict | None = None,
              binary: pathlib.Path | None = None) -> dict:
    """Run the command layer's `command` with `args`; its one JSON answer.

    The arguments travel on standard input: a specification is larger than an
    operating system lets a command line be. Which folder is the works folder is
    `sce-work`'s own answer (`SCE_WORKS_DIR`, else the per-user data directory),
    the same one the application opens, so the two cannot be pointed at
    different folders by this module.
    """
    binary = default_work_binary() if binary is None else binary
    if not binary.is_file():
        raise WorksError(
            "unavailable",
            f"sce-work is not at {binary}: the works folder is reached through it. Build it "
            f"(cargo build -p sce-app-core --features cli --bin sce-work), or name it "
            f"with SCE_WORK")
    try:
        done = process.run([str(binary), "call", command, "--args-stdin"],
                           timeout=WORK_SECONDS,
                           stdin_text=json.dumps(args if args is not None else {}))
    except process.ProcessTimeout as exc:
        raise WorksError("timeout", f"the works folder did not answer: {exc}") from exc
    if done.returncode == 0:
        try:
            return json.loads(done.stdout)
        except json.JSONDecodeError as exc:
            raise WorksError(
                "failed", f"sce-work answered something that is not JSON: {exc}") from exc
    raise _refusal(done.stderr, done.returncode)


def _refusal(stderr: str, returncode: int) -> WorksError:
    """The refusal `sce-work` wrote on its standard error, or what it said when it
    did not write one (a command line it could not read)."""
    try:
        error = json.loads(stderr.strip().splitlines()[-1])["error"]
        return WorksError(str(error["kind"]), str(error["message"]), error.get("detail"))
    except (IndexError, KeyError, TypeError, ValueError):
        said = stderr.strip() or "nothing"
        return WorksError("failed", f"sce-work stopped with status {returncode} and said: {said}")


def list_works() -> dict:
    """Every work, oldest first, and any folder that should have been one and
    could not be read."""
    return call_work("list_works")


def read_work(work: str) -> dict:
    """A work as an authoring client needs it: who it is, its text now, its model
    with where that stands to the text, the owner's answers to what the model left
    open, the requirement list the text was read into, and whether the owner has
    accepted the design (`acceptance`, only for a work that has a list and a model).

    The text, the model, the answers and the list are ONE state of the work
    (`read_work_snapshot`), so a save or a publication cannot land between them: a
    model of one generation is never read beside a list of another. `bundle` is the
    bundle they are the ones of, for a work that keeps bundles, and `request` is
    where the work's latest request stands (the owner may have asked for a model that
    nobody has taken yet). The acceptance is the product's to answer and is asked
    after, so it is the one part that is not of the same moment; each `standing` is
    the command layer's own comparison of a `written_for` with the text's head.
    """
    state = call_work("read_work_snapshot", {"id": work})
    source = state["source"]
    source_head = source["revision"] if source is not None else None
    requirements = _requirements_of(state["requirements"], state["requirements_standing"],
                                    source_head)
    return {"work": state["work"], "source": source,
            "model": _model_of({"model": state["model"], "standing": state["model_standing"],
                                "source_head": source_head}),
            "answers": state["answers"], "requirements": requirements,
            "acceptance": (read_acceptance(work)
                           if requirements is not None and state["model"] is not None else None),
            "bundle": state["bundle"],
            "request": call_work("read_work_heads", {"id": work})["request"]}


def read_answers(work: str) -> dict | None:
    """The owner's answers to the questions the model left open, `{"revision",
    "entries": {question id: {"answer", "answered_at"}}}`, or None when they have
    answered nothing. The owner writes them in the application; nothing here does."""
    return call_work("read_answers", {"id": work})["answers"]


def read_requirements(work: str) -> dict | None:
    """The requirement list the work's text was read into, or None when it has none.

    `manifest_text` and `sidecar_text` are the two files exactly as they were saved
    (the names the requirement tools take them under), `written_for` the text
    revision the list says it was read from, and `standing` the command layer's own
    comparison of that with the text's head: `current`, `behind` or `unstated`. The
    sidecar is left out for a list that came without one.
    """
    answer = call_work("read_requirements", {"id": work})
    return _requirements_of(answer["requirements"], answer["standing"], answer["source_head"])


def _requirements_of(held: dict | None, standing: str | None,
                     source_head: str | None) -> dict | None:
    """The requirement list as a client reads it, from what the command layer answered."""
    if held is None:
        return None
    read = {"revision": held["revision"], "written_for": held["written_for"],
            "standing": standing, "source_head": source_head,
            "manifest_text": held["manifest"]}
    if held["sidecar"] is not None:
        read["sidecar_text"] = held["sidecar"]
    return read


def read_acceptance(work: str) -> dict:
    """Whether the owner has accepted this work's design, and whether it still holds.

    `standing` is `none` (nothing accepted), `holds`, or `lapsed`; a lapse carries the
    product's own sentence of what moved (`lapse`). An acceptance says when it was
    made (`accepted_at`), the `channel` it was stated on (`direct`: the owner's own
    press in the application; an acceptance relayed by a client says `relayed`), and
    what the design left open when it was accepted (`open`).

    ⚠ Read here and never written: the owner accepts in the application. The command
    layer states every acceptance it records as the application's own, so a client
    that could call it would be stating, as the owner's press, what the owner never
    pressed.

    The product answers whether it holds, so the product not answering (`sce-*`) is
    reported as `unavailable` with its words and does not stop a work being read.
    """
    try:
        answer = call_work("read_acceptance", {"id": work})
    except WorksError as exc:
        if not exc.kind.startswith("sce-"):
            raise
        return {"standing": "unavailable", "kind": exc.kind, "message": str(exc)}
    held = answer["acceptance"]
    if held is None:
        return {"standing": "none"}
    read = {"standing": answer["standing"], "accepted_at": held["accepted_at"],
            "channel": held["channel"], "open": held["open"]}
    if answer["lapse"] is not None:
        read["lapse"] = answer["lapse"]
    return read


def save_requirements(work: str, base: str | None, written_for: str | None, *,
                      manifest: str, sidecar: str | None = None) -> dict:
    """Save the work's next requirement list: the `manifest` (coordinates) and the
    `sidecar` (the quoted sentences), each as the text `scxml_requirement_set` made.

    `base` is the list revision the writer read (None for a work's first list); a base
    that is no longer current is refused as a `conflict` and nothing is written.
    `written_for` is the text revision the list was read from, or None when the writer
    cannot say -- which the application shows as unstated, and not as current.
    """
    return call_work("save_requirements", {
        "id": work, "base": base, "written_for": written_for, "manifest": manifest,
        **({"sidecar": sidecar} if sidecar is not None else {})})


def _model_of(answer: dict) -> dict | None:
    """The model as a client reads it. A model of ONE document is `text`, as it has
    always been; a model of several is `entry` (the file the product is asked about)
    and `documents` (each under the file name its imports know it by), and no `text`:
    a second copy of the entry would be tokens spent twice on a statechart's worth."""
    model = answer["model"]
    if model is None:
        return None
    standing = {"standing": answer["standing"], "source_head": answer["source_head"]}
    if len(model["documents"]) <= 1:
        return {key: value for key, value in model.items()
                if key not in ("entry", "documents")} | standing
    return {key: value for key, value in model.items() if key != "text"} | standing


def save_model(work: str, base: str | None, written_for: str | None, *,
               text: str | None = None, documents: list[dict] | None = None,
               entry: str | None = None) -> dict:
    """Save the work's next model: `text` (one document), or `documents` (several that
    name each other, as `{"name", "text"}`) with the `entry` the product is asked about.

    `base` is the model revision the writer read (None for a work's first model);
    a base that is no longer current is refused as a `conflict` and nothing is
    written. `written_for` is the text revision the writer read, or None when it
    cannot say -- which the screen shows as "unstated", and not as current.
    """
    model = ({"text": text} if documents is None else
             {"documents": documents, **({"entry": entry} if entry is not None else {})})
    return call_work("save_model", {"id": work, "base": base,
                                    "written_for": written_for, **model})


# What a request's holder is told when the request is no longer its to work on: the owner
# called it off, a save moved the text it was asked about, or somebody took it.
_ENDED_KINDS = frozenset({"request-ended", "not-holder", "request-held", "not-found"})

# A lease is sixty seconds unless asked otherwise. A quarter of it is time enough for three
# renewals to fail before the request is read as let go of.
HEARTBEAT_SECONDS = 15.0

# How many requests that ended under a client are remembered, to tell it at its next word.
_LOST_KEPT = 64


class Generation:
    """A request this process holds for an authoring client: whose it is to work on, which
    attempt, and what was said of the work along the way."""

    def __init__(self, work: str, request: str, attempt: int, source: str):
        self.work = work
        self.request = request
        self.attempt = attempt
        # The text revision the request was asked about: what a model written for it is
        # written for, whatever the client says.
        self.source = source
        # Checks only the client's side ran, kept as it reports them when the generation
        # is finished (the core runs its own check of the model at that step).
        self.checks: list[dict] = []


class Generations:
    """The generations begun through this process, and the renewing of their leases.

    A request is held for a lease that runs out. A client in a person's own terminal
    thinks, drafts and checks for longer than a lease, and never says it is still there:
    the only thing that can is this process, which is why it renews them itself. A
    request whose lease is not renewed is read as interrupted, by the clock, by every
    screen of the owner -- so a process that dies leaves the owner told, and a process
    that lives leaves the request running.

    What ends a generation is the core's word: a renewal or a save that is refused
    because the request ended (the owner called it off, the text it was asked about was
    saved) or is somebody else's. It is remembered and told at the client's next word,
    and nothing is renewed for it again.
    """

    def __init__(self, call=None, interval: float = HEARTBEAT_SECONDS,
                 holder: str | None = None):
        self._call = call_work if call is None else call
        self._interval = interval
        # The name every word to the core is said under; a request names its holder.
        self.holder = holder if holder is not None else f"mcp-{os.getpid()}"
        self._held: dict[str, Generation] = {}
        self._lost: dict[str, WorksError] = {}
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    def begin(self, work: str) -> Generation:
        """Take the work's request for this process, or ask for one when there is none.

        The request the owner made is the one taken (`queued`, or `interrupted` as the
        next attempt); one held by somebody else is refused as the core refuses it
        (`request-held`). A work nobody asked a model for is asked for one here, from
        the text and the answers as they are now, so that every model written to a work
        is the answer to a request and the owner sees it as one. Beginning again for a
        work this process already holds is the generation it holds.
        """
        with self._lock:
            for held in self._held.values():
                if held.work == work:
                    return held
        state = self._call("read_work_snapshot", {"id": work})
        source = state["source"]
        if source is None:
            raise WorksError(
                "no-text", f"work `{work}` has no text yet: there is nothing to write a model "
                           f"from, and the owner writes it in the application")
        latest = self._call("read_work_heads", {"id": work})["request"]
        resume = False
        if latest is not None and latest["state"] in ("queued", "running", "interrupted"):
            request = latest["id"]
            resume = latest["state"] == "interrupted"
        else:
            answers = state["answers"]
            made = self._call("request_generation", {
                "id": work, "key": f"mcp-{uuid.uuid4().hex}", "origin": "mcp",
                "expect": {"source": source["revision"],
                           "answers": answers["revision"] if answers is not None else None}})
            request = made["request"]["id"]
        claimed = self._call("claim_request", {
            "id": work, "request": request, "holder": self.holder, "resume": resume})["request"]
        generation = Generation(work, request, claimed["attempt"], claimed["inputs"]["source"])
        with self._lock:
            self._held[request] = generation
            self._lost.pop(request, None)
            self._start()
        return generation

    def get(self, request: str) -> Generation:
        """The generation of `request`, or why it is not this process's to speak for."""
        with self._lock:
            held = self._held.get(request)
            lost = self._lost.get(request)
        if held is not None:
            return held
        if lost is not None:
            raise WorksError(
                "generation-ended",
                f"request {request} is no longer yours to work on ({lost}): read the work "
                f"again, and begin another generation if the owner still wants one",
                {"reason": lost.kind, **(lost.detail if isinstance(lost.detail, dict) else {})})
        raise WorksError(
            "unknown-generation",
            f"this server did not begin request {request}, or let go of it: begin it with "
            f"works_begin_generation (a request that was let go of is taken again there)")

    def save_candidate(self, request: str, model: dict | None = None,
                       requirements: dict | None = None,
                       instructions: str | None = None) -> dict:
        """Write the model, the requirement list, or both for the request, as its holder.
        What is written is the request's and not the work's: it becomes the work's when
        the generation is finished. `instructions` names the version of what the client
        was told to do (its server's own instructions), which the bundle records so that a
        result can be told apart from one made to other instructions."""
        generation = self.get(request)
        return self._said(generation, "save_request_candidate", {
            **(model or {}), **(requirements or {}),
            **({"instructions": instructions} if instructions is not None else {})})

    def finish(self, request: str) -> dict:
        """Say the generation is done: the core checks the model itself and, when it
        accepts, publishes the model and the list together as one bundle. A refusal leaves
        the generation held, to be said again once the candidate is fixed."""
        generation = self.get(request)
        done = self._said(generation, "complete_request", {"checks": list(generation.checks)})
        self._release(request)
        return done

    def fail(self, request: str, reason: str) -> dict:
        """Say the generation could not be done, and why, and let go of it."""
        generation = self.get(request)
        failed = self._said(generation, "fail_request", {"reason": reason})
        self._release(request)
        return failed

    def _said(self, generation: Generation, command: str, extra: dict) -> dict:
        try:
            return self._call(command, {"id": generation.work, "request": generation.request,
                                        "holder": self.holder, "attempt": generation.attempt,
                                        **extra})
        except WorksError as exc:
            if exc.kind in _ENDED_KINDS:
                self._lose(generation.request, exc)
            raise

    def beat(self) -> None:
        """Renew every held generation once. A renewal the core did not answer is tried
        again at the next beat: the lease runs out only if it keeps not answering."""
        if self._stop.is_set():
            return
        with self._lock:
            held = list(self._held.values())
        for generation in held:
            try:
                self._call("heartbeat_request", {
                    "id": generation.work, "request": generation.request,
                    "holder": self.holder, "attempt": generation.attempt})
            except WorksError as exc:
                if exc.kind in _ENDED_KINDS:
                    self._lose(generation.request, exc)

    def close(self) -> None:
        """Stop renewing. What is held is let go of by the lease running out."""
        self._stop.set()
        thread = self._thread
        if thread is not None and thread is not threading.current_thread():
            thread.join(timeout=5)

    def _lose(self, request: str, why: WorksError) -> None:
        with self._lock:
            if self._held.pop(request, None) is not None:
                self._lost[request] = why
                while len(self._lost) > _LOST_KEPT:
                    del self._lost[next(iter(self._lost))]

    def _release(self, request: str) -> None:
        with self._lock:
            self._held.pop(request, None)

    def _start(self) -> None:
        """Start the renewing, once. Called with the lock held."""
        if self._thread is None and not self._stop.is_set():
            self._thread = threading.Thread(target=self._run, name="generation-leases",
                                            daemon=True)
            self._thread.start()

    def _run(self) -> None:
        while not self._stop.wait(self._interval):
            self.beat()


_GENERATIONS: Generations | None = None
_GENERATIONS_LOCK = threading.Lock()


def generations() -> Generations:
    """The generations of this process: one set, because one process is one holder."""
    global _GENERATIONS
    with _GENERATIONS_LOCK:
        if _GENERATIONS is None:
            _GENERATIONS = Generations()
        return _GENERATIONS
