#pragma once

#ifndef __EMSCRIPTEN__

#include "IXMLDocument.h"
#include "IXMLElement.h"
#include "IXMLParser.h"
#include <libxml++/libxml++.h>
#include <memory>
#include <string>

namespace RSM {

/**
 * @brief libxml++ element wrapper
 */
class LibXMLElement : public IXMLElement {
public:
    explicit LibXMLElement(xmlpp::Element *element);

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

    // Internal: Get raw libxml++ element
    xmlpp::Element *getRawElement() const {
        return element_;
    }

private:
    xmlpp::Element *element_;  // Non-owning pointer (owned by xmlpp::Document)
};

/**
 * @brief libxml++ document wrapper
 */
class LibXMLDocument : public IXMLDocument {
public:
    explicit LibXMLDocument(std::shared_ptr<xmlpp::DomParser> parser);

    std::shared_ptr<IXMLElement> getRootElement() override;
    bool processXInclude() override;
    std::string getErrorMessage() const override;
    bool isValid() const override;

private:
    std::shared_ptr<xmlpp::DomParser> parser_;  // Keep parser alive to maintain document
    std::string errorMessage_;
};

/**
 * @brief libxml++ parser implementation (native builds only)
 *
 * Uses libxml++ DOM parser with XInclude support
 */
class LibXMLParser : public IXMLParser {
public:
    LibXMLParser() = default;
    ~LibXMLParser() override = default;

    std::shared_ptr<IXMLDocument> parseFile(const std::string &filename) override;
    std::shared_ptr<IXMLDocument> parseContent(const std::string &content) override;
    std::string getLastError() const override;

private:
    std::string lastError_;
};

}  // namespace RSM

#endif  // !__EMSCRIPTEN__
