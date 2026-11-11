# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Why a recorded input trace, or a value of one, is not read
(docs/SCE_TTCN3_LANE.md §10.2): the caller's input, refused by name before
anything of the run happens.

It is a module of its own, as the C++ runtime's ``test/TraceRefusal.h`` is, so
that the readers of the canonical form (``from_canonical``) and the reader of a
trace (``trace``), each of which reads with the other, both raise it without
importing each other.

Cross-language siblings: ``sce_rust_runtime::test::TraceRefusal`` (Rust),
``com.sce.runtime.test.TraceRefusal`` (Kotlin), ``SCE::Test::TraceRefusal``
(C++), whose words these are.
"""

from __future__ import annotations

from typing import Sequence


class TraceRefusal(Exception):
    """Why a trace, or a value of one, is not read."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        #: Why.
        self.reason = reason

    @classmethod
    def not_an_input(cls, system_port: str, type_name: str, system_ports: Sequence[str]) -> "TraceRefusal":
        """An input of the record type ``type_name`` on ``system_port``, which
        the configuration whose system ports are ``system_ports`` does not take:
        the port is none of them, or its type's ``in`` list does not name the
        type (docs/SCE_TTCN3_LANE.md §10.2)."""
        if system_port in system_ports:
            return cls(f"'{type_name}' is not on the `in` list of the type of the system port '{system_port}'")
        return cls(f"'{system_port}' is not a system port of the test case's configuration")
