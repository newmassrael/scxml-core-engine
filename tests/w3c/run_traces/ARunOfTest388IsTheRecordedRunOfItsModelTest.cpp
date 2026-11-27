// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of W3C test388 by the C++ lane is the recorded run of its model,
// `docs/formal/W3CTest388.tla` over `AppendixD.tla` (docs/adr/0115, D4 and D8;
// RunTraceRecording.h). Its document re-enters a state through a deep and a shallow history, which
// restore the configuration the state last left.
//
// This program builds the machine the conformance program w3c_cpp_test_388 builds,
// with `SCE_TRACE_PORT=1`, so its engine tells the port each step it takes. That
// program builds it with nothing set, where the engine holds no recorder.

#include "RunTraceRecording.h"
#include "test388_sm.h"

namespace SCE::Tests {

TEST(ARunOfTest388IsTheRecordedRunOfItsModel, TheTracedRunIsTheRecordedOne) {
    RunTraceRecording::holdTheRunToTheRecordedRunOf<SCE::Generated::test388::test388>("W3CTest388");
}

}  // namespace SCE::Tests
