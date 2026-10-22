# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A ``datamodel="sce-static"`` machine's whole state, saved at a macrostep
boundary and restored into a new process (SCE Accepted Subset section 2.15, E17).

What a saved state holds is everything a macrostep boundary holds that the
document cannot recompute: where the machine is (its configuration and current
leaf), every variable, what each ``<history>`` recorded, the delayed ``<send>``s
still waiting, the external queue in order, and, as a tree, every running child
session with the same (docs/adr/0045).

A delayed send is saved as the moment it comes due on the host's WALL clock,
milliseconds since the Unix epoch, not as a wait: a wait would start again when
the process came back, and a timer that ran out while it was dead would be late
by exactly as long as it was dead.

The format is one JSON document (``sce-saved-state``, version :data:`SAVED_FORMAT`),
the same on every backend, so what one backend saved another can read. A variable
is keyed by its document id and written as its ``sce:type`` says: a number, except
a 64-bit integer, written as a text so no reader that holds numbers as doubles
loses its low bits; a byte string as its Latin-1 text; a record as an object of
its fields; a list as an array.

A saved state is bound to the SHAPE of the document it was saved from, a digest
the generator computes of every state with its kind and parent and every variable
with its type, not to the document's text.

Cross-language siblings: ``backends/go/runtime/saved_state.go``,
``com.sce.runtime.SavedState`` (Kotlin) and ``sce_rust_runtime::saved_state``
(Rust).
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field, replace
from typing import Any, Callable, Dict, List, Optional, Tuple

from .saved_json import JsonError, JsonNumber, parse_json, write_json
from .session_id import refusal_of as session_id_refusal_of

__all__ = [
    "AddedField",
    "AddedVariant",
    "EnumHolder",
    "EnumLineage",
    "RecordHolder",
    "RecordLineage",
    "SAVED_FORMAT",
    "SAVED_FORMAT_1",
    "SAVED_FORMAT_2",
    "SAVED_SCHEMA_STATUS",
    "SavedAct",
    "SavedChild",
    "SavedEvent",
    "SavedHistory",
    "SavedHostInvoke",
    "SavedInvokeStart",
    "SavedInvokes",
    "SavedRevision",
    "SavedSend",
    "SavedShape",
    "SavedState",
    "StateRefusal",
    "check_saved_shape",
    "refusal_within",
    "saved_state_from_json",
    "saved_wall_clock_ms",
    "upgrade_saved_state",
]

# A state that names its running children by id alone.
SAVED_FORMAT_1 = 1
# A tree of sessions, each running child saved inside its parent (docs/adr/0045).
SAVED_FORMAT_2 = 2
# The newest format version this runtime writes; it reads this one and format 1.
SAVED_FORMAT = SAVED_FORMAT_2

# The stability status of the saved-state wire surface, held in lockstep with
# ``x-sce-schema-status`` in ``schemas/sce-saved-state.v1.schema.json`` and
# ``schemas/sce-saved-state.v2.schema.json`` (SCE_WIRE_CONTRACTS.md). A flip to
# "stable" changes all three in one commit.
SAVED_SCHEMA_STATUS = "pre-release"

# What a refusal inside a child session of a tree begins with, before the
# session's path.
_SESSION_AT = "the session at "

_INT64_MAX = (1 << 63) - 1

# What a waiting ``<send>`` does when it comes due: an event for the session that
# started this one (``#_parent``), for a running child (``#_<invokeid>``), for
# this session's external queue, for its internal queue, or a ``<send>`` a
# host-served processor performs (section 6.2.5 of the W3C text).
ACT_PARENT = "parent"
ACT_CHILD = "child"
ACT_RAISE = "raise"
ACT_INTERNAL = "internal"
ACT_HOST = "host"


class StateRefusal(Exception):
    """Why a saved state cannot be restored, or a machine cannot be saved."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


def refusal_within(err: Exception, path: str) -> StateRefusal:
    """``err``, a refusal of the child session at ``path`` of the session that
    reads it (``invokes[0].session``): a tree is refused as a whole, and the reason
    names the session it is about by its path from the root (docs/adr/0045,
    section 2), ``invokes[0].session.invokes[1].session`` for a grandchild."""
    reason = str(err)
    if reason.startswith(_SESSION_AT):
        return StateRefusal(_SESSION_AT + path + "." + reason[len(_SESSION_AT):])
    return StateRefusal(_SESSION_AT + path + ": " + reason)


@dataclass
class SavedHistory:
    """What one ``<history>`` recorded when its parent was last exited (3.10):
    state ids in document order, under the history's id."""

    id: str
    states: List[str]


@dataclass
class SavedInvokeStart:
    """How many times the ``<invoke>`` ``id`` has started in a session."""

    id: str
    count: int


