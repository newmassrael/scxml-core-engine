// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "SimpleAotTest.h"
#include "test223_sm.h"

namespace SCE::W3C::AotTests {

/**
 * @brief invoke idlocation attribute
 *
 * If the 'idlocation' attribute is present, the SCXML Processor MUST generate
 * an id automatically when the invoke element is evaluated and store it in the
 * location specified by 'idlocation'.
 */
struct Test223 : public SimpleAotTest<Test223, 223> {
    using SM = SCE::Generated::test223::test223;
};

inline static AotTestRegistrar<Test223> registrar_Test223;

}  // namespace SCE::W3C::AotTests
