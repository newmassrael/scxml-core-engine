// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "ScheduledAotTest.h"
#include "test237_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief invoke cancellation on state exit
 *
 * Tests that when a parent state exits while an invoked child is running,
 * the invocation is cancelled and no done.invoke event is received.
 *
 * Test scenario:
 * - Parent state s0 invokes child with 2-second delay to termination
 * - Parent transitions to s1 after 1 second (exits s0, cancelling invoke)
 * - s1 waits 1.5 seconds for any events
 * - If done.invoke received → fail (cancellation didn't work)
 * - If timeout2 fires without done.invoke → pass (cancellation worked)
 *
 * W3C SCXML 6.4: Invoke mechanism with automatic cancellation on state exit
 * W3C SCXML 6.2: Delayed send requires event scheduler polling (ScheduledAotTest)
 */
struct Test237 : public ScheduledAotTest<Test237, 237> {
    using SM = SCE::Generated::test237::test237;

    // W3C SCXML 6.2: the pass path waits for `timeout1` (1s) and then
    // `timeout2` (1.5s), so the run needs 2.5s — past the 2s default. It
    // once finished inside the default only because a spurious
    // `cancel.invoke` reached s1's `*` transition at 1s; §scxml-6.4 defines
    // no such event, and without it the document runs its full course.
    std::chrono::seconds getTimeout() const override {
        return std::chrono::seconds(5);
    }
};

// Auto-register
inline static AotTestRegistrar<Test237> registrar_Test237;

}  // namespace SCE::W3C::AotTests
