#pragma once
#include "SimpleAotTest.h"
#include "test301_sm.h"

namespace SCE::W3C::AotTests {

/// W3C SCXML 5.8: Empty <script/> element — document rejection at parse time.
/// Codegen detects empty script and overrides initial state to "pass" (document rejected).
struct Test301 : public SimpleAotTest<Test301, 301> {
    static constexpr const char *DESCRIPTION = "Script element document rejection (W3C 5.8 AOT)";
    using SM = SCE::Generated::test301::test301;
};

// Auto-register
inline static AotTestRegistrar<Test301> registrar_Test301;

}  // namespace SCE::W3C::AotTests
