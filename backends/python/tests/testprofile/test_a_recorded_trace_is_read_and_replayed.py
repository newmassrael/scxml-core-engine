# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The recorded input trace of a test case (docs/SCE_TTCN3_LANE.md §10), its
player behind the system adapter, the runner of a suite against its traces and
the runner program's command line (§4), in
``sce_runtime.testprofile.trace``, ``suite`` and ``runner``.

Each case is the twin of the Rust case of the same name in
``backends/rust/runtime/src/test/trace.rs`` and ``runner.rs`` and of the Kotlin
case in ``TraceTest.kt``, named ``test_<case>`` after the Rust one, with the
same texts and refusals. The cases of the program's standard output and of a
suite whose trace is refused run over test cases written here, whose runs are
refused or end before any machine starts; the generated suite replayed to the
log every runtime writes is
``backends/python/tests/integration/static_datamodel/test_a_test_suite_runs_through_its_runner.py``.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, List, Optional

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime import saved_values  # noqa: E402 -- path inserted above
from sce_runtime.saved_json import parse_json  # noqa: E402
from sce_runtime.testprofile import (  # noqa: E402
    RUNNER_USAGE,
    TEST_TRACE_SCHEMA,
    TEST_TRACE_SCHEMA_STATUS,
    ComponentMachine,
    Configuration,
    Execute,
    Message,
    ModuleParameters,
    ModuleParameterValue,
    OverrideRefusal,
    RunnerOptions,
    RunnerUsageRefusal,
    Suite,
    TestCase,
    Trace,
    TraceAdapter,
    TraceInput,
    TraceRefusal,
    Verdict,
    bool_parameter,
    bytes_parameter,
    canonical,
    enum_parameter,
    features_of,
    from_canonical,
    integer_parameter,
    no_inputs,
    read_trace,
    run_runner,
    run_suite_with_traces,
    runner_main,
    text_parameter,
    undeclared_parameter,
)

#: The repository's root, above ``backends/python/tests/testprofile``.
REPO_ROOT = Path(__file__).resolve().parents[4]

ONE_INPUT = (
    '{"inputs":[{"at":"500","port":"sut","type":"job_reply",'
    '"value":{"code":"0","job":"7"}}],'
    '"schema":"sce-test-trace.v1","suite":"runtime_suite",'
    '"test_case":"answers_the_job"}'
)


def refused(text: str) -> str:
    """The reason ``Trace.parse`` refuses ``text`` for."""
    with pytest.raises(TraceRefusal) as refusal:
        Trace.parse(text)
    return refusal.value.reason


def changed(from_: str, to: str) -> str:
    """``ONE_INPUT`` with ``from_``, which it holds, replaced once by ``to``."""
    assert from_ in ONE_INPUT, from_
    return ONE_INPUT.replace(from_, to, 1)


class JobReply:
    """A reply, as a hand-written reader of a configuration reads one."""

    def __init__(self, job: int, code: int) -> None:
        self.job = job
        self.code = code


def reply_canonical(reply: JobReply) -> Any:
    return canonical.record(("job", canonical.integer(reply.job)), ("code", canonical.integer(reply.code)))


def read_reply(system_port: str, type_name: str, value: Any) -> Message:
    """The reader of a configuration whose one system port, ``sut``, takes
    ``job_reply``."""
    if system_port == "sut" and type_name == "job_reply":
        held = from_canonical.record(value, ["job", "code"], "job_reply")
        reply = JobReply(
            job=from_canonical.integer(from_canonical.field(held, "job", "job_reply.job"), from_canonical.UINT32, "job_reply.job"),
            code=from_canonical.integer(
                from_canonical.field(held, "code", "job_reply.code"), from_canonical.UINT32, "job_reply.code"
            ),
        )
        return Message.of("job.reply", reply, "job_reply", reply_canonical)
    raise TraceRefusal.not_an_input(system_port, type_name, ["sut"])


def test_schema_file_declares_status() -> None:
    """The status the runtime declares is the one the trace's schema file
    declares, and the schema pins the name of the shape the runtime reads.
    ``SCE_WIRE_CONTRACTS.md`` requires one commit to move both."""
    schema = parse_json((REPO_ROOT / "schemas" / "sce-test-trace.v1.schema.json").read_text(encoding="utf-8"))
    assert schema["x-sce-schema-status"] == TEST_TRACE_SCHEMA_STATUS
    assert schema["properties"]["schema"]["const"] == TEST_TRACE_SCHEMA


