// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test158_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Executable content document order (AOT)
 */
struct Test158 : public SimpleAotTest<Test158, 158> {
    using SM = SCE::Generated::test158::test158;
};

// Auto-register
inline static AotTestRegistrar<Test158> registrar_Test158;

}  // namespace SCE::W3C::AotTests
