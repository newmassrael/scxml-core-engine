#pragma once

// All platforms: pugixml
#include <pugixml.hpp>

#include <memory>
#include <string>
#include <vector>

namespace RSM {

// Forward declarations
class XMLElement;
class XMLDocument;

/**
 * W3C SCXML B.2: XML DOM wrapper for XML integration
 * Provides JavaScript-accessible DOM API for XML content
 *
 * Unified implementation: pugixml-based for all platforms
 */
class XMLElement {
public:
    explicit XMLElement(pugi::xml_node node);
    ~XMLElement() = default;

    // DOM API methods
    std::vector<std::shared_ptr<XMLElement>> getElementsByTagName(const std::string &tagName);
    std::string getAttribute(const std::string &attrName);
    std::string getTagName() const;

    // Internal access
    pugi::xml_node getNode() const {
        return node_;
    }

    pugi::xml_node node_;

public:
    static void findElementsByTagNameStatic(pugi::xml_node node, const std::string &tagName,
                                            std::vector<std::shared_ptr<XMLElement>> &result);

private:
    void findElementsByTagName(pugi::xml_node node, const std::string &tagName,
                               std::vector<std::shared_ptr<XMLElement>> &result);
};

/**
 * W3C SCXML B.2: XML Document wrapper
 * Root object for XML DOM tree
 */
class XMLDocument {
public:
    explicit XMLDocument(const std::string &xmlContent);
    ~XMLDocument();

    // DOM API methods
    std::vector<std::shared_ptr<XMLElement>> getElementsByTagName(const std::string &tagName);
    std::shared_ptr<XMLElement> getDocumentElement();

    bool isValid() const {
        return !doc_.empty();
    }

    std::string getErrorMessage() const {
        return errorMessage_;
    }

private:
    pugi::xml_document doc_;
    std::string errorMessage_;
};

}  // namespace RSM
