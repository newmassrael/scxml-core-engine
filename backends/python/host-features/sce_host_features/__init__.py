# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The reference host of the standard host features
(docs/SCE_HOST_FEATURES.md): clock, key-value store, notifications and HTTP, as
the operations a generated machine's host-run invokes ask for.

A `ReferenceHost` is the whole of the four features' semantics as one object. It
is told the start of an operation (its type and the request's text parameters)
and answers with an `Outcome`: a completion, a refusal, or nothing yet. It knows
no engine, so the same object serves a generated machine through `serve`, a
test, or another embedding. Time comes from a `ClockSource` and the network from
a `Transport`, both the embedding's to supply.
"""

from .clock import ClockSource, Reading, ScriptedClock, SystemClock, Zone
from .host import DONE, HOLD, REFUSE, TYPES, Outcome, ReferenceHost, Start
from .http import HttpRequest, HttpResponse, HttpTransport, Transport, TransportFailure
from .serve import serve

__all__ = [
    "ClockSource",
    "DONE",
    "HOLD",
    "HttpRequest",
    "HttpResponse",
    "HttpTransport",
    "Outcome",
    "REFUSE",
    "Reading",
    "ReferenceHost",
    "ScriptedClock",
    "Start",
    "SystemClock",
    "TYPES",
    "Transport",
    "TransportFailure",
    "Zone",
    "serve",
]
