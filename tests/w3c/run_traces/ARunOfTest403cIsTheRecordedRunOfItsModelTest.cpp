// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of W3C test403c by the C++ lane is the recorded run of its model,
// `docs/formal/W3CTest403c.tla` over `AppendixD.tla` (docs/adr/0115, D4 and D8;
// RunTraceRecording.h). Its document takes the optimal enabled transition set of a parallel state,
// in which a transition of a region preempts the transitions of another.
//
// This program builds the machine the conformance program w3c_cpp_test_403 builds,
// with `SCE_TRACE_PORT=1`, so its engine tells the port each step it takes. That
// program builds it with nothing set, where the engine holds no recorder.

#include "RunTraceRecording.h"
#include "test403c_sm.h"

namespace SCE::Tests {

TEST(ARunOfTest403cIsTheRecordedRunOfItsModel, TheTracedRunIsTheRecordedOne) {
    RunTraceRecording::holdTheRunToTheRecordedRunOf<SCE::Generated::test403c::test403c>("W3CTest403c");
}

}  // namespace SCE::Tests
