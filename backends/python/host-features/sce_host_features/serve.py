# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A `ReferenceHost` as the invoker of every standard type of an engine."""

from __future__ import annotations

from typing import Callable, Optional

from sce_runtime import HostInvokeEvent, HostInvokeResponse

from .host import TYPES, Invocation, ReferenceHost
from .records import DONE, REFUSE


def _invoker(operation: str, host: ReferenceHost) -> Callable[[HostInvokeEvent], Optional[HostInvokeResponse]]:
    def handle(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if event.cancel is not None:
            host.cancel(event.cancel.invoke_id)
            return None
        if event.start is None:
            return None
        params = {name: values[0] for name, values in event.start.params.items() if values}
        # docs/adr/0107, D4: the invocation's identity, which a restore starts it
        # again under.
        invocation = Invocation(event.start.session_id, event.start.invoke_id, event.start.start)
        outcome = host.start(operation, invocation, params)
        if outcome is None:
            return None
        if outcome.kind == DONE:
            return HostInvokeResponse(done_data=outcome.data)
        if outcome.kind == REFUSE:
            return HostInvokeResponse(refusal=outcome.data)
        return None

    return handle


def serve(engine, host: ReferenceHost) -> None:
    """Register `host` as the invoker of every standard type on `engine`.

    A start is answered with its completion (a failure is a completion whose
    `error` is a code) or its refusal, or not at all when the host holds it. The
    cancel the engine delivers when a state exits or a deadline passes reaches
    the host too."""
    for operation in TYPES:
        engine.register_invoker(operation, _invoker(operation, host))
