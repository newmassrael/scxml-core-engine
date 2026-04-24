// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/PugiXMLParser.h"
#include "core/LogMacros.h"
#include "parsing/IXMLElement.h"
#include "parsing/TemplateConstants.h"
#include "parsing/TemplateError.h"
#include "parsing/TemplateExpander.h"
#include <cstring>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string_view>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace SCE {

// ============================================================================
// PugiXMLElement implementation
// ============================================================================

PugiXMLElement::PugiXMLElement(pugi::xml_node node, std::shared_ptr<pugi::xml_document> doc) : node_(node), doc_(doc) {}

std::string PugiXMLElement::getName() const {
    if (!node_) {
        return "";
    }
    return node_.name();
}

std::string PugiXMLElement::getAttribute(const std::string &name) const {
    if (!node_) {
        return "";
    }

    auto attr = node_.attribute(name.c_str());
    if (attr) {
        return attr.value();
    }
    return "";
}

bool PugiXMLElement::hasAttribute(const std::string &name) const {
    if (!node_) {
        return false;
    }
    return node_.attribute(name.c_str()) != nullptr;
}

std::unordered_map<std::string, std::string> PugiXMLElement::getAttributes() const {
    std::unordered_map<std::string, std::string> result;

    if (!node_) {
        return result;
    }

    for (const auto &attr : node_.attributes()) {
        result[attr.name()] = attr.value();
    }

    return result;
}

std::string PugiXMLElement::getNamespace() const {
    if (!node_) {
        return "";
    }

    // WASM limitation: pugixml doesn't support namespace URIs directly
    // W3C SCXML: Namespace support not required for current test suite
    // Future: Implement xmlns extraction if WASM builds require namespace-aware parsing
    return "";
}

std::vector<std::shared_ptr<IXMLElement>> PugiXMLElement::getChildren() const {
    std::vector<std::shared_ptr<IXMLElement>> result;

    if (!node_) {
        return result;
    }

    for (const auto &child : node_.children()) {
        if (child.type() == pugi::node_element) {
            result.push_back(std::make_shared<PugiXMLElement>(child, doc_));
        }
    }

    return result;
}

std::vector<std::shared_ptr<IXMLElement>> PugiXMLElement::getChildrenByTagName(const std::string &tagName) const {
    std::vector<std::shared_ptr<IXMLElement>> result;

    if (!node_) {
        return result;
    }

    for (const auto &child : node_.children(tagName.c_str())) {
        result.push_back(std::make_shared<PugiXMLElement>(child, doc_));
    }

    return result;
}

std::string PugiXMLElement::getTextContent() const {
    if (!node_) {
        return "";
    }

    // First try direct text child
    auto textNode = node_.child_value();
    if (textNode && strlen(textNode) > 0) {
        return textNode;
    }

    // If no direct text, get inner XML (handles <cpp>...</cpp> etc.)
    std::ostringstream ss;
    for (const auto &child : node_.children()) {
        child.print(ss, "", pugi::format_raw);
    }
    return ss.str();
}

bool PugiXMLElement::importNode(const std::shared_ptr<IXMLElement> &source) {
    if (!node_ || !source) {
        return false;
    }

    auto *pugiSource = dynamic_cast<PugiXMLElement *>(source.get());
    if (!pugiSource) {
        SCE_LOG_ERROR("PugiXMLElement::importNode - Source is not PugiXMLElement");
        return false;
    }

    try {
        // pugixml: append_copy returns the new node
        auto sourceNode = pugiSource->getRawNode();
        for (const auto &child : sourceNode.children()) {
            node_.append_copy(child);
        }
        return true;
    } catch (const std::exception &ex) {
        SCE_LOG_ERROR("PugiXMLElement::importNode - {}", ex.what());
        return false;
    }
}

bool PugiXMLElement::remove() {
    if (!node_) {
        return false;
    }

    try {
        auto parent = node_.parent();
        if (parent) {
            parent.remove_child(node_);
            node_ = pugi::xml_node();  // Invalidate
            return true;
        }
        return false;
    } catch (const std::exception &ex) {
        SCE_LOG_ERROR("PugiXMLElement::remove - {}", ex.what());
        return false;
    }
}

std::shared_ptr<IXMLElement> PugiXMLElement::getParent() const {
    if (!node_) {
        return nullptr;
    }

    auto parent = node_.parent();
    if (parent && parent.type() == pugi::node_element) {
        return std::make_shared<PugiXMLElement>(parent, doc_);
    }

    return nullptr;
}

