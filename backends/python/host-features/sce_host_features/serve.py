# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A `ReferenceHost` as the invoker of every standard type of an engine."""

from __future__ import annotations

from typing import Callable, Optional

from sce_runtime import HostInvokeEvent, HostInvokeResponse

from .host import TYPES, ReferenceHost
from .records import DONE, REFUSE


def _invoker(operation: str, host: ReferenceHost) -> Callable[[HostInvokeEvent], Optional[HostInvokeResponse]]:
    def handle(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if event.cancel is not None:
            host.cancel(event.cancel.invoke_id)
            return None
        if event.start is None:
            return None
        params = {name: values[0] for name, values in event.start.params.items() if values}
        outcome = host.start(operation, event.start.invoke_id, params)
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
