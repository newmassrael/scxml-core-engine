#pragma once
#include "HttpAotTest.h"
#include "test520_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP content element as message body
 *
 * Test that <content> gets sent as the body of the HTTP message when no event attribute is specified.
 * W3C SCXML C.2 specifies that for HTTP event processors, the event name is optional when content is provided.
 * The content will be sent as the HTTP POST body with empty event name.
 */
struct Test520 : public HttpAotTest<Test520, 520> {
    static constexpr const char *DESCRIPTION = "BasicHTTP content element (W3C C.2 AOT Static)";
    using SM = RSM::Generated::test520::test520;
};

// Auto-register
inline static AotTestRegistrar<Test520> registrar_Test520;

}  // namespace RSM::W3C::AotTests