std::string PugiXMLElement::serializeChildContent() const {
    if (!node_) {
        return "";
    }

    // W3C SCXML B.2: Full XML serialization preserving structure
    // Use pugixml's print() to serialize all child nodes
    std::ostringstream oss;

    for (const auto &child : node_.children()) {
        // pugi::format_raw: No indentation, no line breaks (compact serialization)
        child.print(oss, "", pugi::format_raw);
    }

    return oss.str();
}

// ============================================================================
// PugiXMLDocument implementation
// ============================================================================

PugiXMLDocument::PugiXMLDocument(std::shared_ptr<pugi::xml_document> doc) : doc_(doc) {}

std::shared_ptr<IXMLElement> PugiXMLDocument::getRootElement() {
    if (!doc_) {
        return nullptr;
    }

    auto root = doc_->document_element();
    if (!root) {
        return nullptr;
    }

    return std::make_shared<PugiXMLElement>(root, doc_);
}

bool PugiXMLDocument::processXInclude() {
    if (!doc_) {
        errorMessage_ = "Document is null";
        return false;
    }

    try {
        // W3C XInclude: Manual implementation for pugixml
        auto root = doc_->document_element();
        if (!root) {
            errorMessage_ = "Document has no root element";
            return false;
        }

        bool success = processXIncludeRecursive(root, 0);
        if (success) {
            SCE_LOG_DEBUG("PugiXMLDocument: XInclude processing successful");
        }
        return success;

    } catch (const std::exception &ex) {
        errorMessage_ = "XInclude processing failed: " + std::string(ex.what());
        SCE_LOG_WARN("PugiXMLDocument: {}", errorMessage_);
        return false;
    }
}

bool PugiXMLDocument::processXIncludeRecursive(pugi::xml_node node, int depth) {
    if (depth >= MAX_XINCLUDE_DEPTH) {
        SCE_LOG_WARN("PugiXMLDocument: Maximum XInclude depth reached");
        return false;
    }

    // Find all xi:include elements
    std::vector<pugi::xml_node> includeNodes;
    for (const auto &child : node.children()) {
        std::string nodeName = child.name();
        if (nodeName == "include" || nodeName == "xi:include") {
            includeNodes.push_back(child);
        } else if (child.type() == pugi::node_element) {
            // Recursively process children
            processXIncludeRecursive(child, depth + 1);
        }
    }

    // Process each xi:include
    for (const auto &includeNode : includeNodes) {
        auto hrefAttr = includeNode.attribute("href");
        if (!hrefAttr) {
            SCE_LOG_WARN("PugiXMLDocument: xi:include missing href attribute");
            continue;
        }

        std::string href = hrefAttr.value();
        if (href.empty()) {
            SCE_LOG_WARN("PugiXMLDocument: xi:include href is empty");
            continue;
        }

        // Resolve file path
        std::string fullPath = resolveFilePath(href);
        if (fullPath.empty()) {
            SCE_LOG_ERROR("PugiXMLDocument: Could not resolve file path: {}", href);
            continue;
        }

        SCE_LOG_DEBUG("PugiXMLDocument: Loading XInclude: {}", fullPath);

        // Load included document
        auto includedDoc = std::make_shared<pugi::xml_document>();
        pugi::xml_parse_result result = includedDoc->load_file(fullPath.c_str());

        if (!result) {
            SCE_LOG_ERROR("PugiXMLDocument: Failed to parse included file: {} - {}", fullPath, result.description());
            continue;
        }

        // Recursively process XIncludes in included document
        auto includedRoot = includedDoc->document_element();
        if (includedRoot) {
            processXIncludeRecursive(includedRoot, depth + 1);
        }

        // Import all children of included root into parent
        auto parent = includeNode.parent();
        if (includedRoot) {
            for (const auto &child : includedRoot.children()) {
                parent.insert_copy_before(child, includeNode);
            }
        }

        // Remove xi:include node
        parent.remove_child(includeNode);
    }

    return true;
}

