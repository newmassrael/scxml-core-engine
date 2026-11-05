// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/DoneDataParser.h"
#include "common/LogToLogger.h"
#include "parsing/ParsingCommon.h"
#include "parsing/XmlSerializationHelper.h"

SCE::DoneDataParser::DoneDataParser(std::shared_ptr<NodeFactory> factory) : factory_(factory) {
    SCE_LOG_DEBUG("Creating DoneData parser");
}

bool SCE::DoneDataParser::parseDoneData(const std::shared_ptr<IXMLElement> &doneDataElement, IStateNode *stateNode) {
    if (!doneDataElement || !stateNode) {
        SCE_LOG_ERROR("Null doneData element or state node");
        return false;
    }

    SCE_LOG_DEBUG("Parsing <donedata> for state {}", stateNode->getId());

    bool hasContent = false;
    bool hasParam = false;

    // Parse <content> element
    auto contentElement = ParsingCommon::findFirstChildElement(doneDataElement, "content");
    if (contentElement) {
        hasContent = parseContent(contentElement, stateNode);
        SCE_LOG_DEBUG("Found <content> element: {}", (hasContent ? "valid" : "invalid"));
    }

    // Parse <param> elements
    auto paramElements = ParsingCommon::findChildElements(doneDataElement, "param");
    for (const auto &paramElement : paramElements) {
        if (parseParam(paramElement, stateNode)) {
            hasParam = true;
        }
    }

    SCE_LOG_DEBUG("Found {} <param> elements: {}", paramElements.size(), (hasParam ? "valid" : "invalid"));

    // §scxml-5.5.2: a conformant document names either a single <content> child or
    // one or more <param> children under <donedata>, never both.
    if (hasContent && hasParam) {
        SCE_LOG_ERROR("<content> and <param> cannot be used together in <donedata>");

        // Clear conflict to satisfy XOR condition
        if (hasContent) {
            // Keep content, remove param
            stateNode->clearDoneDataParams();
            hasParam = false;
        } else {
            // Keep param, remove content
            stateNode->setDoneDataContentExpression("");
            hasContent = false;
        }

        return false;
    }

    return hasContent || hasParam;
}

bool SCE::DoneDataParser::parseContent(const std::shared_ptr<IXMLElement> &contentElement, IStateNode *stateNode) {
    if (!contentElement || !stateNode) {
        SCE_LOG_ERROR("Null content element or state node");
        return false;
    }

    // Check expr attribute
    std::string exprValue;
    if (contentElement->hasAttribute("expr")) {
        exprValue = contentElement->getAttribute("expr");
        SCE_LOG_DEBUG("Found 'expr' attribute: {}", exprValue);
    }

    // The children: their markup when they hold elements, their character
    // data otherwise — read by the helper every in-line `<content>` is read
    // by, so a done event and a `<send>` carry one text for one element.
    const std::string textContent = XmlSerializationHelper::serializeContentBody(contentElement);
    if (!textContent.empty()) {
        SCE_LOG_DEBUG("Found text content: {}",
                      (textContent.length() > 30 ? textContent.substr(0, 27) + "..." : textContent));
    }

    // §scxml-5.6.2: a conformant document does not give <content> both an 'expr'
    // attribute and child content.
    if (!exprValue.empty() && !textContent.empty()) {
        SCE_LOG_ERROR("<content> cannot have both 'expr' attribute and child content");
        return false;
    }

    // §scxml-B-2-6: under the ECMAScript data model a <content> child of <donedata> is
    // interpreted as §scxml-B-2-8-1 defines _event.data, which is why the children
    // are read as the data model reads in-line content rather than used verbatim.
    // §scxml-5.5 + 5.6 + §scxml-B-2-6:
    //   - `expr` attribute => Expression (evaluated against the datamodel).
    //   - Children => InlineText, given the one reading at run time
    //     (`DataModelInitHelper::inlineContentEventData`, the reading a
    //     `<send>`'s in-line `<content>` gets): an expression when the data
    //     model reads them as one, so `<content>21</content>` yields number 21
    //     for `_event.data == 21` style guards; the XML as written when they
    //     open with `<`; and otherwise the space-normalized string, so
    //     `<content>hello world</content>` is the string the generated
    //     engines carry rather than an expression that can only fail. The
    //     "null" datamodel has no expressions, so under it the first step
    //     reads none and no script engine is asked.
    if (!exprValue.empty()) {
        stateNode->setDoneDataContentExpression(exprValue);
        return true;
    } else if (!textContent.empty()) {
        stateNode->setDoneDataContentInlineText(textContent, datamodelType_ == "null");
        return true;
    }

    // Empty <content/> — no content: the done event carries no data.
    stateNode->setDoneDataContentExpression("");
    return true;
}

bool SCE::DoneDataParser::parseParam(const std::shared_ptr<IXMLElement> &paramElement, IStateNode *stateNode) {
    if (!paramElement || !stateNode) {
        SCE_LOG_ERROR("Null param element or state node");
        return false;
    }

    // name attribute (required)
    if (!paramElement->hasAttribute("name")) {
        SCE_LOG_ERROR("<param> element must have 'name' attribute");
        return false;
    }

    std::string nameValue = paramElement->getAttribute("name");

    // §scxml-5.7: exactly one of expr and location — refused, not dropped, so
    // the Interpreter accepts the documents the code generator accepts.
    ParsingCommon::checkParamAttributes(paramElement);

    if (paramElement->hasAttribute("location")) {
        std::string locationValue = paramElement->getAttribute("location");
        stateNode->addDoneDataParam(nameValue, locationValue);
        SCE_LOG_DEBUG("Added param: {} with location: {}", nameValue, locationValue);
        return true;
    }

    std::string exprValue = paramElement->getAttribute("expr");
    stateNode->addDoneDataParam(nameValue, exprValue);
    SCE_LOG_DEBUG("Added param: {} with expr: {}", nameValue, exprValue);
    return true;
}