@dataclass
class SavedChild:
    """A running child session, saved inside the session that started it
    (docs/adr/0045, section 1)."""

    # The id the document gives the ``<invoke>`` that started it.
    id: str
    # Which start of that ``<invoke>`` this is in its parent, counted from 1: the
    # count the parent's ``invoke_starts`` holds for it.
    start: int
    # The id the child was started under, as ``_event.invokeid`` and an
    # ``idlocation`` variable hold it: carried, not computed again.
    invoke_id: str
    # The document the child runs: the child's name for a static ``<invoke>``, the
    # candidate's for a hybrid one.
    document: str
    # The child's own state, its running children included.
    session: "SavedState"


@dataclass
class SavedInvokes:
    """The running child sessions of a saved session, as the format it was
    written in holds them.

    ``resumed`` is format 2: each running child is saved with its whole state and a
    restore resumes it where it stood. In format 1 the text holds the id of each
    ``<invoke>`` whose child is running and nothing of the child, so a restore
    starts each again from its beginning."""

    resumed: bool = False
    # Format 1's running invocations, by the id the document gives them.
    ids: List[str] = field(default_factory=list)
    # Format 2's running children, in document order.
    children: List[SavedChild] = field(default_factory=list)

    def invoke_ids(self) -> List[str]:
        """The id the document gives each running ``<invoke>``, in order."""
        if not self.resumed:
            return list(self.ids)
        return [child.id for child in self.children]


@dataclass
class SavedRevision:
    """The revision of one record schema or enum a state was saved at."""

    # The event the record schema declares, or ``enum:`` and the enum's document,
    # which names it in ``revisions``.
    key: str
    # The ``sce:revision`` where the state was saved, 2 or later.
    revision: int
    # The SHA-256, as lowercase hex, of what that revision declares.
    fields: str


@dataclass
class SavedEvent:
    """One event of a saved external queue: its name and the ``_event`` fields a
    document can read (5.10.1). A typed payload is not saved; it is lifted again
    from ``data`` when the event is delivered, the one path every other delivery
    takes.

    ``host_invoke_token`` is the token a host-run invocation's completion or
    failure was stamped with when it was accepted, ``None`` for any other event. A
    completion that was accepted and is still queued has to keep the stamp across
    a save, or the restored machine would refuse the one answer it was waiting
    for."""

    name: str = ""
    data: str = ""
    type: str = ""
    send_id: str = ""
    origin: str = ""
    origin_type: str = ""
    invoke_id: str = ""
    host_invoke_token: Optional[int] = None


@dataclass
class SavedHostInvoke:
    """An ``<invoke>`` a declared host invoker is running, as it is started again
    (6.4.1): every field is what the request the host was handed carried.

    ``due`` is when its deadline comes due, in milliseconds since the Unix epoch
    on the wall clock of the host that saved it; ``None`` for an invocation that
    has none."""

    processor_type: str = ""
    invoke_id: str = ""
    src: str = ""
    params: Dict[str, List[str]] = field(default_factory=dict)
    data: str = ""
    content: str = ""
    due: Optional[int] = None


@dataclass
class SavedAct:
    """What a waiting ``<send>`` does when it comes due. A delayed send to any
    other session is none of the five kinds, and a document that makes one is
    generated without the save API (2.15).

    ``event`` is the event's name, as the receiver resolves it; ``data`` is
    ``_event.data``, as the wire carries it; ``send_id`` is the ``<send>``'s id,
    which a ``<cancel>`` names; ``origin`` is ``_event.origin``; ``invoke_id``
    names the child a child act is for, and is ``_event.invokeid`` of the event a
    host act was sent while processing. ``processor_type``, ``target``,
    ``content`` and ``params`` are a host act's: every field is what the document
    wrote."""

    kind: str
    event: str = ""
    data: str = ""
    send_id: str = ""
    origin: str = ""
    invoke_id: str = ""
    processor_type: str = ""
    target: str = ""
    content: str = ""
    params: Dict[str, List[str]] = field(default_factory=dict)


@dataclass
class SavedSend:
    """One delayed ``<send>`` that has not been delivered yet. ``due`` is when it
    comes due, in milliseconds since the Unix epoch on the wall clock of the host
    that saved it."""

    due: int
    act: SavedAct


