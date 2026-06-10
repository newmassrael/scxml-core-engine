// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLDocument.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Abstract XML parser interface
 *
 * Platform-agnostic XML parser abstraction for multi-backend support.
 * Implementation: PugiXMLParser (unified for all platforms)
 *
 * ARCHITECTURE.md: Zero Duplication - Single interface for multiple backends
 * Implementation: PugiXMLParser (unified for all platforms)
 * Backend: pugixml (lightweight, W3C compliant, WASM-compatible)
 */
class IXMLParser {
public:
    virtual ~IXMLParser() = default;

    /**
     * @brief Parse XML from file
     * @param filename Absolute path to XML file
     * @return Parsed document on success
     * @throws SCE::parsing::ParseFileNotFound when the file does not exist
     *         or cannot be opened (§wire-W4 D1-C typed-throw refit).
     * @throws SCE::parsing::ParseXmlFailed when the underlying parser
     *         (pugixml) reports a malformed-XML failure.
     *
     * @note Pre-§wire-W4, this method returned `nullptr` on failure
     *       and the caller polled `getLastError()`. The poll contract
     *       is deprecated; consumers should catch
     *       `SCE::parsing::ParseError&` instead. See the W4 closure
     *       memo for the full transition.
     */
    virtual std::shared_ptr<IXMLDocument> parseFile(const std::string &filename) = 0;

    /**
     * @brief Parse XML from string content
     * @param content XML string content
     * @return Parsed document on success
     * @throws SCE::parsing::ParseXmlFailed when the underlying parser
     *         (pugixml) reports a malformed-XML failure.
     */
    virtual std::shared_ptr<IXMLDocument> parseContent(const std::string &content) = 0;

    /**
     * @brief Factory method to create platform-specific parser
     * @return Parser instance (PugiXMLParser on all platforms)
     */
    static std::shared_ptr<IXMLParser> create();
};

}  // namespace SCE
