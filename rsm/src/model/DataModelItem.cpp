#include "DataModelItem.h"
#include "common/Logger.h"
#ifndef __EMSCRIPTEN__
#include <libxml++/parsers/domparser.h>
#endif
#include <sstream>

RSM::DataModelItem::DataModelItem(const std::string &id, const std::string &expr)
    : id_(id), expr_(expr), scope_("global") {
    LOG_DEBUG("Creating data model item: {}", id);
}

RSM::DataModelItem::~DataModelItem() {
    LOG_DEBUG("Destroying data model item: {}", id_);
    // unique_ptr automatically manages memory - no manual delete needed
}

const std::string &RSM::DataModelItem::getId() const {
    return id_;
}

void RSM::DataModelItem::setExpr(const std::string &expr) {
    LOG_DEBUG("Setting expression for {}: {}", id_, expr);
    expr_ = expr;
}

const std::string &RSM::DataModelItem::getExpr() const {
    return expr_;
}

void RSM::DataModelItem::setType(const std::string &type) {
    LOG_DEBUG("Setting type for {}: {}", id_, type);
    type_ = type;
}

const std::string &RSM::DataModelItem::getType() const {
    return type_;
}

void RSM::DataModelItem::setScope(const std::string &scope) {
    LOG_DEBUG("Setting scope for {}: {}", id_, scope);
    scope_ = scope;
}

const std::string &RSM::DataModelItem::getScope() const {
    return scope_;
}

void RSM::DataModelItem::setContent(const std::string &content) {
    LOG_DEBUG("Setting content for {}", id_);

    // Try XML parsing if data model is xpath or xml type
    if (type_ == "xpath" || type_ == "xml") {
        setXmlContent(content);
    } else {
        // Handle other types as plain string
        content_ = content;

#ifndef __EMSCRIPTEN__
        // Remove XML content if it existed (Native builds only)
        xmlContent_.reset();
#endif
    }

    // Add to contentItems_ in all cases
    contentItems_.push_back(content);
}

void RSM::DataModelItem::addContent(const std::string &content) {
    LOG_DEBUG("Adding content for {}", id_);

    // Always add to contentItems_
    contentItems_.push_back(content);

#ifndef __EMSCRIPTEN__
    // Try adding to DOM if XML type (Native builds only)
    if (type_ == "xpath" || type_ == "xml") {
        if (xmlContent_) {
            try {
                // Parse as temporary XML document
                xmlpp::DomParser parser;
                parser.parse_memory(content);
                xmlpp::Document *tempDoc = parser.get_document();

                if (tempDoc && tempDoc->get_root_node()) {
                    // Get root node
                    xmlpp::Node *root = xmlContent_->get_root_node();
                    if (root) {
                        // Add new content to existing tree
                        xmlpp::Node *importedNode = tempDoc->get_root_node();
                        if (importedNode) {
                            root->import_node(importedNode);
                        }
                    }
                }
            } catch (const std::exception &ex) {
                LOG_ERROR("Failed to parse XML content: {}", ex.what());
            }
        } else {
            // Create new if xmlContent_ doesn't exist
            setXmlContent(content);
        }
    } else {
#endif
        // Add to string if not XML type (or WASM build)
        if (!content_.empty()) {
            content_ += content;
        } else {
            content_ = content;
        }
#ifndef __EMSCRIPTEN__
    }
#endif
}

const std::string &RSM::DataModelItem::getContent() const {
#ifndef __EMSCRIPTEN__
    // Serialize XML to string if XML content exists and content_ is empty (Native builds only)
    if (xmlContent_ && content_.empty()) {
        static std::string serialized;
        serialized.clear();

        try {
            // Serialize XML document to string
            xmlContent_->write_to_string(serialized);
        } catch (const std::exception &ex) {
            LOG_ERROR("Failed to serialize XML: {}", ex.what());
        }

        return serialized;
    }
#endif

    return content_;
}

void RSM::DataModelItem::setSrc(const std::string &src) {
    LOG_DEBUG("Setting source URL for {}: {}", id_, src);
    src_ = src;
}

const std::string &RSM::DataModelItem::getSrc() const {
    return src_;
}

