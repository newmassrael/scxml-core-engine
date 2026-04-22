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

// Canonicalise a resolved template path for cycle-stack membership
// checks. `std::filesystem::canonical` dereferences symlinks and
// resolves relative components, so `./foo.xml`, `../dir/foo.xml`,
// and `foo.xml` all reduce to the same key. On error (file no
// longer exists at the moment we canonicalise, race with a
// deletion, etc.) we fall back to the resolved path unchanged so
// membership comparison still trips for the common case of
// repeated identical strings — mirrors Rust's
// `std::fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone())`
// in `sce-build/src/template.rs`.
std::filesystem::path canonicaliseTemplatePath(const std::string &resolvedPath) {
    std::error_code ec;
    auto canon = std::filesystem::canonical(resolvedPath, ec);
    if (ec) {
        return std::filesystem::path(resolvedPath);
    }
    return canon;
}

// Render a cycle chain as `outer -> inner -> ...` for the
// `TemplateCycle` message. `stack` is the current recursion path
// (caller-file at index 0, then each nested template in order);
// `next` is the template whose expansion would have reopened the
// cycle. The rendered string matches Rust's `render_chain` in
// `sce-build/src/template.rs` so cross-language diagnostic
// consumers (agents, CI parsers) can key on the same separator
// convention.
std::string renderTemplateChain(const std::vector<std::filesystem::path> &stack,
                                const std::filesystem::path &next) {
    // The arrow glyph (U+2192) matches Rust's " → " separator.
    static const std::string arrow = " \xe2\x86\x92 ";
    std::string out;
    for (const auto &entry : stack) {
        if (!out.empty()) {
            out.append(arrow);
        }
        out.append(entry.string());
    }
    if (!out.empty()) {
        out.append(arrow);
    }
    out.append(next.string());
    return out;
}

}  // namespace

bool PugiXMLDocument::processSceTemplate() {
    // Phase B M3: recursive `<sce:use>` expansion with parameter
    // substitution, cycle detection, and depth enforcement. Mirrors
    // `sce-build/src/template.rs::expand` by seeding a canonical-path
    // stack with the caller document (so a top-level self-reference
    // trips immediately), then recursing into every loaded template
    // until the leaf is reached or `MAX_TEMPLATE_DEPTH` is hit.
    //
    // Error classification (M4 closes the typed-subtype mapping):
    //   - `TemplateMissingAttribute` (M4) for a call-site `<sce:use>`
    //     with no `template` attribute or an empty string.
    //   - `TemplateNotFound` (M4) for resolver miss, with the search
    //     trail attached verbatim to the Rust `resolve_template_path`
    //     `tried` rendering.
    //   - `TemplateReadError` / `TemplateMalformed` (M4) split file-
    //     load failures by pugixml status: I/O-class statuses
    //     (`status_file_not_found`, `status_io_error`,
    //     `status_out_of_memory`, `status_internal_error`) route to
    //     `TemplateReadError`; every other non-OK status is a parse
    //     failure and routes to `TemplateMalformed`.
    //   - `TemplateMalformed` (M4) also covers structural errors on
    //     the template side: wrong root element, `<sce:param>`
    //     missing `name`, invalid name pattern, duplicate name, bad
    //     `required` value, or `required` + `default` declared
    //     together.
    //   - `TemplateUnknownParam` / `TemplateMissingParam` (M2) for
    //     call-site parameter mismatches.
    //   - `TemplateCycle` (M3) for self- or mutually-recursive
    //     templates.
    //   - `TemplateTooDeep` (M3) for acyclic but pathologically long
    //     chains.
    //
    // After M4, every throw in this function is a proper named
    // subtype — `TemplateNotImplemented` remains only as the base-
    // class sentinel declared in `TemplateError.h`, and M5 deletes
    // it once the `docs/SCE_ACCEPTED_SUBSET.md` §2.9 flip lands.
    //
    // Full design contract: `claudedocs/rfc-sce-template-phase-b.md`.
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

    // Seed the cycle-detection stack with the caller document's
    // own canonical path so a top-level `<sce:use template="self"/>`
    // is trapped before we re-load the same file. In-memory parses
    // (parseContent) leave `sourcePath_` empty; the stack then begins
    // empty and trips on the first templates that reopen later. This
    // matches Rust's behaviour when callers pass an in-memory label
    // through `expand(self_path, ...)`.
    std::vector<std::filesystem::path> stack;
    if (!sourcePath_.empty()) {
        stack.push_back(canonicaliseTemplatePath(sourcePath_));
    }

    expandAllUsesInTree(root, basePath_, stack, 0);
    return true;
}

