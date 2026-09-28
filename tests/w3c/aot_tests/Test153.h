// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test153_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach array iteration order (AOT JSEngine)
 */
struct Test153 : public SimpleAotTest<Test153, 153> {
    using SM = SCE::Generated::test153::test153;
};

// Auto-register
inline static AotTestRegistrar<Test153> registrar_Test153;

}  // namespace SCE::W3C::AotTests
