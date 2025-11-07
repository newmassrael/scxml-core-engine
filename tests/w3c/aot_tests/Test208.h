#pragma once

#include "ScheduledAotTest.h"
#include "test208_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML 6.3: The Processor SHOULD make its best attempt to cancel all delayed events with the specified id.
 *
 * Requires event scheduler polling for delayed send processing.
 */
struct Test208 : public ScheduledAotTest<Test208, 208> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 6.3: The Processor SHOULD make its best attempt to cancel all delayed events with the specified id.";
    using SM = SCE::Generated::test208::test208;
};

// Auto-register
inline static AotTestRegistrar<Test208> registrar_Test208;

}  // namespace SCE::W3C::AotTests