void PugiXMLDocument::expandAllUsesInTree(pugi::xml_node root,
                                          const std::string &baseDir,
                                          std::vector<std::filesystem::path> &stack,
                                          int depth) {
    // Depth gate fires at entry per Rust `expand_impl` semantics —
    // a chain of `MAX_TEMPLATE_DEPTH` nested templates hits the
    // `depth >= MAX_TEMPLATE_DEPTH` check on the deepest recursion
    // attempt and is rejected before another template is loaded.
    if (depth >= SCE::parsing::MAX_TEMPLATE_DEPTH) {
        throw SCE::parsing::TemplateTooDeep(
            "<sce:use> template nesting exceeds depth limit of " +
            std::to_string(SCE::parsing::MAX_TEMPLATE_DEPTH));
    }

    // Document-order collection decouples traversal from mutation:
    // `insert_copy_before` / `remove_child` on the caller side
    // during `expandSceUse` can rearrange the tree under us if we
    // iterated `children()` directly. Pre-collecting captures the
    // top-level uses before any splicing begins.
    std::vector<pugi::xml_node> useNodes;
    collectSceUses(root, useNodes);

    for (auto &useNode : useNodes) {
        expandSceUse(useNode, baseDir, stack, depth);
    }
}

void PugiXMLDocument::expandSceUse(pugi::xml_node useNode,
                                    const std::string &baseDir,
                                    std::vector<std::filesystem::path> &stack,
                                    int depth) {
    // 1. Caller must carry a `template` attribute. Empty-string is
    // equivalent to missing per Rust's
    // `TemplateError::MissingTemplateAttribute` semantics.
    const auto templateAttr = useNode.attribute("template");
    const std::string templateHref = templateAttr ? templateAttr.value() : std::string();
    if (templateHref.empty()) {
        throw SCE::parsing::TemplateMissingAttribute(
            "<sce:use> missing required `template` attribute");
    }

    // 2. Resolve the template path against the caller's base
    // directory. The member `basePath_` holds the outer document's
    // base; the recursion passes template-local base directories
    // through `baseDir` so a nested `<sce:use>` inside a template
    // body resolves relative to the TEMPLATE's location, matching
    // Rust's `nested_base = resolved.parent()` in
    // `sce-build/src/template.rs::expand_impl`. The search-trail
    // overload captures the paths that were tried so the NotFound
    // diagnostic renders the same comma-separated list Rust emits.
    std::vector<std::string> searchedPaths;
    const std::string resolvedPath =
        resolveFilePathInBase(templateHref, baseDir, searchedPaths);
    if (resolvedPath.empty()) {
        std::string searchedRendered;
        for (const auto &path : searchedPaths) {
            if (!searchedRendered.empty()) {
                searchedRendered.append(", ");
            }
            searchedRendered.append(path);
        }
        throw SCE::parsing::TemplateNotFound(
            "<sce:use template=\"" + templateHref + "\">: file not "
            "found (searched: " + searchedRendered + ")");
    }

    // 2b. Cycle detection via canonicalised path. If the template
    // we are about to load is already on the recursion stack, the
    // chain would loop — stop before reopening the file. The
    // membership check uses canonical paths so aliased forms
    // (`./foo.xml`, `foo.xml`, `../dir/foo.xml`) collapse to the
    // same key, mirroring Rust's `std::fs::canonicalize` check.
    const auto canonResolved = canonicaliseTemplatePath(resolvedPath);
    for (const auto &entry : stack) {
        if (entry == canonResolved) {
            throw SCE::parsing::TemplateCycle(
                "<sce:use template=\"" + templateHref + "\">: cycle detected (" +
                renderTemplateChain(stack, canonResolved) + ")");
        }
    }

    // 3. Load + parse the template file. Split I/O-class failures
    // (ReadError) from document-shape failures (Malformed) by
    // pugi::xml_parse_status — the Rust side separates these into
    // two DiagnosticCodes so agent dispatch can pick an I/O vs
    // content fix without reparsing the message body.
    pugi::xml_document templateDoc;
    const auto loadResult = templateDoc.load_file(resolvedPath.c_str());
    if (!loadResult) {
        const auto status = loadResult.status;
        const bool ioClass = status == pugi::status_file_not_found ||
                             status == pugi::status_io_error ||
                             status == pugi::status_out_of_memory ||
                             status == pugi::status_internal_error;
        if (ioClass) {
            throw SCE::parsing::TemplateReadError(
                "<sce:use template=\"" + templateHref + "\">: cannot "
                "read: " + loadResult.description());
        }
        throw SCE::parsing::TemplateMalformed(
            "<sce:use template=\"" + templateHref + "\">: template is "
            "malformed: " + loadResult.description());
    }

    const auto templateRoot = templateDoc.document_element();
    if (!templateRoot || std::string(templateRoot.name()) != "sce:template") {
        const std::string rootName = templateRoot ? templateRoot.name() : "<none>";
        throw SCE::parsing::TemplateMalformed(
            "<sce:use template=\"" + templateHref + "\">: template is "
            "malformed: root element must be <sce:template>, got <" +
            rootName + ">");
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
            throw SCE::parsing::TemplateMalformed(
                "<sce:use template=\"" + templateHref + "\">: template "
                "is malformed: <sce:param> missing required `name` "
                "attribute");
        }
        const std::string paramName = nameAttr.value();
        if (!SCE::parsing::is_valid_param_name(paramName)) {
            throw SCE::parsing::TemplateMalformed(
                "<sce:use template=\"" + templateHref + "\">: template "
                "is malformed: <sce:param name=\"" + paramName +
                "\"> name must match " +
                std::string(SCE::parsing::PARAM_NAME_PATTERN));
        }
        for (const auto &prev : decls) {
            if (prev.name == paramName) {
                throw SCE::parsing::TemplateMalformed(
                    "<sce:use template=\"" + templateHref + "\">: "
                    "template is malformed: duplicate <sce:param "
                    "name=\"" + paramName + "\"> declaration");
            }
        }

        ParamDecl decl;
        decl.name = paramName;
        if (const auto reqAttr = child.attribute("required")) {
            const std::string reqVal = reqAttr.value();
            if (reqVal == "true") {
                decl.required = true;
            } else if (reqVal != "false") {
                throw SCE::parsing::TemplateMalformed(
                    "<sce:use template=\"" + templateHref + "\">: "
                    "template is malformed: <sce:param name=\"" +
                    paramName + "\"> `required` must be \"true\" or "
                    "\"false\", got \"" + reqVal + "\"");
            }
        }
        if (const auto defAttr = child.attribute("default")) {
            decl.hasDefault = true;
            decl.defaultValue = defAttr.value();
        }
        if (decl.required && decl.hasDefault) {
            throw SCE::parsing::TemplateMalformed(
                "<sce:use template=\"" + templateHref + "\">: template "
                "is malformed: <sce:param name=\"" + paramName +
                "\"> declares both `required=\"true\"` and "
                "`default=\"...\"` — mutually exclusive");
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

    // 8. Substitute `{$name}` in the template body children in-place
    // inside `templateDoc`. Substitution runs BEFORE the nested
    // recursion so any `{$name}` appearing inside a nested
    // `<sce:use>` attribute (e.g. `<sce:use y="{$x}"/>`) is resolved
    // against the OUTER call's bindings before the nested template
    // even loads — mirrors Rust
    // `substitute_into_template_with_map` → `expand_impl` order in
    // `sce-build/src/template.rs`. `<sce:param>` declarations are
    // skipped so `<sce:param default="{$a}">` literals never expand
    // (RFC §6.1 literal-only defaults).
    for (auto child : templateRoot.children()) {
        if (child.type() == pugi::node_element &&
            std::string(child.name()) == "sce:param") {
            continue;
        }
        substituteInSubtree(child, params);
    }

    // 9. Recursively expand nested `<sce:use>` inside the templateDoc
    // body. `templateBaseDir` is the template file's own directory,
    // so a nested `<sce:use template="sibling.scxml"/>` resolves
    // relative to the template, not the outer caller. The canonical
    // path is pushed onto the cycle stack for the duration of the
    // recursion, then popped so sibling expansions at the same depth
    // do not see each other as cycles.
    const std::string templateBaseDir =
        std::filesystem::path(resolvedPath).parent_path().string();
    stack.push_back(canonResolved);
    try {
        expandAllUsesInTree(templateRoot, templateBaseDir, stack, depth + 1);
    } catch (...) {
        stack.pop_back();
        throw;
    }
    stack.pop_back();

    // 10. Splice body children (non-param) into the caller parent in
    // place of the `<sce:use>` node. `insert_copy_before` clones
    // into the caller document so `templateDoc` can go out of scope
    // safely; substitution has already been applied to the source
    // nodes, so the clones carry the resolved values without a
    // second traversal pass.
    //
    // `useNode.parent()` is always non-empty here because every
    // `useNode` arrives via `collectSceUses`, which walks children
    // of a passed-in root and only pushes descendants — never the
    // root node itself. Removing the belt-and-suspenders
    // `!callerParent` check in M4 closes the last M4-labelled
    // `TemplateNotImplemented` throw without introducing a retype
    // target for an unreachable branch.
    auto callerParent = useNode.parent();
    for (auto child : templateRoot.children()) {
        if (child.type() == pugi::node_element &&
            std::string(child.name()) == "sce:param") {
            continue;
        }
        callerParent.insert_copy_before(child, useNode);
    }
    callerParent.remove_child(useNode);
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

        // Parse file using pugixml
        auto doc = std::make_shared<pugi::xml_document>();
        pugi::xml_parse_result result = doc->load_file(filename.c_str());

        if (!result) {
            lastError_ = "Parse error: " + std::string(result.description());
            SCE_LOG_ERROR("PugiXMLParser: {}", lastError_);
            return nullptr;
        }

        // Create document wrapper with base path + source path.
        // Base path feeds `<xi:include>` / `<sce:use template>`
        // resolution; source path seeds the cycle-detection stack in
        // `processSceTemplate` so a top-level document that references
        // itself via `<sce:use template="self.scxml"/>` is caught
        // before loading the template file a second time. Mirrors
        // Rust's `sce-build/src/template.rs::expand(self_path, ...)`
        // plumbing at the parser boundary so the C++ runtime has the
        // same self-reference coverage as the AOT expander.
        auto wrappedDoc = std::make_shared<PugiXMLDocument>(doc);
        std::filesystem::path filePath(filename);
        wrappedDoc->setBasePath(filePath.parent_path().string());
        wrappedDoc->setSourcePath(filename);

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
