// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLDocument.h"
#include "IXMLElement.h"
#include "IXMLParser.h"
#include <memory>
#include <pugixml.hpp>
#include <string>

namespace SCE {

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
    bool processSceTemplate() override;
    std::string getErrorMessage() const override;
    bool isValid() const override;

    // Internal: Set base path for XInclude resolution
    void setBasePath(const std::string &basePath) {
        basePath_ = basePath;
    }

    // Internal: Set the absolute source path of the document this
    // wrapper was loaded from. Consumed by `processSceTemplate` to
    // seed the cycle-detection stack so a top-level document that
    // says `<sce:use template="self.scxml"/>` (pointing back at
    // itself) is caught before loading the template file the second
    // time. In-memory documents (parseContent) leave sourcePath_
    // empty; cycle detection then only trips once the stack has
    // accumulated at least one loaded template. Mirrors Rust's
    // `expand(self_path, ...)` parameter in `sce-build/src/template.rs`.
    void setSourcePath(const std::string &sourcePath) {
        sourcePath_ = sourcePath;
    }

private:
    bool processXIncludeRecursive(pugi::xml_node node, int depth = 0);
    std::string resolveFilePath(const std::string &href) const;

    // Expand a single `<sce:use>` node in place. Loads the template
    // file, validates params against the caller's attribute set,
    // substitutes `{$name}` tokens in the cloned body, and splices
    // the result into the caller parent before removing the original
    // `<sce:use>`. Throws `SCE::parsing::TemplateError` (or one of
    // its subtypes) on any failure — callers plumb the exception up
    // through `processSceTemplate` for `SCXMLParser::parseFile` to
    // convert into `addError` messages.
    void expandSceUse(pugi::xml_node useNode);

    std::shared_ptr<pugi::xml_document> doc_;
    std::string errorMessage_;
    std::string basePath_;
    std::string sourcePath_;
    static constexpr int MAX_XINCLUDE_DEPTH = 10;
};

/**
 * @brief pugixml parser implementation (All platforms)
 *
 * Lightweight, W3C compliant XML parser with manual XInclude implementation
 * Unified XML parser for all platforms (Native and WASM)
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

}  // namespace SCE
