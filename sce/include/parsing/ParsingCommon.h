// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "parsing/IXMLElement.h"
#include <string>
#include <unordered_map>
#include <vector>

/**
 * @brief Common utility class for parsing
 *
 * This class provides functions and constants commonly used
 * across multiple parser classes. Includes utilities for
 * namespace handling, attribute validation, path processing, etc.
 */

namespace SCE {

class ParsingCommon {
public:
    /**
     * @brief SCXML-related constants
     */
    struct Constants {
        static const std::string SCXML_NAMESPACE;
        static const std::string CODE_NAMESPACE;
        static const std::string CTX_NAMESPACE;
        static const std::string DI_NAMESPACE;
    };

    /**
     * @brief Compare node names considering namespace
     * @param nodeName Node name to check
     * @param baseName Base name (without namespace)
     * @return Whether they match
     */
    static bool matchNodeName(const std::string &nodeName, const std::string &baseName);

    /**
     * @brief True when `element`'s namespace URI is the W3C SCXML
     *        namespace, or empty (legacy/test fixtures that omit the
     *        `xmlns="http://www.w3.org/2005/07/scxml"` declaration).
     *
     * Lenient on empty namespace mirrors the AOT-side parser
     * (`sce-build/src/parser.rs::is_scxml_ns`) so both engines apply
     * identical foreign-namespace policy per `SCE_FORGE.md` §3.1.
     * Prefixed foreign-NS elements like `<framework:onentry>` are
     * correctly rejected because their resolved namespace URI is
     * `http://example.com/framework`, not the SCXML URI.
     */
    static bool isScxmlNamespace(const std::shared_ptr<IXMLElement> &element);

    /**
     * @brief Find child elements with matching local name AND the W3C
     *        SCXML namespace (`isScxmlNamespace`). Filters out foreign-
     *        namespace elements whose local name collides with a W3C
     *        element (e.g. `<framework:onentry>`).
     * @param element Parent element
     * @param childName Child local name to find
     * @return Found child elements
     */
    static std::vector<std::shared_ptr<IXMLElement>> findChildElements(const std::shared_ptr<IXMLElement> &element,
                                                                       const std::string &childName);

    /**
     * @brief Find first child element with matching local name AND the
     *        W3C SCXML namespace. See `findChildElements` for the
     *        namespace-filter contract.
     * @param element Parent element
     * @param childName Child local name to find
     * @return Found child element, nullptr if not found
     */
    static std::shared_ptr<IXMLElement> findFirstChildElement(const std::shared_ptr<IXMLElement> &element,
                                                              const std::string &childName);

    /**
     * @brief Find ID attribute in element or parent element
     * @param element Element to check
     * @return Found ID, empty string if not found
     */
    static std::string findElementId(const std::shared_ptr<IXMLElement> &element);

    /**
     * @brief Get attribute value (try multiple names)
     * @param element Element with attributes
     * @param attrNames List of attribute names to try
     * @return Attribute value, empty string if not found
     */
    static std::string getAttributeValue(const std::shared_ptr<IXMLElement> &element,
                                         const std::vector<std::string> &attrNames);

    /**
     * @brief Collect all attributes into map
     * @param element Element with attributes
     * @param excludeAttrs List of attribute names to exclude
     * @return Attribute map (name -> value)
     */
    static std::unordered_map<std::string, std::string>
    collectAttributes(const std::shared_ptr<IXMLElement> &element, const std::vector<std::string> &excludeAttrs = {});

    /**
     * @brief Extract text content from element
     * @param element Element containing text
     * @param trimWhitespace Whether to trim whitespace
     * @return Extracted text
     */
    static std::string extractTextContent(const std::shared_ptr<IXMLElement> &element, bool trimWhitespace = true);

    /**
     * @brief Extract XML element name (remove namespace)
     * @param element XML element
     * @return Element name without namespace
     */
    static std::string getLocalName(const std::shared_ptr<IXMLElement> &element);

    /**
     * @brief Find child elements with specified name and namespace
     * @param parent Parent element
     * @param elementName Element name to find
     * @param namespaceURI Namespace URI
     * @return Found child elements
     */
    static std::vector<std::shared_ptr<IXMLElement>>
    findChildElementsWithNamespace(const std::shared_ptr<IXMLElement> &parent, const std::string &elementName,
                                   const std::string &namespaceURI);

    /**
     * @brief Convert relative path to absolute path
     * @param basePath Base path
     * @param relativePath Relative path
     * @return Absolute path
     */
    static std::string resolveRelativePath(const std::string &basePath, const std::string &relativePath);

    static std::string trimString(const std::string &str);

private:
    // Prevent instance creation
    ParsingCommon() = delete;
};

}  // namespace SCE