void RSM::DataModelItem::setAttribute(const std::string &name, const std::string &value) {
    LOG_DEBUG("Setting attribute for {}: {} = {}", id_, name, value);
    attributes_[name] = value;
}

const std::string &RSM::DataModelItem::getAttribute(const std::string &name) const {
    auto it = attributes_.find(name);
    if (it != attributes_.end()) {
        return it->second;
    }
    return emptyString_;
}

const std::unordered_map<std::string, std::string> &RSM::DataModelItem::getAttributes() const {
    return attributes_;
}

void RSM::DataModelItem::setXmlContent(const std::string &content) {
    LOG_DEBUG("Setting XML content for {}", id_);

#ifndef __EMSCRIPTEN__
    // Delete existing XML document if present
    xmlContent_.reset();

    try {
        // Parse XML
        xmlpp::DomParser parser;
        parser.parse_memory(content);

        // Create new document and get content (Document is not copyable)
        xmlContent_ = std::make_unique<xmlpp::Document>();
        if (parser.get_document() && parser.get_document()->get_root_node()) {
            xmlContent_->create_root_node_by_import(parser.get_document()->get_root_node());
        }

        // Clear content_ if parsing succeeds (regenerate in getContent() if needed)
        content_ = "";
    } catch (const std::exception &ex) {
        LOG_ERROR("Failed to parse XML content: {}", ex.what());
        xmlContent_.reset();

        // Store as plain string if parsing fails
        content_ = content;
    }
#else
    // WASM builds: Store as plain string (no XML parsing support)
    content_ = content;
#endif
}

#ifndef __EMSCRIPTEN__
xmlpp::Node *RSM::DataModelItem::getXmlContent() const {
    if (xmlContent_) {
        return xmlContent_->get_root_node();
    }
    return nullptr;
}
#endif

const std::vector<std::string> &RSM::DataModelItem::getContentItems() const {
    return contentItems_;
}

bool RSM::DataModelItem::isXmlContent() const {
#ifndef __EMSCRIPTEN__
    return xmlContent_ != nullptr;
#else
    // WASM builds: No XML content support
    return false;
#endif
}

std::optional<std::string> RSM::DataModelItem::queryXPath(const std::string &xpath) const {
#ifndef __EMSCRIPTEN__
    if (!xmlContent_ || !xmlContent_->get_root_node()) {
        return std::nullopt;
    }

    try {
        xmlpp::Node *root = xmlContent_->get_root_node();
        auto nodes = root->find(xpath);

        if (nodes.empty()) {
            return std::nullopt;
        }

        if (nodes.size() == 1) {
            // Single node case
            auto node = nodes[0];
            // Find text node
            auto child = node->get_first_child();
            if (child && dynamic_cast<xmlpp::TextNode *>(child)) {
                return dynamic_cast<xmlpp::TextNode *>(child)->get_content();
            } else {
                // Return node path if no text node
                return node->get_path();
            }
        } else {
            // Multiple nodes case, combine results
            std::stringstream result;
            for (auto node : nodes) {
                auto child = node->get_first_child();
                if (child && dynamic_cast<xmlpp::TextNode *>(child)) {
                    if (result.tellp() > 0) {
                        result << " ";
                    }
                    result << dynamic_cast<xmlpp::TextNode *>(child)->get_content();
                }
            }
            return result.str();
        }
    } catch (const std::exception &ex) {
        LOG_ERROR("XPath query failed: {}", ex.what());
    }

    return std::nullopt;
#else
    // WASM builds: XPath not supported
    (void)xpath;  // Suppress unused parameter warning
    LOG_WARN("XPath queries not supported in WASM builds");
    return std::nullopt;
#endif
}

bool RSM::DataModelItem::supportsDataModel(const std::string &dataModelType) const {
    // xpath and xml data models support XML processing
    if (dataModelType == "xpath" || dataModelType == "xml") {
#ifndef __EMSCRIPTEN__
        return true;
#else
        // WASM builds: Limited XML support (no DOM parsing)
        return false;
#endif
    }

    // ecmascript data model supports basic string processing
    if (dataModelType == "ecmascript") {
        return true;
    }

    // null data model has limited support
    if (dataModelType == "null") {
        return true;
    }

    // Other data models not supported
    return false;
}
