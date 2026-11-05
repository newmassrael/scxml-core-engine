// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test518_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP Event I/O Processor Namelist Encoding
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly encodes
 * namelist values as HTTP POST parameters.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Encode namelist variables as POST parameters (_scxmleventname for event name)
 * - Send HTTP POST request to target URI with application/x-www-form-urlencoded
 * - Include all namelist variables in POST body (e.g., "Var1=2&_scxmleventname=test")
 *
 * Expected behavior:
 * - The datamodel declares Var1=2
 * - <send event="test" namelist="Var1"> posts to
 *   targetexpr="_ioprocessors['basichttp'].location": the location HttpAotTest
 *   hands the machine from the W3CHttpTestServer it starts in process, on the
 *   port that listener bound (the kernel's, unless SCE_W3C_HTTP_PORT names one)
 * - NamelistHelper::evaluateNamelist() evaluates Var1 through the script engine
 * - W3CHttpTestServer receives POST with "Var1=2&_scxmleventname=test"
 * - Server sends the "test" event back to the state machine
 * - State machine transitions to pass state; any other event takes it to fail
 *
 * Shared helpers (Zero Duplication):
 * - SendHelper.isHttpTarget() routes the HTTP target for both engines
 * - NamelistHelper::evaluateNamelist() evaluates the namelist variables
 * - EventDataHelper::buildJsonFromParams() builds the POST parameter encoding
 *
 * The document names no URL: its target is the processor's published
 * location, an expression the script engine evaluates, as is Var1.
 */
struct Test518 : public HttpAotTest<Test518, 518> {
    using SM = SCE::Generated::test518::test518;
};

// Auto-register
inline static AotTestRegistrar<Test518> registrar_Test518;

}  // namespace SCE::W3C::AotTests
