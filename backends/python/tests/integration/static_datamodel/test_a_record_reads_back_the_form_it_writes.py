# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The reader and the writer of the canonical form a test configuration's unit
generates for each record its system ports take (docs/SCE_TTCN3_LANE.md §10.3,
§10.5; docs/SCE_TEST_PROFILE.md §12.5), imported here over records of every
form Python holds: order_taken (a list of records with a closed enum, an
optional text and integer, a record with a truth value, a byte string),
job_status (a list of integers, optional integers) and form_extras (an enum of
an open set, a list of optional elements, a 64-bit integer, a list with a lower
count, an optional list and a list of lists). The modules are generated from
``sce-build/tests/fixtures/test_profile/trace_forms/`` beside this file (regen:
``scripts/regen_static_datamodel_python.sh``; the modules are not committed).

The twin of ``tests/integration/ARecordReadsBackTheFormItWritesTest.cpp``, with
the same values and the same refusals, and of
``backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs`` but for
the union the Rust configuration's order_placed holds, which Python does not
lower: order_taken holds every other form order_placed does, and a byte string
in the union's place. A value written by ``canonical_<record>`` and read back by
``read_<record>`` is the value it was, through the canonical text; each
malformed spelling of a form is refused, naming where it stands; and a trace of
one record of each type replays to a log that holds each value as the trace
does. What the generator writes for the configuration is held by
``a_configuration_reads_a_record_of_every_form`` in
``sce-build/tests/a_test_suite_is_generated_in_python.rs``.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, Callable, List, TypeVar

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

# The units are imported as modules and read through them: a test case's unit
# declares `test_case`, which pytest would collect as a test of this module were
# it imported by its own name.
from integration.static_datamodel import (  # noqa: E402 -- path inserted above
    form_extras,
    job_status,
    order_customer,
    order_line,
    order_taken,
    paint_color,
    replays_every_unionless_form,
    shape_kind_open,
)
from integration.static_datamodel import unionless_forms_configuration as forms  # noqa: E402
from sce_runtime.saved_json import parse_json, write_jcs  # noqa: E402
from sce_runtime.testprofile import (  # noqa: E402
    Execute,
    Suite,
    Trace,
    TraceInput,
    TraceRefusal,
    Verdict,
    features_of,
    run_suite_with_traces,
)

#: A record the configuration reads and writes.
V = TypeVar("V")

Color = paint_color.PaintColor
Shape = shape_kind_open.ShapeKindOpen


def line(sku: str, quantity: int, color: Color) -> Any:
    """A line of an order."""
    return order_line.OrderLinePayload(sku=sku, quantity=quantity, color=color)


def order(voucher: bytes) -> Any:
    """An order of two lines, one with a text of a character of two bytes and an
    escaped quote, the widest order id, a note and no priority, paid with the
    voucher ``voucher``."""
    return order_taken.OrderTakenPayload(
        order_id=4294967295,
        lines=[line("A-1", 2, Color.GREEN), line('"q" é', 65535, Color.BLUE)],
        note="ring twice",
        priority=None,
        customer=order_customer.OrderCustomerPayload(full_name="Ada", vip=True),
        voucher_code=voucher,
    )


#: The vouchers the order is paid with: three octets, and none.
VOUCHERS = [bytes([0x00, 0xAB, 0xFF]), b""]


def statuses() -> List[Any]:
    return [
        job_status.JobStatusPayload(elapsed_ms=0, note=None, codes=[], tag=255, retry=None, count=7),
        job_status.JobStatusPayload(elapsed_ms=12, note="late", codes=[3, 9, 5, 0], tag=None, retry=0, count=0),
    ]


def extras() -> List[Any]:
    """The extras: a declared variant and a number no variant declares, a list
    with an absent element, the least and the greatest 64-bit integer, a list at
    its lower count and at its bound, an optional list present and absent, and a
    list of lists with an empty one."""
    return [
        form_extras.FormExtrasPayload(
            shape=Shape.CIRCLE,
            slots=[1, None, 65535],
            least=-9223372036854775808,
            flags=[True],
            spare=[0, 255],
            grid=[[1, 2], []],
        ),
        form_extras.FormExtrasPayload(
            shape=Shape.from_underlying(9),
            slots=[],
            least=9223372036854775807,
            flags=[False, True],
            spare=None,
            grid=[],
        ),
    ]


def round_trip(value: V, write: Callable[[V], Any], read: Callable[[Any], V]) -> None:
    """``value`` written by ``write`` in its canonical text, read back from that
    text by ``read``, and written again: the same text, so the reader holds every
    field the writer wrote, as the writer wrote it."""
    written = write_jcs(write(value))
    assert write_jcs(write(read(parse_json(written)))) == written


def test_a_value_of_each_form_reads_back_from_its_canonical_text() -> None:
    """docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the
    text its writer gives as the value it was."""
    for voucher in VOUCHERS:
        round_trip(order(voucher), forms.canonical_order_taken, forms.read_order_taken)
    for status in statuses():
        round_trip(status, forms.canonical_job_status, forms.read_job_status)
    for extra in extras():
        round_trip(extra, forms.canonical_form_extras, forms.read_form_extras)
    read = forms.read_form_extras(forms.canonical_form_extras(extras()[1]))
    assert read.shape.to_underlying() == 9, "a number no variant declares"
    assert read.least == 9223372036854775807
    paid = forms.read_order_taken(forms.canonical_order_taken(order(VOUCHERS[0])))
    assert paid == order(VOUCHERS[0])
    assert paid.voucher_code == bytes([0x00, 0xAB, 0xFF])
    assert paid.lines[1].sku == '"q" é'
    assert paid.lines[1].color is Color.BLUE
    assert paid.priority is None


