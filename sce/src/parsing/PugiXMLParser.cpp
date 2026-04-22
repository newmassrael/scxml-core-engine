// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/PugiXMLParser.h"
#include "core/LogMacros.h"
#include "parsing/IXMLElement.h"
#include "parsing/TemplateConstants.h"
#include "parsing/TemplateError.h"
#include <cstring>
#include <filesystem>
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

// Collect every `<sce:use>` descendant in document order. Used by
// the splice loop so mutations of the parent/child lists while
// expanding happen against a pre-captured node list; iterating
// `children()` while calling `insert_copy_before` + `remove_child`
// would invalidate the walker.
void collectSceUses(pugi::xml_node node, std::vector<pugi::xml_node> &out) {
    for (const auto &child : node.children()) {
        if (child.type() != pugi::node_element) {
            continue;
        }
        if (std::string(child.name()) == "sce:use") {
            out.push_back(child);
        } else {
            collectSceUses(child, out);
        }
    }
}

// Single-pass `{$name}` substitution, mirroring Rust's
// `apply_substitution_with_tracking` semantics
// (`sce-build/src/template.rs`). No back-references: a param value
// that itself contains `{$other}` is emitted verbatim (RFC §6.1
// literal-only defaults). `{$` sequences that do not match a valid
// declared param are emitted verbatim as source bytes. The C++
// runtime does not track a PositionMap (RFC §1 Q3 defers that to
// M4 conditional).
std::string substituteTokens(std::string_view body,
                             const std::unordered_map<std::string, std::string> &params) {
    std::string out;
    out.reserve(body.size());
    size_t pos = 0;
    while (pos < body.size()) {
        size_t start = body.find("{$", pos);
        if (start == std::string_view::npos) {
            out.append(body.data() + pos, body.size() - pos);
            break;
        }
        out.append(body.data() + pos, start - pos);
        const size_t after = start + 2;
        const size_t end = body.find('}', after);
        if (end != std::string_view::npos) {
            const std::string_view name(body.data() + after, end - after);
            if (SCE::parsing::is_valid_param_name(name)) {
                const auto it = params.find(std::string(name));
                if (it != params.end()) {
                    out.append(it->second);
                    pos = end + 1;
                    continue;
                }
            }
        }
        // Not a valid `{$name}` token — emit `{$` literally and
        // advance past it. The `}` (if any) stays in the stream
        // so adjacent literal braces are preserved byte-for-byte.
        out.append("{$");
        pos = after;
    }
    return out;
}

// Walk a DOM subtree, substituting `{$name}` in every attribute
// value and every pcdata / cdata text node. Elements are recursed.
// Substitution is applied in-place after `insert_copy_before` so
// the caller does not need to build a substituted string tree
// before splicing — cheaper memory and one less serialise step.
void substituteInSubtree(pugi::xml_node node,
                         const std::unordered_map<std::string, std::string> &params) {
    if (node.type() == pugi::node_element) {
        for (auto attr : node.attributes()) {
            const std::string_view raw(attr.value());
            if (raw.find("{$") == std::string_view::npos) {
                continue;
            }
            attr.set_value(substituteTokens(raw, params).c_str());
        }
        for (auto child : node.children()) {
            substituteInSubtree(child, params);
        }
    } else if (node.type() == pugi::node_pcdata || node.type() == pugi::node_cdata) {
        const std::string_view raw(node.value());
        if (raw.find("{$") == std::string_view::npos) {
            return;
        }
        node.set_value(substituteTokens(raw, params).c_str());
    }
}

struct ParamDecl {
    std::string name;
    bool required = false;
    bool hasDefault = false;
    std::string defaultValue;
};

// Detect nested `<sce:use>` inside an expanded template body —
// recursive expansion is the M3 deliverable per RFC §3 M3. Until
// M3 lands, recognise this shape explicitly and raise
// `TemplateNotImplemented` so nested-template authors see a
// pointed diagnostic naming the milestone rather than a silent
// mis-expansion (the exact failure Phase B exists to close).
bool bodyHasNestedSceUse(pugi::xml_node node) {
    if (node.type() != pugi::node_element) {
        return false;
    }
    if (std::string(node.name()) == "sce:use") {
        return true;
    }
    for (auto child : node.children()) {
        if (bodyHasNestedSceUse(child)) {
            return true;
        }
    }
    return false;
}

}  // namespace

