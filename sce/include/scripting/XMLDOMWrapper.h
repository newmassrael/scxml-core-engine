// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

// All platforms: pugixml
#include <pugixml.hpp>

#include <memory>
#include <string>
#include <vector>

namespace SCE {

// Forward declarations
class XMLElement;
class XMLDocument;

/// DOM Level 1 Core node types — the numbers `getNodeType()` reports.
///
/// Four of the twelve, because four is what these trees hold: comments
/// and processing instructions never become nodes (pugixml's
/// `parse_default` omits `parse_comments` and `parse_pi`) and the rest —
/// attributes as nodes, entities, fragments — belong to interfaces this
/// surface does not carry. The four are shared with the six mirrors so
/// `nodeType` means the same number on every backend.
namespace DomNodeType {
constexpr int Element = 1;
constexpr int Text = 3;
constexpr int CdataSection = 4;
constexpr int Document = 9;
}  // namespace DomNodeType

/**
 * §scxml-B-2: XML DOM wrapper for XML integration
 * Provides JavaScript-accessible DOM API for XML content
 *
 * Unified implementation: pugixml-based for all platforms
 */
class XMLElement {
public:
    explicit XMLElement(pugi::xml_node node);
    ~XMLElement() = default;

    // DOM API methods
    std::vector<std::shared_ptr<XMLElement>> getElementsByTagName(const std::string &tagName);
    std::string getAttribute(const std::string &attrName);
    /// DOM Level 2 Core: tells an absent attribute from one present and
    /// empty, which getAttribute's "" cannot.
    bool hasAttribute(const std::string &attrName) const;
    std::string getTagName() const;

    // The ECMAScript data model appendix obliges the Processor to create
    // "the corresponding DOM structure", so a handle carries DOM Level 1
    // Core's Node interface and not only the calls the W3C IRP suite
    // happens to read (measured 2026-08-18: two of them). One handle type
    // answers for every node kind, mirroring pugixml's own single
    // `xml_node`; getNodeType() is how a document tells the kinds apart.
    int getNodeType() const;
    std::string getNodeName() const;
    /// True when this node kind has a nodeValue at all — character data
    /// does, an element does not (DOM Level 1 Core gives it null).
    bool hasNodeValue() const;
    std::string getNodeValue() const;
    /// Every descendant character-data node's content, concatenated in
    /// document order (DOM Level 3 Core `textContent`).
    std::string getTextContent() const;
    bool hasChildNodes() const;
    std::vector<std::shared_ptr<XMLElement>> getChildNodes() const;
    std::shared_ptr<XMLElement> getFirstChild() const;
    std::shared_ptr<XMLElement> getLastChild() const;
    std::shared_ptr<XMLElement> getNextSibling() const;
    std::shared_ptr<XMLElement> getPreviousSibling() const;
    std::shared_ptr<XMLElement> getParentNode() const;

    // Internal access
    pugi::xml_node getNode() const {
        return node_;
    }

    pugi::xml_node node_;

public:
    static void findElementsByTagNameStatic(pugi::xml_node node, const std::string &tagName,
                                            std::vector<std::shared_ptr<XMLElement>> &result);

private:
    void findElementsByTagName(pugi::xml_node node, const std::string &tagName,
                               std::vector<std::shared_ptr<XMLElement>> &result);
};

/**
 * §scxml-B-2: XML Document wrapper
 * Root object for XML DOM tree
 */
class XMLDocument {
public:
    explicit XMLDocument(const std::string &xmlContent);
    ~XMLDocument();

    // DOM API methods
    std::vector<std::shared_ptr<XMLElement>> getElementsByTagName(const std::string &tagName);
    std::shared_ptr<XMLElement> getDocumentElement();

    bool isValid() const {
        return !doc_.empty();
    }

    std::string getErrorMessage() const {
        return errorMessage_;
    }

private:
    pugi::xml_document doc_;
    std::string errorMessage_;
};

}  // namespace SCE