def test_a_trace_is_read_from_its_canonical_text_and_written_back() -> None:
    """A trace is read from its canonical text, with or without the line feed a
    file ends in, and written back as the same bytes."""
    trace = Trace.parse(ONE_INPUT)
    assert trace.suite == "runtime_suite"
    assert trace.test_case == "answers_the_job"
    assert trace.inputs == [
        TraceInput(at=500, port="sut", message_type="job_reply", value={"code": "0", "job": "7"})
    ]
    assert trace.canonical_text() == ONE_INPUT
    assert Trace.parse(ONE_INPUT + "\n") == trace


def test_a_trace_that_is_not_of_its_shape_is_refused_by_name() -> None:
    """Each change of one member of a trace it first reads is refused, by
    name."""
    for text, expected in [
        (changed("sce-test-trace.v1", "sce-test-trace.v2"), "schema is not sce-test-trace.v1"),
        (changed('"500"', "500"), "the instant of input 0"),
        (changed('"500"', '"0500"'), "the instant of input 0"),
        (changed('"500"', '"-5"'), "the instant of input 0"),
        (changed('"500"', '"9223372036854775808"'), "past the microseconds"),
        (changed('"port":"sut"', '"port":""'), "the port of input 0"),
        (changed('"type"', '"kind"'), "input 0 has no member 'kind'"),
        (changed(',"suite":"runtime_suite"', ""), "the trace has no 'suite'"),
        (changed('{"inputs"', '{ "inputs"'), "one canonical text"),
        (changed("[{", "{"), "not JSON"),
        # The `json` module reads these; the trace's reader does not.
        (changed('"value":{"code":"0","job":"7"}', '"value":NaN'), "not JSON"),
        (changed('"value":{"code":"0","job":"7"}', '"value":{"code":"0","code":"1"}'), "appears twice"),
        # JSON allows the whitespace; the one canonical text does not.
        (ONE_INPUT + "\r\n", "one canonical text"),
        (ONE_INPUT + "\n\n", "one canonical text"),
        ("[" * 100000, "nests deeper than the reader reads"),
    ]:
        reason = refused(text)
        assert expected in reason, (expected, reason)
    out_of_order = (
        '{"inputs":[{"at":"5","port":"sut","type":"t","value":{}},'
        '{"at":"4","port":"sut","type":"t","value":{}}],'
        '"schema":"sce-test-trace.v1","suite":"s","test_case":"c"}'
    )
    assert "input 1, at 4, comes after an input at 5" in refused(out_of_order)


def test_a_trace_whose_value_holds_a_number_past_a_double_is_refused() -> None:
    """A number past the range of a double has no canonical text, so a trace
    whose value holds one is refused as not written in it, not an error of the
    reader. The twin of the Rust and Kotlin cases of the same name."""
    text = changed('{"code":"0","job":"7"}', "1e400")
    assert refused(text) == (
        "the trace is not written in its one canonical text, the JSON Canonicalization Scheme"
    )


def test_an_input_the_configuration_does_not_take_is_refused_by_name() -> None:
    """An input on a port the configuration does not have, or of a type its port
    does not take, is refused, naming which, and the whole trace with it."""
    assert (
        TraceRefusal.not_an_input("sut", "job_request", ["sut"]).reason
        == "'job_request' is not on the `in` list of the type of the system port 'sut'"
    )
    assert (
        TraceRefusal.not_an_input("oracle", "job_reply", ["sut"]).reason
        == "'oracle' is not a system port of the test case's configuration"
    )
    trace = Trace.parse(ONE_INPUT)
    with pytest.raises(TraceRefusal) as refusal:
        TraceAdapter.replaying(trace, no_inputs)
    assert refusal.value.reason == "input 0, at 500: 'sut' is not a system port of the test case's configuration"
    bad = Trace.parse(changed('"job":"7"', '"job":"07"'))
    with pytest.raises(TraceRefusal) as refusal:
        TraceAdapter.replaying(bad, read_reply)
    assert refusal.value.reason == "input 0, at 500: job_reply.job: '07' is not the text of an integer's decimal digits"


