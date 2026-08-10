// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <string>

namespace SCE {

/**
 * @brief Helper for RFC 3986 URL encoding (§scxml-C-2 BasicHTTP)
 *
 * ARCHITECTURE.md: Zero Duplication - Single Source of Truth for URL encoding logic.
 * Used by both Interpreter and AOT engines for HTTP event transmission.
 *
 * Usage:
 * - Interpreter: HttpEventTarget for BasicHTTP Event I/O Processor (sce/src/events/HttpEventTarget.cpp)
 * - AOT: Same HttpEventTarget shared infrastructure (no duplication)
 *
 * §scxml-C-2: BasicHTTP Event I/O Processor requires application/x-www-form-urlencoded
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
     * §scxml-C-2: Form data encoding for BasicHTTP Event I/O Processor.
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

    /**
     * @brief Inverse of urlEncode
     *
     * An encoder without a decoder means whatever it produced can be written
     * but never read back. `_ioprocessors` publishes a percent-encoded
     * session id inside the SCXML processor location, and §scxml-C-1 requires
     * that location to work as a <send> target — which needs the id read out
     * of it again.
     *
     * A malformed escape (`%` with fewer than two hex digits after it, or
     * non-hex digits) is passed through literally rather than dropped: the
     * input is an address, and silently deleting part of one turns "no such
     * session" into "a different session".
     *
     * @param str Percent-encoded string
     * @return Decoded string
     *
     * Example:
     *   urlDecode("hello%20world") → "hello world"
     *   urlDecode("param1") → "param1"
     *   urlDecode("50%") → "50%" (trailing % is not an escape)
     */
    static std::string urlDecode(const std::string &str);
};

}  // namespace SCE
