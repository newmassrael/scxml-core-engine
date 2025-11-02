#pragma once

#ifdef __EMSCRIPTEN__

#include "IXMLDocument.h"
#include "IXMLElement.h"
#include "IXMLParser.h"
#include <memory>
#include <pugixml.hpp>
#include <string>

namespace RSM {

/**
 * @brief pugixml element wrapper
 */
class PugiXMLElement : public IXMLElement {
public:
    explicit PugiXMLElement(pugi::xml_node node, std::shared_ptr<pugi::xml_document> doc);

    std::string getName() const override;
    std::string getAttribute(const std::string &name) const override;
    bool hasAttribute(const std::string &name) const override;
    std::unordered_map<std::string, std::string> getAttributes() const override;
    std::string getNamespace() const override;
    std::vector<std::shared_ptr<IXMLElement>> getChildren() const override;
    std::vector<std::shared_ptr<IXMLElement>> getChildrenByTagName(const std::string &tagName) const override;
    std::string getTextContent() const override;
    bool importNode(const std::shared_ptr<IXMLElement> &source) override;
    bool remove() override;
    std::shared_ptr<IXMLElement> getParent() const override;
    std::string serializeChildContent() const override;

    // Internal: Get raw pugixml node
    pugi::xml_node getRawNode() const {
        return node_;
    }

private:
    pugi::xml_node node_;
    std::shared_ptr<pugi::xml_document> doc_;  // Keep document alive
};

/**
 * @brief pugixml document wrapper
 */
class PugiXMLDocument : public IXMLDocument {
public:
    explicit PugiXMLDocument(std::shared_ptr<pugi::xml_document> doc);

    std::shared_ptr<IXMLElement> getRootElement() override;
    bool processXInclude() override;
    std::string getErrorMessage() const override;
    bool isValid() const override;

    // Internal: Set base path for XInclude resolution
    void setBasePath(const std::string &basePath) {
        basePath_ = basePath;
    }

private:
    bool processXIncludeRecursive(pugi::xml_node node, int depth = 0);
    std::string resolveFilePath(const std::string &href) const;

    std::shared_ptr<pugi::xml_document> doc_;
    std::string errorMessage_;
    std::string basePath_;
    static constexpr int MAX_XINCLUDE_DEPTH = 10;
};

/**
 * @brief pugixml parser implementation (WASM builds only)
 *
 * Lightweight XML parser optimized for WebAssembly
 * Manual XInclude implementation (pugixml doesn't have native support)
 */
class PugiXMLParser : public IXMLParser {
public:
    PugiXMLParser() = default;
    ~PugiXMLParser() override = default;

    std::shared_ptr<IXMLDocument> parseFile(const std::string &filename) override;
    std::shared_ptr<IXMLDocument> parseContent(const std::string &content) override;
    std::string getLastError() const override;

private:
    std::string lastError_;
};

}  // namespace RSM

#endif  // __EMSCRIPTEN__
