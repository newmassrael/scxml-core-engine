# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5):
it runs every test case a world selects against its recorded input trace and
writes each one's canonical test log.

The unit of a suite generates the program's ``main``, which hands
``runner_main`` the suite's tables and the reader of its module parameters
(``module_parameter``), and runs it when the unit is run as a module
(``python -m <package>.<suite>``): a unit imports its siblings relative to its
package, so it is run by its module's name and never by its file's path. The
command line is the same on every backend::

    <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...

- ``--trace-dir``: the directory of the traces, ``<test case>.json`` each
  (``schemas/sce-test-trace.v1.schema.json``);
- ``--log-dir``: the directory the logs are written to, ``<test case>.log.json``
  each, the canonical test log's text with nothing after it
  (``schemas/sce-test-log.v1.schema.json``), made where it is missing;
- ``--world``: the features the world enables, by name, separated by commas;
  none where it is not given (ADR 0057 §2);
- ``--module-parameter``: a module parameter's value in place of its default,
  written as ``<sce:module-parameter value>`` writes a literal of its type
  (docs/SCE_TEST_PROFILE.md §11.4, §11.8); repeated, one per parameter.

Each test case run prints ``<test case> <verdict>`` on its own line, and the
standard output holds nothing else: every diagnostic of the run goes to the
standard error. A test case whose trace is missing or refused ends with
``error`` and its log is written, the reason on the standard error (P13). The
exit status is 0 when every selected test case ran and its log was written,
whatever its verdict: the verdict is the log's. It is 2 when the command line
is not one of the above, a module parameter's value is refused, or a log cannot
be written.

The command line is read by the runner's own reader rather than by
``argparse``, which would also take ``--trace-dir=<dir>``, an abbreviation of
an option and ``--help``, and word its refusals in its own words: the line is
the contract's alone, refused in the words of the other backends' runners.

Cross-language siblings: ``sce_rust_runtime::test::runner`` (Rust),
``com.sce.runtime.test.runRunner`` and ``runnerMain`` (Kotlin),
``SCE::Test::runnerMain`` (C++).
"""

from __future__ import annotations

import re
import sys
from contextlib import redirect_stdout
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, List, Optional, Sequence, Tuple

from ..saved_values import saved_write_bool, saved_write_bytes, saved_write_string
from . import from_canonical
from .module_parameters import ModuleParameters, ModuleParameterValue
from .run import LogKind
from .suite import Features, OverrideRefusal, Suite, run_suite_with_traces
from .trace import Trace
from .trace_refusal import TraceRefusal

#: The reader of a suite's module parameters, generated as ``module_parameter``
#: in the suite's unit: the saved value of the module parameter ``id`` of its
#: declared type, read from ``literal``, the value the command line gives it,
#: called as ``parameter(id, literal)``. Raises an ``OverrideRefusal`` naming
#: why, where the suite declares no such parameter or the literal is no value of
#: its type.
ParameterReader = Callable[[str, str], ModuleParameterValue]

#: The program's usage line.
RUNNER_USAGE = (
    "usage: <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] "
    "[--module-parameter <id>=<value>]..."
)

#: A byte string's literal: hex digits of either case, two per octet. ``[0-9]``
#: is the ASCII digits alone, where ``bytes.fromhex`` would also skip
#: whitespace.
_HEX = re.compile(r"(?:[0-9A-Fa-f]{2})*")


class RunnerUsageRefusal(Exception):
    """A command line that is not the runner's, refused, naming why."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        #: Why.
        self.reason = reason