@dataclass
class SavedState:
    """A machine's whole state at a macrostep boundary.

    ``shape`` is the shape of the document the machine was generated from;
    ``configuration`` the active configuration, as state ids; ``current`` the
    current leaf, as a state id: which of a ``<parallel>``'s regions the machine
    last stood in, which the configuration alone cannot say; ``variables`` every
    variable by document id, in declaration order, each value as the JSON value
    model holds it; ``history`` what each ``<history>`` recorded, ordered by the
    history's id (one that has recorded nothing is absent); ``pending`` the delayed
    ``<send>``s still waiting, in the order they would be delivered; ``invokes``
    the ``<invoke>``s whose child session is running (6.4); ``host_invokes`` the
    ``<invoke>``s a declared host invoker is running (6.4.1), by ``(type, id)``;
    ``host_invoke_token`` the token the next host-run start receives;
    ``auto_send_seq`` how many ids the machine has generated for a
    ``<send idlocation>``; ``external`` the external queue, front first;
    ``revisions`` the revision each record schema and enum past its first was
    saved at, ordered by key (empty when every one is in its first revision);
    ``session_id`` the session's own id (``_sessionid``), an XML Nmtoken in format
    2 and empty in a format-1 state, which holds none; ``invoke_starts`` how many
    times each ``<invoke>`` of the document has started in this session, for each
    started at least once, in document order (empty in a format-1 state)."""

    shape: str = ""
    configuration: List[str] = field(default_factory=list)
    current: str = ""
    variables: Dict[str, Any] = field(default_factory=dict)
    history: List[SavedHistory] = field(default_factory=list)
    pending: List[SavedSend] = field(default_factory=list)
    invokes: SavedInvokes = field(default_factory=SavedInvokes)
    host_invokes: List[SavedHostInvoke] = field(default_factory=list)
    host_invoke_token: int = 0
    auto_send_seq: int = 0
    external: List[SavedEvent] = field(default_factory=list)
    revisions: List[SavedRevision] = field(default_factory=list)
    session_id: str = ""
    invoke_starts: List[SavedInvokeStart] = field(default_factory=list)

    def variable(self, id: str) -> Any:
        """The variable ``id``, or a refusal naming it."""
        if id not in self.variables:
            raise StateRefusal(f"the saved state has no variable '{id}'")
        return self.variables[id]

    def format(self) -> int:
        """The format this state is of: 2 for a tree of sessions, 1 for a state
        that names its running children by id alone."""
        return SAVED_FORMAT_2 if self.invokes.resumed else SAVED_FORMAT_1

    def bound_session_id(self) -> Optional[str]:
        """The id a restore binds the session to: the saved one, for a format-2
        state, since the session carries on and every child's id and a peer's
        address name it (docs/adr/0045); ``None`` for a format-1 state, which holds
        none, so the restored machine keeps the id it was created with."""
        if self.format() == SAVED_FORMAT_2:
            return self.session_id
        return None

    def invoke_start(self, id: str) -> int:
        """How many times the ``<invoke>`` ``id`` had started in this session when
        it was saved: 0 for one that had not, and for every one of a format-1
        state, which holds no count."""
        for started in self.invoke_starts:
            if started.id == id:
                return started.count
        return 0

    def to_json(self) -> str:
        """The state as ``sce-saved-state`` JSON, in the format it is of: the root
        of the text, with its ``format``."""
        members: Dict[str, Any] = {"format": JsonNumber(str(self.format()))}
        self._write_session_members(members)
        return write_json(members)

    def _session_members(self) -> Dict[str, Any]:
        """The members of this session, in the order every backend writes them:
        format 1's, and in format 2 ``sessionid`` after ``shape`` and
        ``invokestarts`` after ``invokes``, each running child's session written
        by the same rule."""
        members: Dict[str, Any] = {}
        self._write_session_members(members)
        return members

    def _write_session_members(self, members: Dict[str, Any]) -> None:
        members["shape"] = self.shape
        if self.invokes.resumed:
            members["sessionid"] = self.session_id
        members["configuration"] = list(self.configuration)
        members["current"] = self.current
        members["variables"] = self.variables
        members["history"] = {recorded.id: list(recorded.states) for recorded in self.history}
        members["pending"] = [_send_to_json(send) for send in self.pending]
        if self.invokes.resumed:
            members["invokes"] = [_child_to_json(child) for child in self.invokes.children]
            members["invokestarts"] = {started.id: str(started.count) for started in self.invoke_starts}
        else:
            members["invokes"] = list(self.invokes.ids)
        members["hostinvokes"] = [_host_invoke_to_json(host) for host in self.host_invokes]
        members["hostinvoketoken"] = str(self.host_invoke_token)
        members["sendseq"] = str(self.auto_send_seq)
        members["external"] = [_event_to_json(event) for event in self.external]
        # Absent, not empty, when no record schema is past its first revision: the
        # text of such a state is the text it was before revisions existed.
        if self.revisions:
            members["revisions"] = {
                saved.key: {"revision": JsonNumber(str(saved.revision)), "fields": saved.fields}
                for saved in self.revisions
            }


def _optional_count(count: Optional[int]) -> Optional[str]:
    """A count as the text a saved state writes it as, or ``None``."""
    return None if count is None else str(count)


def _params_to_json(params: Dict[str, List[str]]) -> Dict[str, Any]:
    """``params`` as a saved state writes them: by name, so the text one machine
    writes is every backend's."""
    return {name: list(params[name]) for name in sorted(params)}


def _child_to_json(child: SavedChild) -> Dict[str, Any]:
    return {
        "id": child.id,
        "start": str(child.start),
        "invokeid": child.invoke_id,
        "document": child.document,
        "session": child.session._session_members(),
    }


