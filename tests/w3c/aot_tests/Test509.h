#pragma once
#include "HttpAotTest.h"
#include "test509_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor POST Method
 *
 * Tests that SCXML Processor accepts messages at the access URI as HTTP POST requests.
 *
 * Expected behavior:
 * - Send event via BasicHTTPEventProcessor with target="http://localhost:8080/test"
 * - HTTP server receives POST request and validates method
 * - Server responds with event, state machine processes it
 * - Test transitions to pass if event received via HTTP POST
 *
 * Uses Static Hybrid approach: static state machine structure with
 * HttpAotTest providing real HTTP server infrastructure.
 */
struct Test509 : public HttpAotTest<Test509, 509> {
    static constexpr const char *DESCRIPTION = "BasicHTTP POST method (W3C C.2 AOT Static Hybrid)";
    using SM = RSM::Generated::test509::test509;
};

// Auto-register
inline static AotTestRegistrar<Test509> registrar_Test509;

}  // namespace RSM::W3C::AotTests
