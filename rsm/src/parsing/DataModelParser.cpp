#include "parsing/DataModelParser.h"
#include "common/FileLoadingHelper.h"
#include "common/Logger.h"
#include "common/XmlSerializationHelper.h"
#include "parsing/ParsingCommon.h"
#include <algorithm>

#ifndef __EMSCRIPTEN__
#endif

RSM::DataModelParser::DataModelParser(std::shared_ptr<NodeFactory> nodeFactory) : nodeFactory_(nodeFactory) {
    LOG_DEBUG("Creating data model parser");
}

RSM::DataModelParser::~DataModelParser() {
    LOG_DEBUG("Destroying data model parser");
}

std::vector<std::shared_ptr<RSM::IDataModelItem>>
RSM::DataModelParser::parseDataModelNode(const std::shared_ptr<IXMLElement> &datamodelNode,
                                         const RSM::SCXMLContext &context) {
    std::vector<std::shared_ptr<IDataModelItem>> items;

    if (!datamodelNode) {
        LOG_WARN("Null datamodel node");
        return items;
    }

    LOG_DEBUG("Parsing datamodel node");

    // Parse data nodes
    auto dataNodes = RSM::ParsingCommon::findChildElements(datamodelNode, "data");
    for (const auto &dataElement : dataNodes) {
        auto dataItem = parseDataModelItem(dataElement, context);
        if (dataItem) {
            items.push_back(dataItem);
            LOG_DEBUG("Added data model item: {}", dataItem->getId());
        }
    }

    LOG_DEBUG("Parsed {} data model items", items.size());
    return items;
}

std::shared_ptr<RSM::IDataModelItem>
RSM::DataModelParser::parseDataModelItem(const std::shared_ptr<IXMLElement> &dataNode,
                                         const RSM::SCXMLContext &context) {
    if (!dataNode) {
        LOG_WARN("Null data node");
        return nullptr;
    }

    if (!dataNode->hasAttribute("id")) {
        LOG_WARN("Data node missing id attribute");
        return nullptr;
    }

    std::string id = dataNode->getAttribute("id");
    std::string expr;

    if (dataNode->hasAttribute("expr")) {
        expr = dataNode->getAttribute("expr");
    }

    LOG_DEBUG("Parsing data model item: {}", id);
    auto dataItem = nodeFactory_->createDataModelItem(id, expr);

    // Process src attribute
    bool hasSrc = dataNode->hasAttribute("src");
    if (hasSrc) {
        std::string src = dataNode->getAttribute("src");
        dataItem->setSrc(src);
        LOG_DEBUG("Source URL: {}", src);

        loadExternalContent(src, dataItem);

        if (!expr.empty()) {
            LOG_WARN("Data element cannot have both 'src' and 'expr' attributes: {}", id);
        }

        if (!dataNode->getTextContent().empty()) {
            LOG_WARN("Data element cannot have both 'src' attribute and content: {}", id);
        }
    }

    // Process type attribute
    if (dataNode->hasAttribute("type")) {
        std::string type = dataNode->getAttribute("type");
        dataItem->setType(type);
        LOG_DEBUG("Type: {}", type);
    } else if (!context.getDatamodelType().empty()) {
        dataItem->setType(context.getDatamodelType());
        LOG_DEBUG("Using parent datamodel type: {}", context.getDatamodelType());
    }

    // Process scope attribute
    if (dataNode->hasAttribute("code:scope")) {
        std::string scope = dataNode->getAttribute("code:scope");
        dataItem->setScope(scope);
        LOG_DEBUG("Scope: {}", scope);
    } else if (dataNode->hasAttribute("scope")) {
        std::string scope = dataNode->getAttribute("scope");
        dataItem->setScope(scope);
        LOG_DEBUG("Scope: {}", scope);
    }

    // Process content only if src doesn't exist with expr or content
    if (!hasSrc || (hasSrc && expr.empty() && dataNode->getTextContent().empty())) {
        parseDataContent(dataNode, dataItem);
    }

    LOG_DEBUG("Data model item parsed successfully");
    return dataItem;
}

void RSM::DataModelParser::parseDataContent(const std::shared_ptr<IXMLElement> &dataNode,
                                            std::shared_ptr<IDataModelItem> dataItem) {
    if (!dataNode || !dataItem) {
        return;
    }

    // W3C SCXML B.2: Serialize data element content
    // ARCHITECTURE.md Zero Duplication: Use XmlSerializationHelper
    std::string textContent = XmlSerializationHelper::serializeContent(dataNode);

    if (!textContent.empty()) {
        dataItem->setContent(textContent);
        LOG_DEBUG("Added text content");
    }
}

std::vector<std::shared_ptr<RSM::IDataModelItem>>
RSM::DataModelParser::parseDataModelInState(const std::shared_ptr<IXMLElement> &stateNode,
                                            const RSM::SCXMLContext &context) {
    std::vector<std::shared_ptr<IDataModelItem>> items;

    if (!stateNode) {
        LOG_WARN("Null state node");
        return items;
    }

    LOG_DEBUG("Parsing datamodel in state");

    // Find datamodel element
    auto datamodelNode = RSM::ParsingCommon::findFirstChildElement(stateNode, "datamodel");
    if (datamodelNode) {
        auto stateItems = parseDataModelNode(datamodelNode, context);
        items.insert(items.end(), stateItems.begin(), stateItems.end());
    }

    LOG_DEBUG("Found {} data items in state", items.size());
    return items;
}

std::string RSM::DataModelParser::extractDataModelType(const std::shared_ptr<IXMLElement> &datamodelNode) const {
    if (!datamodelNode) {
        return "";
    }

    if (datamodelNode->hasAttribute("type")) {
        return datamodelNode->getAttribute("type");
    }

    return "";
}

bool RSM::DataModelParser::isDataModelItem(const std::shared_ptr<IXMLElement> &element) const {
    if (!element) {
        return false;
    }

    std::string nodeName = element->getName();
    return matchNodeName(nodeName, "data");
}

bool RSM::DataModelParser::matchNodeName(const std::string &nodeName, const std::string &searchName) const {
    // Exact match
    if (nodeName == searchName) {
        return true;
    }

    // With namespace (e.g., "code:data")
    size_t colonPos = nodeName.find(':');
    if (colonPos != std::string::npos && colonPos + 1 < nodeName.length()) {
        std::string localName = nodeName.substr(colonPos + 1);
        return localName == searchName;
    }

    return false;
}

void RSM::DataModelParser::loadExternalContent(const std::string &src, std::shared_ptr<IDataModelItem> dataItem) {
    // W3C SCXML 5.2.2: Load data from external sources
    // ARCHITECTURE.MD: Zero Duplication - Use FileLoadingHelper (Single Source of Truth)

    LOG_DEBUG("Loading content from: {}", src);

    // W3C SCXML 5.2.2: Handle file:// or file: URIs
    if (src.find("file://") == 0 || src.find("file:") == 0 || src.find("/") == 0 || src.find("./") == 0) {
        std::string content;
        bool success = FileLoadingHelper::loadFromSrc(src, content);

        if (success) {
            dataItem->setContent(content);
            LOG_DEBUG("Content loaded from file via FileLoadingHelper");
        } else {
            LOG_ERROR("Failed to load file via FileLoadingHelper: {}", src);
        }
    } else if (src.find("http://") == 0 || src.find("https://") == 0) {
        // HTTP requests require more complex implementation and external libraries are recommended
        // Implementation omitted here, log only
        LOG_WARN("HTTP loading not implemented: {}", src);
    } else {
        // Handle other protocols or relative paths
        LOG_WARN("Unsupported URL format: {}", src);
    }
}