def _event_to_json(event: SavedEvent) -> Dict[str, Any]:
    return {
        "name": event.name,
        "data": event.data,
        "type": event.type,
        "sendid": event.send_id,
        "origin": event.origin,
        "origintype": event.origin_type,
        "invokeid": event.invoke_id,
        "hostinvoketoken": _optional_count(event.host_invoke_token),
    }


def _host_invoke_to_json(host: SavedHostInvoke) -> Dict[str, Any]:
    return {
        "type": host.processor_type,
        "id": host.invoke_id,
        "src": host.src,
        "params": _params_to_json(host.params),
        "data": host.data,
        "content": host.content,
        "due": _optional_count(host.due),
    }


def _send_to_json(send: SavedSend) -> Dict[str, Any]:
    members: Dict[str, Any] = {"due": str(send.due)}
    act = send.act
    members["act"] = act.kind
    if act.kind in (ACT_PARENT, ACT_RAISE, ACT_INTERNAL):
        members["event"] = act.event
        members["data"] = act.data
        members["sendid"] = act.send_id
        members["origin"] = act.origin
    elif act.kind == ACT_CHILD:
        members["invokeid"] = act.invoke_id
        members["event"] = act.event
        members["data"] = act.data
        members["sendid"] = act.send_id
        members["origin"] = act.origin
    elif act.kind == ACT_HOST:
        members["type"] = act.processor_type
        members["event"] = act.event
        members["target"] = act.target
        members["content"] = act.content
        members["params"] = _params_to_json(act.params)
        members["sendid"] = act.send_id
        members["data"] = act.data
        members["invokeid"] = act.invoke_id
    else:
        raise ValueError("a saved act of a kind no format names")
    return members


def saved_state_from_json(text: str) -> SavedState:
    """Reads ``sce-saved-state`` JSON, of format 1 or 2. The shape is not judged
    here; the machine that restores it knows its own. Raises
    :class:`StateRefusal` when the text is not one."""
    try:
        value = parse_json(text)
    except JsonError as err:
        raise StateRefusal(f"the saved state is {err}") from None
    if not isinstance(value, dict):
        raise StateRefusal("the saved state is not an object")
    if "format" not in value:
        raise StateRefusal("the saved state has no 'format'")
    written = value["format"]
    if written == JsonNumber("1"):
        form = SAVED_FORMAT_1
    elif written == JsonNumber("2"):
        form = SAVED_FORMAT_2
    else:
        raise StateRefusal(
            f"the saved state is format {write_json(written)}, and this runtime reads formats "
            f"{SAVED_FORMAT_1} and {SAVED_FORMAT_2}"
        )
    return _saved_session_from_json(value, form)


def _saved_session_from_json(members: Dict[str, Any], form: int) -> SavedState:
    """The session ``members`` hold, of a text of ``form``: the root, or a child's
    ``session``, which states no ``format`` of its own."""

    def read_field(name: str) -> Any:
        if name not in members:
            raise StateRefusal(f"the saved state has no '{name}'")
        return members[name]

    def text_of(value: Any, what: str) -> str:
        if not isinstance(value, str):
            raise StateRefusal(f"'{what}' is not a text")
        return value

    def field_text(name: str) -> str:
        return text_of(read_field(name), name)

    def field_array(name: str) -> list:
        items = read_field(name)
        if not isinstance(items, list):
            raise StateRefusal(f"'{name}' is not an array")
        return items

    state = SavedState()

    state.configuration = [text_of(item, "configuration") for item in field_array("configuration")]

    variables = read_field("variables")
    if not isinstance(variables, dict):
        raise StateRefusal("'variables' is not an object")
    state.variables = variables

    recorded = read_field("history")
    if not isinstance(recorded, dict):
        raise StateRefusal("'history' is not an object")
    for id, listed in recorded.items():
        if not isinstance(listed, list):
            raise StateRefusal(f"'history.{id}' is not an array")
        state.history.append(SavedHistory(id, [text_of(item, "history." + id) for item in listed]))

    for at, item in enumerate(field_array("pending")):
        state.pending.append(_saved_send_from_json(item, f"pending[{at}]", form))

    listed = field_array("invokes")
    if form == SAVED_FORMAT_1:
        state.invokes = SavedInvokes(resumed=False, ids=[text_of(item, "invokes") for item in listed])
    else:
        state.invokes = SavedInvokes(
            resumed=True, children=[_saved_child_from_json(item, at) for at, item in enumerate(listed)]
        )

    # Format 2's own members: the session's id and its count of starts.
    if form == SAVED_FORMAT_2:
        session_id = field_text("sessionid")
        if session_id_refusal_of(session_id) is not None:
            raise StateRefusal(f"'sessionid' ({session_id}) is not an XML Nmtoken, which a session id is")
        state.session_id = session_id
        counted = read_field("invokestarts")
        if not isinstance(counted, dict):
            raise StateRefusal("'invokestarts' is not an object")
        for id, count in counted.items():
            if not isinstance(count, str):
                raise StateRefusal(f"'invokestarts.{id}' is not a text")
            state.invoke_starts.append(SavedInvokeStart(id, _read_saved_count(count, "invokestarts." + id)))

    for at, item in enumerate(field_array("hostinvokes")):
        state.host_invokes.append(_saved_host_invoke_from_json(item, f"hostinvokes[{at}]"))

    counters: List[int] = []
    for name in ("hostinvoketoken", "sendseq"):
        digits = read_field(name)
        if not isinstance(digits, str):
            raise StateRefusal(f"'{name}' is not a text")
        counters.append(_read_saved_token(digits, name))
    state.host_invoke_token, state.auto_send_seq = counters

    for at, item in enumerate(field_array("external")):
        state.external.append(_saved_event_from_json(item, f"external[{at}]"))

    # Optional, unlike every member above: a state whose record schemas are all in
    # their first revision leaves it out, as every state saved before schemas had
    # revisions does.
    if "revisions" in members:
        named = members["revisions"]
        if not isinstance(named, dict):
            raise StateRefusal("'revisions' is not an object")
        if not named:
            raise StateRefusal(
                "'revisions' names no schema; a state whose record schemas are all in their first "
                "revision leaves it out"
            )
        for key, value in named.items():
            state.revisions.append(_saved_revision_from_json(key, value, "revisions." + key))

    state.shape = field_text("shape")
    state.current = field_text("current")
    return state