@dataclass(frozen=True)
class RunnerOptions:
    """What the command line asks of the runner."""

    #: The directory of the traces, ``<test case>.json`` each.
    trace_dir: Path
    #: The directory the logs are written to, ``<test case>.log.json`` each.
    log_dir: Path
    #: The features the world enables.
    world: Features = frozenset()
    #: Each ``--module-parameter``, its id and the literal it gives, in the
    #: order given.
    module_parameters: List[Tuple[str, str]] = field(default_factory=list)

    @classmethod
    def parse(cls, args: Sequence[str]) -> "RunnerOptions":
        """The options ``args`` give, the program's name not among them.

        Raises ``RunnerUsageRefusal`` naming why, where they are not the
        program's command line.
        """
        trace_dir: Optional[str] = None
        log_dir: Optional[str] = None
        world: Optional[str] = None
        module_parameters: List[Tuple[str, str]] = []
        # Every option takes one value: the line is read a pair at a time.
        for at in range(0, len(args), 2):
            option = args[at]
            if at + 1 == len(args):
                raise RunnerUsageRefusal(f"{option} is given no value")
            value = args[at + 1]
            if option == "--trace-dir" and trace_dir is None:
                trace_dir = value
            elif option == "--log-dir" and log_dir is None:
                log_dir = value
            elif option == "--world" and world is None:
                world = value
            elif option == "--module-parameter":
                id_, split, literal = value.partition("=")
                if not split:
                    raise RunnerUsageRefusal(f"--module-parameter {value} is not <id>=<value>")
                if any(given == id_ for given, _ in module_parameters):
                    raise RunnerUsageRefusal(f"the module parameter '{id_}' is given twice")
                module_parameters.append((id_, literal))
            elif option in ("--trace-dir", "--log-dir", "--world"):
                raise RunnerUsageRefusal(f"{option} is given twice")
            else:
                raise RunnerUsageRefusal(f"{option} is no option of the runner")
        if trace_dir is None:
            raise RunnerUsageRefusal("--trace-dir is not given")
        if log_dir is None:
            raise RunnerUsageRefusal("--log-dir is not given")
        features = frozenset(feature for feature in (world or "").split(",") if feature)
        return cls(
            trace_dir=Path(trace_dir),
            log_dir=Path(log_dir),
            world=features,
            module_parameters=module_parameters,
        )


def read_trace(directory: Path, test_case: str) -> Trace:
    """The trace of the test case ``test_case``, ``<directory>/<test case>.json``.

    Raises a ``TraceRefusal`` where it is not read, naming its path, or is
    refused. The file is read as its bytes and decoded as UTF-8 by hand, since a
    text read in Python's text mode has each ``\\r\\n`` turned into a ``\\n``,
    which would read a file that is not the trace's canonical text as if it
    were.
    """
    path = directory / f"{test_case}.json"
    try:
        text = path.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise TraceRefusal(f"{path} is not read: {error}") from None
    return Trace.parse(text)


def run_runner(
    suite: Suite,
    parameter: ParameterReader,
    args: Sequence[str],
    out: Callable[[str], None],
    err: Callable[[str], None],
) -> int:
    """The runner program of ``suite``, its module parameters read by
    ``parameter``, on the command line ``args``, the program's name not among
    them (docs/SCE_TTCN3_LANE.md §4), each line of the standard output handed
    to ``out`` and of the standard error to ``err``: see the module's
    documentation. Answers the program's exit status."""

    def usage(why: str) -> int:
        err(why)
        err(RUNNER_USAGE)
        return 2

    try:
        options = RunnerOptions.parse(args)
    except RunnerUsageRefusal as refused:
        return usage(refused.reason)
    overrides = ModuleParameters()
    for id_, literal in options.module_parameters:
        try:
            overrides.set(id_, parameter(id_, literal))
        except OverrideRefusal as refused:
            return usage(f"--module-parameter {id_}={literal}: {refused.reason}")
    try:
        results = run_suite_with_traces(
            suite,
            options.world,
            lambda test_case: read_trace(options.trace_dir, test_case),
            overrides,
        )
    except OverrideRefusal as refused:
        return usage(refused.reason)
    try:
        options.log_dir.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        err(f"{options.log_dir} is not made: {error}")
        return 2
    for result in results:
        for line in result.result.log:
            if line.kind == LogKind.ERROR:
                err(f"{result.test_case}: {line.component}: {line.detail}")
        path = options.log_dir / f"{result.test_case}.log.json"
        try:
            # As bytes, so that nothing is added to the canonical text or
            # changed in it on the way to the file.
            path.write_bytes(result.canonical_log(suite.name).encode("utf-8"))
        except OSError as error:
            err(f"{path} is not written: {error}")
            return 2
        # `!s`: the verdict's spelling (`Verdict.__str__`). An `IntEnum`'s
        # format is its number's on the interpreters whose `IntEnum` formats as
        # `int` does.
        out(f"{result.test_case} {result.result.verdict!s}")
    return 0


