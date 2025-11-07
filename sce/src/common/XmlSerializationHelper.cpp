#include "common/XmlSerializationHelper.h"
#include "common/Logger.h"
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