def test_a_trace_is_played_at_its_instants() -> None:
    """docs/SCE_TEST_PROFILE.md §3.2 step 2, §3.3: the player announces the
    instant of its next input, delivers the inputs due at an instant in the
    trace's order, the value read by the configuration's reader, and records
    what is sent to the system ports."""
    trace = Trace(
        suite="s",
        test_case="c",
        inputs=[
            TraceInput(at=0, port="sut", message_type="job_reply", value={"code": "0", "job": "1"}),
            TraceInput(at=0, port="sut", message_type="job_reply", value={"code": "0", "job": "2"}),
            TraceInput(at=250, port="sut", message_type="job_reply", value={"code": "3", "job": "3"}),
        ],
    )
    adapter = TraceAdapter.replaying(trace, read_reply)
    assert adapter.next_input() == 0
    first = adapter.take_inputs_due(0)
    assert [(due.system_port, due.message.value.job) for due in first] == [("sut", 1), ("sut", 2)]
    assert first[0].message.type_name == "job_reply"
    assert first[0].message.canonical_value() == {"code": "0", "job": "1"}
    assert adapter.next_input() == 250
    assert adapter.take_inputs_due(249) == []
    assert [due.message.value.code for due in adapter.take_inputs_due(300)] == [3]
    assert adapter.next_input() is None
    sent = Message.of("job.request", None, "job_request", lambda _: {})
    adapter.send("sut", sent, 7)
    assert [(output.at, output.system_port, output.message) for output in adapter.sent] == [(7, "sut", sent)]


def lone(mtc_type: str) -> Configuration:
    """A configuration of one MTC of ``mtc_type`` and no system port."""
    return Configuration(mtc_type=mtc_type)


def no_machine(component_type: str) -> Optional[ComponentMachine]:
    """A factory that makes no machine, so a run that starts ends with
    ``error`` as its MTC is created (P13)."""
    return None


def test_a_test_case_whose_trace_does_not_fit_ends_in_error_before_it_starts() -> None:
    """docs/SCE_TTCN3_LANE.md §10.5, P13: a test case whose trace is not given,
    is of another test case or suite, or does not fit its configuration ends
    with ``error`` before any component is created, its log line naming why;
    one whose trace fits is run."""
    suite = Suite(
        name="s",
        executes=[
            Execute(TestCase("missing", lone("Asker"), no_machine)),
            Execute(TestCase("elsewhere", lone("Asker"), no_machine)),
            Execute(TestCase("unfit", lone("Asker"), no_machine)),
            Execute(TestCase("fits", lone("Asker"), no_machine, inputs=read_reply)),
        ],
    )

    def traces(test_case: str) -> Trace:
        if test_case == "missing":
            raise TraceRefusal("traces/missing.json is not read: no such file")
        if test_case == "elsewhere":
            return Trace(suite="other", test_case="elsewhere")
        return Trace.parse(changed('"answers_the_job"', f'"{test_case}"').replace("runtime_suite", "s"))

    results = run_suite_with_traces(suite, features_of(), traces)
    why = {result.test_case: result.result.log[0].detail for result in results}
    assert why == {
        "missing": "the trace is refused: traces/missing.json is not read: no such file",
        "elsewhere": "the trace is of the test case 'elsewhere' of the suite 'other'",
        "unfit": "the trace is refused: input 0, at 500: 'sut' is not a system port of the test case's configuration",
        "fits": "is of the component type 'Asker', which the factory of machines does not make",
    }
    assert all(result.result.verdict == Verdict.ERROR for result in results)
    assert [len(result.result.records) for result in results[:3]] == [2, 2, 2], "started and ended alone"
    overrides = ModuleParameters()
    overrides.set("limit", saved_values.saved_write_uint32(1))
    with pytest.raises(OverrideRefusal):
        run_suite_with_traces(suite, features_of(), traces, overrides)


def args(*items: str) -> List[str]:
    return list(items)


