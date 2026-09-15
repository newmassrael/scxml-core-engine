// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "ScheduledAotTest.h"
#include "test186_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Delayed send with params (AOT)
 *
 * Requires event scheduler polling for delayed send processing.
 */
struct Test186 : public ScheduledAotTest<Test186, 186> {
    using SM = SCE::Generated::test186::test186;
};

// Auto-register
inline static AotTestRegistrar<Test186> registrar_Test186;

}  // namespace SCE::W3C::AotTests
