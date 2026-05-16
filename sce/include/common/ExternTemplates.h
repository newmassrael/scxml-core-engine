// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Force-included into every sce_base translation unit via
// target_compile_options(... -include common/ExternTemplates.h). Suppresses
// implicit instantiation of std::basic_string<char> members in sce_base .o
// files. libstdc++.so ships the explicit instantiation, so consumers'
// std::string references resolve from the DSO rather than dragging
// sce_base archive members (and their static initializers) into the
// consumer binary. This is what keeps tc8-harness-style AOT consumers
// that do not actually call SCE symbols from inheriting ~1 MB of
// Logger/SpdlogBackend/JsonUtils bloat via weak-template archive drag.

#pragma once

#include <string>

// libstdc++ places basic_string<char> in the inline `__cxx11` ABI namespace,
// and libstdc++.so is what ships the matching explicit instantiation we rely
// on here. Other standard libraries (libc++ on Clang/Emscripten, MSVC STL)
// don't have that namespace, so the optimization is a no-op there.
#if defined(__GLIBCXX__)
extern template class std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char>>;
#endif