SceTemplateResult PugiXMLDocument::processSceTemplate() {
    // String-level `<sce:use>` expansion. Serialises the current
    // (post-XInclude) DOM, hands it to `SCE::parsing::expandString`
    // which mirrors `sce-build/src/template.rs::expand`, then
    // reparses the expanded text back into `doc_` so downstream
    // validation sees the expanded tree. The returned PositionMap
    // tracks every emitted byte back to File/CallSite origins for
    // diagnostic remapping (RFC §3 P2, see
    // claudedocs/rfc-sce-template-phase-c.md).
    //
    // Error classification flows through `TemplateError` subtypes
    // thrown by the expander (TemplateMissingAttribute,
    // TemplateNotFound, TemplateReadError, TemplateMalformed,
    // TemplateUnknownParam, TemplateMissingParam, TemplateCycle,
    // TemplateTooDeep). `SCXMLParser::parseFile`'s std::exception
    // catch-all collects the message via addError.
    SceTemplateResult result;
    if (!doc_) {
        errorMessage_ = "Document is null";
        return result;
    }

    // Fast path: documents whose captured source contains no
    // `<sce:use>` bypass serialise/reparse entirely, preserving the
    // existing DOM pointers. Returns an identity PositionMap over
    // the author's raw source bytes so diagnostic lookups resolve
    // to (file, row, col) without a post-normalisation skew.
    if (!sourceText_.empty() &&
        sourceText_.find("sce:use") == std::string::npos) {
        result.ok = true;
        result.positions = SCE::parsing::PositionMap::identity(
            std::filesystem::path(sourcePath_), sourceText_);
        return result;
    }

    // Coordinate space selection for `expandString`'s identity
    // map. Two cases:
    //
    //   (a) `sourceText_` is available AND no `<xi:include>` element
    //       is present in the author's bytes — then `processXInclude`
    //       above was a structural no-op, the DOM matches
    //       `sourceText_` byte-for-byte, and feeding the expander the
    //       author bytes directly preserves author-source (row, col)
    //       through every `PositionMap::lookup` downstream. This is
    //       the path that Phase C P3 coord-parity fixtures depend on
    //       (see claudedocs/rfc-sce-template-phase-c.md §3 P3).
    //
    //   (b) `sourceText_` is unavailable OR the document actually
    //       uses `<xi:include>` — fall back to serialising the
    //       post-XInclude DOM through `format_raw`, accepting the
    //       documented xinclude-fragment coordinate skew (Phase X
    //       closes this, RFC §3 P2 item #5).
    //
    // The check uses substring search rather than DOM traversal to
    // stay cheap on the hot path; `xi:include` is prefixed with
    // the reserved XInclude namespace and so no false positives
    // appear in author content outside of actual include sites.
    std::string content;
    const bool useAuthorBytes =
        !sourceText_.empty() &&
        sourceText_.find("xi:include") == std::string::npos;
    if (useAuthorBytes) {
        content = sourceText_;
    } else {
        std::ostringstream serialised;
        doc_->save(serialised, "", pugi::format_raw | pugi::format_no_declaration);
        content = serialised.str();
    }

    // Secondary fast path: the serialised post-XInclude text may
    // also lack `<sce:use>` if XInclude neither introduced nor
    // preserved one. Identity over the serialised bytes gets
    // PositionMap lookups pointing at the expanded DOM without a
    // second expander invocation.
    if (content.find("sce:use") == std::string::npos) {
        result.ok = true;
        result.positions = SCE::parsing::PositionMap::identity(
            std::filesystem::path(sourcePath_), content);
        return result;
    }

    auto expanded =
        SCE::parsing::expandString(content, sourcePath_, basePath_);

    // Reparse into the same shared_ptr'd document so every
    // `IXMLElement` the caller has already retrieved continues to
    // see the expanded tree. `xml_document::reset()` clears without
    // deallocating the owning shared_ptr, then load_buffer populates
    // it with the expanded bytes.
    doc_->reset();
    const auto parseResult = doc_->load_buffer(
        expanded.expanded_text.data(), expanded.expanded_text.size());
    if (!parseResult) {
        errorMessage_ = "Failed to reparse expanded template: " +
                        std::string(parseResult.description());
        return result;
    }

    result.ok = true;
    result.positions = std::move(expanded.positions);
    return result;
}

std::string PugiXMLDocument::resolveFilePath(const std::string &href) const {
    // Member-bound convenience wrapper around the base-directory
    // static helper. Used by `processXInclude`; the template
    // expander routes through `resolveFilePathInBase` directly so
    // nested recursion can pass a template-local base directory
    // without mutating `basePath_`.
    return resolveFilePathInBase(href, basePath_);
}

std::string PugiXMLDocument::resolveFilePathInBase(const std::string &href,
                                                    const std::string &baseDir) {
    std::vector<std::string> discarded;
    return resolveFilePathInBase(href, baseDir, discarded);
}

