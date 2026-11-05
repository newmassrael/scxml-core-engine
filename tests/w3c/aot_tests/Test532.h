// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test532_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief HTTP Method Name Fallback for Event Naming
 *
 * Tests that when <send> has NO event attribute and NO _scxmleventname parameter,
 * the SCXML Processor uses the HTTP method name (HTTP.POST) as the event name
 * for BasicHTTP Event I/O Processor.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Use HTTP method name as event name when both event attribute and _scxmleventname are absent
 * - Send HTTP POST request with content as message body
 * - Server responds with HTTP method name (HTTP.POST) as the event name
 *
 * Test SCXML structure:
 * ```xml
 * <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
 *       targetexpr="_ioprocessors['basichttp'].location">
 *   <content>some content</content>
 * </send>
 * ```
 *
 * Expected behavior:
 * 1. <send> has NO event attribute and NO _scxmleventname parameter
 * 2. Only a <content> element, so the POST has a message body
 * 3. The target is `_ioprocessors['basichttp'].location`: the location
 *    HttpAotTest hands the machine from the W3CHttpTestServer it starts in
 *    process (W3CHttpTestServer::accessUri), on the port that listener bound,
 *    which the kernel assigns unless SCE_W3C_HTTP_PORT names one
 * 4. W3CHttpTestServer receives the POST with the content as its body
 * 5. Server names the event after the HTTP method: "HTTP.POST"
 * 6. State machine receives "HTTP.POST" and transitions to pass; any other
 *    event, the 3s `timeout` among them, takes it to fail
 *
 * Key W3C SCXML C.2 Feature:
 * - When BOTH event attribute AND _scxmleventname parameter are ABSENT, HTTP method name becomes event name
 * - This is the fallback mechanism when no explicit event name is provided
 * - Server MUST use HTTP method name (e.g., "HTTP.POST") as the event name
 *
 * Difference from other HTTP tests:
 * - test 518: event="test" + namelist (event name in attribute)
 * - test 519: event="test" + param (event name in attribute)
 * - test 520: NO event + content-only (first W3C test for HTTP method fallback)
 * - test 531: NO event + _scxmleventname param (event name from parameter)
 * - test 532: NO event + NO _scxmleventname + content (HTTP method name fallback)
 *
 * The POST is real network traffic to the in-process listener, at the location
 * the machine publishes; the document names no port.
 */
struct Test532 : public HttpAotTest<Test532, 532> {
    using SM = SCE::Generated::test532::test532;
};

// Auto-register
inline static AotTestRegistrar<Test532> registrar_Test532;

}  // namespace SCE::W3C::AotTests
