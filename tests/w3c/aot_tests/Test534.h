// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test534_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP Event I/O Processor _scxmleventname parameter transmission
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor correctly sends
 * the event name as the _scxmleventname parameter in HTTP POST requests.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Include _scxmleventname parameter in HTTP POST with event name value
 * - Send HTTP POST request with application/x-www-form-urlencoded
 * - State machine can access _scxmleventname parameter from event data
 *
 * Expected behavior:
 * - <send event="test" targetexpr="_ioprocessors['basichttp'].location"> posts
 *   to the location HttpAotTest hands the machine from the W3CHttpTestServer
 *   it starts in process, on the port that listener bound (the kernel's,
 *   unless SCE_W3C_HTTP_PORT names one)
 * - HTTP POST includes "_scxmleventname=test" parameter
 * - W3CHttpTestServer receives POST with "_scxmleventname=test"
 * - Server echoes "test" event back, its data carrying _scxmleventname
 * - The transition guarded by _event.data["_scxmleventname"] == "test" takes
 *   the machine to pass; any other event takes it to fail
 *
 * Shared helpers (Zero Duplication):
 * - SendHelper.isHttpTarget() routes the HTTP target for both engines
 * - EventDataHelper::buildJsonFromParams() builds the POST parameter encoding
 *
 * The script engine evaluates the target expression and the guard, which
 * reads the event's data.
 */
struct Test534 : public HttpAotTest<Test534, 534> {
    using SM = SCE::Generated::test534::test534;
};

// Auto-register
inline static AotTestRegistrar<Test534> registrar_Test534;

}  // namespace SCE::W3C::AotTests
