// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test531_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief _scxmleventname Parameter Event Name Override
 *
 * Tests that when <send> has NO event attribute, the _scxmleventname parameter
 * value becomes the event name for BasicHTTP Event I/O Processor.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Use _scxmleventname parameter value as event name when event attribute is absent
 * - Send HTTP POST request with _scxmleventname in POST parameters
 * - Server extracts _scxmleventname value and uses it as the event name
 *
 * Test SCXML structure:
 * ```xml
 * <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
 *       targetexpr="_ioprocessors['basichttp'].location">
 *   <param name="_scxmleventname" expr="'test'"/>
 * </send>
 * ```
 *
 * Expected behavior:
 * 1. <send> has NO event attribute (only type and targetexpr)
 * 2. <param name="_scxmleventname" expr="'test'"> specifies event name
 * 3. The target is the location HttpAotTest hands the machine from the
 *    W3CHttpTestServer it starts in process, on the port that listener bound
 *    (the kernel's, unless SCE_W3C_HTTP_PORT names one)
 * 4. EventDataHelper::buildJsonFromParams() encodes as POST parameters
 * 5. W3CHttpTestServer receives POST with "_scxmleventname=test"
 * 6. Server extracts "test" from _scxmleventname param and echoes "test" event
 * 7. State machine receives "test" event and transitions to pass state; any
 *    other event, the 3s `timeout` among them, takes it to fail
 *
 * Key W3C SCXML C.2 Feature:
 * - When event attribute is ABSENT, _scxmleventname param value becomes event name
 * - This allows dynamic event name specification via parameters
 * - Server MUST extract _scxmleventname from POST parameters and use as event name
 *
 * Difference from other HTTP tests:
 * - test 518: event="test" + namelist (event name in attribute)
 * - test 519: event="test" + param (event name in attribute)
 * - test 520: NO event + content-only (server generates HTTP.POST event)
 * - test 531: NO event + _scxmleventname param (event name from parameter)
 *
 * Shared helpers (Zero Duplication):
 * - SendHelper.isHttpTarget() routes the HTTP target for both engines
 * - EventDataHelper::buildJsonFromParams() builds the POST parameter encoding
 * - W3CHttpTestServer's _scxmleventname handling serves both engines
 *
 * The POST is real network traffic to the in-process listener, at the location
 * the machine publishes; the document names no port.
 */
struct Test531 : public HttpAotTest<Test531, 531> {
    using SM = SCE::Generated::test531::test531;
};

// Auto-register
inline static AotTestRegistrar<Test531> registrar_Test531;

}  // namespace SCE::W3C::AotTests
