#include "parsing/ParsingCommon.h"
#include "common/Logger.h"
#include <algorithm>
#include <cctype>

namespace SCE {

const std::string ParsingCommon::Constants::SCXML_NAMESPACE = "http://www.w3.org/2005/07/scxml";
const std::string ParsingCommon::Constants::CODE_NAMESPACE = "http://tempuri.org/code";
const std::string ParsingCommon::Constants::CTX_NAMESPACE = "http://tempuri.org/context";
const std::string ParsingCommon::Constants::DI_NAMESPACE = "http://www.omg.org/spec/SCXML/20150901/DI";

bool ParsingCommon::matchNodeName(const std::string &nodeName, const std::string &baseName) {
    // Check exact match first
    if (nodeName == baseName) {
        return true;
    }

    // Check if nodeName ends with :baseName (namespace prefix match)
    size_t colonPos = nodeName.rfind(':');
    if (colonPos != std::string::npos) {
        std::string localName = nodeName.substr(colonPos + 1);
        return localName == baseName;
    }

    return false;
}

std::vector<std::shared_ptr<IXMLElement>> ParsingCommon::findChildElements(const std::shared_ptr<IXMLElement> &element,
                                                                           const std::string &childName) {
    if (!element) {
        return {};
    }

    std::vector<std::shared_ptr<IXMLElement>> result;
    auto children = element->getChildren();

    for (const auto &child : children) {
        if (matchNodeName(child->getName(), childName)) {
            result.push_back(child);
        }
    }

    return result;
}

std::shared_ptr<IXMLElement> ParsingCommon::findFirstChildElement(const std::shared_ptr<IXMLElement> &element,
                                                                  const std::string &childName) {
    if (!element) {
        return nullptr;
    }

    auto children = element->getChildren();

    for (const auto &child : children) {
        if (matchNodeName(child->getName(), childName)) {
            return child;
        }
    }

    return nullptr;
}

std::string ParsingCommon::findElementId(const std::shared_ptr<IXMLElement> &element) {
    if (!element) {
        return "";
    }

    // Try "id" attribute first
    if (element->hasAttribute("id")) {
        return element->getAttribute("id");
    }

    // Try parent element
    auto parent = element->getParent();
    if (parent && parent->hasAttribute("id")) {
        return parent->getAttribute("id");
    }

    return "";
}

std::string ParsingCommon::getAttributeValue(const std::shared_ptr<IXMLElement> &element,
                                             const std::vector<std::string> &attrNames) {
    if (!element) {
        return "";
    }

    for (const auto &attrName : attrNames) {
        if (element->hasAttribute(attrName)) {
            return element->getAttribute(attrName);
        }
    }

    return "";
}

std::unordered_map<std::string, std::string>
ParsingCommon::collectAttributes(const std::shared_ptr<IXMLElement> &element,
                                 const std::vector<std::string> &excludeAttrs) {
    std::unordered_map<std::string, std::string> result;

    if (!element) {
        return result;
    }

    auto allAttrs = element->getAttributes();

    for (const auto &[name, value] : allAttrs) {
        bool excluded = false;
        for (const auto &excludeAttr : excludeAttrs) {
            if (name == excludeAttr) {
                excluded = true;
                break;
            }
        }

        if (!excluded) {
            result[name] = value;
        }
    }

    return result;
}

std::string ParsingCommon::resolveRelativePath(const std::string &basePath, const std::string &relativePath) {
    if (relativePath.empty()) {
        return basePath;
    }

    // If relativePath is absolute, return it as-is
    if (!relativePath.empty() && relativePath[0] == '/') {
        return relativePath;
    }

    // If basePath is empty, return relativePath
    if (basePath.empty()) {
        return relativePath;
    }

    // Find last '/' in basePath
    size_t lastSlash = basePath.rfind('/');
    if (lastSlash == std::string::npos) {
        return relativePath;
    }

    // Combine basePath directory + relativePath
    return basePath.substr(0, lastSlash + 1) + relativePath;
}

std::string ParsingCommon::extractTextContent(const std::shared_ptr<IXMLElement> &element, bool trimWhitespace) {
    if (!element) {
        return "";
    }

    std::string content = element->getTextContent();

    if (trimWhitespace) {
        return trimString(content);
    }

    return content;
}

std::string ParsingCommon::getLocalName(const std::shared_ptr<IXMLElement> &element) {
    if (!element) {
        return "";
    }

    std::string fullName = element->getName();

    // Remove namespace prefix if present
    size_t colonPos = fullName.rfind(':');
    if (colonPos != std::string::npos) {
        return fullName.substr(colonPos + 1);
    }

    return fullName;
}

std::vector<std::shared_ptr<IXMLElement>>
ParsingCommon::findChildElementsWithNamespace(const std::shared_ptr<IXMLElement> &parent,
                                              const std::string &elementName, const std::string &namespaceURI) {
    std::vector<std::shared_ptr<IXMLElement>> result;

    if (!parent) {
        return result;
    }

    auto children = parent->getChildren();

    for (const auto &child : children) {
        std::string childNamespace = child->getNamespace();
        std::string childName = getLocalName(child);

        if (childNamespace == namespaceURI && childName == elementName) {
            result.push_back(child);
        }
    }

    return result;
}

std::string ParsingCommon::trimString(const std::string &str) {
    if (str.empty()) {
        return str;
    }

    // Find first non-whitespace character
    size_t start = 0;
    while (start < str.length() && std::isspace(static_cast<unsigned char>(str[start]))) {
        ++start;
    }

    // If all whitespace, return empty string
    if (start == str.length()) {
        return "";
    }

    // Find last non-whitespace character
    size_t end = str.length();
    while (end > start && std::isspace(static_cast<unsigned char>(str[end - 1]))) {
        --end;
    }

    return str.substr(start, end - start);
}

}  // namespace SCE