bool PugiXMLDocument::processSceTemplate() {
    // Phase B M2: handle single-level `<sce:use>` expansion with
    // parameter substitution. Nested `<sce:use>` inside a template
    // body raises `TemplateNotImplemented` naming M3. Unknown /
    // missing parameters raise the named subtypes added in M2
    // (`TemplateUnknownParam`, `TemplateMissingParam`). Template
    // file load / parse failures and malformed templates raise
    // `TemplateNotImplemented` naming M4 — typed subtypes for those
    // arrive in that milestone per RFC §3 M4 contract. Full design
    // contract: `claudedocs/rfc-sce-template-phase-b.md`.
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

    // Document-order collection decouples traversal from mutation:
    // `insert_copy_before` / `remove_child` can rearrange the tree
    // under us if we were still iterating `children()` directly.
    std::vector<pugi::xml_node> useNodes;
    collectSceUses(root, useNodes);

    for (auto &useNode : useNodes) {
        expandSceUse(useNode);
    }

    return true;
}

void PugiXMLDocument::expandSceUse(pugi::xml_node useNode) {
    // 1. Caller must carry a `template` attribute. Empty-string is
    // equivalent to missing per Rust's
    // `TemplateError::MissingTemplateAttribute` semantics; the
    // typed C++ subtype `TemplateMissingAttribute` arrives in M4.
    const auto templateAttr = useNode.attribute("template");
    const std::string templateHref = templateAttr ? templateAttr.value() : std::string();
    if (templateHref.empty()) {
        throw SCE::parsing::TemplateNotImplemented(
            "<sce:use> missing or empty `template` attribute — typed "
            "SCE::parsing::TemplateMissingAttribute arrives in Phase B "
            "M4 (claudedocs/rfc-sce-template-phase-b.md §3 M4).");
    }

    // 2. Resolve the template path. For the M2 success path this is
    // a best-effort match against basePath_ + cwd; M4 adds the typed
    // `TemplateNotFound` variant carrying the search trail.
    const std::string resolvedPath = resolveFilePath(templateHref);
    if (resolvedPath.empty()) {
        throw SCE::parsing::TemplateNotImplemented(
            "<sce:use template=\"" + templateHref + "\">: template file "
            "could not be located. Typed SCE::parsing::TemplateNotFound "
            "arrives in Phase B M4.");
    }

    // 3. Load + parse the template file.
    pugi::xml_document templateDoc;
    const auto loadResult = templateDoc.load_file(resolvedPath.c_str());
    if (!loadResult) {
        throw SCE::parsing::TemplateNotImplemented(
            "<sce:use template=\"" + templateHref + "\">: template file "
            "failed to load (" + loadResult.description() + "). Typed "
            "SCE::parsing::TemplateReadError / TemplateMalformed arrive "
            "in Phase B M4.");
    }

    const auto templateRoot = templateDoc.document_element();
    if (!templateRoot || std::string(templateRoot.name()) != "sce:template") {
        throw SCE::parsing::TemplateNotImplemented(
            "<sce:use template=\"" + templateHref + "\">: template root "
            "must be <sce:template>. Typed SCE::parsing::TemplateMalformed "
            "arrives in Phase B M4.");
    }

    // 4. Collect `<sce:param>` declarations.
    std::vector<ParamDecl> decls;
    for (auto child : templateRoot.children()) {
        if (child.type() != pugi::node_element) {
            continue;
        }
        if (std::string(child.name()) != "sce:param") {
            continue;
        }
        const auto nameAttr = child.attribute("name");
        if (!nameAttr) {
            throw SCE::parsing::TemplateNotImplemented(
                "<sce:param> missing `name` attribute in template \"" +
                templateHref + "\". Typed SCE::parsing::TemplateMalformed "
                "arrives in Phase B M4.");
        }
        const std::string paramName = nameAttr.value();
        if (!SCE::parsing::is_valid_param_name(paramName)) {
            throw SCE::parsing::TemplateNotImplemented(
                "<sce:param name=\"" + paramName + "\"> in template \"" +
                templateHref + "\": name must match " +
                std::string(SCE::parsing::PARAM_NAME_PATTERN) +
                ". Typed SCE::parsing::TemplateMalformed arrives in "
                "Phase B M4.");
        }
        for (const auto &prev : decls) {
            if (prev.name == paramName) {
                throw SCE::parsing::TemplateNotImplemented(
                    "<sce:param name=\"" + paramName + "\">: duplicate "
                    "declaration in template \"" + templateHref + "\". "
                    "Typed SCE::parsing::TemplateMalformed arrives in "
                    "Phase B M4.");
            }
        }

        ParamDecl decl;
        decl.name = paramName;
        if (const auto reqAttr = child.attribute("required")) {
            const std::string reqVal = reqAttr.value();
            if (reqVal == "true") {
                decl.required = true;
            } else if (reqVal != "false") {
                throw SCE::parsing::TemplateNotImplemented(
                    "<sce:param name=\"" + paramName + "\"> `required` "
                    "must be \"true\" or \"false\", got \"" + reqVal +
                    "\". Typed SCE::parsing::TemplateMalformed arrives "
                    "in Phase B M4.");
            }
        }
        if (const auto defAttr = child.attribute("default")) {
            decl.hasDefault = true;
            decl.defaultValue = defAttr.value();
        }
        if (decl.required && decl.hasDefault) {
            throw SCE::parsing::TemplateNotImplemented(
                "<sce:param name=\"" + paramName + "\"> declares both "
                "`required=\"true\"` and `default=\"...\"` — mutually "
                "exclusive. Typed SCE::parsing::TemplateMalformed "
                "arrives in Phase B M4.");
        }
        decls.push_back(std::move(decl));
    }

    // 5. Gather caller bindings. Skip `template` and `xmlns`/`xmlns:*`
    // — pugixml treats namespace declarations as regular attributes,
    // so the bindings set must filter them out explicitly.
    std::unordered_map<std::string, std::string> callerBindings;
    for (const auto attr : useNode.attributes()) {
        const std::string attrName = attr.name();
        if (attrName == "template") {
            continue;
        }
        if (attrName == "xmlns" ||
            (attrName.size() > 6 && attrName.compare(0, 6, "xmlns:") == 0)) {
            continue;
        }
        callerBindings.emplace(attrName, attr.value());
    }

    // 6. Every caller binding must name a declared param. Mirrors
    // Rust's UnknownParam classification; this is the M2 typed throw.
    for (const auto &kv : callerBindings) {
        bool declared = false;
        for (const auto &d : decls) {
            if (d.name == kv.first) {
                declared = true;
                break;
            }
        }
        if (declared) {
            continue;
        }
        std::string declaredList;
        for (const auto &d : decls) {
            if (!declaredList.empty()) {
                declaredList.append(", ");
            }
            declaredList.append(d.name);
        }
        if (declaredList.empty()) {
            declaredList = "<none>";
        }
        throw SCE::parsing::TemplateUnknownParam(
            "<sce:use template=\"" + templateHref + "\">: unknown "
            "parameter '" + kv.first + "' (declared: " + declaredList +
            ")");
    }

    // 7. Bind: caller value > default > empty. Missing required is
    // the second M2 typed throw.
    std::unordered_map<std::string, std::string> params;
    params.reserve(decls.size());
    for (const auto &d : decls) {
        const auto it = callerBindings.find(d.name);
        if (it != callerBindings.end()) {
            params.emplace(d.name, it->second);
            continue;
        }
        if (d.required) {
            throw SCE::parsing::TemplateMissingParam(
                "<sce:use template=\"" + templateHref + "\">: missing "
                "required parameter '" + d.name + "'");
        }
        params.emplace(d.name, d.hasDefault ? d.defaultValue : std::string());
    }

    // 8. Recursive expansion is M3. Surface any body element that
    // contains `<sce:use>` (directly or transitively) with a typed
    // sentinel so the failure mode is pointed rather than silent.
    for (auto child : templateRoot.children()) {
        if (child.type() != pugi::node_element) {
            continue;
        }
        if (std::string(child.name()) == "sce:param") {
            continue;
        }
        if (bodyHasNestedSceUse(child)) {
            throw SCE::parsing::TemplateNotImplemented(
                "<sce:use template=\"" + templateHref + "\">: template "
                "body contains nested <sce:use>; recursive expansion "
                "arrives in Phase B M3 "
                "(claudedocs/rfc-sce-template-phase-b.md §3 M3).");
        }
    }

    // 9. Splice body children (non-param) into the caller parent in
    // place of the `<sce:use>` node, substituting `{$name}` in the
    // copy. `insert_copy_before` clones into the caller document so
    // `templateDoc` can go out of scope; `substituteInSubtree`
    // mutates the clone, leaving `templateDoc` untouched in case a
    // future fixture asserts template-file reuse semantics.
    auto callerParent = useNode.parent();
    if (!callerParent) {
        throw SCE::parsing::TemplateNotImplemented(
            "<sce:use> at document root has no parent to splice into; "
            "root-level template invocation is out of scope until "
            "Phase B M4 root-splice handling lands.");
    }
    for (auto child : templateRoot.children()) {
        if (child.type() == pugi::node_element &&
            std::string(child.name()) == "sce:param") {
            continue;
        }
        auto inserted = callerParent.insert_copy_before(child, useNode);
        substituteInSubtree(inserted, params);
    }
    callerParent.remove_child(useNode);
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