def test_the_command_line_is_read_as_every_runner_reads_it() -> None:
    """The command line every backend's runner takes, refused in the same
    words where it is not."""
    options = RunnerOptions.parse(
        args(
            "--trace-dir", "traces", "--log-dir", "logs", "--world", "ptcs,guard",
            "--module-parameter", "job=9", "--module-parameter", "label=a=b",
        )
    )
    assert options.trace_dir == Path("traces")
    assert options.log_dir == Path("logs")
    assert options.world == features_of("ptcs", "guard")
    assert options.module_parameters == [("job", "9"), ("label", "a=b")]
    bare = RunnerOptions.parse(args("--log-dir", "l", "--trace-dir", "t"))
    assert bare.world == features_of()
    assert RunnerOptions.parse(args("--trace-dir", "t", "--log-dir", "l", "--world", "")).world == features_of()
    for line, why in [
        (args("--log-dir", "l"), "--trace-dir is not given"),
        (args("--trace-dir", "t"), "--log-dir is not given"),
        (args("--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"), "--trace-dir is given twice"),
        (args("--trace-dir", "t", "--log-dir", "l", "--seed", "1"), "--seed is no option"),
        (args("--trace-dir"), "--trace-dir is given no value"),
        (args("--trace-dir=t", "--log-dir", "l"), "--trace-dir=t is no option"),
        (args("--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job"), "is not <id>=<value>"),
        (
            args("--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job=1", "--module-parameter", "job=2"),
            "'job' is given twice",
        ),
    ]:
        with pytest.raises(RunnerUsageRefusal) as refusal:
            RunnerOptions.parse(line)
        assert why in refusal.value.reason, (why, refusal.value.reason)


def test_a_module_parameter_is_read_as_the_saved_value_of_its_type() -> None:
    """A module parameter's literal is read as the saved value of its type, and
    one that is no value of it is refused, naming the parameter."""
    assert integer_parameter("job", "9", from_canonical.UINT32) == 9
    assert integer_parameter("wide", "18446744073709551615", from_canonical.UINT64) == 18446744073709551615
    for literal, width in [("256", from_canonical.UINT8), ("09", from_canonical.UINT32)]:
        with pytest.raises(OverrideRefusal) as refusal:
            integer_parameter("job", literal, width)
        assert refusal.value.reason.startswith("job: "), refusal.value.reason
    assert bool_parameter("armed", "true") == saved_values.saved_write_bool(True)
    assert bool_parameter("armed", "false") == saved_values.saved_write_bool(False)
    with pytest.raises(OverrideRefusal):
        bool_parameter("armed", "yes")
    assert text_parameter("nightly") == saved_values.saved_write_string("nightly")
    assert enum_parameter("green") == saved_values.saved_write_string("green")
    assert bytes_parameter("key", "0aFF") == saved_values.saved_write_bytes(bytes([0x0A, 0xFF]))
    for literal in ["0a0", "0g", "+a", "0a f"]:
        with pytest.raises(OverrideRefusal) as refusal:
            bytes_parameter("key", literal)
        assert refusal.value.reason == f"key: '{literal}' is not octets in hex"
    assert undeclared_parameter("limit").reason == "the suite declares no module parameter 'limit'"


def job_parameter(parameter_id: str, literal: str) -> ModuleParameterValue:
    """The reader of a suite that declares the module parameter ``job``."""
    if parameter_id == "job":
        return saved_values.saved_write_uint32(integer_parameter(parameter_id, literal, from_canonical.UINT32))
    raise undeclared_parameter(parameter_id)


def noisy_machine(component_type: str) -> Optional[ComponentMachine]:
    """A factory that writes to the standard output, as a machine's ``<log>``
    does, and makes no machine."""
    print(f"making {component_type}")
    return None


def one_case_suite() -> Suite:
    """A suite of one test case, ``c``, whose run ends with ``error`` as its MTC
    is created, after its factory wrote to the standard output."""
    return Suite(name="s", executes=[Execute(TestCase("c", lone("Asker"), noisy_machine))])


def write_trace(directory: Path) -> None:
    directory.mkdir()
    (directory / "c.json").write_text(Trace(suite="s", test_case="c").canonical_text() + "\n", encoding="utf-8")


