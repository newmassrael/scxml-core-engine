// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test156_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief Foreach error handling stops loop (AOT JSEngine)
 */
struct Test156 : public SimpleAotTest<Test156, 156> {
    static constexpr const char *DESCRIPTION = "W3C SCXML 4.6: foreach stops execution on child action error";
    using SM = SCE::Generated::test156::test156;
};

// Auto-register
inline static AotTestRegistrar<Test156> registrar_Test156;

}  // namespace SCE::W3C::AotTests
