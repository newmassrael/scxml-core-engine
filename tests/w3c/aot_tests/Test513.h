// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test513_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP Event I/O Processor Success Response
 *
 * Tests that SCXML Processor's BasicHTTP Event I/O Processor responds with
 * HTTP 200 OK when receiving well-formed events.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Accept HTTP POST requests at the access URI
 * - Respond with 2XX success code for well-formed events
 * - Place received events in the external event queue
 *
 * Expected behavior:
 * - HttpAotTest starts W3CHttpTestServer in process and hands the machine the
 *   location that listener bound (W3CHttpTestServer::accessUri): its port is
 *   the one the kernel assigned, unless SCE_W3C_HTTP_PORT names one
 * - The machine sends `test` to `_ioprocessors['basichttp'].location`, the
 *   location it was handed, so the POST reaches that listener
 * - The listener answers 200 OK and the event is placed in the external queue
 * - The machine takes `test` to pass; `timeout` (30s) takes it to fail
 *
 * Original W3C test is manual (requires external wget/curl to send HTTP POST).
 * This implementation automates validation by having the state machine send
 * HTTP events to itself, validating successful 200 OK response reception.
 *
 * The target is the processor's published location, an expression the script
 * engine evaluates, not a URL written into the document: the document names
 * no port, so it posts wherever the listener of the run bound.
 */
struct Test513 : public HttpAotTest<Test513, 513> {
    using SM = SCE::Generated::test513::test513;
};

// Auto-register
inline static AotTestRegistrar<Test513> registrar_Test513;

}  // namespace SCE::W3C::AotTests
