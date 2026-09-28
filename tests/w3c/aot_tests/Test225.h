// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test225_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief W3C SCXML Test 225
 *
 * Auto-generated AOT test registry.
 */
struct Test225 : public SimpleAotTest<Test225, 225> {
    using SM = SCE::Generated::test225::test225;
};

inline static AotTestRegistrar<Test225> registrar_Test225;

}  // namespace SCE::W3C::AotTests