std::string PugiXMLDocument::resolveFilePathInBase(const std::string &href,
                                                    const std::string &baseDir,
                                                    std::vector<std::string> &searched) {
    // Mirrors `sce-build/src/template.rs::resolve_template_path`: each
    // branch that checks `exists()` appends the candidate to
    // `searched` on miss, so the NotFound diagnostic carries the same
    // trail the Rust side emits (absolute → base → cwd).
    std::filesystem::path hrefPath(href);
    if (hrefPath.is_absolute()) {
        if (std::filesystem::exists(hrefPath)) {
            return hrefPath.string();
        }
        searched.push_back(hrefPath.string());
        return "";
    }

    // Try relative to base directory
    if (!baseDir.empty()) {
        std::filesystem::path fullPath = std::filesystem::path(baseDir) / href;
        if (std::filesystem::exists(fullPath)) {
            return std::filesystem::absolute(fullPath).string();
        }
        searched.push_back(fullPath.string());
    }

    // Try current directory
    if (std::filesystem::exists(href)) {
        return std::filesystem::absolute(href).string();
    }
    searched.push_back(href);

    return "";
}

std::string PugiXMLDocument::getErrorMessage() const {
    return errorMessage_;
}

bool PugiXMLDocument::isValid() const {
    return doc_ != nullptr && doc_->document_element();
}

// ============================================================================
// PugiXMLParser implementation
// ============================================================================

std::shared_ptr<IXMLDocument> PugiXMLParser::parseFile(const std::string &filename) {
    try {
        // Check if file exists
        if (!std::filesystem::exists(filename)) {
            lastError_ = "File not found: " + filename;
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }

        SCE_LOG_INFO("PugiXMLParser: Parsing file: {}", filename);

        // Read the file into an in-memory buffer first so the
        // `PugiXMLDocument` wrapper can stash the exact bytes pugixml
        // parsed — `processSceTemplate` needs a stable view of the
        // author's source text to build a `SCE::parsing::PositionMap`
        // identity entry (RFC §3 P2). `load_buffer` parses directly
        // out of that buffer without a second file read, keeping the
        // cached `sourceText_` byte-identical to what pugixml saw.
        std::ifstream in(filename, std::ios::binary);
        if (!in) {
            lastError_ = "Cannot open file: " + filename;
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }
        std::ostringstream buffer;
        buffer << in.rdbuf();
        std::string sourceText = buffer.str();

        auto doc = std::make_shared<pugi::xml_document>();
        pugi::xml_parse_result result =
            doc->load_buffer(sourceText.data(), sourceText.size());

        if (!result) {
            lastError_ = "Parse error: " + std::string(result.description());
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }

        // Create document wrapper with base path + source path +
        // source text. Base path feeds `<xi:include>` / `<sce:use
        // template>` resolution; source path seeds the
        // cycle-detection stack in `processSceTemplate` so a top-level
        // document that references itself via `<sce:use
        // template="self.scxml"/>` is caught before loading the
        // template file a second time; source text seeds the
        // PositionMap identity entry so post-expansion diagnostics
        // can be remapped to (file, row, col) in the author's
        // source. Mirrors Rust's `sce-build/src/template.rs::expand
        // (content, self_path, ...)` plumbing at the parser boundary
        // so the C++ runtime has the same self-reference +
        // coordinate-remap plumbing as the AOT expander.
        auto wrappedDoc = std::make_shared<PugiXMLDocument>(doc);
        std::filesystem::path filePath(filename);
        wrappedDoc->setBasePath(filePath.parent_path().string());
        wrappedDoc->setSourcePath(filename);
        wrappedDoc->setSourceText(sourceText);

        return wrappedDoc;

    } catch (const std::exception &ex) {
        lastError_ = "Exception while parsing file: " + std::string(ex.what());
        SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
        return nullptr;
    }
}

std::shared_ptr<IXMLDocument> PugiXMLParser::parseContent(const std::string &content) {
    try {
        SCE_LOG_INFO("PugiXMLParser: Parsing content");

        // Parse from string using pugixml
        auto doc = std::make_shared<pugi::xml_document>();
        pugi::xml_parse_result result = doc->load_string(content.c_str());

        if (!result) {
            lastError_ = "Parse error: " + std::string(result.description());
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }

        // Capture the caller's content verbatim so `processSceTemplate`
        // can build a PositionMap identity entry keyed by the same
        // bytes pugixml parsed. `parseContent` does not receive a
        // source path (in-memory documents are anonymous), so only
        // sourceText_ is set; the absent sourcePath_ leaves cycle
        // detection dormant until a nested template load introduces
        // a real path into the stack.
        auto wrappedDoc = std::make_shared<PugiXMLDocument>(doc);
        wrappedDoc->setSourceText(content);

        return wrappedDoc;

    } catch (const std::exception &ex) {
        lastError_ = "Exception while parsing content: " + std::string(ex.what());
        SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
        return nullptr;
    }
}

std::string PugiXMLParser::getLastError() const {
    return lastError_;
}

}  // namespace SCE
