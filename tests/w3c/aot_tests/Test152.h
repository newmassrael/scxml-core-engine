// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test152_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach error handling (AOT JSEngine)
 */
struct Test152 : public SimpleAotTest<Test152, 152> {
    static constexpr const char *DESCRIPTION =
        "W3C SCXML 4.6: foreach error handling - illegal array/item raises error.execution";
    using SM = SCE::Generated::test152::test152;
};

// Auto-register
inline static AotTestRegistrar<Test152> registrar_Test152;

}  // namespace SCE::W3C::AotTests
