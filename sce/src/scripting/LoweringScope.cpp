// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "scripting/LoweringScope.h"

#ifdef SCE_HAS_LOWERING_FFI
// sce-build's ECMAScript frontend, linked beside lua54 by
// `cmake/SCEBuildLowering.cmake`. The definition and the link are set on the
// same two lines of `sce/CMakeLists.txt`, so this cannot be reached without
// the symbols behind it.
//
// This is the ONE translation unit that names the C surface. An engine calls
// the class next door instead, which is why `LuaEngine.cpp` no longer carries
// a preprocessor branch: a build with no frontend gets a scope that refuses
// everything, and refusal is already a normal answer on that path.
#include "scripting/SceLowering.h"
#endif

namespace SCE {

LoweringScope::LoweringScope() {
#ifdef SCE_HAS_LOWERING_FFI
    scope_ = sce_scope_new();
#endif
}

LoweringScope::~LoweringScope() {
#ifdef SCE_HAS_LOWERING_FFI
    sce_scope_free(scope_);
#endif
}

void LoweringScope::declare(const std::string &name) {
    if (scope_ == nullptr || name.empty()) {
        return;
    }
#ifdef SCE_HAS_LOWERING_FFI
    sce_scope_declare(scope_, name.c_str());
#endif
    ++generation_;
}

void LoweringScope::declareChunk(const std::string &source) {
    if (scope_ == nullptr || source.empty()) {
        return;
    }
#ifdef SCE_HAS_LOWERING_FFI
    sce_scope_declare_chunk(scope_, source.c_str());
#endif
    ++generation_;
}

std::optional<std::string> LoweringScope::lowerValue([[maybe_unused]] const std::string &source) const {
    if (scope_ == nullptr) {
        return std::nullopt;
    }
#ifdef SCE_HAS_LOWERING_FFI
    char *lowered = sce_lower_value(source.c_str(), scope_);
    if (lowered == nullptr) {
        return std::nullopt;
    }
    std::string text(lowered);
    // Freed on the frontend's own allocator before the string leaves, so a
    // caller never holds a pointer it would have to remember to release.
    sce_lower_free(lowered);
    return text;
#else
    return std::nullopt;
#endif
}

}  // namespace SCE
