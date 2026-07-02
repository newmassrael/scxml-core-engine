# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""W3C SCXML 5.5 + 6.3.1 donedata surfacing on the Python binding's
local-invoke path.

Closes the W3C IRP coverage gap: no public IRP test exercises
`<donedata>` on the invoked child's top-level `<final>` combined with
`done.invoke.<id>._event.data` readback on the parent. Mirrors:

  - tests/integration/DonedataLocalInvokeTest.cpp           (C++ Interpreter,
                                                             fb8e3c79..00f347cb)
  - backends/kotlin/tests/.../DonedataLocalInvokeTest.kt         (Kotlin AOT,
                                                             4d284cb4..b070a7ad)
  - backends/rust/tests/tests/donedata_local_invoke.rs           (Rust AOT,
                                                             b44f02fe..f9e236f6)
  - backends/go/tests/donedata_local_invoke/donedata_local_invoke_test.go
                                                            (Go AOT,
                                                             fde0432f..70310c29)

The Python binding wraps `ReadySCXMLEngine`, which delegates to the
C++ Interpreter. The Interpreter contract is already exercised by the
C++ integration test above; this script makes the same contract
load-bearing through the Python pybind11 surface so that any future
regression in the Python wrapper (e.g., losing `EventMetadata.data` on
the way through `sendExternalEvent` / `getCurrentState`) is caught here.

Plain script harness (no pytest dependency), matching `test_w3c.py`.

Run:
    PYTHONPATH=build_python/sce-python:sce-python/python \\
        python3 sce-python/tests/test_donedata_local_invoke.py
"""

import os
import sys
import time

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.abspath(os.path.join(_SCRIPT_DIR, "..", ".."))

sys.path.insert(0, os.path.join(_SCRIPT_DIR, "..", "python"))

import _sce as sce

FIXTURE_PATH = os.path.join(
    _REPO_ROOT,
    "integration_resources",
    "donedata_local_invoke",
    "donedata_local_invoke.scxml",
)

TIMEOUT_SEC = 5.0
POLL_INTERVAL_SEC = 0.01


def run_until_completion(engine) -> tuple[str, list[str]]:
    """Poll until a final state is reached or the engine stops.

    Returns (current_state, active_states) at the first observation of
    completion (running == False) or when the current state is a known
    final id.
    """
    deadline = time.monotonic() + TIMEOUT_SEC
    while time.monotonic() < deadline:
        state = engine.current_state
        if state in ("pass", "fail"):
            return state, engine.active_states
        if not engine.running:
            return engine.current_state, engine.active_states
        time.sleep(POLL_INTERVAL_SEC)
    return engine.current_state, engine.active_states


def main() -> int:
    if not os.path.exists(FIXTURE_PATH):
        print(f"FAIL: fixture not found at {FIXTURE_PATH}")
        return 1

    engine = sce.Engine.from_file(FIXTURE_PATH)
    if not engine.start():
        print(f"FAIL: engine.start() returned False: {engine.last_error}")
        return 1

    try:
        final_state, active = run_until_completion(engine)
    finally:
        try:
            if engine.running:
                engine.stop()
        except Exception:
            pass

    if final_state == "pass" or "pass" in active:
        print(f"PASS: donedata_local_invoke reached pass (state={final_state!r}, active={active})")
        return 0

    print(
        "FAIL: parent reached "
        f"state={final_state!r} active={active}, want 'pass'.\n"
        "  `_event.data.result === 42` (param branch) or "
        "`_event.data === 'hello_content'` (content branch) failed.\n"
        "  An empty `_event.data` on `done.invoke.<id>` means the C++ "
        "Interpreter's `SCXMLInvokeHandler` completion callback dropped\n"
        "  the child's stashed donedata before the Python wrapper re-emitted "
        "it. Check `sce/src/runtime/StateMachine.cpp` "
        "`pendingDonedataAtFinal_`\n"
        "  / `donedataAtFinal()` and the `SCXMLInvokeHandler` completion "
        "lambda — these were wired in fb8e3c79..00f347cb."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