def runner_main(suite: Suite, parameter: ParameterReader, args: Optional[Sequence[str]] = None) -> int:
    """The runner program of ``suite``, its module parameters read by
    ``parameter``, on the command line ``args``, or the process's own after the
    program's name where it is ``None`` (docs/SCE_TTCN3_LANE.md §4):
    ``run_runner`` over the standard output and the standard error. Answers the
    exit status, which the generated ``main`` ends the process with.

    The standard output is the contract's, the ``<test case> <verdict>`` lines
    and nothing else, which a lane parses. So the program keeps it for those
    lines and hands ``sys.stdout`` to the standard error while the cases run: a
    machine's ``<log>``, which a generated Python statechart writes with
    ``print``, and any other diagnostic of the run go there, as the ``logging``
    module's do already.
    """
    given = sys.argv[1:] if args is None else list(args)
    contract = sys.stdout
    with redirect_stdout(sys.stderr):
        status = run_runner(
            suite,
            parameter,
            given,
            out=lambda line: print(line, file=contract),
            err=lambda line: print(line, file=sys.stderr),
        )
    contract.flush()
    sys.stderr.flush()
    return status


def undeclared_parameter(id_: str) -> OverrideRefusal:
    """The refusal of a value for the module parameter ``id_``, which the suite
    does not declare."""
    return OverrideRefusal(f"the suite declares no module parameter '{id_}'")


def integer_parameter(id_: str, literal: str, width: from_canonical.IntegerType) -> int:
    """A value of the integer type ``width`` of the module parameter ``id_``:
    the text of its decimal digits, as the canonical form writes one. The
    suite's unit saves it with the ``saved_write_<type>`` of its type.

    Raises an ``OverrideRefusal`` naming the parameter, where it is no such text
    or past the range of ``width``.
    """
    try:
        return from_canonical.integer(literal, width, id_)
    except TraceRefusal as refused:
        raise OverrideRefusal(refused.reason) from None


def bool_parameter(id_: str, literal: str) -> ModuleParameterValue:
    """A value of the ``bool`` module parameter ``id_``: ``true`` or ``false``.

    Raises an ``OverrideRefusal`` otherwise.
    """
    if literal == "true":
        return saved_write_bool(True)
    if literal == "false":
        return saved_write_bool(False)
    raise OverrideRefusal(f"{id_}: '{literal}' is neither true nor false")


def text_parameter(literal: str) -> ModuleParameterValue:
    """A value of a text module parameter: the text itself. Its bound, like
    every value's type, is held where the component reads it, as for a
    default."""
    return saved_write_string(literal)


def enum_parameter(literal: str) -> ModuleParameterValue:
    """A value of an enum module parameter: the name its document declares for
    the variant, which is how a value of an enum is saved. A name its enum does
    not declare is refused where the component reads it, as for a default."""
    return saved_write_string(literal)


def bytes_parameter(id_: str, literal: str) -> ModuleParameterValue:
    """A value of the byte string module parameter ``id_``: its octets in hex,
    two digits each, as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them.

    Raises an ``OverrideRefusal`` where it is not.
    """
    if _HEX.fullmatch(literal) is None:
        raise OverrideRefusal(f"{id_}: '{literal}' is not octets in hex")
    return saved_write_bytes(bytes.fromhex(literal))
