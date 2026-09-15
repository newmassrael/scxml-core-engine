// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test173_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief targetexpr evaluates current datamodel value at send execution time
 */
struct Test173 : public SimpleAotTest<Test173, 173> {
    using SM = SCE::Generated::test173::test173;
};

// Auto-register
inline static AotTestRegistrar<Test173> registrar_Test173;

}  // namespace SCE::W3C::AotTests
