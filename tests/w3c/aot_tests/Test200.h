// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test200_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief SCXML event processor support (AOT)
 */
struct Test200 : public SimpleAotTest<Test200, 200> {
    using SM = SCE::Generated::test200::test200;
};

// Auto-register
inline static AotTestRegistrar<Test200> registrar_Test200;

}  // namespace SCE::W3C::AotTests
