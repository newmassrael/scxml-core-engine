#pragma once

// Include SCE Logger first - provides safe logging alternative
#include "common/Logger.h"

// Include standard iostream (for other necessary features)
#include <iostream>

// Prevent conflicts with system headers (GoogleTest, OpenSSL, etc.)
// Apply cout/cerr prohibition macros only in SCE source files
#ifndef GTEST_INCLUDE_GTEST_GTEST_H_  // Exclude GoogleTest headers
#ifndef _OPENSSL_BIO_H                // Exclude OpenSSL BIO headers
#ifndef _STDIO_H                      // Exclude standard C stdio headers

// Completely prohibit std::cout, std::cerr, std::clog at compile time
#define cout                                                                                                           \
    static_assert(false, "std::cout is forbidden in SCE codebase. "                                                    \
                         "Use LOG_INFO(...) instead. "                                                                 \
                         "Example: LOG_INFO(\"Value: {}\", value);")

#define cerr                                                                                                           \
    static_assert(false, "std::cerr is forbidden in SCE codebase. "                                                    \
                         "Use LOG_ERROR(...) instead. "                                                                \
                         "Example: LOG_ERROR(\"Error: {}\", errorMsg);")

#define clog                                                                                                           \
    static_assert(false, "std::clog is forbidden in SCE codebase. "                                                    \
                         "Use LOG_INFO(...) or LOG_DEBUG(...) instead. "                                               \
                         "Example: LOG_DEBUG(\"Debug info: {}\", debugData);")

#endif  // _STDIO_H
#endif  // _OPENSSL_BIO_H
#endif  // GTEST_INCLUDE_GTEST_GTEST_H_

// Also prohibit printf family (for consistency)
// printf family is disabled due to system header conflicts
// Handle directly in individual source files if needed
#ifdef SCE_STRICT_NO_PRINTF_DISABLED_DUE_TO_SYSTEM_CONFLICTS
#define printf                                                                                                         \
    static_assert(false, "printf is forbidden in SCE codebase. "                                                       \
                         "Use LOG_INFO(...) instead. "                                                                 \
                         "Example: LOG_INFO(\"Value: {}\", value);")

#define fprintf                                                                                                        \
    static_assert(false, "fprintf is forbidden in SCE codebase. "                                                      \
                         "Use LOG_ERROR(...) for stderr or LOG_INFO(...) instead.")
#endif

// puts, putchar are also disabled due to OpenSSL conflicts
#ifdef SCE_STRICT_NO_PUTS_DISABLED_DUE_TO_SYSTEM_CONFLICTS
#define puts                                                                                                           \
    static_assert(false, "puts is forbidden in SCE codebase. "                                                         \
                         "Use LOG_INFO(...) instead.")

#define putchar                                                                                                        \
    static_assert(false, "putchar is forbidden in SCE codebase. "                                                      \
                         "Use LOG_INFO(...) instead.")
#endif

// Provide safe alternative macros
#define SAFE_PRINT(...) LOG_INFO(__VA_ARGS__)
#define SAFE_PRINT_ERROR(...) LOG_ERROR(__VA_ARGS__)
#define SAFE_PRINT_WARN(...) LOG_WARN(__VA_ARGS__)
#define SAFE_PRINT_DEBUG(...) LOG_DEBUG(__VA_ARGS__)

// Helper macros for migrating existing code
#define COUT_TO_LOG_INFO(...) LOG_INFO(__VA_ARGS__)
#define CERR_TO_LOG_ERROR(...) LOG_ERROR(__VA_ARGS__)

// Safe output functions for use within SCE namespace
namespace SCE {
namespace SafeOutput {

// Safe output that only works in debug builds (ignored in release)
template <typename... Args> inline void debugPrint(const std::string &format, Args &&...args) {
#ifndef NDEBUG
    LOG_DEBUG(format, std::forward<Args>(args)...);
#else
    (void)format;       // Prevent compiler warnings
    ((void)args, ...);  // Ignore all args with fold expression
#endif
}

// Conditional output (prints only when condition is true)
template <typename... Args> inline void conditionalPrint(bool condition, const std::string &format, Args &&...args) {
    if (condition) {
        LOG_INFO(format, std::forward<Args>(args)...);
    }
}

// Output by error level
template <typename... Args> inline void errorPrint(const std::string &format, Args &&...args) {
    LOG_ERROR(format, std::forward<Args>(args)...);
}

template <typename... Args> inline void warningPrint(const std::string &format, Args &&...args) {
    LOG_WARN(format, std::forward<Args>(args)...);
}

template <typename... Args> inline void infoPrint(const std::string &format, Args &&...args) {
    LOG_INFO(format, std::forward<Args>(args)...);
}

}  // namespace SafeOutput
}  // namespace SCE

// Provide usage examples in comments
/*
Usage examples:

Existing code:
    std::cout << "Hello " << name << std::endl;
    std::cerr << "Error: " << error << std::endl;

New code:
    LOG_INFO("Hello {}", name);
    LOG_ERROR("Error: {}", error);

Or use namespace functions:
    SCE::SafeOutput::infoPrint("Hello {}", name);
    SCE::SafeOutput::errorPrint("Error: {}", error);
    SCE::SafeOutput::debugPrint("Debug: {}", debugData);  // Only outputs in debug builds
    SCE::SafeOutput::conditionalPrint(verbose, "Verbose: {}", data);  // Conditional output
*/