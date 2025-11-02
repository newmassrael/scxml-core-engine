#pragma once

#ifndef __EMSCRIPTEN__
// Native builds: libxml2
#include <libxml/parser.h>
#include <libxml/tree.h>
#else
// WASM builds: pugixml
#include <pugixml.hpp>
#endif

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
 * Platform support:
 * - Native builds: libxml2-based implementation
 * - WASM builds: pugixml-based implementation
 */
class XMLElement {
public:
#ifndef __EMSCRIPTEN__
    explicit XMLElement(xmlNodePtr node);
#else
    explicit XMLElement(pugi::xml_node node);
#endif
    ~XMLElement() = default;

    // DOM API methods
    std::vector<std::shared_ptr<XMLElement>> getElementsByTagName(const std::string &tagName);
    std::string getAttribute(const std::string &attrName);
    std::string getTagName() const;

    // Internal access
#ifndef __EMSCRIPTEN__
    xmlNodePtr getNode() const {
        return node_;
    }

    xmlNodePtr node_;
#else
    pugi::xml_node getNode() const {
        return node_;
    }

    pugi::xml_node node_;
#endif

public:
#ifndef __EMSCRIPTEN__
    static void findElementsByTagNameStatic(xmlNodePtr node, const std::string &tagName,
                                            std::vector<std::shared_ptr<XMLElement>> &result);
#else
    static void findElementsByTagNameStatic(pugi::xml_node node, const std::string &tagName,
                                            std::vector<std::shared_ptr<XMLElement>> &result);
#endif

private:
#ifndef __EMSCRIPTEN__
    void findElementsByTagName(xmlNodePtr node, const std::string &tagName,
                               std::vector<std::shared_ptr<XMLElement>> &result);
#else
    void findElementsByTagName(pugi::xml_node node, const std::string &tagName,
                               std::vector<std::shared_ptr<XMLElement>> &result);
#endif
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
#ifndef __EMSCRIPTEN__
        return doc_ != nullptr;
#else
        return !doc_.empty();
#endif
    }

    std::string getErrorMessage() const {
        return errorMessage_;
    }

private:
#ifndef __EMSCRIPTEN__
    xmlDocPtr doc_;
#else
    pugi::xml_document doc_;
#endif
    std::string errorMessage_;
};

}  // namespace RSM
