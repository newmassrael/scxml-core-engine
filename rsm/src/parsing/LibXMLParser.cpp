#ifndef __EMSCRIPTEN__

#include "parsing/LibXMLParser.h"
#include "common/Logger.h"
#include <algorithm>
#include <filesystem>
#include <libxml++/nodes/textnode.h>
#include <libxml/tree.h>

namespace RSM {

// ============================================================================
// LibXMLElement implementation
// ============================================================================

LibXMLElement::LibXMLElement(xmlpp::Element *element) : element_(element) {}

std::string LibXMLElement::getName() const {
    if (!element_) {
        return "";
    }
    return element_->get_name();
}

std::string LibXMLElement::getAttribute(const std::string &name) const {
    if (!element_) {
        return "";
    }

    auto attr = element_->get_attribute(name);
    if (attr) {
        return attr->get_value();
    }
    return "";
}

bool LibXMLElement::hasAttribute(const std::string &name) const {
    if (!element_) {
        return false;
    }
    return element_->get_attribute(name) != nullptr;
}

std::unordered_map<std::string, std::string> LibXMLElement::getAttributes() const {
    std::unordered_map<std::string, std::string> result;

    if (!element_) {
        return result;
    }

    auto attributes = element_->get_attributes();
    for (auto *attr : attributes) {
        result[attr->get_name()] = attr->get_value();
    }

    return result;
}

std::string LibXMLElement::getNamespace() const {
    if (!element_) {
        return "";
    }

    auto ns = element_->get_namespace_uri();
    return ns.empty() ? "" : ns;
}

std::vector<std::shared_ptr<IXMLElement>> LibXMLElement::getChildren() const {
    std::vector<std::shared_ptr<IXMLElement>> result;

    if (!element_) {
        return result;
    }

    auto children = element_->get_children();
    for (auto *child : children) {
        auto *childElement = dynamic_cast<xmlpp::Element *>(child);
        if (childElement) {
            result.push_back(std::make_shared<LibXMLElement>(childElement));
        }
    }

    return result;
}

std::vector<std::shared_ptr<IXMLElement>> LibXMLElement::getChildrenByTagName(const std::string &tagName) const {
    std::vector<std::shared_ptr<IXMLElement>> result;

    if (!element_) {
        return result;
    }

    auto children = element_->get_children(tagName);
    for (auto *child : children) {
        auto *childElement = dynamic_cast<xmlpp::Element *>(child);
        if (childElement) {
            result.push_back(std::make_shared<LibXMLElement>(childElement));
        }
    }

    return result;
}

std::string LibXMLElement::getTextContent() const {
    if (!element_) {
        return "";
    }

    auto textNode = element_->get_first_child_text();
    if (textNode) {
        return textNode->get_content();
    }

    return "";
}

bool LibXMLElement::importNode(const std::shared_ptr<IXMLElement> &source) {
    if (!element_ || !source) {
        return false;
    }

    auto *libxmlSource = dynamic_cast<LibXMLElement *>(source.get());
    if (!libxmlSource) {
        LOG_ERROR("LibXMLElement::importNode - Source is not LibXMLElement");
        return false;
    }

    try {
        element_->import_node(libxmlSource->getRawElement());
        return true;
    } catch (const std::exception &ex) {
        LOG_ERROR("LibXMLElement::importNode - {}", ex.what());
        return false;
    }
}

bool LibXMLElement::remove() {
    if (!element_) {
        return false;
    }

    try {
        xmlpp::Node::remove_node(element_);
        element_ = nullptr;
        return true;
    } catch (const std::exception &ex) {
        LOG_ERROR("LibXMLElement::remove - {}", ex.what());
        return false;
    }
}

std::shared_ptr<IXMLElement> LibXMLElement::getParent() const {
    if (!element_) {
        return nullptr;
    }

    auto parent = element_->get_parent();
    auto *parentElement = dynamic_cast<xmlpp::Element *>(parent);
    if (parentElement) {
        return std::make_shared<LibXMLElement>(parentElement);
    }

    return nullptr;
}

std::string LibXMLElement::serializeChildContent() const {
    if (!element_) {
        return "";
    }

    // W3C SCXML B.2: Full XML serialization preserving structure
    std::string xmlContent;
    auto children = element_->get_children();

    for (auto child : children) {
        // Serialize element nodes
        if (auto *childElement = dynamic_cast<xmlpp::Element *>(child)) {
            xmlNodePtr node = const_cast<xmlNodePtr>(childElement->cobj());
            xmlBufferPtr buf = xmlBufferCreate();
            if (buf) {
                xmlNodeDump(buf, node->doc, node, 0, 0);
                xmlContent += reinterpret_cast<const char *>(xmlBufferContent(buf));
                xmlBufferFree(buf);
            }
        }
        // Include text nodes
        else if (auto *textNode = dynamic_cast<xmlpp::TextNode *>(child)) {
            std::string text = textNode->get_content();
            // Only add non-whitespace text
            if (!std::all_of(text.begin(), text.end(), ::isspace)) {
                xmlContent += text;
            }
        }
    }

    return xmlContent;
}

// ============================================================================
// LibXMLDocument implementation
// ============================================================================

LibXMLDocument::LibXMLDocument(std::shared_ptr<xmlpp::DomParser> parser) : parser_(parser) {}

std::shared_ptr<IXMLElement> LibXMLDocument::getRootElement() {
    if (!parser_) {
        return nullptr;
    }

    auto *doc = parser_->get_document();
    if (!doc) {
        return nullptr;
    }

    auto *root = doc->get_root_node();
    if (!root) {
        return nullptr;
    }

    return std::make_shared<LibXMLElement>(root);
}

bool LibXMLDocument::processXInclude() {
    if (!parser_) {
        errorMessage_ = "Parser is null";
        return false;
    }

    auto *doc = parser_->get_document();
    if (!doc) {
        errorMessage_ = "Document is null";
        return false;
    }

    try {
        // W3C XInclude: Use libxml2's native XInclude processor
        doc->process_xinclude();
        LOG_DEBUG("LibXMLDocument: XInclude processing successful");
        return true;
    } catch (const std::exception &ex) {
        errorMessage_ = "XInclude processing failed: " + std::string(ex.what());
        LOG_WARN("LibXMLDocument: {}", errorMessage_);
        return false;
    }
}

std::string LibXMLDocument::getErrorMessage() const {
    return errorMessage_;
}

bool LibXMLDocument::isValid() const {
    if (!parser_) {
        return false;
    }
    auto *doc = parser_->get_document();
    return doc != nullptr;
}

// ============================================================================
// LibXMLParser implementation
// ============================================================================

std::shared_ptr<IXMLDocument> LibXMLParser::parseFile(const std::string &filename) {
    try {
        // Check if file exists
        if (!std::filesystem::exists(filename)) {
            lastError_ = "File not found: " + filename;
            LOG_ERROR("LibXMLParser: {}", lastError_);
            return nullptr;
        }

        LOG_INFO("LibXMLParser: Parsing file: {}", filename);

        // Parse file using libxml++
        auto parser = std::make_shared<xmlpp::DomParser>();
        parser->set_validate(false);
        parser->set_substitute_entities(true);  // Enable entity substitution
        parser->parse_file(filename);

        // Wrap parser in document wrapper (keeps parser alive)
        return std::make_shared<LibXMLDocument>(parser);

    } catch (const std::exception &ex) {
        lastError_ = "Exception while parsing file: " + std::string(ex.what());
        LOG_ERROR("LibXMLParser: {}", lastError_);
        return nullptr;
    }
}

std::shared_ptr<IXMLDocument> LibXMLParser::parseContent(const std::string &content) {
    try {
        LOG_INFO("LibXMLParser: Parsing content");

        // Parse from string using libxml++
        auto parser = std::make_shared<xmlpp::DomParser>();
        parser->set_validate(false);
        parser->set_substitute_entities(true);
        parser->set_throw_messages(true);  // Enable XML namespace recognition
        parser->parse_memory(content);

        // Wrap parser in document wrapper (keeps parser alive)
        return std::make_shared<LibXMLDocument>(parser);

    } catch (const std::exception &ex) {
        lastError_ = "Exception while parsing content: " + std::string(ex.what());
        LOG_ERROR("LibXMLParser: {}", lastError_);
        return nullptr;
    }
}

std::string LibXMLParser::getLastError() const {
    return lastError_;
}

}  // namespace RSM

#endif  // !__EMSCRIPTEN__