def _member_text(members: Dict[str, Any], what: str, key: str) -> str:
    """The text member ``key`` of ``members``, which ``what`` names in a
    refusal."""
    if key not in members:
        raise StateRefusal(f"'{what}' has no '{key}'")
    value = members[key]
    if not isinstance(value, str):
        raise StateRefusal(f"'{what}.{key}' is not a text")
    return value


def _member_object(value: Any, what: str) -> Dict[str, Any]:
    """``value`` as an object, which ``what`` names in a refusal."""
    if not isinstance(value, dict):
        raise StateRefusal(f"'{what}' is not an object")
    return value


def _saved_child_from_json(value: Any, index: int) -> SavedChild:
    what = f"invokes[{index}]"
    members = _member_object(value, what)

    def non_empty(key: str) -> str:
        written = _member_text(members, what, key)
        if written == "":
            raise StateRefusal(f"'{what}.{key}' is empty")
        return written

    start = _read_saved_count(non_empty("start"), what + ".start")
    if "session" not in members:
        raise StateRefusal(f"'{what}' has no 'session'")
    session_members = members["session"]
    if not isinstance(session_members, dict):
        raise StateRefusal(f"'{what}.session' is not an object")
    if "format" in session_members:
        raise StateRefusal(f"'{what}.session' states a format, which only the root of a tree does")
    try:
        session = _saved_session_from_json(session_members, SAVED_FORMAT_2)
    except StateRefusal as err:
        raise refusal_within(err, what + ".session") from None
    return SavedChild(
        id=non_empty("id"),
        start=start,
        invoke_id=non_empty("invokeid"),
        document=non_empty("document"),
        session=session,
    )


def _parse_saved_digits(written: str) -> Optional[int]:
    """``written`` as a signed 64-bit whole number, when it is nothing but ASCII
    digits."""
    if written == "" or any(c < "0" or c > "9" for c in written):
        return None
    n = int(written)
    return n if n <= _INT64_MAX else None


def _read_saved_count(written: str, what: str) -> int:
    """A count of starts, as the text a saved state writes it as: a whole number
    from 1 with no leading zero, within a signed 64-bit count, as every backend
    reads one."""
    n = _parse_saved_digits(written)
    if n is None or n < 1 or written.startswith("0"):
        raise StateRefusal(f"'{what}' ({written}) is not a count of starts, a whole number from 1")
    return n


def _read_saved_token(written: str, what: str) -> int:
    """A host-run invocation's token, as the text a saved state writes it as:
    digits, within a signed 64-bit count, as every backend reads one."""
    n = _parse_saved_digits(written)
    if n is None:
        raise StateRefusal(f"'{what}' ({written}) is not a whole number a token can be")
    return n


def _read_saved_moment(written: str, what: str) -> int:
    """A moment, as the text a saved state writes it as, milliseconds since the Unix
    epoch, which ``what`` (the member, named in a refusal) holds: a text of digits,
    as every 64-bit integer is, and one the other backends hold too."""
    n = _parse_saved_digits(written)
    if n is None:
        raise StateRefusal(f"'{what}' ({written}) is not a whole number of milliseconds")
    return n


