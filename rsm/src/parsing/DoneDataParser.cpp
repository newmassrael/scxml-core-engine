#include "parsing/DoneDataParser.h"
#include "common/Logger.h"
#include "parsing/ParsingCommon.h"

RSM::DoneDataParser::DoneDataParser(std::shared_ptr<NodeFactory> factory) : factory_(factory) {
    LOG_DEBUG("Creating DoneData parser");
}

bool RSM::DoneDataParser::parseDoneData(const std::shared_ptr<IXMLElement> &doneDataElement, IStateNode *stateNode) {
    if (!doneDataElement || !stateNode) {
        LOG_ERROR("Null doneData element or state node");
        return false;
    }

    LOG_DEBUG("Parsing <donedata> for state {}", stateNode->getId());

    bool hasContent = false;
    bool hasParam = false;

    // Parse <content> element
    auto contentElement = ParsingCommon::findFirstChildElement(doneDataElement, "content");
    if (contentElement) {
        hasContent = parseContent(contentElement, stateNode);
        LOG_DEBUG("Found <content> element: {}", (hasContent ? "valid" : "invalid"));
    }

    // Parse <param> elements
    auto paramElements = ParsingCommon::findChildElements(doneDataElement, "param");
    for (const auto &paramElement : paramElements) {
        if (parseParam(paramElement, stateNode)) {
            hasParam = true;
        }
    }

    LOG_DEBUG("Found {} <param> elements: {}", paramElements.size(), (hasParam ? "valid" : "invalid"));

    // <content> and <param> cannot be used together
    if (hasContent && hasParam) {
        LOG_ERROR("<content> and <param> cannot be used together in <donedata>");

        // Clear conflict to satisfy XOR condition
        if (hasContent) {
            // Keep content, remove param
            stateNode->clearDoneDataParams();
            hasParam = false;
        } else {
            // Keep param, remove content
            stateNode->setDoneDataContent("");
            hasContent = false;
        }

        return false;
    }

    return hasContent || hasParam;
}

bool RSM::DoneDataParser::parseContent(const std::shared_ptr<IXMLElement> &contentElement, IStateNode *stateNode) {
    if (!contentElement || !stateNode) {
        LOG_ERROR("Null content element or state node");
        return false;
    }

    // Check expr attribute
    std::string exprValue;
    if (contentElement->hasAttribute("expr")) {
        exprValue = contentElement->getAttribute("expr");
        LOG_DEBUG("Found 'expr' attribute: {}", exprValue);
    }

    // Check content
    std::string textContent = contentElement->getTextContent();
    textContent = ParsingCommon::trimString(textContent);
    if (!textContent.empty()) {
        LOG_DEBUG("Found text content: {}",
                  (textContent.length() > 30 ? textContent.substr(0, 27) + "..." : textContent));
    }

    // expr and content cannot be used together
    if (!exprValue.empty() && !textContent.empty()) {
        LOG_ERROR("<content> cannot have both 'expr' attribute and child content");
        return false;
    }

    // Set expr or content
    if (!exprValue.empty()) {
        stateNode->setDoneDataContent(exprValue);
        return true;
    } else if (!textContent.empty()) {
        stateNode->setDoneDataContent(textContent);
        return true;
    }

    // Handle empty content
    stateNode->setDoneDataContent("");
    return true;
}

bool RSM::DoneDataParser::parseParam(const std::shared_ptr<IXMLElement> &paramElement, IStateNode *stateNode) {
    if (!paramElement || !stateNode) {
        LOG_ERROR("Null param element or state node");
        return false;
    }

    // name attribute (required)
    if (!paramElement->hasAttribute("name")) {
        LOG_ERROR("<param> element must have 'name' attribute");
        return false;
    }

    std::string nameValue = paramElement->getAttribute("name");

    // Check expr and location attributes (only one can be used)
    bool hasExpr = paramElement->hasAttribute("expr");
    bool hasLocation = paramElement->hasAttribute("location");

    if (hasExpr && hasLocation) {
        LOG_ERROR("<param> cannot have both 'expr' and 'location' attributes");
        return false;
    }

    // Process location attribute
    if (hasLocation) {
        std::string locationValue = paramElement->getAttribute("location");
        stateNode->addDoneDataParam(nameValue, locationValue);
        LOG_DEBUG("Added param: {} with location: {}", nameValue, locationValue);
        return true;
    }

    // Process expr attribute
    if (hasExpr) {
        std::string exprValue = paramElement->getAttribute("expr");
        stateNode->addDoneDataParam(nameValue, exprValue);
        LOG_DEBUG("Added param: {} with expr: {}", nameValue, exprValue);
        return true;
    }

    LOG_ERROR("<param> must have either 'expr' or 'location' attribute");
    return false;
}
