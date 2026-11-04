// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/XmlSerializationHelper.h"
#include "common/LogToLogger.h"
#include "parsing/ParsingCommon.h"
#include <algorithm>

namespace SCE {

std::string XmlSerializationHelper::serializeContent(const std::shared_ptr<IXMLElement> &element) {
    if (!element) {
        return "";
    }

    // ARCHITECTURE.md: Polymorphic dispatch - each implementation handles serialization
    // pugixml: Full XML structure preservation with pugi::xml_node::print()
    // WASM (PugiXMLElement): Text content only (pugixml serialization limitation)
    return element->serializeChildContent();
}

std::string XmlSerializationHelper::serializeContentBody(const std::shared_ptr<IXMLElement> &content) {
    if (!content) {
        return "";
    }

    // The markup when there is any, the character data otherwise. Character
    // data is read as its characters (`getTextContent`), not as printed XML,
    // so `a &lt; b` is the three characters the author meant.
    const std::string body =
        content->getChildren().empty() ? content->getTextContent() : content->serializeElementChildren();
    return ParsingCommon::trimString(body);
}

std::string XmlSerializationHelper::escapeForJavaScript(const std::string &content) {
    std::string escaped;
    escaped.reserve(content.length() * 2);  // Reserve space for potential escapes

    for (char c : content) {
        switch (c) {
        case '"':
            escaped += "\\\"";
            break;
        case '\\':
            escaped += "\\\\";
            break;
        case '\n':
            escaped += "\\n";
            break;
        case '\r':
            escaped += "\\r";
            break;
        default:
            escaped += c;
            break;
        }
    }

    return "\"" + escaped + "\"";
}

}  // namespace SCE
