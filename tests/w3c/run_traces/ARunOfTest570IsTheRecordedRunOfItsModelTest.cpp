// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of W3C test570 by the C++ lane is the recorded run of its model,
// `docs/formal/W3CTest570.tla` over `AppendixD.tla` (docs/adr/0115, D4 and D8;
// RunTraceRecording.h). Its document ends the regions of a parallel state one by one, and the
// done events of each region and of the parallel state are taken in order.
//
// This program builds the machine the conformance program w3c_cpp_test_570 builds,
// with `SCE_TRACE_PORT=1`, so its engine tells the port each step it takes. That
// program builds it with nothing set, where the engine holds no recorder.

#include "RunTraceRecording.h"
#include "test570_sm.h"

namespace SCE::Tests {

TEST(ARunOfTest570IsTheRecordedRunOfItsModel, TheTracedRunIsTheRecordedOne) {
    RunTraceRecording::holdTheRunToTheRecordedRunOf<SCE::Generated::test570::test570>("W3CTest570");
}

}  // namespace SCE::Tests
