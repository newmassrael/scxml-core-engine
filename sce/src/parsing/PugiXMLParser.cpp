// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/PugiXMLParser.h"
#include "core/LogMacros.h"
#include "parsing/IXMLElement.h"
#include "parsing/TemplateError.h"
#include <cstring>
#include <filesystem>
#include <sstream>
#include <unordered_set>

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

namespace {

// `<sce:use>` detection as a DOM walk. pugixml stores element
// names including their namespace prefix, so `sce:use` appears as
// a literal element name string — the check mirrors Rust's
// `content.contains("sce:use")` fast path (`sce-build/src/template.rs`),
// just on the already-parsed tree instead of the raw source.
// Matching on the prefixed form "sce:" deliberately; the AOT
// expander is strict about the sce: namespace binding per
// `claudedocs/rfc-sce-template-sce-param.md` §3, and the runtime
// mirrors that strictness so a document accepted by one path
// binds identically on the other.
bool containsSceUse(pugi::xml_node node) {
    for (const auto &child : node.children()) {
        if (std::string(child.name()) == "sce:use") {
            return true;
        }
        if (containsSceUse(child)) {
            return true;
        }
    }
    return false;
}

}  // namespace

bool PugiXMLDocument::processSceTemplate() {
    // Phase B M1: no-op passthrough only. Full design contract in
    // `claudedocs/rfc-sce-template-phase-b.md`. Any `<sce:use>`
    // element in the document is out of M1 scope; the skeleton
    // throws `SCE::parsing::TemplateNotImplemented` naming the
    // milestone that will support the shape rather than silently
    // passing through an unexpanded `<sce:use>` (which would
    // diverge from AOT behaviour — the failure mode Phase B
    // exists to close).
    if (!doc_) {
        errorMessage_ = "Document is null";
        return false;
    }

    auto root = doc_->document_element();
    if (!root) {
        // Empty document has no `<sce:use>` by definition —
        // trivially passes the passthrough contract.
        return true;
    }

    if (!containsSceUse(root)) {
        SCE_LOG_DEBUG("PugiXMLDocument: processSceTemplate no-op (no <sce:use> present)");
        return true;
    }

    throw SCE::parsing::TemplateNotImplemented(
        "<sce:use> expansion requires Phase B milestone M2 "
        "(claudedocs/rfc-sce-template-phase-b.md). M1 only supports "
        "the no-op passthrough case; any document containing "
        "<sce:use> must currently be compiled through the sce-build "
        "AOT pipeline (see docs/SCE_ACCEPTED_SUBSET.md §2.9).");
}

std::string PugiXMLDocument::resolveFilePath(const std::string &href) const {
    // Use as-is if absolute path
    std::filesystem::path hrefPath(href);
    if (hrefPath.is_absolute()) {
        if (std::filesystem::exists(hrefPath)) {
            return hrefPath.string();
        }
        return "";
    }

    // Try relative to base path
    if (!basePath_.empty()) {
        std::filesystem::path fullPath = std::filesystem::path(basePath_) / href;
        if (std::filesystem::exists(fullPath)) {
            return std::filesystem::absolute(fullPath).string();
        }
    }

    // Try current directory
    if (std::filesystem::exists(href)) {
        return std::filesystem::absolute(href).string();
    }

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

        // Parse file using pugixml
        auto doc = std::make_shared<pugi::xml_document>();
        pugi::xml_parse_result result = doc->load_file(filename.c_str());

        if (!result) {
            lastError_ = "Parse error: " + std::string(result.description());
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }

        // Create document wrapper with base path
        auto wrappedDoc = std::make_shared<PugiXMLDocument>(doc);
        std::filesystem::path filePath(filename);
        wrappedDoc->setBasePath(filePath.parent_path().string());

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

        return std::make_shared<PugiXMLDocument>(doc);

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
