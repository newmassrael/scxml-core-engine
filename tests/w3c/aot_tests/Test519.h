// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test519_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP Event I/O Processor Param Encoding
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly encodes
 * <param> values as HTTP POST parameters.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Map param names and values to HTTP POST parameters
 * - Send HTTP POST request with application/x-www-form-urlencoded
 * - Include all param elements in POST body (e.g., "param1=1&_scxmleventname=test")
 *
 * Expected behavior:
 * - <send event="test"> with <param name="param1" expr="1"> posts to
 *   targetexpr="_ioprocessors['basichttp'].location": the location HttpAotTest
 *   hands the machine from the W3CHttpTestServer it starts in process, on the
 *   port that listener bound (the kernel's, unless SCE_W3C_HTTP_PORT names one)
 * - EventDataHelper::buildJsonFromParams() encodes params as POST parameters
 * - W3CHttpTestServer receives POST with "param1=1&_scxmleventname=test"
 * - Server echoes the "test" event back, its data carrying param1
 * - The transition guarded by _event.data["param1"] == 1 takes the machine to
 *   pass; any other event takes it to fail
 *
 * Difference from test 518 (namelist):
 * - test 518: Uses namelist="Var1" (evaluates datamodel variables)
 * - test 519: Uses <param name="param1" expr="1"> (inline parameter values)
 * - Both tests validate HTTP POST parameter encoding (W3C SCXML C.2)
 *
 * Shared helpers (Zero Duplication):
 * - SendHelper.isHttpTarget() routes the HTTP target for both engines
 * - EventDataHelper::buildJsonFromParams() builds the POST parameter encoding
 *
 * The document names no URL: its target is the processor's published
 * location, an expression the script engine evaluates.
 */
struct Test519 : public HttpAotTest<Test519, 519> {
    using SM = SCE::Generated::test519::test519;
};

// Auto-register
inline static AotTestRegistrar<Test519> registrar_Test519;

}  // namespace SCE::W3C::AotTests
