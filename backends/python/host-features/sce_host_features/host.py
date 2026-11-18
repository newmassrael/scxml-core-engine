# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The reference host: the four features' semantics over a clock and a transport."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Sequence, Tuple

from . import clock, http, kv, model, notify
from .clock import ClockSource
from .http import Transport
from .model import MODEL_GENERATE, ModelProvider, ScriptedProvider
from .records import DONE, HOLD, INVALID_ARGUMENT, REFUSE, Outcome, failure

__all__ = ["DONE", "HOLD", "REFUSE", "TYPES", "Invocation", "Outcome", "ReferenceHost", "Start"]

#: Every operation type of the standard host features, in the order the
#: specification lists them.
TYPES: Tuple[str, ...] = (
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-put-if",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
    MODEL_GENERATE,
)


@dataclass(frozen=True)
class Invocation:
    """The identity of a host-run invocation (docs/SCE_HOST_FEATURES.md, section 1,
    "Identity"): the session that started it, the ``<invoke>``'s id, and which
    start of that ``<invoke>`` in that session it is. A restore starts a running
    invocation again under the identity it had, so two starts with one identity
    are one operation asked twice."""

    session_id: str
    id: str
    start: int


@dataclass
class Start:
    """A start the host was given: the operation, the invocation's id and the
    request's parameters, with the rest of the invocation's identity: the session
    that started it and which start of its ``<invoke>`` it is."""

    operation: str
    id: str
    params: Dict[str, str]
    session_id: str = ""
    number: int = 0


