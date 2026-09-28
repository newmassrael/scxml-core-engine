// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test147_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief If/elseif/else conditionals with datamodel
 */
struct Test147 : public SimpleAotTest<Test147, 147> {
    using SM = SCE::Generated::test147::test147;
};

// Auto-register
inline static AotTestRegistrar<Test147> registrar_Test147;

}  // namespace SCE::W3C::AotTests
