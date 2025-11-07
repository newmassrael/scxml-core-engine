#include "scripting/XMLDOMWrapper.h"
#include "common/Logger.h"
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
    for (pugi::xml_node child : node.children()) {
        findElementsByTagNameStatic(child, tagName, result);
    }
}

std::vector<std::shared_ptr<XMLElement>> XMLElement::getElementsByTagName(const std::string &tagName) {
    std::vector<std::shared_ptr<XMLElement>> result;

    // Search starting from this element's children
    for (pugi::xml_node child : node_.children()) {
        findElementsByTagNameStatic(child, tagName, result);
    }

    return result;
}

// XMLDocument implementation

XMLDocument::XMLDocument(const std::string &xmlContent) {
    // W3C SCXML B.2: Parse XML string into DOM structure
    pugi::xml_parse_result parseResult = doc_.load_string(xmlContent.c_str());

    if (!parseResult) {
        errorMessage_ = "Failed to parse XML content: ";
        errorMessage_ += parseResult.description();
        LOG_ERROR("XMLDocument: {}", errorMessage_);
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