def refused(value: V, write: Callable[[V], Any], read: Callable[[Any], V], from_: str, to: str) -> str:
    """The reason ``read`` refuses the canonical text of ``value`` with ``from_``
    replaced once by ``to`` for."""
    text = write_jcs(write(value))
    assert from_ in text, f"`{from_}` in {text}"
    changed = parse_json(text.replace(from_, to, 1))
    with pytest.raises(TraceRefusal) as refusal:
        read(changed)
    return refusal.value.reason


def test_a_malformed_spelling_of_each_form_is_refused_by_name() -> None:
    """docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is
    refused, naming the place it stands at: the record's type, each field by its
    id and ``[]`` for an element of a list."""
    paid = order(VOUCHERS[0])
    extra = extras()[0]
    status = statuses()[0]

    def of_order(from_: str, to: str) -> str:
        return refused(paid, forms.canonical_order_taken, forms.read_order_taken, from_, to)

    def of_extras(from_: str, to: str) -> str:
        return refused(extra, forms.canonical_form_extras, forms.read_form_extras, from_, to)

    def of_status(from_: str, to: str) -> str:
        return refused(status, forms.canonical_job_status, forms.read_job_status, from_, to)

    long_name = '"full_name":"' + "x" * 25 + '"'
    more_lines = '"lines":[' + '{"color":"red","quantity":"1","sku":"s"},' * 3
    cases = [
        (
            of_order('"order_id":"4294967295"', '"order_id":"007"'),
            "order_taken.order_id: '007' is not the text of an integer's decimal digits",
        ),
        (
            of_order('"quantity":"2"', '"quantity":"65536"'),
            "order_taken.lines[].quantity: 65536 is past the range of its type",
        ),
        (of_order('"vip":true', '"vip":"true"'), "order_taken.customer.vip: a text is not a truth value"),
        (
            of_order('"full_name":"Ada"', long_name),
            "order_taken.customer.full_name: a text of 25 bytes is past the 24 its type holds",
        ),
        (
            of_order('"color":"green"', '"color":"purple"'),
            "order_taken.lines[].color: 'purple' names no variant of its enum",
        ),
        (of_order('"note":"ring twice"', '"note":null'), "order_taken.note: null is not a text"),
        (of_order('{"customer":', '{"rush":"1","customer":'), "order_taken: 'rush' is no field of its record"),
        (
            of_order('"order_id":"4294967295",', ""),
            "order_taken.order_id: the field is absent, and its type is not optional",
        ),
        (
            of_order('"lines":[', more_lines),
            "order_taken.lines: a list of 5 elements is outside its bounds, 0 to 4",
        ),
        (
            of_order('"voucher_code":"00ABFF"', '"voucher_code":"00abff"'),
            "order_taken.voucher_code: '00abff' is not octets in upper-case hex, two digits each",
        ),
        (
            of_order('"voucher_code":"00ABFF"', '"voucher_code":"0011223344"'),
            "order_taken.voucher_code: a byte string of 5 octets is past the 4 its type holds",
        ),
        (of_status('"tag":"255"', '"tag":"256"'), "job_status.tag: 256 is past the range of its type"),
        (
            of_status('"codes":[]', '"codes":["x"]'),
            "job_status.codes[]: 'x' is not the text of an integer's decimal digits",
        ),
        (
            of_extras('"shape":"circle"', '"shape":"1"'),
            "form_extras.shape: 1 is the number of a declared variant, which is written by its name",
        ),
        (
            of_extras('"shape":"circle"', '"shape":"hexagon"'),
            "form_extras.shape: 'hexagon' names no variant of its enum, and is no number",
        ),
        (
            of_extras('"slots":["1",null,', '"slots":["1","x",'),
            "form_extras.slots[]: 'x' is not the text of an integer's decimal digits",
        ),
        (
            of_extras('"flags":[true]', '"flags":[]'),
            "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2",
        ),
        (
            of_extras('"grid":[["1","2"]', '"grid":[["1","2","3"]'),
            "form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2",
        ),
        (
            of_extras('"least":"-9223372036854775808"', '"least":"-9223372036854775809"'),
            "form_extras.least: -9223372036854775809 is past the range of its type",
        ),
        (
            of_extras('"spare":["0","255"]', '"spare":null'),
            "form_extras.spare: null is not a list, which is an array",
        ),
    ]
    for reason, expected in cases:
        assert reason == expected


def test_a_trace_of_every_form_replays_to_the_values_it_holds() -> None:
    """docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the
    system port takes, two at the start and one later, replays to the listener,
    which takes one a round and passes; the log holds each value as the trace
    does."""
    trace = Trace(
        suite="forms_suite",
        test_case=replays_every_unionless_form.NAME,
        inputs=[
            TraceInput(0, "sut", "order_taken", forms.canonical_order_taken(order(VOUCHERS[0]))),
            TraceInput(0, "sut", "job_status", forms.canonical_job_status(statuses()[0])),
            TraceInput(250, "sut", "form_extras", forms.canonical_form_extras(extras()[0])),
        ],
    )
    text = trace.canonical_text()
    suite = Suite(name="forms_suite", executes=[Execute(replays_every_unionless_form.test_case())])
    results = run_suite_with_traces(suite, features_of(), lambda _: Trace.parse(text))
    assert len(results) == 1
    assert results[0].result.verdict == Verdict.PASS, results[0].result.log
    assert results[0].result.ended_at == 250
    log = results[0].canonical_log("forms_suite")
    for given in trace.inputs:
        received = (
            f'"kind":"message-received","port":"sut","type":"{given.message_type}",'
            f'"value":{write_jcs(given.value)}'
        )
        assert received in log, f"`{received}` in {log}"
