#pragma once

#include "SimpleAotTest.h"
#include "test198_sm.h"

namespace RSM::W3C::AotTests {

/**
 * @brief W3C SCXML 6.2: Default send type is SCXMLEventProcessor when type/typeexpr not specified
 *
 * Tests that when neither 'type' nor 'typeexpr' attributes are specified in a <send> element,
 * the SCXML Processor defaults to using the SCXML Event I/O Processor
 * (http://www.w3.org/TR/scxml/#SCXMLEventProcessor), verifiable through _event.origintype.
 */
struct Test198 : public SimpleAotTest<Test198, 198> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.2: Default send type is SCXMLEventProcessor when type/typeexpr not specified";
    using SM = RSM::Generated::test198::test198;
};

inline static AotTestRegistrar<Test198> registrar_Test198;

}  // namespace RSM::W3C::AotTests
