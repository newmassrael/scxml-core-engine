# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The saved-state model of the Python runtime: the texts every backend reads
(docs/adr/0045, section 8), the refusals each member that breaks a format is held
to, an event's origin as format 3 holds it and as a format-2 text is read
(docs/adr/0090, section 7), the value types a state is made of, and the upgrade of
a state saved before a record schema or an enum was revised (docs/adr/0044).
Cross-language sibling: ``backends/go/runtime/saved_state_test.go``."""

from __future__ import annotations

import re
from dataclasses import replace
from pathlib import Path
from typing import Dict

import pytest

from sce_runtime.event_origin import EventOrigin
from sce_runtime.io_processors import SCXML_EVENT_PROCESSOR_URI, scxml_origin
from sce_runtime.saved_json import JsonNumber, parse_json, write_json
from sce_runtime.saved_state import (
    AddedField,
    AddedVariant,
    EnumHolder,
    EnumLineage,
    RecordHolder,
    RecordLineage,
    SAVED_FORMAT,
    SAVED_FORMAT_1,
    SAVED_FORMAT_3,
    SavedInvokes,
    SavedRevision,
    SavedShape,
    SavedState,
    StateRefusal,
    WidenedField,
    check_saved_features,
    refusal_within,
    saved_state_from_json,
    upgrade_saved_state,
)
from sce_runtime.saved_values import (
    saved_read_bool,
    saved_read_bounded_string,
    saved_read_bytes,
    saved_read_float32,
    saved_read_float64,
    saved_read_int8,
    saved_read_int64,
    saved_read_list,
    saved_read_string,
    saved_read_uint8,
    saved_read_uint32,
    saved_read_uint64,
    saved_write_bytes,
    saved_write_float32,
    saved_write_float64,
    saved_write_int64,
    saved_write_uint8,
    saved_write_uint64,
)
from sce_runtime import session_id

_SAVED = Path(__file__).resolve().parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"


def shared_saved_texts() -> Dict[Path, str]:
    """The shared saved-state texts every backend reads and writes back as they
    are: format 3 in ``saved/`` and the unchanged format 1 in ``saved/format1/``.
    The format-2 texts in ``saved/format2/`` are read and written back as format 3
    (:func:`format_2_readings`)."""
    texts: Dict[Path, str] = {}
    for directory in (_SAVED, _SAVED / "format1"):
        for path in sorted(directory.glob("*.json")):
            texts[path] = path.read_text(encoding="utf-8").strip()
    assert len(texts) >= 20, f"the shared saved states lost instances: {len(texts)} read"
    return texts


@pytest.mark.parametrize("path", sorted(shared_saved_texts()), ids=lambda p: f"{p.parent.name}/{p.name}")
def test_every_shared_saved_text_reads_and_writes_back_as_it_was(path: Path) -> None:
    text = shared_saved_texts()[path]
    assert saved_state_from_json(text).to_json() == text


def test_a_format_one_text_is_of_format_one_and_holds_no_session() -> None:
    saved = saved_state_from_json(shared_saved_texts()[_SAVED / "format1" / "static_invoke_working.json"])
    assert saved.format() == SAVED_FORMAT_1
    assert saved.invokes.ids
    assert saved.session_id == ""
    assert saved.bound_session_id() is None


def test_a_tree_text_holds_every_child_with_its_session() -> None:
    saved = saved_state_from_json(shared_saved_texts()[_SAVED / "static_invoke_tree_working.json"])
    assert saved.format() == SAVED_FORMAT_3
    assert SAVED_FORMAT == SAVED_FORMAT_3, "this runtime writes format 3"
    assert saved.invokes.children
    child = saved.invokes.children[0]
    assert child.session.session_id == f"{saved.session_id}.{child.invoke_id}"
    assert saved.invoke_start(child.id) == child.start
    # The child's answer, queued in the root, carries the child's pair and the
    # child as its sender; the parent's `swap`, queued in the child, carries none.
    (answer,) = saved.external
    assert (answer.origin, answer.sender) == (scxml_origin(child.session.session_id), child.session.session_id)
    (swap,) = child.session.external
    assert (swap.origin, swap.sender) == (None, None)
    # Each waiting send of a session carries that session's pair and its id.
    (nudge,) = saved.pending
    assert (nudge.act.origin, nudge.act.sender) == (scxml_origin(saved.session_id), saved.session_id)
    (report,) = child.session.pending
    assert (report.act.origin, report.act.sender) == (
        scxml_origin(child.session.session_id),
        child.session.session_id,
    )


_READ_AS = _SAVED / "format2" / "read_as"


def format_2_readings() -> Dict[str, str]:
    """The format-2 reading fixtures, by name (docs/adr/0090, section 7): each is
    read as the format-3 text of the same name in ``saved/format2/read_as/``."""
    texts = {
        path.name: path.read_text(encoding="utf-8").strip()
        for path in sorted((_SAVED / "format2").glob("*.json"))
    }
    assert len(texts) >= 4, f"the format-2 reading fixtures lost instances: {len(texts)} read"
    return texts


@pytest.mark.parametrize("name", sorted(format_2_readings()))
def test_a_format_two_text_is_read_by_the_table_and_written_back_as_format_three(name: str) -> None:
    saved = saved_state_from_json(format_2_readings()[name])
    assert saved.format() == SAVED_FORMAT_3
    assert saved.to_json() == (_READ_AS / name).read_text(encoding="utf-8").strip()


def test_a_format_two_origin_is_read_as_the_pair_the_table_gives() -> None:
    # The rows saved/format2/README.md names for the root's queue: a stored id and
    # an SCXML location are the session's pair, an empty address is none whatever
    # type beside it, and anything else is the pair as stored, with no sender.
    saved = saved_state_from_json(format_2_readings()["static_invoke_tree_working_origins.json"])
    worker = "fixture-root.working.2.worker"
    read = {event.name: (event.origin, event.sender) for event in saved.external}
    assert read == {
        "answer": (scxml_origin(worker), worker),
        "report": (scxml_origin(worker), worker),
        "go": (None, None),
        "swap": (None, None),
        "stamped": (scxml_origin("peer:1"), "peer:1"),
        "ask": (scxml_origin("peer:1"), "peer:1"),
        "arm": (EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://ecu"), None),
        "rest": (EventOrigin("x-sce-host", "motor"), None),
        "back": (EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://ecu"), None),
    }
    # An unencoded location is written again as the session publishes it.
    assert scxml_origin("peer:1").address == "sce://scxml/peer%3A1"


def test_a_format_two_internal_act_is_read_with_no_origin_and_no_sender() -> None:
    # Its stored origin was the sender's id, and the event it delivers is internal.
    saved = saved_state_from_json(format_2_readings()["static_timers.json"])
    read = [(send.act.kind, send.act.origin, send.act.sender) for send in saved.pending]
    root = scxml_origin("fixture-root")
    assert read == [
        ("internal", None, None),
        ("raise", root, "fixture-root"),
        ("raise", root, "fixture-root"),
        ("raise", root, "fixture-root"),
    ]


def test_a_format_two_act_with_an_empty_origin_was_sent_by_the_session_that_holds_it() -> None:
    # Each session's own sends: the root's `raise` and the child's `parent` are
    # read as sent by the `sessionid` of the session record each sits in, and are
    # written back as format 3 with that session's pair. The internal act carries
    # none whatever was stored.
    def send(act: str) -> str:
        return '{"due":"5","act":"' + act + '","event":"e","data":"","sendid":"","origin":""}'

    child_session = _MINIMAL.replace('"root"', '"root.w"', 1).replace(
        '"pending":[]', '"pending":[' + send("parent") + "]", 1
    )
    child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + child_session + "}}"
    text = (
        '{"format":2,'
        + _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1).replace(
            '"pending":[]', '"pending":[' + send("raise") + "," + send("internal") + "]", 1
        )
        + "}"
    )
    saved = saved_state_from_json(text)
    assert [(s.act.kind, s.act.origin, s.act.sender) for s in saved.pending] == [
        ("raise", scxml_origin("root"), "root"),
        ("internal", None, None),
    ]
    (parent,) = saved.invokes.children[0].session.pending
    assert (parent.act.origin, parent.act.sender) == (scxml_origin("root.w"), "root.w")
    again = saved_state_from_json(saved.to_json())
    assert again.format() == SAVED_FORMAT_3
    assert again.pending[0].act.sender == "root"


def test_a_format_one_act_with_an_empty_origin_names_no_session() -> None:
    # A format-1 text holds no session id: the act is read with neither member,
    # and the machine that restores it arms it as its own send.
    text = (
        '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
        '"pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":""}],"invokes":[],'
        '"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}'
    )
    saved = saved_state_from_json(text)
    assert (saved.pending[0].act.origin, saved.pending[0].act.sender) == (None, None)
    assert saved.to_json() == text


def test_a_format_one_text_keeps_its_origins_as_format_two_stored_them() -> None:
    # Read without a machine and written back, a format-1 text stays format 1: an
    # event's origin is the sender's id for a session's send and the address
    # otherwise, its type the pair's processor, and a raise act stores the id.
    events = (
        '{"name":"a","data":"","type":"external","sendid":"","origin":"kid","origintype":"'
        + SCXML_EVENT_PROCESSOR_URI
        + '","invokeid":"","hostinvoketoken":null},'
        '{"name":"b","data":"","type":"external","sendid":"","origin":"motor","origintype":"x-sce-host",'
        '"invokeid":"","hostinvoketoken":null}'
    )
    text = (
        '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
        '"pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":"root"}],"invokes":[],'
        '"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[' + events + "]}"
    )
    saved = saved_state_from_json(text)
    assert [(event.origin, event.sender) for event in saved.external] == [
        (scxml_origin("kid"), "kid"),
        (EventOrigin("x-sce-host", "motor"), None),
    ]
    assert (saved.pending[0].act.origin, saved.pending[0].act.sender) == (scxml_origin("root"), "root")
    assert saved.to_json() == text


_MINIMAL = (
    '"shape":"s","sessionid":"root","configuration":["a"],"current":"a","variables":{},'
    '"history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0",'
    '"sendseq":"0","external":[]'
)


@pytest.mark.parametrize(
    "text, reason",
    [
        ('{"format":', "the saved state is not JSON (it ends where a value was expected)"),
        ("[]", "the saved state is not an object"),
        ("{}", "the saved state has no 'format'"),
        # A format of the future is refused by its `format` before anything else.
        ('{"format":4}', "the saved state is format 4, and this runtime reads formats 1, 2 and 3"),
        ('{"format":"2"}', 'the saved state is format "2", and this runtime reads formats 1, 2 and 3'),
        (
            '{"format":3,' + _MINIMAL.replace('"root"', '"a b"', 1) + "}",
            "'sessionid' (a b) is not an XML Nmtoken, which a session id is",
        ),
        # Format 2 is a tree too, held to the same members.
        (
            '{"format":2,' + _MINIMAL.replace('"root"', '"a b"', 1) + "}",
            "'sessionid' (a b) is not an XML Nmtoken, which a session id is",
        ),
        (
            '{"format":3,' + _MINIMAL.replace('"invokestarts":{}', '"invokestarts":{"w":"01"}', 1) + "}",
            "'invokestarts.w' (01) is not a count of starts, a whole number from 1",
        ),
        (
            '{"format":3,' + _MINIMAL.replace('"hostinvoketoken":"0"', '"hostinvoketoken":"x"', 1) + "}",
            "'hostinvoketoken' (x) is not a whole number a token can be",
        ),
        (
            '{"format":3,' + _MINIMAL + ',"revisions":{}}',
            "'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out",
        ),
        (
            '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
            '"pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],'
            '"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}',
            "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1",
        ),
        (
            '{"format":3,' + _MINIMAL.replace('"pending":[]', '"pending":[{"due":"5","act":"fax"}]', 1) + "}",
            "'pending[0].act' is 'fax', which is not raise, internal, host, parent or child",
        ),
        # A stored origin that is the SCXML prefix alone names no session, and a
        # session's own send names the session that sent it.
        (
            '{"format":2,'
            + _MINIMAL.replace(
                '"pending":[]',
                '"pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":"sce://scxml/"}]',
                1,
            )
            + "}",
            "'pending[0].origin' is 'sce://scxml/', the SCXML prefix alone, which names no session: a raise act "
            "is a session's own send",
        ),
        (
            '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
            '"pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":"sce://scxml/"}],'
            '"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}',
            "'pending[0].origin' is 'sce://scxml/', the SCXML prefix alone, which names no session: a raise act "
            "is a session's own send",
        ),
    ],
)
def test_a_text_the_formats_do_not_carry_is_refused_for_the_member_that_breaks_it(text: str, reason: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(text)
    assert str(refused.value) == reason


# The SCXML pair of the session `root`, as format 3 writes it.
_ROOT_PAIR = '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"sce://scxml/root"}'


def _event_members(origin: str, sender: str) -> str:
    """A queued event of a format-3 text with `origin` and `sender` as written."""
    return (
        '{"name":"e","data":"","type":"external","sendid":"","origin":' + origin + ',"sender":' + sender
        + ',"invokeid":"","hostinvoketoken":null}'
    )


def _holding_event(event: str) -> str:
    return '{"format":3,' + _MINIMAL.replace('"external":[]', '"external":[' + event + "]", 1) + "}"


def _holding_act(act: str, origin: str, sender: str) -> str:
    """A format-3 text whose one waiting send is an `act` act to the session's
    queues with `origin` and `sender` as written."""
    send = (
        '{"due":"5","act":"' + act + '","event":"e","data":"","sendid":"","origin":' + origin
        + ',"sender":' + sender + "}"
    )
    return '{"format":3,' + _MINIMAL.replace('"pending":[]', '"pending":[' + send + "]", 1) + "}"


@pytest.mark.parametrize(
    "text, reason",
    [
        # An origin that is neither null nor a pair.
        (
            _holding_event(_event_members('"root"', "null")),
            "'external[0].origin' is neither null nor a pair of 'processor' and 'address'",
        ),
        (
            _holding_event(_event_members("[]", "null")),
            "'external[0].origin' is neither null nor a pair of 'processor' and 'address'",
        ),
        # A pair with a member missing, empty, not a text or added.
        (
            _holding_event(_event_members('{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '"}', "null")),
            "'external[0].origin' has no 'address'",
        ),
        (
            _holding_event(_event_members('{"processor":"","address":"mesh://ecu"}', "null")),
            "'external[0].origin.processor' is empty, and a pair is never half of one",
        ),
        (
            _holding_event(_event_members('{"processor":"x-sce-host","address":""}', "null")),
            "'external[0].origin.address' is empty, and a pair is never half of one",
        ),
        (
            _holding_event(_event_members('{"processor":1,"address":"motor"}', "null")),
            "'external[0].origin.processor' is not a text",
        ),
        (
            _holding_event(_event_members('{"processor":"x-sce-host","address":"motor","sender":"root"}', "null")),
            "'external[0].origin' holds 'sender', and a pair holds 'processor' and 'address' alone",
        ),
        # A sender that is not a session id.
        (
            _holding_event(_event_members(_ROOT_PAIR, "1")),
            "'external[0].sender' is neither a text nor null",
        ),
        (
            _holding_event(_event_members(_ROOT_PAIR, '"a b"')),
            "'external[0].sender' (a b) is not an XML Nmtoken, which a session id is",
        ),
        # A sender beside no origin: a session that sent an event has a reply
        # address.
        (
            _holding_event(_event_members("null", '"root"')),
            "'external[0].sender' is 'root' beside a null 'origin', and a session that sent an event has a "
            "reply address",
        ),
        # Both members are always written.
        (
            _holding_event(
                '{"name":"e","data":"","type":"external","sendid":"","origin":null,"invokeid":"",'
                '"hostinvoketoken":null}'
            ),
            "'external[0]' has no 'sender'",
        ),
        (
            _holding_event(
                '{"name":"e","data":"","type":"external","sendid":"","origin":"","origintype":"",'
                '"invokeid":"","hostinvoketoken":null}'
            ),
            "'external[0].origin' is neither null nor a pair of 'processor' and 'address'",
        ),
        (
            '{"format":3,'
            + _MINIMAL.replace(
                '"pending":[]', '"pending":[{"due":"5","act":"raise","event":"e","data":"","sendid":""}]', 1
            )
            + "}",
            "'pending[0]' has no 'origin'",
        ),
        # A session's own send with no sender, and an internal act that holds
        # either member.
        (
            _holding_act("raise", _ROOT_PAIR, "null"),
            "'pending[0]' is a raise act with no 'sender', and the session that sent it is its sender",
        ),
        (
            _holding_act("parent", "null", "null"),
            "'pending[0]' is a parent act with no 'sender', and the session that sent it is its sender",
        ),
        (
            _holding_act("internal", _ROOT_PAIR, '"root"'),
            "'pending[0]' is an internal act and holds an origin or a sender, which an internal event has not",
        ),
        (
            _holding_act("internal", _ROOT_PAIR, "null"),
            "'pending[0]' is an internal act and holds an origin or a sender, which an internal event has not",
        ),
        # A session's own send carries its own SCXML pair, unlike a queued event,
        # whose address may be any reply address.
        (
            _holding_act("raise", _ROOT_PAIR, '"other"'),
            "'pending[0].sender' is 'other', and this session is 'root': a raise act is this session's own send",
        ),
        (
            _holding_act(
                "raise", '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"sce://scxml/other"}', '"other"'
            ),
            "'pending[0].sender' is 'other', and this session is 'root': a raise act is this session's own send",
        ),
        (
            _holding_act(
                "parent", '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"mesh://dev/m/root"}', '"root"'
            ),
            "'pending[0].sender' is 'root', and its 'origin' is not the SCXML pair of that session",
        ),
        (
            _holding_act("raise", "null", '"root"'),
            "'pending[0].sender' is 'root' beside a null 'origin', and a session that sent an event has a "
            "reply address",
        ),
    ],
)
def test_a_format_three_origin_that_no_run_writes_is_refused_and_not_repaired(text: str, reason: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(text)
    assert str(refused.value) == reason


def test_a_format_three_origin_is_read_as_written_and_written_back_as_it_was() -> None:
    # Neither member is derived from the other: an SCXML pair at an address that
    # names no session has no sender, a host's pair keeps its own type, and a
    # queued event's sender stands beside whatever reply address the processor
    # that delivered it wrote: a remote child's Mesh endpoint, or a location that
    # is not the sender's own.
    mesh_child = '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"mesh://dev/machine/root.w"}'
    unencoded = '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"sce://scxml/peer:1"}'
    events = ",".join(
        [
            _event_members(
                '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"sce://scxml/peer%3A1"}', '"peer:1"'
            ),
            _event_members('{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"mesh://ecu"}', "null"),
            _event_members('{"processor":"x-sce-host","address":"motor"}', "null"),
            _event_members("null", "null"),
            _event_members(mesh_child, '"root.w"'),
            _event_members(_ROOT_PAIR, '"other"'),
            _event_members(unencoded, '"peer:1"'),
            _event_members('{"processor":"x-sce-host","address":"motor"}', '"motor"'),
        ]
    )
    acts = ",".join(
        [
            '{"due":"5","act":"raise","event":"e","data":"","sendid":"","origin":' + _ROOT_PAIR + ',"sender":"root"}',
            '{"due":"5","act":"internal","event":"e","data":"","sendid":"","origin":null,"sender":null}',
        ]
    )
    text = (
        '{"format":3,'
        + _MINIMAL.replace('"external":[]', '"external":[' + events + "]", 1).replace(
            '"pending":[]', '"pending":[' + acts + "]", 1
        )
        + "}"
    )
    saved = saved_state_from_json(text)
    assert [(event.origin, event.sender) for event in saved.external] == [
        (scxml_origin("peer:1"), "peer:1"),
        (EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://ecu"), None),
        (EventOrigin("x-sce-host", "motor"), None),
        (None, None),
        (EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://dev/machine/root.w"), "root.w"),
        (scxml_origin("root"), "other"),
        (EventOrigin(SCXML_EVENT_PROCESSOR_URI, "sce://scxml/peer:1"), "peer:1"),
        (EventOrigin("x-sce-host", "motor"), "motor"),
    ]
    assert [(send.act.origin, send.act.sender) for send in saved.pending] == [
        (scxml_origin("root"), "root"),
        (None, None),
    ]
    assert saved.to_json() == text


def test_a_child_session_is_refused_by_its_path() -> None:
    def with_session(session: str) -> str:
        return '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + session + "}}"

    def holding(child: str) -> str:
        return _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1)

    bad = with_session(_MINIMAL.replace('"configuration":["a"]', '"configuration":[1]', 1))
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json('{"format":3,' + holding(bad) + "}")
    assert str(refused.value) == "the session at invokes[0].session: 'configuration' is not a text"
    with pytest.raises(StateRefusal) as grandchild:
        saved_state_from_json('{"format":3,' + holding(with_session(holding(bad))) + "}")
    assert (
        str(grandchild.value)
        == "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text"
    )


def test_a_child_that_states_a_format_is_refused() -> None:
    child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":3,' + _MINIMAL + "}}"
    text = '{"format":3,' + _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1) + "}"
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(text)
    assert str(refused.value) == "'invokes[0].session' states a format, which only the root of a tree does"


def test_a_child_session_is_read_in_the_format_of_its_root() -> None:
    # One format per text: a child of a format-2 root stores its queue as format 2
    # did, and a child of a format-3 root holds the pair.
    stored = (
        '{"name":"e","data":"","type":"external","sendid":"","origin":"root","origintype":"",'
        '"invokeid":"w","hostinvoketoken":null}'
    )
    child_session = _MINIMAL.replace('"root"', '"root.w"', 1).replace('"external":[]', '"external":[' + stored + "]", 1)
    child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + child_session + "}}"
    holding = _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1)
    (event,) = saved_state_from_json('{"format":2,' + holding + "}").invokes.children[0].session.external
    assert (event.origin, event.sender) == (scxml_origin("root"), "root")
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json('{"format":3,' + holding + "}")
    assert str(refused.value) == (
        "the session at invokes[0].session: 'external[0].origin' is neither null nor a pair of "
        "'processor' and 'address'"
    )


def test_a_childs_own_send_names_the_child_and_not_its_parent() -> None:
    # A child's `parent` act is the child's own send: it carries the child's pair
    # and id, and one that names the root is refused by the child's path.
    def holding_parent_act(origin: str, sender: str) -> str:
        send = (
            '{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":' + origin + ',"sender":'
            + sender + "}"
        )
        child_session = _MINIMAL.replace('"root"', '"root.w"', 1).replace('"pending":[]', '"pending":[' + send + "]", 1)
        child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + child_session + "}}"
        return '{"format":3,' + _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1) + "}"

    child_pair = '{"processor":"' + SCXML_EVENT_PROCESSOR_URI + '","address":"sce://scxml/root.w"}'
    (report,) = saved_state_from_json(holding_parent_act(child_pair, '"root.w"')).invokes.children[0].session.pending
    assert (report.act.origin, report.act.sender) == (scxml_origin("root.w"), "root.w")
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(holding_parent_act(_ROOT_PAIR, '"root"'))
    assert str(refused.value) == (
        "the session at invokes[0].session: 'pending[0].sender' is 'root', and this session is 'root.w': "
        "a parent act is this session's own send"
    )


def test_a_saved_state_is_read_back_as_it_was_written() -> None:
    # The configuration a session was taken in (docs/adr/0057, section 7) is the
    # last member of the session, after `revisions`, in the root and in each child.
    child_session = _MINIMAL.replace('"root"', '"root.w"', 1) + ',"features":[]'
    child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + child_session + "}}"
    revisions = '"revisions":{"reading.taken":{"revision":3,"fields":"' + "0" * 64 + '"}}'
    text = (
        '{"format":3,'
        + _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1)
        + ","
        + revisions
        + ',"features":["session-unicast","tls"]}'
    )
    saved = saved_state_from_json(text)
    assert saved.features == ["session-unicast", "tls"]
    # The empty configuration is one, and no configuration is another.
    assert saved.invokes.children[0].session.features == []
    assert saved.to_json() == text
    # A machine of no product line records no configuration, and its text is the
    # one it was before configurations were recorded.
    plain = replace(saved, features=None)
    assert plain.to_json() == text.replace(',"features":["session-unicast","tls"]', "", 1)
    assert saved_state_from_json(plain.to_json()).features is None
    # Format 1 records it too, after `external` when no revision is past its first.
    format1 = (
        '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
        '"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],'
        '"features":["tls"]}'
    )
    taken = saved_state_from_json(format1)
    assert taken.format() == SAVED_FORMAT_1
    assert taken.features == ["tls"]
    assert taken.to_json() == format1


def test_a_state_of_another_configuration_is_refused_by_the_features_that_differ() -> None:
    # A restore judges the configuration a state was taken in before its shape,
    # and names the features that differ (docs/adr/0057, section 7).
    def taken(features):
        return SavedState(shape="a" * 64, features=None if features is None else list(features))

    def refused(saved: SavedState, built) -> str:
        with pytest.raises(StateRefusal) as refusal:
            check_saved_features(saved, built)
        return str(refusal.value)

    check_saved_features(taken(None), None)
    check_saved_features(taken([]), [])
    check_saved_features(taken(["a", "b"]), ["a", "b"])
    assert refused(taken(["a", "b"]), ["a", "c"]) == (
        "the saved state was taken with the features [a, b] on, and this machine is built "
        "with [a, c]: [b] on only where it was saved, [c] on only in this build"
    )
    assert refused(taken(["a"]), []) == (
        "the saved state was taken with the features [a] on, and this machine is built "
        "with []: [a] on only where it was saved"
    )
    assert refused(taken(None), ["a"]) == (
        "the saved state records no features, and this machine is built with the features [a] on"
    )
    assert refused(taken([]), None) == (
        "the saved state was taken with the features [] on, and this machine is built "
        "from a document of no feature model"
    )
    # Judged before the shape: a state of another build is refused for its
    # features even where its shape differs too.
    shape = SavedShape(
        first="first",
        revised="revised",
        format2_first="first2",
        format2_revised="revised2",
        features=["a"],
    )
    with pytest.raises(StateRefusal) as other_build:
        upgrade_saved_state(taken(["b"]), shape)
    assert str(other_build.value).startswith("the saved state was taken with the features [b] on")
    # Of the same configuration, the shape is judged next.
    with pytest.raises(StateRefusal) as other_shape:
        upgrade_saved_state(taken(["a"]), shape)
    assert str(other_shape.value) == (
        f"the saved state is of a document of shape {'a' * 64}, and this machine's is first"
    )


def _format1_with_features(features: str) -> str:
    return (
        '{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},'
        '"pending":[],"invokes":[],"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[],'
        '"features":' + features + "}"
    )


def test_the_features_of_a_saved_state_are_a_sorted_set() -> None:
    # `features` is a set, written sorted: a text that repeats a feature or names
    # them out of order is refused as it is read.
    assert saved_state_from_json(_format1_with_features('["a","b"]')).features == ["a", "b"]
    for written, reason in [
        ('["b","a"]', "'features' is not in order or names a feature twice; it is a set, sorted"),
        ('["a","a"]', "'features' is not in order or names a feature twice; it is a set, sorted"),
        ('"a"', "'features' is not an array"),
        ('["a",1]', "'features' is not a text"),
    ]:
        with pytest.raises(StateRefusal) as refused:
            saved_state_from_json(_format1_with_features(written))
        assert str(refused.value) == reason, written


def test_a_json_number_keeps_the_spelling_it_was_written_in() -> None:
    text = '{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"\\u00e9\\ud83d\\ude00"}'
    # A character written as a \u escape is written as the character.
    assert write_json(parse_json(text)) == '{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"é😀"}'


def test_a_json_object_keeps_the_order_its_members_were_written_in() -> None:
    assert "".join(parse_json('{"z":1,"a":2,"m":3}')) == "zam"


@pytest.mark.parametrize(
    "text, want",
    [
        ('{"a":1,"a":2}', 'not JSON (the field "a" appears twice)'),
        ("1 2", "more than one JSON value"),
        ("[1,]", "not JSON (unexpected ']')"),
        ('"\\q"', "not JSON (unknown escape '\\q')"),
        ('"\\u12"', "not JSON (a \\u escape is short)"),
        ('"\\u12zz"', "not JSON (a \\u escape is not hex)"),
        ("-", "not JSON (a number has no digits)"),
        ('{"a" 1}', "not JSON (a ':' was expected)"),
        ("tru", "not JSON (expected 'true')"),
    ],
)
def test_a_text_that_is_not_one_json_value_is_refused(text: str, want: str) -> None:
    from sce_runtime.saved_json import JsonError

    with pytest.raises(JsonError) as refused:
        parse_json(text)
    assert str(refused.value) == want


def test_a_lone_surrogate_reads_as_the_replacement_character() -> None:
    assert parse_json('"\\ud800x\\udc00"') == "\ufffdx\ufffd"


def test_every_value_type_is_written_as_its_saved_form_and_read_back_as_it_was() -> None:
    assert write_json(saved_write_uint8(200)) == "200"
    assert write_json(saved_write_int64(-9223372036854775808)) == '"-9223372036854775808"'
    assert write_json(saved_write_uint64(18446744073709551615)) == '"18446744073709551615"'
    assert write_json(saved_write_float32(0.30000001192092896)) == "0.30000001192092896"
    assert write_json(saved_write_float64(5.0)) == "5"
    assert write_json(saved_write_float64(1e21)) == "1e+21"
    assert write_json(saved_write_bytes(bytes([0, 0xE9, 0xFF]))) == '"\\u0000\u00e9\u00ff"'
    reads = [
        (saved_read_uint8, "255", 255),
        (saved_read_int8, "-128", -128),
        (saved_read_uint32, "4294967295", 4294967295),
        (saved_read_int64, '"-9223372036854775808"', -9223372036854775808),
        (saved_read_uint64, '"18446744073709551615"', 18446744073709551615),
        (saved_read_float64, "1.5", 1.5),
        (saved_read_float32, '"-Infinity"', float("-inf")),
        (saved_read_float64, "1e999", float("inf")),
        (saved_read_bool, "true", True),
    ]
    for read, text, want in reads:
        assert read(parse_json(text), "x") == want, text


def test_a_32_bit_real_is_rounded_to_the_single_it_is() -> None:
    assert saved_read_float32(parse_json("0.3"), "x") == 0.30000001192092896


@pytest.mark.parametrize(
    "text, read, want",
    [
        ("256", saved_read_uint8, "'x' (256) is not a whole number its type can hold"),
        ("-1", saved_read_uint8, "'x' (-1) is not a whole number its type can hold"),
        ("1.5", saved_read_uint8, "'x' (1.5) is not a whole number its type can hold"),
        ('"1"', saved_read_uint8, "'x' is not a number"),
        ("null", saved_read_uint8, "'x' is absent (null), and its type is not optional"),
        ("5", saved_read_int64, "'x' is not a 64-bit whole number written as a text"),
        (
            '"9223372036854775808"',
            saved_read_int64,
            "'x' (9223372036854775808) is not a whole number its type can hold",
        ),
        ("1", saved_read_string, "'x' is not a text"),
        ('"true"', saved_read_bool, "'x' is not a truth value"),
    ],
)
def test_a_value_of_another_kind_or_another_width_is_refused_not_converted(text: str, read, want: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        read(parse_json(text), "x")
    assert str(refused.value) == want


def test_a_text_and_a_list_are_held_to_the_bounds_the_machine_has() -> None:
    with pytest.raises(StateRefusal) as past_text:
        saved_read_bounded_string("h\u00e9llo", "t", 5)
    assert str(past_text.value) == "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by"
    with pytest.raises(StateRefusal) as no_byte:
        saved_read_bytes("a\u0100", "b", 8)
    assert str(no_byte.value) == "'b' holds U+0100, which is past U+00FF and so no byte"
    with pytest.raises(StateRefusal) as past_bytes:
        saved_read_bytes("abc", "b", 2)
    assert str(past_bytes.value) == "'b' holds 3 bytes, past the 2 it is bounded by"
    items = parse_json("[1,2,300]")
    with pytest.raises(StateRefusal) as element:
        saved_read_list(items, "l", 8, saved_read_uint8)
    assert str(element.value) == "'l[2]' (300) is not a whole number its type can hold"
    with pytest.raises(StateRefusal) as past_list:
        saved_read_list(items, "l", 2, saved_read_uint8)
    assert str(past_list.value) == "'l' holds 3 elements, past the 2 it is bounded by"


def test_an_upgrade_gives_a_record_the_defaults_of_the_fields_a_later_revision_added() -> None:
    digest = "a" * 64
    shape = SavedShape(
        first="first",
        revised="revised",
        format2_first="first2",
        format2_revised="revised2",
        records=[
            RecordLineage(
                event="shown",
                revision=2,
                revisions=[digest],
                added=[AddedField(id="weekday", since=2, default="0")],
                holders=[RecordHolder("one", False), RecordHolder("many", True)],
            )
        ],
    )
    given = '{"one":{"year":2027},"many":[{"year":1},{"year":2}]}'
    saved = SavedState(shape="first2", variables=parse_json(given), invokes=SavedInvokes(resumed=True))
    upgraded = upgrade_saved_state(saved, shape)
    assert (
        write_json(upgraded.variables)
        == '{"one":{"year":2027,"weekday":0},"many":[{"year":1,"weekday":0},{"year":2,"weekday":0}]}'
    )
    assert write_json(saved.variables) == given, "the upgrade changed the state it was given"

    # A record that already holds a field its revision does not declare.
    saved.variables = parse_json('{"one":{"year":1,"weekday":3},"many":[]}')
    with pytest.raises(StateRefusal) as held:
        upgrade_saved_state(saved, shape)
    assert str(held.value) == "'one' holds 'weekday', which revision 1 of 'shown' does not declare: revision 2 added it"

    # A state of another shape.
    saved.shape = "other"
    with pytest.raises(StateRefusal) as other:
        upgrade_saved_state(saved, shape)
    assert str(other.value) == "the saved state is of a document of shape other, and this machine's is first2"

    # A revision later than the machine's.
    saved.shape = "revised2"
    saved.revisions = [SavedRevision(key="shown", revision=3, fields=digest)]
    with pytest.raises(StateRefusal) as later:
        upgrade_saved_state(saved, shape)
    assert str(later.value) == (
        "the saved state was saved at revision 3 of 'shown', later than this machine's 2: "
        "a machine does not restore what it does not know"
    )


def test_a_null_saved_before_the_field_was_widened_is_refused() -> None:
    # docs/adr/0055, section 6: the third revision of `shown` widened `year`, a
    # field of the first, and `weekday`, which the second added, to optional.
    second, third = "2" * 64, "3" * 64
    shape = SavedShape(
        first="first",
        revised="revised",
        format2_first="first2",
        format2_revised="revised2",
        records=[
            RecordLineage(
                event="shown",
                revision=3,
                revisions=[second, third],
                added=[AddedField(id="weekday", since=2, default="0")],
                widened=[
                    WidenedField(id="year", since=1, optional_since=3),
                    WidenedField(id="weekday", since=2, optional_since=3),
                ],
                holders=[RecordHolder("one", False), RecordHolder("many", True)],
            )
        ],
    )

    def saved_at(revision: int, one: str, many: str) -> SavedState:
        saved = SavedState(
            shape="first2" if revision == 1 else "revised2",
            variables=parse_json(f'{{"one":{one},"many":{many}}}'),
            invokes=SavedInvokes(resumed=True),
        )
        if revision > 1:
            saved.revisions = [SavedRevision(key="shown", revision=revision, fields=second if revision == 2 else third)]
        return saved

    # A record variable, a field of the first revision, saved at the first.
    with pytest.raises(StateRefusal) as variable:
        upgrade_saved_state(saved_at(1, '{"year":null}', "[]"), shape)
    assert str(variable.value) == (
        "'one.year' is null, which revision 1 of 'shown' holds as a value: revision 3 widened it"
    )
    # An element of a list of records.
    with pytest.raises(StateRefusal) as element:
        upgrade_saved_state(saved_at(1, '{"year":4}', '[{"year":1},{"year":null}]'), shape)
    assert "'many[1].year' is null" in str(element.value)
    # A field the second revision added, saved at the second.
    with pytest.raises(StateRefusal) as added:
        upgrade_saved_state(saved_at(2, '{"year":4,"weekday":null}', "[]"), shape)
    assert str(added.value) == (
        "'one.weekday' is null, which revision 2 of 'shown' holds as a value: revision 3 widened it"
    )
    # The field the second added, held by a record saved at the first, is refused as
    # a field that revision does not declare, which is the truer reason.
    with pytest.raises(StateRefusal) as undeclared:
        upgrade_saved_state(saved_at(1, '{"year":4,"weekday":null}', "[]"), shape)
    assert "'one' holds 'weekday'" in str(undeclared.value)

    # Saved at the revision that widened both, `null` is absent.
    kept = saved_at(3, '{"year":null,"weekday":null}', '[{"year":null,"weekday":2}]')
    assert write_json(upgrade_saved_state(kept, shape).variables) == write_json(kept.variables)
    # A record saved before the widening, each field present, restores as it was.
    earlier = upgrade_saved_state(saved_at(1, '{"year":4}', '[{"year":1}]'), shape)
    assert write_json(earlier.variables) == '{"one":{"year":4,"weekday":0},"many":[{"year":1,"weekday":0}]}'
    # What is no record where one is held is left to the variable's own read.
    assert write_json(upgrade_saved_state(saved_at(1, "null", "[{}]"), shape).variables) == (
        '{"one":null,"many":[{"weekday":0}]}'
    )


def test_an_enum_variant_a_later_revision_added_is_refused_in_a_state_saved_before_it() -> None:
    shape = SavedShape(
        first="",
        revised="",
        format2_first="first2",
        format2_revised="revised2",
        enums=[
            EnumLineage(
                key="enum:status",
                revision=2,
                revisions=["b" * 64],
                added=[AddedVariant(name="Done", since=2)],
                holders=[EnumHolder(variable="item", field="status"), EnumHolder(variable="state")],
            )
        ],
    )
    saved = SavedState(
        shape="first2",
        variables=parse_json('{"item":{"status":"Done"},"state":"Open"}'),
        invokes=SavedInvokes(resumed=True),
    )
    with pytest.raises(StateRefusal) as refused:
        upgrade_saved_state(saved, shape)
    assert str(refused.value) == (
        "'item.status' holds 'Done', which revision 1 of 'enum:status' does not declare: revision 2 added it"
    )


def test_a_session_id_is_an_xml_nmtoken_and_a_drawn_one_is_a_version_4_uuid() -> None:
    for id in ("fixture-root", "a.b:c_d", "\u00e91", "x\u00b7y"):
        assert session_id.refusal_of(id) is None, id
    assert session_id.refusal_of("") == (
        "the session id is empty, and a session id is one name character or more (an XML Nmtoken)"
    )
    assert session_id.refusal_of("a b") == (
        "the session id holds U+0020, which is not a name character: a session id is an XML Nmtoken "
        "(letters, digits, '.', '-', '_', ':')"
    )
    pattern = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
    first, second = session_id.draw(), session_id.draw()
    assert pattern.match(first) and session_id.is_nmtoken(first) and first != second


def test_a_refusal_inside_a_child_names_the_session_it_is_about() -> None:
    inner = refusal_within(StateRefusal("'x' is not a text"), "invokes[1].session")
    assert str(inner) == "the session at invokes[1].session: 'x' is not a text"
    outer = refusal_within(inner, "invokes[0].session")
    assert str(outer) == "the session at invokes[0].session.invokes[1].session: 'x' is not a text"
