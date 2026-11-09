# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The runtime of the test profile (docs/SCE_TEST_PROFILE.md §12.2): the round
scheduler, ``NOW``, port queues and snapshots, timer bindings, verdicts, the
entity table and the guard, written once for every suite.

A suite's generated modules reach it from here: a test component's machine
through its ``ComponentContext``, which it keeps from ``ComponentMachine.start``
on; a configuration as a ``Configuration`` and a ``MachineFactory``; a test case
and a suite as a ``TestCase`` and a ``Suite``, which ``run_suite`` runs. A
template's length restriction on a text counts it with ``text_length``. The
algorithm is the one the document specifies and the C++ runtime
(``sce/include/test/``), the Rust one (``sce_rust_runtime::test``), the Kotlin
one (``com.sce.runtime.test``) and the Go one (``testprofile``) implement, type
for type. When this package and the Rust one diverge, Rust wins, as it does for
the rest of the Python runtime.

The name
--------
The other runtimes call the module ``test``, and the Go one ``testprofile``.
Python's tooling claims both of the spellings that come first: the standard
library holds a top-level package ``test`` (CPython's own regression suite), and
pytest collects every module whose file is named ``test_*.py`` or ``*_test.py``
and every class whose name begins with ``Test`` as a test. ``testprofile``
names the document this package implements and is neither: no file of it is
collected, and no import of it can be read as the standard library's. The
classes whose names begin with ``Test`` (``TestCaseRun``, ``TestCase``,
``TestCaseResult``, ``TestValue`` and the test log's) say ``__test__ = False``,
so a test that imports one is not asked to collect it. For the same reason the
module of the test log is ``testlog``.

The test log
------------
A run's test log (§12.5) is its ``TestCaseResult.records``, written in its one
canonical text by ``TestCaseResult.canonical_log``. A message's value reaches it
in the canonical form ``canonical`` spells, which the generated unit of a
template over a record writes (``<template>.canonical``), with the name of its
record type (``<template>.TYPE``): a send carries the three together as one
``TestValue`` (``message_value``).

Ownership
---------
The state of a run is shared between the run and a context per component, as
the C++ runtime's context holds a reference to its run: the machine keeps its
context from ``start`` on, and so may the host processors it registers on its
engine for the doors of a send and of a PTC. A run is driven on one thread
(§12.4), the one that calls ``TestCaseRun.run``, so the state takes no lock,
where the Rust runtime's takes a mutex because its engine holds those processors
as ``Send`` closures.

The run never holds its state across a call into a machine: every step of it
marks the state held for its own length, and a machine is called between steps.
So every call a machine makes into its context finds the state free. A call
that finds it held is a call made while the run, or the same component, is
already inside the state, which on one thread can only be a re-entrant call (a
snapshot reader that calls its context, say), and it raises ``RuntimeError``
naming the component, as the Rust runtime panics. An exception that leaves a
step part way goes out of the run, as a panic does in Rust.
"""

from . import canonical
from .module_parameters import ModuleParameters, ModuleParameterValue
from .port import Duration, Instant, Message, Port, TestValue, message_value
from .run import (
    MTC_NAME,
    ComponentContext,
    ComponentMachine,
    Configuration,
    Connection,
    Endpoint,
    LogKind,
    LogRecord,
    MachineFactory,
    Mapping,
    SystemAdapter,
    SystemInput,
    TestCaseResult,
    TestCaseRun,
    receives,
)
from .suite import (
    CaseResult,
    Execute,
    Features,
    OverrideRefusal,
    Selection,
    Suite,
    TestCase,
    enabled,
    features_of,
    run_suite,
)
from .testlog import TEST_LOG_SCHEMA, TestLogEvent, TestLogRecord
from .text_length import text_length
from .timer import Timer, TimerStatus
from .verdict import Verdict, worse

__all__ = [
    "MTC_NAME",
    "TEST_LOG_SCHEMA",
    "CaseResult",
    "ComponentContext",
    "ComponentMachine",
    "Configuration",
    "Connection",
    "Duration",
    "Endpoint",
    "Execute",
    "Features",
    "Instant",
    "LogKind",
    "LogRecord",
    "MachineFactory",
    "Mapping",
    "Message",
    "ModuleParameterValue",
    "ModuleParameters",
    "OverrideRefusal",
    "Port",
    "Selection",
    "Suite",
    "SystemAdapter",
    "SystemInput",
    "TestCase",
    "TestCaseResult",
    "TestCaseRun",
    "TestLogEvent",
    "TestLogRecord",
    "TestValue",
    "Timer",
    "TimerStatus",
    "Verdict",
    "canonical",
    "enabled",
    "features_of",
    "message_value",
    "receives",
    "run_suite",
    "text_length",
    "worse",
]
