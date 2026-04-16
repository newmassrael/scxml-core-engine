// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

// Include standard iostream (for other necessary features)
#include <iostream>

// Prevent conflicts with system headers (GoogleTest, OpenSSL, etc.)
// Apply cout/cerr prohibition macros only in SCE source files
#ifndef GTEST_INCLUDE_GTEST_GTEST_H_  // Exclude GoogleTest headers
#ifndef _OPENSSL_BIO_H                // Exclude OpenSSL BIO headers
#if !defined(_STDIO_H) && !defined(_STDIO_H_INCLUDED) && !defined(__STDIO_H_INCLUDED)  // Exclude standard C stdio headers

// Completely prohibit std::cout, std::cerr, std::clog at compile time
#define cout                                                                                                           \
    static_assert(false, "std::cout is forbidden in SCE codebase. "                                                    \
                         "Use SCE_LOG_INFO(...) instead. "                                                             \
                         "Example: SCE_LOG_INFO(\"Value: {}\", value);")

#define cerr                                                                                                           \
    static_assert(false, "std::cerr is forbidden in SCE codebase. "                                                    \
                         "Use SCE_LOG_ERROR(...) instead. "                                                            \
                         "Example: SCE_LOG_ERROR(\"Error: {}\", errorMsg);")

#define clog                                                                                                           \
    static_assert(false, "std::clog is forbidden in SCE codebase. "                                                    \
                         "Use SCE_LOG_DEBUG(...) instead. "                                                            \
                         "Example: SCE_LOG_DEBUG(\"Debug info: {}\", debugData);")

#endif  // _STDIO_H / _STDIO_H_INCLUDED / __STDIO_H_INCLUDED
#endif  // _OPENSSL_BIO_H
#endif  // GTEST_INCLUDE_GTEST_GTEST_H_
