# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A test suite of the test profile as its runner reads it
(docs/SCE_TEST_PROFILE.md §4.11, §11.7, §11.8, §12.2): the test cases in run
order, each with its guard and the selection that runs it in a world of the
suite's features, and the suite's module parameters; and the runner, which runs
the cases a world selects one at a time, in that order.

The tables are generated from the ``test-case`` and ``test-suite`` documents (a
case's configuration from its ``test-configuration``). They are data: what a
case does is its components' machines, which the run drives (``TestCaseRun``).
The runner is a library function, with no ``main``: a generated or hand-written
test calls it with the system adapters that play the SUT.

Cross-language siblings: ``sce_rust_runtime::test::run_suite`` (Rust),
``SCE::Test::runSuite`` (C++), ``com.sce.runtime.test.runSuite`` (Kotlin),
``testprofile.RunSuite`` (Go).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import AbstractSet, Callable, List, Optional

from .module_parameters import ModuleParameters, copy_of
from .port import Duration
from .run import (
    MTC_NAME,
    Configuration,
    LogKind,
    LogRecord,
    MachineFactory,
    SystemAdapter,
    TestCaseResult,
    TestCaseRun,
)
from .testlog import TestCaseEnded, TestCaseStarted, TestLogRecord
from .verdict import Verdict

#: The features a world of a suite's feature model enables (ADR 0057 §2): a
#: configuration of the model, closed under its ``requires`` edges.
Features = AbstractSet[str]


def features_of(*features: str) -> Features:
    """The world that enables each of ``features`` and no other."""
    return frozenset(features)


def enabled(world: Features, feature: str) -> bool:
    """Whether ``world`` enables ``feature``: the one test a selection is made
    of."""
    return feature in world


@dataclass
class TestCase:
    """A ``test-case`` (§11.7): its name and what it runs in.

    The configuration and the machines of its components are its
    configuration's; the module parameters are the ids every component of the
    configuration reads, which the suite must give.
    """

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    #: The test case's name in the suite and in the test log.
    name: str
    #: The configuration it runs.
    configuration: Configuration
    #: Builds the machines of the configuration's component types.
    machines: MachineFactory
    #: The ids of the module parameters its components read.
    module_parameters: List[str] = field(default_factory=list)


#: A selection (§11.8): whether a world of the suite's features runs a test
#: case. A condition of ADR 0057's grammar, lowered to the features it names.
Selection = Callable[[Features], bool]


@dataclass
class Execute:
    """One ``<sce:execute>`` (§11.8, P11): a test case, its guard, and its
    selection."""

    #: The test case.
    test_case: TestCase
    #: The test case's time limit, or ``None`` for none.
    guard: Optional[Duration] = None
    #: The selection, or ``None`` for a test case every world runs.
    selection: Optional[Selection] = None


@dataclass
class Suite:
    """A ``test-suite`` (§11.8): its test cases in run order and its module
    parameters at their defaults."""

    #: The suite's name.
    name: str
    #: The module parameters at their defaults; ``None`` is none.
    module_parameters: Optional[ModuleParameters] = None
    #: The test cases, in run order.
    executes: List[Execute] = field(default_factory=list)


@dataclass
class CaseResult:
    """What the runner reports of one test case it ran."""

    #: The test case's name.
    test_case: str
    #: Its verdict and logs.
    result: TestCaseResult

    def canonical_log(self, suite: str) -> str:
        """The test log of this run of the test case in the suite ``suite``, in
        its one canonical text (``TestCaseResult.canonical_log``)."""
        return self.result.canonical_log(suite, self.test_case)


class OverrideRefusal(Exception):
    """An override of a module parameter the suite does not declare: the
    caller's mistake, not a verdict of any test case, so no case is run."""

    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        #: Why.
        self.reason = reason


def _refused(test_case: str, why: str) -> CaseResult:
    """P13: the test case cannot start, so it ends at once with ``error``. No
    component was created, so the test log holds its start and its end alone."""
    return CaseResult(
        test_case=test_case,
        result=TestCaseResult(
            verdict=Verdict.ERROR,
            ended_at=0,
            log=[LogRecord(at=0, component=MTC_NAME, kind=LogKind.ERROR, detail=why)],
            records=[
                TestLogRecord(0, TestCaseStarted()),
                TestLogRecord(0, TestCaseEnded(Verdict.ERROR)),
            ],
        ),
    )


def run_suite(
    suite: Suite,
    world: Features,
    adapters: Callable[[str], Optional[SystemAdapter]],
    overrides: Optional[ModuleParameters] = None,
) -> List[CaseResult]:
    """Run the test cases of ``suite`` that ``world`` selects (P11).

    One at a time, on the calling thread, in the suite's order, each a fresh run
    from ``NOW = 0`` with fresh entities against the adapter ``adapters`` builds
    for it, given the test case's name, so that no case sees another's trace.
    Each component reads the suite's module parameters, with the values
    ``overrides`` gives in place of their defaults; ``None`` overrides none.

    A test case whose components read a module parameter the run has no value
    for ends with ``error`` (P13) before any component is created, as does one
    for which ``adapters`` builds no adapter (answers ``None``).

    Raises ``OverrideRefusal``, and runs no case, when ``overrides`` gives a
    module parameter the suite does not declare.
    """
    parameters = copy_of(suite.module_parameters)
    given = copy_of(overrides)
    for parameter in given.ids():
        if not parameters.has(parameter):
            raise OverrideRefusal(
                f"overrides the module parameter '{parameter}', which the suite '{suite.name}' "
                "does not declare"
            )
        parameters.set(parameter, given.of(parameter))
    results: List[CaseResult] = []
    for execute in suite.executes:
        if execute.selection is not None and not execute.selection(world):
            continue
        test_case = execute.test_case
        missing = next((p for p in test_case.module_parameters if not parameters.has(p)), None)
        if missing is not None:
            results.append(
                _refused(
                    test_case.name,
                    f"the test case reads the module parameter '{missing}', which is given no value",
                )
            )
            continue
        adapter = adapters(test_case.name)
        if adapter is None:
            results.append(_refused(test_case.name, "no system adapter is given for the test case"))
            continue
        run = TestCaseRun(test_case.configuration, test_case.machines, adapter, execute.guard, parameters)
        results.append(CaseResult(test_case=test_case.name, result=run.run()))
    return results
