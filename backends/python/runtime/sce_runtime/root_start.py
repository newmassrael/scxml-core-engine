# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Why `Engine.initialize_as_root` refused to start a machine.

The default is to run such a machine — its `#_parent` sends then raise
error.communication (W3C SCXML C.1). A host that would rather not start a
machine that needs a parent, when it has none to give, asks for the refusal by
starting it with `Engine.initialize_as_root`.
"""

from enum import Enum


class RootStartRefusal(Enum):
    """W3C SCXML 6.2.4: ``NEEDS_PARENT`` — the document sends to `#_parent`,
    and a session its host started has no parent to reach."""

    NEEDS_PARENT = "the machine sends to #_parent and was started with no parent session"

    @property
    def reason(self) -> str:
        """The refusal as a sentence, for a host to report — the same words
        on every engine."""
        return self.value
