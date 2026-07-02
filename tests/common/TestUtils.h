// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <chrono>
#include <cstdlib>
#include <string>

namespace SCE {
namespace Test {
namespace Utils {

// Common Test Timing Constants
constexpr auto POLL_INTERVAL_MS = std::chrono::milliseconds(10);   // Polling interval for state checks
constexpr auto STANDARD_WAIT_MS = std::chrono::milliseconds(100);  // Standard wait time for async operations
constexpr auto LONG_WAIT_MS = std::chrono::milliseconds(200);      // Long wait time for complex operations

/**
 * @brief Check if this is a ThreadSanitizer build
 *
 * Compile-time detection of `-fsanitize=thread` (GCC `__SANITIZE_THREAD__`,
 * Clang `__has_feature(thread_sanitizer)`). Used to skip HTTP tests, whose
 * cpp-httplib/SimpleMockHttpServer thread creation is incompatible with TSAN,
 * and to widen timing delays for TSAN's instrumentation overhead. TSAN is a
 * host-native build (`cmake -DENABLE_TSAN=ON`); no container is required.
 *
 * @return true when compiled with ThreadSanitizer, false otherwise
 */
inline bool isThreadSanitizerBuild() {
#if defined(__SANITIZE_THREAD__)
    return true;
#elif defined(__has_feature)
#if __has_feature(thread_sanitizer)
    return true;
#else
    return false;
#endif
#else
    return false;
#endif
}

/**
 * @brief Get base delay for timing-sensitive tests
 *
 * Returns a base delay value (in milliseconds) that accounts for TSAN overhead.
 * In TSAN builds, scheduling and synchronization operations are slower, so
 * tests need longer delays to avoid flaky behavior.
 *
 * @param normalDelay Delay to use in normal (non-TSAN) builds
 * @return Delay value adjusted for a ThreadSanitizer build if applicable
 */
inline int getBaseDelay(int normalDelay = 50) {
    // TSAN builds need 4x longer delays due to instrumentation overhead
    return isThreadSanitizerBuild() ? (normalDelay * 4) : normalDelay;
}

}  // namespace Utils
}  // namespace Test
}  // namespace SCE
