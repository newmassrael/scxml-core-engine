#pragma once

#include "HttpAotTest.h"
#include "test201_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML C.2: BasicHTTP Event I/O Processor support
 *
 * Tests that the processor supports the BasicHTTP event I/O processor (optional feature).
 * Platforms are not required to support BasicHTTP event I/O, this is a compliance check.
 *
 * W3C SCXML C.2: BasicHTTP Event I/O Processor
 * W3C SCXML 6.2: send element with type attribute
 */
struct Test201 : public HttpAotTest<Test201, 201> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: Processors that support HTTP POST must use the value "
        "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor for the \"type\" attribute";
    using SM = SCE::Generated::test201::test201;
};

inline static AotTestRegistrar<Test201> registrar_Test201;

}  // namespace SCE::W3C::AotTests
