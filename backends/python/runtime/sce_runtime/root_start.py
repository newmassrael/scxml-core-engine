# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Why `Engine.initialize_as_root` refused to start a machine.

The default is to run such a machine — its `#_parent` sends then raise
error.communication (W3C SCXML C.1). A host that would rather not start a
machine that needs a parent, when it has none to give, asks for the refusal by
starting it with `Engine.initialize_as_root`.

That start refuses for either of two reasons, the two every engine spells
`NeedsParent` and `ScriptSession`: `RootStartRefusal.NEEDS_PARENT` before
anything is established, and `RootStartRefusal.ScriptSession` when the
machine's script session cannot be established (docs/adr/0096). A refusal
starts nothing.

A closed family of two: a host tells them apart with ``is`` for the first,
which is one value, and ``isinstance`` for the second, which carries the
failure.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, Type

from .script_session import ScriptSessionFailure


class RootStartRefusal:
    """Why `Engine.initialize_as_root` refused to start a machine: one of
    `RootStartRefusal.NEEDS_PARENT` and `RootStartRefusal.ScriptSession`.

    `NEEDS_PARENT` keeps the spelling it had while this was an enumeration of
    one value, so a host that compares a refusal with it reads the same."""

    #: The machine sends to `#_parent` and a session its host started has no
    #: parent to reach. The one value of `RootStartNeedsParent`.
    NEEDS_PARENT: ClassVar["RootStartNeedsParent"]
    #: The class of `NEEDS_PARENT`, under the name every engine gives it.
    NeedsParent: ClassVar[Type["RootStartNeedsParent"]]
    #: The refusal of a machine whose script session cannot be established,
    #: built with the failure, under the name every engine gives it.
    ScriptSession: ClassVar[Type["RootStartScriptSession"]]

    @property
    def reason(self) -> str:
        """The refusal as a sentence, for a host to report — the same words
        on every engine."""
        raise NotImplementedError

    def __str__(self) -> str:
        """The `reason`, which is what a host prints."""
        return self.reason


@dataclass(frozen=True)
class RootStartNeedsParent(RootStartRefusal):
    """W3C SCXML 6.2.4: the document sends to `#_parent`, and a session its
    host started has no parent to reach. Its one value is
    `RootStartRefusal.NEEDS_PARENT`."""

    @property
    def reason(self) -> str:
        return "the machine sends to #_parent and was started with no parent session"


@dataclass(frozen=True)
class RootStartScriptSession(RootStartRefusal):
    """The machine's script session could not be established, so nothing was
    entered and nothing ran; `failure` names the step and carries the engine's
    words, which are the `reason` (docs/adr/0096)."""

    #: Why the session could not be established.
    failure: ScriptSessionFailure

    @property
    def reason(self) -> str:
        return str(self.failure)


RootStartRefusal.NeedsParent = RootStartNeedsParent
RootStartRefusal.ScriptSession = RootStartScriptSession
RootStartRefusal.NEEDS_PARENT = RootStartNeedsParent()
