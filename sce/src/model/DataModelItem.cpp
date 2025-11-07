#include "DataModelItem.h"
#include "common/Logger.h"
#include "parsing/IXMLDocument.h"
#include "parsing/IXMLElement.h"
#include "parsing/IXMLParser.h"
#include <sstream>

SCE::DataModelItem::DataModelItem(const std::string &id, const std::string &expr)
    : id_(id), expr_(expr), scope_("global") {
    LOG_DEBUG("Creating data model item: {}", id);
}

SCE::DataModelItem::~DataModelItem() {
    LOG_DEBUG("Destroying data model item: {}", id_);
    // unique_ptr automatically manages memory - no manual delete needed
}

const std::string &SCE::DataModelItem::getId() const {
    return id_;
}

void SCE::DataModelItem::setExpr(const std::string &expr) {
    LOG_DEBUG("Setting expression for {}: {}", id_, expr);
    expr_ = expr;
}

const std::string &SCE::DataModelItem::getExpr() const {
    return expr_;
}

void SCE::DataModelItem::setType(const std::string &type) {
    LOG_DEBUG("Setting type for {}: {}", id_, type);
    type_ = type;
}

const std::string &SCE::DataModelItem::getType() const {
    return type_;
}

void SCE::DataModelItem::setScope(const std::string &scope) {
    LOG_DEBUG("Setting scope for {}: {}", id_, scope);
    scope_ = scope;
}

const std::string &SCE::DataModelItem::getScope() const {
    return scope_;
}

void SCE::DataModelItem::setContent(const std::string &content) {
    LOG_DEBUG("Setting content for {}", id_);

    // Try XML parsing if data model is xpath or xml type
    if (type_ == "xpath" || type_ == "xml") {
        setXmlContent(content);
    } else {
        // Handle other types as plain string
        content_ = content;
        // Remove XML content if it existed
        xmlContent_.reset();
    }

    // Add to contentItems_ in all cases
    contentItems_.push_back(content);
}

void SCE::DataModelItem::addContent(const std::string &content) {
    LOG_DEBUG("Adding content for {}", id_);

    // Always add to contentItems_
    contentItems_.push_back(content);

    // Try adding to DOM if XML type
    if (type_ == "xpath" || type_ == "xml") {
        if (xmlContent_) {
            try {
                // Parse as temporary XML document
                auto parser = IXMLParser::create();
                auto tempDoc = parser->parseContent(content);

                if (tempDoc && tempDoc->isValid() && tempDoc->getRootElement()) {
                    // Get root element
                    auto root = xmlContent_->getRootElement();
                    if (root) {
                        // Add new content to existing tree
                        auto importedNode = tempDoc->getRootElement();
                        if (importedNode) {
                            root->importNode(importedNode);
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
        // Add to string if not XML type
        if (!content_.empty()) {
            content_ += content;
        } else {
            content_ = content;
        }
    }
}

const std::string &SCE::DataModelItem::getContent() const {
    // Serialize XML to string if XML content exists and content_ is empty
    if (xmlContent_ && xmlContent_->isValid() && content_.empty()) {
        static std::string serialized;
        serialized.clear();

        try {
            // Get root element and serialize child content
            auto root = xmlContent_->getRootElement();
            if (root) {
                serialized = root->serializeChildContent();
            }
        } catch (const std::exception &ex) {
            LOG_ERROR("Failed to serialize XML: {}", ex.what());
        }

        return serialized;
    }

    return content_;
}

void SCE::DataModelItem::setSrc(const std::string &src) {
    LOG_DEBUG("Setting source URL for {}: {}", id_, src);
    src_ = src;
}

const std::string &SCE::DataModelItem::getSrc() const {
    return src_;
}

void SCE::DataModelItem::setAttribute(const std::string &name, const std::string &value) {
    LOG_DEBUG("Setting attribute for {}: {} = {}", id_, name, value);
    attributes_[name] = value;
}

const std::string &SCE::DataModelItem::getAttribute(const std::string &name) const {
    auto it = attributes_.find(name);
    if (it != attributes_.end()) {
        return it->second;
    }
    return emptyString_;
}

const std::unordered_map<std::string, std::string> &SCE::DataModelItem::getAttributes() const {
    return attributes_;
}

void SCE::DataModelItem::setXmlContent(const std::string &content) {
    LOG_DEBUG("Setting XML content for {}", id_);

    // Reset existing XML document
    xmlContent_.reset();

    try {
        // Parse XML using platform-appropriate parser
        auto parser = IXMLParser::create();
        xmlContent_ = parser->parseContent(content);

        if (xmlContent_ && xmlContent_->isValid()) {
            // Clear content_ if parsing succeeds (regenerate in getContent() if needed)
            content_ = "";
        } else {
            LOG_ERROR("Failed to parse XML content: {}",
                      xmlContent_ ? xmlContent_->getErrorMessage() : "Parser returned null");
            xmlContent_.reset();
            // Store as plain string if parsing fails
            content_ = content;
        }
    } catch (const std::exception &ex) {
        LOG_ERROR("Failed to parse XML content: {}", ex.what());
        xmlContent_.reset();
        // Store as plain string if parsing fails
        content_ = content;
    }
}

std::shared_ptr<SCE::IXMLElement> SCE::DataModelItem::getXmlContent() const {
    if (xmlContent_ && xmlContent_->isValid()) {
        return xmlContent_->getRootElement();
    }
    return nullptr;
}

const std::vector<std::string> &SCE::DataModelItem::getContentItems() const {
    return contentItems_;
}

bool SCE::DataModelItem::isXmlContent() const {
    return xmlContent_ != nullptr && xmlContent_->isValid();
}

std::optional<std::string> SCE::DataModelItem::queryXPath(const std::string &xpath) const {
    if (!xmlContent_ || !xmlContent_->isValid()) {
        return std::nullopt;
    }

    // W3C SCXML XPath Support: Currently limited to basic queries
    // pugixml: XPath support via custom implementation
    // WASM: Basic pugixml XPath support (subset of XPath 1.0)
    // TODO: Add IXMLElement::queryXPath() interface method for unified XPath support

    LOG_WARN("XPath queries are currently limited. Full XPath support requires interface extension.");
    LOG_DEBUG("XPath query requested: {}", xpath);

    // For now, return nullopt as XPath is not exposed through IXMLElement interface
    // Full implementation requires adding queryXPath() to IXMLElement interface
    return std::nullopt;
}

bool SCE::DataModelItem::supportsDataModel(const std::string &dataModelType) const {
    // xpath and xml data models support XML processing (all platforms)
    if (dataModelType == "xpath" || dataModelType == "xml") {
        return true;
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
