// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "HttpAotTest.h"
#include "test522_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief BasicHTTP Event I/O Processor location field
 *
 * Tests that the location field of the BasicHTTP Event I/O Processor's
 * _ioprocessors entry can be used to send a message to the processor.
 *
 * W3C SCXML C.2 specifies that BasicHTTP Event I/O Processor must:
 * - Publish its access URI as the location of its _ioprocessors entry
 * - Accept HTTP POST requests at that URI
 * - Deliver the resulting event back to the state machine
 *
 * Expected behavior:
 * - <send event="test" targetexpr="_ioprocessors['basichttp'].location">
 *   posts to the location HttpAotTest hands the machine from the
 *   W3CHttpTestServer it starts in process, on the port that listener bound
 *   (the kernel's, unless SCE_W3C_HTTP_PORT names one)
 * - W3CHttpTestServer receives POST and sends "test" event back
 * - Any event other than timeout or error transitions to pass state
 * - Wildcard transition "*" catches the response event
 *
 * Shared infrastructure (Zero Duplication):
 * - HttpEventTarget shared HTTP POST implementation
 * - W3CHttpTestServer shared test infrastructure
 * - EventMatchingHelper for wildcard transition matching
 *
 * The document names no URL: its target is the processor's published
 * location, an expression the script engine evaluates.
 */
struct Test522 : public HttpAotTest<Test522, 522> {
    using SM = SCE::Generated::test522::test522;
};

// Auto-register
inline static AotTestRegistrar<Test522> registrar_Test522;

}  // namespace SCE::W3C::AotTests