def _saved_revision_from_json(key: str, value: Any, what: str) -> SavedRevision:
    members = _member_object(value, what)
    if "revision" not in members:
        raise StateRefusal(f"'{what}' has no 'revision'")
    if "fields" not in members:
        raise StateRefusal(f"'{what}' has no 'fields'")
    number = members["revision"]
    if not isinstance(number, JsonNumber):
        raise StateRefusal(f"'{what}.revision' is not a number")
    written = number.text
    if not _is_digits(written) or int(written) < 2:
        raise StateRefusal(
            f"'{what}.revision' ({written}) is not a revision past the first; a schema in its first "
            "revision is left out"
        )
    fields = members["fields"]
    if not isinstance(fields, str) or not _is_lower_hex_digest(fields):
        raise StateRefusal(f"'{what}.fields' is not a SHA-256 written as lowercase hex")
    return SavedRevision(key=key, revision=int(written), fields=fields)


def _is_digits(text: str) -> bool:
    return text != "" and all("0" <= c <= "9" for c in text)


def _is_lower_hex_digest(text: str) -> bool:
    return len(text) == 64 and all(("0" <= c <= "9") or ("a" <= c <= "f") for c in text)


def _optional_member(
    members: Dict[str, Any], what: str, key: str, read: Callable[[str, str], int]
) -> Optional[int]:
    """The member ``key`` of ``members``, which is null or a text read by
    ``read``."""
    if key not in members:
        raise StateRefusal(f"'{what}' has no '{key}'")
    value = members[key]
    if value is None:
        return None
    if isinstance(value, str):
        return read(value, f"{what}.{key}")
    raise StateRefusal(f"'{what}.{key}' is neither a text nor null")


def _saved_event_from_json(value: Any, what: str) -> SavedEvent:
    members = _member_object(value, what)
    event = SavedEvent()
    # The token is judged first, as every backend judges it.
    event.host_invoke_token = _optional_member(members, what, "hostinvoketoken", _read_saved_token)
    event.name = _member_text(members, what, "name")
    event.data = _member_text(members, what, "data")
    event.type = _member_text(members, what, "type")
    event.send_id = _member_text(members, what, "sendid")
    event.origin = _member_text(members, what, "origin")
    event.origin_type = _member_text(members, what, "origintype")
    event.invoke_id = _member_text(members, what, "invokeid")
    return event


def _saved_params_from_json(members: Dict[str, Any], what: str) -> Dict[str, List[str]]:
    """The member ``params`` of ``members``, read as an object of arrays of texts,
    by name; ``what`` names members in a refusal."""
    if "params" not in members:
        raise StateRefusal(f"'{what}' has no 'params'")
    params = members["params"]
    if not isinstance(params, dict):
        raise StateRefusal(f"'{what}.params' is not an object")
    read: Dict[str, List[str]] = {}
    for name, values in params.items():
        if not isinstance(values, list):
            raise StateRefusal(f"'{what}.params.{name}' is not an array")
        texts: List[str] = []
        for item in values:
            if not isinstance(item, str):
                raise StateRefusal(f"'{what}.params.{name}' holds a value that is not a text")
            texts.append(item)
        read[name] = texts
    return read


def _saved_host_invoke_from_json(value: Any, what: str) -> SavedHostInvoke:
    members = _member_object(value, what)
    host = SavedHostInvoke()
    host.due = _optional_member(members, what, "due", _read_saved_moment)
    host.processor_type = _member_text(members, what, "type")
    host.invoke_id = _member_text(members, what, "id")
    host.src = _member_text(members, what, "src")
    host.params = _saved_params_from_json(members, what)
    host.data = _member_text(members, what, "data")
    host.content = _member_text(members, what, "content")
    return host


def _saved_send_from_json(value: Any, what: str, form: int) -> SavedSend:
    """The waiting send ``value``, of a state of ``form``; a send to the parent or
    to a child is an act of format 2 only."""
    members = _member_object(value, what)
    due = _read_saved_moment(_member_text(members, what, "due"), what + ".due")
    name = _member_text(members, what, "act")
    act = SavedAct(kind=name)
    # The members each act reads, in the order every backend reads them.
    if name in (ACT_PARENT, ACT_CHILD):
        if form != SAVED_FORMAT_2:
            raise StateRefusal(f"'{what}.act' is '{name}', which is not raise, internal or host, the acts of format 1")
        keys = (["invokeid"] if name == ACT_CHILD else []) + ["event", "data", "sendid", "origin"]
    elif name in (ACT_RAISE, ACT_INTERNAL):
        keys = ["event", "data", "sendid", "origin"]
    elif name == ACT_HOST:
        keys = ["type", "event", "target", "content"]
    else:
        if form == SAVED_FORMAT_1:
            raise StateRefusal(f"'{what}.act' is '{name}', which is not raise, internal or host, the acts of format 1")
        raise StateRefusal(f"'{what}.act' is '{name}', which is not raise, internal, host, parent or child")
    attributes = {
        "invokeid": "invoke_id",
        "event": "event",
        "data": "data",
        "sendid": "send_id",
        "origin": "origin",
        "type": "processor_type",
        "target": "target",
        "content": "content",
    }
    for key in keys:
        setattr(act, attributes[key], _member_text(members, what, key))
    if name == ACT_HOST:
        act.params = _saved_params_from_json(members, what)
        act.send_id = _member_text(members, what, "sendid")
        act.data = _member_text(members, what, "data")
        act.invoke_id = _member_text(members, what, "invokeid")
    return SavedSend(due=due, act=act)


