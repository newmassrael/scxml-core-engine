#include "scripting/XMLDOMWrapper.h"
#include "common/Logger.h"
#include <cstring>

namespace RSM {

#ifndef __EMSCRIPTEN__
// ============================================================================
// Native builds: libxml2-based implementation
// ============================================================================

// XMLElement implementation

XMLElement::XMLElement(xmlNodePtr node) : node_(node) {}

std::string XMLElement::getTagName() const {
    if (node_ && node_->name) {
        return std::string(reinterpret_cast<const char *>(node_->name));
    }
    return "";
}

std::string XMLElement::getAttribute(const std::string &attrName) {
    if (!node_) {
        return "";
    }

    xmlChar *value = xmlGetProp(node_, reinterpret_cast<const xmlChar *>(attrName.c_str()));
    if (value) {
        std::string result(reinterpret_cast<const char *>(value));
        xmlFree(value);
        return result;
    }

    return "";
}

void XMLElement::findElementsByTagNameStatic(xmlNodePtr node, const std::string &tagName,
                                             std::vector<std::shared_ptr<XMLElement>> &result) {
    if (!node) {
        return;
    }

    // Check current node
    if (node->type == XML_ELEMENT_NODE) {
        const char *nodeName = reinterpret_cast<const char *>(node->name);
        if (nodeName && tagName == nodeName) {
            result.push_back(std::make_shared<XMLElement>(node));
        }
    }

    // Recursively check children
    for (xmlNodePtr child = node->children; child != nullptr; child = child->next) {
        findElementsByTagNameStatic(child, tagName, result);
    }
}

std::vector<std::shared_ptr<XMLElement>> XMLElement::getElementsByTagName(const std::string &tagName) {
    std::vector<std::shared_ptr<XMLElement>> result;

    // Search starting from this element's children
    for (xmlNodePtr child = node_->children; child != nullptr; child = child->next) {
        findElementsByTagNameStatic(child, tagName, result);
    }

    return result;
}

// XMLDocument implementation

XMLDocument::XMLDocument(const std::string &xmlContent) : doc_(nullptr) {
    // W3C SCXML B.2: Parse XML string into DOM structure
    doc_ = xmlReadMemory(xmlContent.c_str(), xmlContent.length(), nullptr, nullptr,
                         XML_PARSE_NOWARNING | XML_PARSE_NOERROR);

    if (!doc_) {
        errorMessage_ = "Failed to parse XML content";
        LOG_ERROR("XMLDocument: {}", errorMessage_);
    }
}

XMLDocument::~XMLDocument() {
    if (doc_) {
        xmlFreeDoc(doc_);
    }
}

std::shared_ptr<XMLElement> XMLDocument::getDocumentElement() {
    if (!doc_) {
        return nullptr;
    }

    xmlNodePtr root = xmlDocGetRootElement(doc_);
    if (!root) {
        return nullptr;
    }

    return std::make_shared<XMLElement>(root);
}

std::vector<std::shared_ptr<XMLElement>> XMLDocument::getElementsByTagName(const std::string &tagName) {
    std::vector<std::shared_ptr<XMLElement>> result;

    if (!doc_) {
        return result;
    }

    xmlNodePtr root = xmlDocGetRootElement(doc_);
    if (!root) {
        return result;
    }

    // Search recursively starting from root
    XMLElement::findElementsByTagNameStatic(root, tagName, result);

    return result;
}

#else
// ============================================================================
// WASM builds: pugixml-based implementation
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

#endif

}  // namespace RSM
