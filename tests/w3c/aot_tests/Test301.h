// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once
#include "test301_sm.h"

#ifdef SCE_DOCUMENT_REJECTED
#include "RejectedDocumentTest.h"
SCE_REJECTED_DOCUMENT_TEST(301)
#else
#include "SimpleAotTest.h"

namespace SCE::W3C::AotTests {

struct Test301 : public SimpleAotTest<Test301, 301> {
    static constexpr const char *DESCRIPTION = "Script element document rejection (W3C 5.8 AOT)";
    using SM = SCE::Generated::test301::test301;
};

inline static AotTestRegistrar<Test301> registrar_Test301;

}  // namespace SCE::W3C::AotTests
#endif