def saved_wall_clock_ms() -> int:
    """The wall clock now, in milliseconds since the Unix epoch: what a host that
    has no clock of its own to give ``save`` and ``restore`` gives them."""
    return time.time_ns() // 1_000_000


@dataclass
class AddedVariant:
    """A variant a revision past the first added to an enum."""

    name: str
    since: int


@dataclass
class EnumHolder:
    """Where a value of an :class:`EnumLineage` is held: ``variable`` is the
    variable's document id; ``field`` the field of the record the variable holds,
    for a value held in a record, ``None`` for a variable of the enum; ``list`` is
    whether the variable holds a list of such records rather than one."""

    variable: str
    field: Optional[str] = None
    list: bool = False


@dataclass
class EnumLineage:
    """An enum a value of a machine's state is of, as a saved state names it
    (docs/adr/0044): ``key`` is ``enum:`` and the enum's document, which names it
    in ``revisions``; ``revision`` the enum's revision in this machine;
    ``revisions`` the digest of what each revision from the second to ``revision``
    declares, in order; ``added`` each variant a revision past the first added;
    ``holders`` where a value of it is held."""

    key: str
    revision: int
    revisions: List[str] = field(default_factory=list)
    added: List[AddedVariant] = field(default_factory=list)
    holders: List[EnumHolder] = field(default_factory=list)


@dataclass
class AddedField:
    """A field a revision past the first added to a record schema: ``id`` is the
    field's id, the member a saved record holds it under; ``default`` its
    ``sce:default`` as saved-state JSON."""

    id: str
    since: int
    default: str


@dataclass
class RecordHolder:
    """A variable that holds a record of a :class:`RecordLineage`."""

    variable: str
    list: bool = False


@dataclass
class RecordLineage:
    """A record schema a machine's variables are of, as a saved state names it:
    ``event`` is the event the schema declares, which names it in ``revisions``;
    ``revision`` the schema's revision in this machine; ``revisions`` the digest of
    what each revision from the second to ``revision`` declares, in order;
    ``added`` each field a revision past the first added, in the schema's order;
    ``holders`` the variables that hold a record of it, or a list of them."""

    event: str
    revision: int
    revisions: List[str] = field(default_factory=list)
    added: List[AddedField] = field(default_factory=list)
    holders: List[RecordHolder] = field(default_factory=list)


@dataclass
class SavedShape:
    """What a machine's saved state is bound to: the document's shape, and the
    revisions of each schema a record variable of it is of (docs/adr/0044). The
    generator computes it, and every saved state is written and judged against it.

    ``first`` is the shape a state names when every record schema it holds is in
    its first revision. ``revised`` is the shape a state names when it names a later
    revision of any record schema; a reader that predates revisions knows only the
    first shape, so it refuses such a state rather than read it without the fields
    it does not know. ``format2_first`` is ``first`` as a format-2 state names it
    (docs/adr/0045, section 2): each enum-typed variable and record field spelled by
    its enum's document rather than by the alias it is imported under, and
    ``format2_revised`` is ``revised`` as a format-2 state names it. ``records`` is
    each record schema a variable is of, by the event it declares, ordered by it,
    and ``enums`` each enum a value of the state is of, by its key, ordered by it."""

    first: str
    revised: str
    format2_first: str
    format2_revised: str
    records: List[RecordLineage] = field(default_factory=list)
    enums: List[EnumLineage] = field(default_factory=list)


def check_saved_shape(saved: SavedState, shape: str) -> None:
    """Refuses ``saved`` unless it was saved from a document of this ``shape``,
    judged before any variable is read, so a state saved from another document is
    refused for that and not for whichever variable it happens to lack."""
    if saved.shape != shape:
        raise StateRefusal(f"the saved state is of a document of shape {saved.shape}, and this machine's is {shape}")


