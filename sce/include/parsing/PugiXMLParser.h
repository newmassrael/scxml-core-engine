// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "IXMLDocument.h"
#include "IXMLElement.h"
#include "IXMLParser.h"
#include <filesystem>
#include <memory>
#include <pugixml.hpp>
#include <string>
#include <vector>

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
    XIncludeResult processXInclude() override;
    SceTemplateResult processSceTemplate(
        const SCE::parsing::PositionMap &upstream) override;
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

    // Internal: Set the raw source text this wrapper was loaded
    // from. Consumed by `processSceTemplate` to seed the
    // `SCE::parsing::PositionMap` identity entry so template
    // expansion can report diagnostic coordinates in the author's
    // source file rather than in the post-expansion in-memory
    // document. Mirrors Rust's `content` parameter to
    // `sce-build/src/template.rs::expand`. `parseFile` reads the
    // file text before handing it to pugixml so the member captures
    // the exact bytes parsed; `parseContent` captures its input
    // string verbatim.
    void setSourceText(const std::string &sourceText) {
        sourceText_ = sourceText;
    }

    // Internal: Return the raw source text captured at load time.
    // Used by `processSceTemplate` to construct the identity
    // `PositionMap`. Empty when the wrapper was constructed without
    // a `setSourceText` call (legacy callers); consumers guard
    // accordingly.
    const std::string &sourceText() const {
        return sourceText_;
    }

private:
    // Resolve an `<sce:use template="href">` value against an explicit
    // base directory. Mirrors `resolveFilePath` but takes baseDir as a
    // parameter so the recursive expander can resolve nested
    // `<sce:use>` inside a template body against the TEMPLATE's
    // directory, not the outer document's `basePath_`. Returns the
    // absolute path on success, empty string on not-found.
    //
    // The out-param overload additionally records the search trail —
    // the paths that were checked and did not exist — so the Phase B
    // M4 `TemplateNotFound` throw site can render the same
    // comma-separated trail Rust emits via
    // `resolve_template_path`'s `tried` vector in
    // `sce-build/src/template.rs`. Callers that do not need the
    // trail use the single-arg form; the two forms share the
    // underlying resolver.
    static std::string resolveFilePathInBase(const std::string &href,
                                             const std::string &baseDir);
    static std::string resolveFilePathInBase(const std::string &href,
                                             const std::string &baseDir,
                                             std::vector<std::string> &searched);

    std::shared_ptr<pugi::xml_document> doc_;
    std::string errorMessage_;
    std::string basePath_;
    std::string sourcePath_;
    std::string sourceText_;
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
