// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test567_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP param encoding in _event.data
 *
 * Verifies that when sending an event with <param> elements via BasicHTTP Event I/O Processor,
 * the processor correctly encodes parameters in the HTTP POST body and populates _event.data
 * with the parameter structure in the response event for ECMAScript datamodel.
 *
 * Test flow:
 * 1. The datamodel declares Var1 = 2; the machine enters s0
 * 2. Entry action in s0: <send event="test"> with <param name="param1" expr="2">
 *    to targetexpr="_ioprocessors['basichttp'].location", the location
 *    HttpAotTest hands the machine from the W3CHttpTestServer it starts in
 *    process, on the port that listener bound (the kernel's, unless
 *    SCE_W3C_HTTP_PORT names one)
 * 3. HttpAotTest's send callback POSTs the URL-encoded body "param1=2&_scxmleventname=test"
 * 4. W3CHttpTestServer makes the event "test" with _event.data {"param1": ...}
 *    and HttpAotTest posts it to the machine's inbox
 * 5. Transition s0 to s1 on "test": assign Var1 = _event.data.param1
 * 6. In s1 the guard "Var1 == 2" takes the machine to pass, otherwise to fail
 *
 * ARCHITECTURE.md Compliance - Static Hybrid Approach:
 *
 * - Static state machine structure (compile-time states/transitions)
 * - JSEngine for ECMAScript datamodel (variable initialization, guard evaluation, _event.data access)
 * - Uses Helper functions: SendHelper (param evaluation), EventDataHelper (JSON building),
 *   SystemVariableHelper (_event.data setup), GuardHelper (guard evaluation)
 * - Zero Duplication: the Interpreter's run serves the same W3CHttpTestServer
 *
 * W3C SCXML Features:
 * - BasicHTTP Event I/O Processor (W3C SCXML C.2)
 * - <param> expression evaluation with JSEngine (W3C SCXML 5.7)
 * - _event.data structure for event parameters (W3C SCXML 5.10)
 * - URL-encoded HTTP POST body with param values (W3C SCXML C.2)
 * - Guard conditions accessing _event.data properties (W3C SCXML 5.9)
 *
 * Infrastructure:
 * - HttpAotTest base class manages W3CHttpTestServer lifecycle
 * - The listener's thread posts each event it makes to the machine's inbox,
 *   which the machine applies at its next tick
 */
struct Test567 : public HttpAotTest<Test567, 567> {
    using SM = SCE::Generated::test567::test567;
};

// Auto-register
inline static AotTestRegistrar<Test567> registrar_Test567;

}  // namespace SCE::W3C::AotTests
