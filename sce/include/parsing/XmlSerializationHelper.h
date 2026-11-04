// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "parsing/IXMLElement.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Helper for XML content serialization across platforms
 *
 * §scxml-B-2: Provides platform-agnostic XML content extraction
 * for assign elements, data elements, and send content elements.
 *
 * ARCHITECTURE.md Zero Duplication: Single Source of Truth for XML serialization
 */
class XmlSerializationHelper {
public:
    /**
     * @brief Serialize XML element content to string
     * @param element XML element (IXMLElement interface)
     * @return Serialized content string
     *
     * W3C SCXML: For WASM builds, uses text content extraction
     * Note: Cannot fully serialize XML structure on WASM (pugixml limitation)
     */
    static std::string serializeContent(const std::shared_ptr<IXMLElement> &element);

    /**
     * @brief The body of an inline `<content>`, of a `<send>` or of an `<invoke>`
     * @param content The `<content>` element
     * @return Its child markup, serialized, when it holds elements, and its
     *         character data otherwise (text and CDATA, unescaped) — trimmed
     *         either way
     *
     * One reading for every inline `<content>`, and the one sce-build's
     * `content_body` gives the generated machines, so both engines hand a
     * host processor and a local delivery the same text for one element. The
     * whitespace that lays the element out in its document is not part of
     * its content: `<content>` written over three lines holds the text of
     * its middle line.
     */
    static std::string serializeContentBody(const std::shared_ptr<IXMLElement> &content);

    /**
     * @brief Escape string for JavaScript literal
     * @param content Raw content string
     * @return JavaScript-escaped string with quotes
     *
     * W3C SCXML test 530: Serialize XML content as string literal for assign elements
     */
    static std::string escapeForJavaScript(const std::string &content);
};

}  // namespace SCE
