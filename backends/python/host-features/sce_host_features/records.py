# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""What a host answers a start with, and the result records of its completions."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Dict

#: A completion: `data` is the result record, as the JSON a completion carries. A
#: failure is such a record whose `error` is a code of the operation's closed set.
DONE = "done"
#: A host that would not start the operation: `data` is the reason, as the JSON
#: `error.execution` carries.
REFUSE = "refuse"
#: A host that has started the operation and answers later, or never.
HOLD = "hold"

#: The refusal the specification gives an argument the operation does not accept.
INVALID_ARGUMENT = '{"reason":"invalid_argument"}'

#: What each operation's result record holds at its zero, which a failure
#: reports beside its code.
_ZERO: Dict[str, dict] = {
    "sce-host-clock-now": {"utc_ms": "0", "monotonic_ms": "0"},
    "sce-host-clock-zone": {"zone_id": "", "utc_offset_s": 0},
    "sce-host-kv-get": {"found": False, "value": ""},
    "sce-host-kv-put": {"replaced": False},
    "sce-host-kv-delete": {"existed": False},
    "sce-host-notify-cancel": {"existed": False},
    "sce-host-kv-next-key": {"found": False, "key": ""},
    "sce-host-notify-permission": {"state": ""},
    "sce-host-notify-request-permission": {"state": ""},
    "sce-host-notify-post": {"status": ""},
    "sce-host-http-fetch": {"status": 0, "headers": "", "body": ""},
}


@dataclass
class Outcome:
    """What a host answers a start with."""

    kind: str
    data: str = ""


def record(fields: dict) -> Outcome:
    """A result record as the JSON a completion carries, its `error` empty. A
    64-bit integer is written as the text of its digits, as a payload's reader
    takes it."""
    return Outcome(DONE, json.dumps({"error": "", **fields}, separators=(",", ":")))


def failure(operation: str, code: str) -> Outcome:
    """The failure `code` of `operation`: its result record with the fields at
    their zero and `error` the code, so a machine reads the failure as a typed
    field."""
    return Outcome(
        DONE, json.dumps({**_ZERO[operation], "error": code}, separators=(",", ":"))
    )
