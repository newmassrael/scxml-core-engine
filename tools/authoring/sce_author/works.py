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
    """A work as an authoring client needs it: who it is, its text now, and its
    model with where that stands to the text.

    Three reads of the command layer, so a text saved between them is possible.
    That is not hidden: every revision in the answer is the one that was read, and
    the model's `standing` is the command layer's own comparison of its
    `written_for` with the text's head at the moment of the model read.
    """
    head = call_work("read_work", {"id": work})
    source = call_work("read_source", {"id": work})["source"]
    model = call_work("read_model", {"id": work})
    return {"work": head["work"], "source": source, "model": _model_of(model)}


def _model_of(answer: dict) -> dict | None:
    model = answer["model"]
    if model is None:
        return None
    return {**model, "standing": answer["standing"], "source_head": answer["source_head"]}


def save_model(work: str, text: str, base: str | None, written_for: str | None) -> dict:
    """Save `text` as the work's next model.

    `base` is the model revision the writer read (None for a work's first model);
    a base that is no longer current is refused as a `conflict` and nothing is
    written. `written_for` is the text revision the writer read, or None when it
    cannot say -- which the screen shows as "unstated", and not as current.
    """
    return call_work("save_model", {"id": work, "text": text, "base": base,
                                    "written_for": written_for})
