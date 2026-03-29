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

#include "parsing/IXMLElement.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Helper for XML content serialization across platforms
 *
 * W3C SCXML B.2: Provides platform-agnostic XML content extraction
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
     * @brief Escape string for JavaScript literal
     * @param content Raw content string
     * @return JavaScript-escaped string with quotes
     *
     * W3C SCXML test 530: Serialize XML content as string literal for assign elements
     */
    static std::string escapeForJavaScript(const std::string &content);
};

}  // namespace SCE
