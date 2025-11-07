// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <string>

namespace SCE {

/**
 * @brief Helper for RFC 3986 URL encoding (W3C SCXML C.2 BasicHTTP)
 *
 * ARCHITECTURE.md: Zero Duplication - Single Source of Truth for URL encoding logic.
 * Used by both Interpreter and AOT engines for HTTP event transmission.
 *
 * Usage:
 * - Interpreter: HttpEventTarget for BasicHTTP Event I/O Processor (rsm/src/events/HttpEventTarget.cpp)
 * - AOT: Same HttpEventTarget shared infrastructure (no duplication)
 *
 * W3C SCXML C.2: BasicHTTP Event I/O Processor requires application/x-www-form-urlencoded
 * format for event transmission. This helper implements RFC 3986 percent-encoding.
 *
 * ARCHITECTURE.md Zero Duplication Pattern:
 * - Single implementation in UrlEncodingHelper
 * - Shared across all HTTP event processing
 * - No engine-specific duplicate code
 */
class UrlEncodingHelper {
public:
    /**
     * @brief Percent-encode string for application/x-www-form-urlencoded
     *
     * W3C SCXML C.2: Form data encoding for BasicHTTP Event I/O Processor.
     * RFC 3986: Unreserved characters (A-Za-z0-9-._~) are not encoded.
     *
     * All other characters are percent-encoded as %XX where XX is the hexadecimal
     * representation of the character's byte value.
     *
     * @param str String to encode
     * @return Percent-encoded string safe for URL transmission
     *
     * Example:
     *   urlEncode("hello world") → "hello%20world"
     *   urlEncode("test@example.com") → "test%40example.com"
     *   urlEncode("param1") → "param1" (no encoding needed)
     */
    static std::string urlEncode(const std::string &str);
};

}  // namespace SCE
