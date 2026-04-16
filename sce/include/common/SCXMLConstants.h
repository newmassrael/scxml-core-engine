// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

/**
 * @file SCXMLConstants.h
 * @brief W3C SCXML specification constants (Single Source of Truth)
 *
 * Standard URIs and identifiers for W3C SCXML 1.0 compliance.
 * Shared between Interpreter and AOT engines.
 */

namespace SCE::Constants {

// ============================================================================
// W3C SCXML Event Processor URIs (W3C SCXML C.1, C.2)
// ============================================================================

/**
 * @brief W3C SCXML C.1: SCXML Event I/O Processor type URL
 *
 * Canonical URL for the SCXML Event I/O Processor.
 * Used for _event.origintype in parent-child communication,
 * invoke event routing, and done.invoke events.
 *
 * @see W3C SCXML 1.0 Appendix C.1
 */
constexpr const char *SCXML_EVENT_PROCESSOR_TYPE = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

/**
 * @brief W3C SCXML C.2: Basic HTTP Event Processor URI
 *
 * Event processor for HTTP-based external communication.
 * Requires target attribute for destination URL.
 *
 * @see W3C SCXML 1.0 Appendix C.2
 */
constexpr const char *BASIC_HTTP_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";

// ============================================================================
// W3C SCXML Invoke Processor URIs (W3C SCXML 6.4)
// ============================================================================

/**
 * @brief W3C SCXML 6.4: Standard SCXML Invoke Processor URI
 *
 * Default invoke processor for SCXML sub-documents.
 * Platforms MUST support this type for invoke elements.
 *
 * @see W3C SCXML 1.0 Section 6.4
 */
constexpr const char *SCXML_INVOKE_PROCESSOR_URI = "http://www.w3.org/TR/scxml/";

}  // namespace SCE::Constants