def upgrade_saved_state(saved: SavedState, shape: SavedShape) -> SavedState:
    """``saved`` as a state of the machine whose saved state is bound to ``shape``,
    or the refusal that says why it is not one (docs/adr/0044), judged before any
    variable is read, as :func:`check_saved_shape` is.

    A state that names no revision is of the first shape; one that does is of the
    revised shape, and each revision it names is of a schema a record variable of
    this machine is of, no later than this machine's revision of it, and declares
    what this machine's revision of that number declares. Such a state is then read
    as one of this machine: each record of a schema saved at an earlier revision
    takes the default of every field a later revision added. A record that already
    holds such a field is refused, since the revision it was saved at does not
    declare it."""
    # A format-2 state names an enum by its document and a format-1 state by the
    # alias it is imported under, so each is judged against its own shape
    # (docs/adr/0045, section 2).
    first, revised = shape.first, shape.revised
    if saved.format() == SAVED_FORMAT_2:
        first, revised = shape.format2_first, shape.format2_revised
    if not saved.revisions:
        check_saved_shape(saved, first)
    elif saved.shape == first:
        raise StateRefusal(
            "the saved state names revisions of its record schemas and has the shape of a state that names none"
        )
    else:
        check_saved_shape(saved, revised)
    for named in saved.revisions:
        # A record schema by the event it declares, an enum by its key.
        found: Optional[Tuple[int, List[str]]] = None
        for record in shape.records:
            if record.event == named.key:
                found = (record.revision, record.revisions)
                break
        if found is None:
            for lineage in shape.enums:
                if lineage.key == named.key:
                    found = (lineage.revision, lineage.revisions)
                    break
        if found is None:
            raise StateRefusal(f"'revisions' names '{named.key}', which no record or enum of this machine is")
        revision, digests = found
        if named.revision > revision:
            raise StateRefusal(
                f"the saved state was saved at revision {named.revision} of '{named.key}', later than this "
                f"machine's {revision}: a machine does not restore what it does not know"
            )
        # The digest of revision r is the (r - 2)th: the first has none.
        at = named.revision - 2
        if at < 0 or at >= len(digests) or digests[at] != named.fields:
            raise StateRefusal(
                f"revision {named.revision} of '{named.key}' declared other members where the state was "
                "saved than it does in this machine"
            )

    def revision_of(key: str) -> int:
        for named in saved.revisions:
            if named.key == key:
                return named.revision
        return 1

    # An enum value saved at an earlier revision of its enum is one of that
    # revision's variants. Judged on the values as saved, before a record is grown
    # by defaults, which are this machine's.
    for lineage in shape.enums:
        at = revision_of(lineage.key)
        later = [variant for variant in lineage.added if variant.since > at]
        if not later:
            continue
        for holder in lineage.holders:
            _check_enum_holder(saved, lineage, holder, at, later)
    variables: Dict[str, Any] = dict(saved.variables)
    for record in shape.records:
        at = revision_of(record.event)
        added = [added_field for added_field in record.added if added_field.since > at]
        if not added:
            continue
        for holder in record.holders:
            if holder.variable not in variables:
                continue
            value = variables[holder.variable]
            if not holder.list:
                variables[holder.variable] = _grown_record(value, holder.variable, record, at, added)
                continue
            if not isinstance(value, list):
                continue
            variables[holder.variable] = [
                _grown_record(item, f"{holder.variable}[{index}]", record, at, added)
                for index, item in enumerate(value)
            ]
    return replace(saved, variables=variables)


def _check_enum_holder(
    saved: SavedState, lineage: EnumLineage, holder: EnumHolder, at: int, later: List[AddedVariant]
) -> None:
    """Refuses a value ``holder`` holds, in ``saved``, that names one of ``later``,
    the variants of ``lineage`` added after ``at``, the revision it was saved at.
    What is not a text where one is held is left for the variable's own read to
    refuse."""
    if holder.variable not in saved.variables:
        return
    value = saved.variables[holder.variable]
    # Each value the holder holds, with what names it in a refusal.
    values: List[Tuple[str, Any]] = []
    if holder.field is None:
        values.append((holder.variable, value))
    elif not holder.list:
        if isinstance(value, dict) and holder.field in value:
            values.append((f"{holder.variable}.{holder.field}", value[holder.field]))
    else:
        items = value if isinstance(value, list) else []
        for index, item in enumerate(items):
            if isinstance(item, dict) and holder.field in item:
                values.append((f"{holder.variable}[{index}].{holder.field}", item[holder.field]))
    for what, held in values:
        if not isinstance(held, str):
            continue
        for variant in later:
            if variant.name == held:
                raise StateRefusal(
                    f"'{what}' holds '{held}', which revision {at} of '{lineage.key}' does not declare: "
                    f"revision {variant.since} added it"
                )


def _grown_record(value: Any, what: str, record: RecordLineage, at: int, added: List[AddedField]) -> Any:
    """The saved record ``value``, of ``record`` saved at revision ``at``, with the
    default of each of ``added``, the fields later revisions added; ``what`` names
    it in a refusal. A value that is not an object is returned as it is, for the
    record's own read to refuse."""
    if not isinstance(value, dict):
        return value
    grown = dict(value)
    for added_field in added:
        if added_field.id in grown:
            raise StateRefusal(
                f"'{what}' holds '{added_field.id}', which revision {at} of '{record.event}' does not "
                f"declare: revision {added_field.since} added it"
            )
        try:
            grown[added_field.id] = parse_json(added_field.default)
        except JsonError as err:
            raise StateRefusal(f"the default of '{added_field.id}' of '{record.event}' is {err}") from None
    return grown
