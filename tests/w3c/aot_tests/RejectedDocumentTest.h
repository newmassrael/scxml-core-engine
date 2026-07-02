// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "AotTestBase.h"
#include "AotTestRegistry.h"

namespace SCE::W3C::AotTests {

/// W3C SCXML 5.8: Base class for tests where document rejection IS the correct behavior.
///
/// When codegen detects a non-conformant document (e.g., unloadable external script),
/// it generates a stub header with `#define SCE_DOCUMENT_REJECTED 1`.
/// This base class auto-passes such tests — the processor correctly refused the document.
///
/// Usage in TestXXX.h:
///   #include "testXXX_sm.h"
///   #ifdef SCE_DOCUMENT_REJECTED
///   SCE_REJECTED_DOCUMENT_TEST(XXX)
///   #else
///   // normal SimpleAotTest definition
///   #endif
template <typename Derived, int TestNum> class RejectedDocumentTest : public AotTestBase {
public:
    static constexpr int TEST_ID = TestNum;

    bool run() override {
        // Document was rejected at codegen time — correct per W3C SCXML 5.8
        return true;
    }

    int getTestId() const override {
        return TEST_ID;
    }

    const char *getDescription() const override {
        if (cachedDescription_.empty()) {
            cachedDescription_ = AotTestBase::loadMetadataDescription(TEST_ID);
        }
        return cachedDescription_.c_str();
    }

    const char *getTestType() const override {
        return "document_rejected";
    }

private:
    mutable std::string cachedDescription_;
};

}  // namespace SCE::W3C::AotTests

/// Convenience macro: defines a rejected-document AOT test + auto-registers it.
/// Place after `#include "testXXX_sm.h"` inside `#ifdef SCE_DOCUMENT_REJECTED`.
#define SCE_REJECTED_DOCUMENT_TEST(NUM)                                                                                \
    namespace SCE::W3C::AotTests {                                                                                     \
    struct Test##NUM : public RejectedDocumentTest<Test##NUM, NUM> {};                                                 \
    inline static AotTestRegistrar<Test##NUM> registrar_Test##NUM;                                                     \
    }