def test_the_runner_program_keeps_its_standard_output_to_the_verdicts(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """docs/SCE_TTCN3_LANE.md §4: the program prints ``<test case> <verdict>``
    and nothing else on the standard output; what the run prints goes to the
    standard error, and so does the cause of an ``error``. The log is the
    canonical text with nothing after it, and the status is 0 whatever the
    verdict."""
    traces = tmp_path / "traces"
    write_trace(traces)
    logs = tmp_path / "made" / "logs"
    status = runner_main(one_case_suite(), job_parameter, ["--trace-dir", str(traces), "--log-dir", str(logs)])
    captured = capsys.readouterr()
    assert status == 0, captured.err
    assert captured.out == "c error\n", "the standard output is the contract's lines alone"
    assert "making Asker\n" in captured.err
    assert "c: mtc: is of the component type 'Asker'" in captured.err
    log = (logs / "c.log.json").read_bytes().decode("utf-8")
    assert log == one_case_suite_log()


def one_case_suite_log() -> str:
    """The log of the one test case's run, which ends at its start."""
    return (
        '{"records":[{"at":"0","kind":"test-case-started"},{"at":"0","component":"mtc","kind":"verdict-set",'
        '"verdict":"error"},{"at":"0","kind":"test-case-ended","verdict":"error"}],"schema":"sce-test-log.v1",'
        '"suite":"s","test_case":"c","verdict":"error"}'
    )


def test_a_command_line_or_a_log_the_runner_cannot_take_exits_two(tmp_path: Path) -> None:
    """§4: a command line that is not the runner's, a module parameter the suite
    does not declare or whose literal is no value of its type, and a log
    directory that cannot be made each exit 2, the reason and the usage on the
    standard error, before any log is written; a test case whose trace is
    missing ends in ``error`` and its log is written, with status 0."""
    traces = tmp_path / "traces"
    traces.mkdir()
    out: List[str] = []
    err: List[str] = []

    def run(*line: str) -> int:
        out.clear()
        err.clear()
        return run_runner(one_case_suite(), job_parameter, list(line), out.append, err.append)

    assert run("--log-dir", str(tmp_path / "l")) == 2
    assert err == ["--trace-dir is not given", RUNNER_USAGE]
    for literal, why in [
        ("limit=1", "--module-parameter limit=1: the suite declares no module parameter 'limit'"),
        ("job=seven", "--module-parameter job=seven: job: 'seven' is not the text of an integer's decimal digits"),
    ]:
        logs = tmp_path / literal
        assert run("--trace-dir", str(traces), "--log-dir", str(logs), "--module-parameter", literal) == 2
        assert err == [why, RUNNER_USAGE]
        assert out == []
        assert not logs.exists(), "no case ran, so no log was written"

    blocked = tmp_path / "blocked"
    blocked.write_text("", encoding="utf-8")
    assert run("--trace-dir", str(traces), "--log-dir", str(blocked)) == 2
    assert any("is not made" in line for line in err), err

    logs = tmp_path / "logs"
    assert run("--trace-dir", str(traces), "--log-dir", str(logs)) == 0
    assert out == ["c error"]
    assert any(line.startswith("c: mtc: the trace is refused: ") and "is not read" in line for line in err), err
    assert (logs / "c.log.json").read_text(encoding="utf-8") == (
        '{"records":[{"at":"0","kind":"test-case-started"},{"at":"0","kind":"test-case-ended","verdict":"error"}],'
        '"schema":"sce-test-log.v1","suite":"s","test_case":"c","verdict":"error"}'
    )


def test_a_trace_file_is_read_as_its_bytes(tmp_path: Path) -> None:
    """A trace file ending in a carriage return and a line feed is not the
    trace's canonical text, which a reader in Python's text mode would read as
    one; a file that is no UTF-8 is not read, naming its path."""
    (tmp_path / "c.json").write_bytes((ONE_INPUT + "\r\n").encode("utf-8"))
    with pytest.raises(TraceRefusal) as refusal:
        read_trace(tmp_path, "c")
    assert "one canonical text" in refusal.value.reason
    (tmp_path / "d.json").write_bytes(b"\xff")
    with pytest.raises(TraceRefusal) as refusal:
        read_trace(tmp_path, "d")
    assert refusal.value.reason.startswith(f"{tmp_path / 'd.json'} is not read: ")
    (tmp_path / "e.json").write_bytes((ONE_INPUT + "\n").encode("utf-8"))
    assert read_trace(tmp_path, "e") == Trace.parse(ONE_INPUT)
