# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Why a machine's script session could not be established, which refuses its
start (docs/adr/0096).

A machine whose document needs a datamodel runs its conditions, assignments and
scripts in a script session. Establishing it is three steps in this order:
create the session, load the shared library into it (the ECMAScript semantics
and the JSON builtins), and bind the system variables `_sessionid`, `_name` and
`_ioprocessors`. A machine started over a session one of them failed for would
run with no datamodel, every condition an error and every assignment lost, and
the document could not even handle it: its means of handling an error runs in
the session that failed. So the start is refused, and the host is handed a
`ScriptSessionFailure`.

Cross-language siblings: `sce_rust_runtime::ScriptSessionFailure` (Rust),
`SCE::ScriptSessionFailure` (C++), `sce.ScriptSessionFailure` (Go) and
`com.sce.runtime.ScriptSessionFailure` (Kotlin), whose step names and sentence
are these.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Optional, Set

from .scripting.i_script_engine import ScriptLibraryNotLoaded


class ScriptSessionStage(Enum):
    """The step of establishing a script session that failed. Each value is
    the step's name, the same word on every backend."""

    #: The script engine could not create the session.
    CREATE = "create"
    #: The shared library did not load into the new session, which the engine
    #: then did not keep (`ScriptLibraryNotLoaded`). A library chunk that does
    #: not load is a defect of the build that shipped it.
    LIBRARY = "library"
    #: The system variables `_sessionid`, `_name` and `_ioprocessors` could not
    #: be bound; the session was destroyed.
    SYSTEM_VARIABLES = "system-variables"

    def __str__(self) -> str:
        """The step's name: `create`, `library` or `system-variables`."""
        return self.value


@dataclass(frozen=True)
class ScriptSessionFailure:
    """Why a machine's script session could not be established, so its start
    was refused: no state was entered, no executable content ran, no event was
    taken, and what the failed steps made was released.

    `Engine.initialize` raises it in a `ScriptSessionUnavailable`,
    `Engine.initialize_as_root` answers it as `RootStartRefusal.ScriptSession`,
    and `Engine.enter_at` carries it in its `ConfigurationVerdict`."""

    #: The step that failed.
    stage: ScriptSessionStage
    #: What the script engine said.
    message: str

    def __str__(self) -> str:
        """The failure as a sentence: the step that failed, then the engine's
        words, the sentence every backend writes."""
        return (
            "the script session could not be established at step "
            f"'{self.stage.value}': {self.message}"
        )

    def child_start_refusal(self, invoke_id: str) -> str:
        """The data of the `error.execution` a parent raises for the child of
        the `<invoke>` `invoke_id` whose start this failure refused: the invoke,
        and the step its child's script session failed at. The parent keeps no
        child and raises no `done.invoke` for it, the words every backend
        writes."""
        return (
            f"<invoke id='{invoke_id}'> did not start: its child's script session "
            f"failed at step '{self.stage.value}'"
        )

    @staticmethod
    def of_creation(failure: BaseException) -> "ScriptSessionFailure":
        """What `IScriptEngine.create_session` failed with: a shared library
        that did not load (a `ScriptLibraryNotLoaded` anywhere in `failure`'s
        chain of causes) is the `library` step, any other failure the `create`
        step."""
        stage = (
            ScriptSessionStage.LIBRARY
            if _library_in_chain(failure)
            else ScriptSessionStage.CREATE
        )
        return ScriptSessionFailure(stage, _words_of(failure))

    @staticmethod
    def of_system_variables(failure: BaseException) -> "ScriptSessionFailure":
        """What `IScriptEngine.setup_system_variables` failed with: the
        `system-variables` step."""
        return ScriptSessionFailure(ScriptSessionStage.SYSTEM_VARIABLES, _words_of(failure))


class ScriptSessionUnavailable(Exception):
    """A machine's start was refused because its script session could not be
    established (`failure` names the step), so nothing was entered and nothing
    ran (docs/adr/0096).

    Raised by `Engine.initialize` before the run is marked running or its inbox
    opened. Every Python machine is built over a script engine and makes its
    session at the start, so any start may raise it."""

    def __init__(self, failure: ScriptSessionFailure) -> None:
        super().__init__(str(failure))
        #: Why the session could not be established.
        self.failure = failure


def _library_in_chain(failure: BaseException) -> bool:
    """Whether `failure`, or anything it was raised from or while handling, is
    a `ScriptLibraryNotLoaded`."""
    seen: Set[int] = set()
    link: Optional[BaseException] = failure
    while link is not None and id(link) not in seen:
        if isinstance(link, ScriptLibraryNotLoaded):
            return True
        seen.add(id(link))
        link = link.__cause__ or link.__context__
    return False


def _words_of(failure: BaseException) -> str:
    """What `failure` says, or the name of its class when it says nothing."""
    return str(failure) or type(failure).__name__
