// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/XMLDOMWrapper.h"
#include "core/LogMacros.h"
#include <cstring>

namespace SCE {

// ============================================================================
// Unified implementation: pugixml-based for all platforms
// ============================================================================

// XMLElement implementation

XMLElement::XMLElement(pugi::xml_node node) : node_(node) {}

std::string XMLElement::getTagName() const {
    if (node_) {
        return std::string(node_.name());
    }
    return "";
}

std::string XMLElement::getAttribute(const std::string &attrName) {
    if (!node_) {
        return "";
    }

    pugi::xml_attribute attr = node_.attribute(attrName.c_str());
    if (attr) {
        return std::string(attr.value());
    }

    return "";
}

bool XMLElement::hasAttribute(const std::string &attrName) const {
    if (!node_) {
        return false;
    }
    return static_cast<bool>(node_.attribute(attrName.c_str()));
}

// === DOM Level 1 Core: the Node interface's read surface ===
//
// pugixml keeps one handle type for every node kind, so this class does
// too and getNodeType() is what separates them. The trees hold four
// kinds (see DomNodeType): parse_default drops comments and PIs.

int XMLElement::getNodeType() const {
    switch (node_.type()) {
    case pugi::node_pcdata:
        return DomNodeType::Text;
    case pugi::node_cdata:
        return DomNodeType::CdataSection;
    case pugi::node_document:
        return DomNodeType::Document;
    default:
        return DomNodeType::Element;
    }
}

std::string XMLElement::getNodeName() const {
    switch (getNodeType()) {
    case DomNodeType::Text:
        return "#text";
    case DomNodeType::CdataSection:
        return "#cdata-section";
    case DomNodeType::Document:
        return "#document";
    default:
        return getTagName();
    }
}

bool XMLElement::hasNodeValue() const {
    const int type = getNodeType();
    return type == DomNodeType::Text || type == DomNodeType::CdataSection;
}

std::string XMLElement::getNodeValue() const {
    if (!hasNodeValue()) {
        return "";
    }
    return std::string(node_.value());
}

std::string XMLElement::getTextContent() const {
    if (hasNodeValue()) {
        return std::string(node_.value());
    }
    std::string text;
    for (const auto &child : node_.children()) {
        text += XMLElement(child).getTextContent();
    }
    return text;
}

bool XMLElement::hasChildNodes() const {
    return static_cast<bool>(node_.first_child());
}

std::vector<std::shared_ptr<XMLElement>> XMLElement::getChildNodes() const {
    std::vector<std::shared_ptr<XMLElement>> children;
    for (const auto &child : node_.children()) {
        children.push_back(std::make_shared<XMLElement>(child));
    }
    return children;
}

std::shared_ptr<XMLElement> XMLElement::getFirstChild() const {
    pugi::xml_node child = node_.first_child();
    return child ? std::make_shared<XMLElement>(child) : nullptr;
}

std::shared_ptr<XMLElement> XMLElement::getLastChild() const {
    pugi::xml_node child = node_.last_child();
    return child ? std::make_shared<XMLElement>(child) : nullptr;
}

std::shared_ptr<XMLElement> XMLElement::getNextSibling() const {
    pugi::xml_node sibling = node_.next_sibling();
    return sibling ? std::make_shared<XMLElement>(sibling) : nullptr;
}

std::shared_ptr<XMLElement> XMLElement::getPreviousSibling() const {
    pugi::xml_node sibling = node_.previous_sibling();
    return sibling ? std::make_shared<XMLElement>(sibling) : nullptr;
}

std::shared_ptr<XMLElement> XMLElement::getParentNode() const {
    pugi::xml_node parent = node_.parent();
    return parent ? std::make_shared<XMLElement>(parent) : nullptr;
}

void XMLElement::findElementsByTagNameStatic(pugi::xml_node node, const std::string &tagName,
                                             std::vector<std::shared_ptr<XMLElement>> &result) {
    if (!node) {
        return;
    }

    // Check current node
    if (node.type() == pugi::node_element) {
        const char *nodeName = node.name();
        if (nodeName && tagName == nodeName) {
            result.push_back(std::make_shared<XMLElement>(node));
        }
    }

    // Recursively check children
    for (const auto &child : node.children()) {
        findElementsByTagNameStatic(child, tagName, result);
    }
}

std::vector<std::shared_ptr<XMLElement>> XMLElement::getElementsByTagName(const std::string &tagName) {
    std::vector<std::shared_ptr<XMLElement>> result;

    // Search starting from this element's children
    for (const auto &child : node_.children()) {
        findElementsByTagNameStatic(child, tagName, result);
    }

    return result;
}

// XMLDocument implementation

XMLDocument::XMLDocument(const std::string &xmlContent) {
    // §scxml-B-2: Parse XML string into DOM structure
    pugi::xml_parse_result parseResult = doc_.load_string(xmlContent.c_str());
    parsed_ = static_cast<bool>(parseResult);

    if (!parsed_) {
        errorMessage_ = "Failed to parse XML content: ";
        errorMessage_ += parseResult.description();
        // Debug rather than error: whether a refusal is a failure is the
        // caller's question. `<data>` content was read by the SCXML parser
        // before it reached here, so a refusal there is an invariant breaking
        // and that caller says so; an arriving `_event.data` that merely opens
        // with `<` has a reading below this one (§scxml-B-2-8-1) and is
        // perfectly ordinary. Logging it as an error here reported the ordinary
        // case as a fault on every platform-error event the engine raises.
        SCE_LOG_DEBUG("XMLDocument: {}", errorMessage_);
    }
}

XMLDocument::~XMLDocument() {
    // pugi::xml_document manages memory automatically
}

std::shared_ptr<XMLElement> XMLDocument::getDocumentElement() {
    if (doc_.empty()) {
        return nullptr;
    }

    pugi::xml_node root = doc_.document_element();
    if (!root) {
        return nullptr;
    }

    return std::make_shared<XMLElement>(root);
}

std::vector<std::shared_ptr<XMLElement>> XMLDocument::getElementsByTagName(const std::string &tagName) {
    std::vector<std::shared_ptr<XMLElement>> result;

    if (doc_.empty()) {
        return result;
    }

    pugi::xml_node root = doc_.document_element();
    if (!root) {
        return result;
    }

    // Search recursively starting from root
    XMLElement::findElementsByTagNameStatic(root, tagName, result);

    return result;
}

}  // namespace SCE