class ReferenceHost:
    """The standard host features' semantics, over a clock and a transport."""

    def __init__(self, clock_source: ClockSource, transport: Transport) -> None:
        self.clock = clock_source
        self.transport = transport
        self.store = kv.Store()
        self.notifications = notify.Notifications()
        #: Failures to inject: the operation type to the code it fails with.
        self.faults: Dict[str, str] = {}
        #: The invocations started and not yet answered, by id.
        self.held: List[str] = []
        #: The invocations the host was told to stop, in the order told.
        self.cancelled: List[str] = []
        #: The highest monotonic reading answered, which no answer goes below.
        self.monotonic_floor: Optional[int] = None
        self._starts: List[Start] = []
        #: Where a generation's text comes from: a scripted provider until the
        #: embedding gives another.
        self.provider: ModelProvider = ScriptedProvider()
        #: The machine's table of answer schemas, which a generation's
        #: `answer_schema` is resolved through.
        self.answer_schemas: List[Any] = []
        #: The first result of each put-if, by the identity of its invocation: what
        #: a repeat of that identity is answered with (docs/adr/0107, D4).
        self._first_results: Dict[Invocation, Outcome] = {}

    def set_model_provider(self, provider: ModelProvider) -> None:
        """Generate texts with `provider` from here on."""
        self.provider = provider

    def set_answer_schemas(self, schemas: Sequence[Any]) -> None:
        """Resolve a generation's `answer_schema` through `schemas`, the table of
        the machine this host serves (its generated ``ANSWER_SCHEMAS``). A request
        that names an id the table does not hold is refused."""
        self.answer_schemas = list(schemas)

    def set_faults(self, faults: Dict[str, str]) -> None:
        """Inject failures: each operation type named fails with the code it maps
        to, once the request has been accepted. An empty map ends them."""
        self.faults = dict(faults)

    def set_quota(self, keys: int) -> None:
        """Bound the store to `keys` keys; a write of a new key past it fails
        `quota_exceeded`."""
        self.store.quota = keys

    def set_permission(self, state: str) -> None:
        """The notification permission, as the platform says it: `granted`,
        `denied` or `undetermined`."""
        self.notifications.permission = state

    def set_user_decision(self, decision: str) -> None:
        """What the user chooses when asked: `granted`, `denied`, or `none` for a
        host with no user to ask."""
        self.notifications.user_decision = decision

    def store_entries(self) -> List[Tuple[str, str]]:
        """The store's keys and values, in key order."""
        return self.store.entries()

    def waiting(self) -> List[str]:
        """The ids of the waiting notifications, in the order posted."""
        return [n.id for n in self.notifications.waiting]

    def shown(self) -> List[str]:
        """The ids of the shown notifications, in the order posted."""
        return [n.id for n in self.notifications.shown]

    def take_starts(self) -> List[Start]:
        """The starts the host was given since this was last called, oldest first."""
        taken, self._starts = self._starts, []
        return taken

    def start(self, operation: str, invocation: Invocation, params: Dict[str, str]) -> Optional[Outcome]:
        """Start the operation of `invocation` with `params`.

        `None` for a type that is not one of the standard host features: the host
        serves none of it. Otherwise the arguments are judged first (a refusal
        starts nothing), then a failure the embedding injected, then the
        operation itself.

        A put-if whose invocation was answered before is answered with that first
        result, a failure included, and changes nothing: the engine starts a
        running invocation again under the same identity when it restores a state
        saved before the answer was taken (docs/adr/0107, D4), and the write it
        asked for was made once."""
        if operation not in TYPES:
            return None
        self._starts.append(
            Start(operation, invocation.id, dict(params), invocation.session_id, invocation.start)
        )
        keeps_first = operation == "sce-host-kv-put-if"
        if keeps_first and invocation in self._first_results:
            return self._first_results[invocation]
        outcome = self._perform(operation, invocation.id, params)
        if keeps_first and outcome.kind == DONE:
            self._first_results[invocation] = outcome
        return outcome

    def _perform(self, operation: str, invoke_id: str, params: Dict[str, str]) -> Outcome:
        """The operation itself, as `start` describes it, for a start that is not a
        repeat."""

        def argument(name: str) -> str:
            return params.get(name, "")

        if operation in ("sce-host-kv-get", "sce-host-kv-delete", "sce-host-kv-put"):
            refused = not kv.valid_key(argument("key"))
        elif operation == "sce-host-kv-put-if":
            refused = not kv.valid_key(argument("key")) or not kv.valid_version(
                argument("expected_version")
            )
        elif operation in ("sce-host-notify-post", "sce-host-notify-cancel"):
            refused = argument("id") == ""
        elif operation == "sce-host-http-fetch":
            refused = not http.acceptable(argument("method"), argument("url"), argument("headers"))
        elif operation == MODEL_GENERATE:
            refused = model.judge(self.answer_schemas, params) is None
        else:
            refused = False
        if refused:
            return Outcome(REFUSE, INVALID_ARGUMENT)
        if operation in self.faults:
            return failure(operation, self.faults[operation])
        if operation == "sce-host-clock-now":
            return clock.now(self)
        if operation == "sce-host-clock-zone":
            return clock.zone(self)
        if operation == "sce-host-kv-get":
            return kv.get(self, argument("key"))
        if operation == "sce-host-kv-put":
            return kv.put(self, argument("key"), argument("value"))
        if operation == "sce-host-kv-put-if":
            return kv.put_if(self, argument("key"), argument("value"), argument("expected_version"))
        if operation == "sce-host-kv-delete":
            return kv.delete(self, argument("key"))
        if operation == "sce-host-kv-next-key":
            return kv.next_key(self, argument("after"), argument("prefix"))
        if operation == "sce-host-notify-permission":
            return notify.permission(self)
        if operation == "sce-host-notify-request-permission":
            return notify.request_permission(self)
        if operation == "sce-host-notify-post":
            return notify.post(
                self, argument("id"), argument("title"), argument("body"), argument("at_ms")
            )
        if operation == "sce-host-notify-cancel":
            return notify.cancel(self, argument("id"))
        if operation == MODEL_GENERATE:
            return model.generate(self, invoke_id, params)
        return http.fetch(
            self, invoke_id, argument("method"), argument("url"), argument("headers"), argument("body")
        )

    def cancel(self, invoke_id: str) -> None:
        """The host is told to stop invocation `invoke_id`, because its state
        exited or its deadline passed. What it held for it is dropped."""
        self.held = [held for held in self.held if held != invoke_id]
        self.cancelled.append(invoke_id)
