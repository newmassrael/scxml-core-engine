// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "parsing/PugiXMLParser.h"
#include "core/LogMacros.h"
#include "parsing/IXMLElement.h"
#include "parsing/ParseError.h"
#include "parsing/TemplateConstants.h"
#include "parsing/TemplateError.h"
#include "parsing/TemplateExpander.h"
#include "parsing/XIncludeError.h"
#include "parsing/XIncludeExpander.h"
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

    // pugixml exposes no namespace-resolution API directly — XML namespace
    // declarations (`xmlns` / `xmlns:prefix`) are surfaced only as ordinary
    // attributes on the declaring element. To return the namespace URI for
    // this node we have to walk up the ancestor chain looking for the
    // matching `xmlns` attribute. Mirrors the Rust AOT pipeline's lookup
    // (roxmltree does this for free) so both engines apply identical
    // namespace semantics, closing the foreign-NS local-name collision
    // footgun documented in `SCE_FORGE.md` §3.1.
    std::string name = node_.name();
    size_t colonPos = name.find(':');
    std::string xmlnsAttrName;
    if (colonPos == std::string::npos) {
        // Unprefixed element: inherits the default namespace from the
        // nearest ancestor declaring `xmlns="..."`.
        xmlnsAttrName = "xmlns";
    } else {
        // Prefixed element: look for the `xmlns:<prefix>="..."` binding
        // in the nearest declaring ancestor.
        xmlnsAttrName = "xmlns:" + name.substr(0, colonPos);
    }

    for (pugi::xml_node ancestor = node_; ancestor; ancestor = ancestor.parent()) {
        pugi::xml_attribute decl = ancestor.attribute(xmlnsAttrName.c_str());
        if (decl) {
            return decl.value();
        }
    }

    // No declaration found — the document omitted the namespace binding.
    // Returning empty is honest about what pugixml saw; the rejection of
    // unnamespaced SCXML happens in `ParsingCommon::isScxmlNamespace`
    // (strict against `Constants::SCXML_NAMESPACE`) and surfaces at the
    // `SCXMLParser::parseInternal` root-namespace check as
    // `ParseWrongRootElement`, matching the AOT pipeline's XSD rejection.
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

    // §scxml-B-2: Full XML serialization preserving structure.
    // Use pugixml's print() to serialize each child node compactly.
    //
    // Namespace propagation across the serialization boundary:
    // pugixml exposes xmlns declarations only as ordinary attributes
    // on the declaring element, so a child that inherits the default
    // xmlns from `node_` (or any ancestor above it) has no xmlns
    // attribute of its own and `pugi::xml_node::print()` therefore
    // omits the binding from the serialized fragment. Without
    // injection the round-trip `<invoke><content><scxml>` → string
    // → `loadSCXMLFromString` re-parse fails the strict
    // `ParsingCommon::isScxmlNamespace` gate landed in a46d2c27
    // (the re-parsed `<scxml>` has no namespace, so it is rejected
    // as `ParseWrongRootElement` even though the original document
    // bound it via the ancestor's `xmlns="http://www.w3.org/2005/07/scxml"`).
    // The fix mirrors the same ancestor-walk `getNamespace()`
    // performs: pre-compute the default xmlns visible at `node_`,
    // then inject it onto each unprefixed element child whose own
    // tag does not already declare a default. Prefixed children
    // (`<framework:foo>`) and children that ship their own
    // `xmlns="..."` are left untouched.
    std::string inheritedDefaultXmlns;
    for (pugi::xml_node ancestor = node_; ancestor; ancestor = ancestor.parent()) {
        pugi::xml_attribute decl = ancestor.attribute("xmlns");
        if (decl) {
            inheritedDefaultXmlns = decl.value();
            break;
        }
    }

    std::ostringstream oss;
    for (const auto &child : node_.children()) {
        if (child.type() != pugi::node_element || inheritedDefaultXmlns.empty()) {
            // Text / CDATA / comment children round-trip verbatim;
            // the no-inherited-xmlns case has nothing to inject.
            child.print(oss, "", pugi::format_raw);
            continue;
        }

        std::string childName = child.name();
        bool childIsPrefixed = childName.find(':') != std::string::npos;
        bool childHasOwnXmlns = static_cast<bool>(child.attribute("xmlns"));
        if (childIsPrefixed || childHasOwnXmlns) {
            child.print(oss, "", pugi::format_raw);
            continue;
        }

        // Patch `xmlns="<inherited>"` into the child's opening tag.
        // Serialize to a temporary buffer first because pugixml has
        // no API to inject an attribute on a const node; the
        // alternative — mutating the document tree — would risk
        // surprising the rest of the parser. The opening tag begins
        // at byte 0 with `<` followed by the local name; the
        // insertion point is the byte immediately after the local
        // name (before any whitespace, `/`, or `>`).
        std::ostringstream child_oss;
        child.print(child_oss, "", pugi::format_raw);
        std::string serialized = child_oss.str();
        if (serialized.size() < 2 || serialized[0] != '<') {
            // Defensive fallback for unexpected shape (no malformed
            // root has reached this point in practice, but pugixml's
            // print contract does not formally guarantee `<` at
            // byte 0 for every pathological tree).
            oss << serialized;
            continue;
        }
        size_t pos = 1;
        while (pos < serialized.size()) {
            char c = serialized[pos];
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '/' || c == '>') {
                break;
            }
            ++pos;
        }
        std::string injection = " xmlns=\"" + inheritedDefaultXmlns + "\"";
        serialized.insert(pos, injection);
        oss << serialized;
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

SCE::parsing::PositionMap PugiXMLDocument::processXInclude() {
    // String-level `<xi:include>` expansion. Hands the captured
    // author bytes to `SCE::parsing::expandStringX` (mirrors
    // `sce-build/src/xinclude.rs::expand` line-for-line), then
    // reparses the spliced output back into `doc_` so downstream
    // validation sees the expanded tree. The returned PositionMap
    // tracks every emitted byte back to FileOrigin entries —
    // outer-content regions resolve to the host document, fragment
    // regions resolve to the included file.
    //
    // Failure surface (§wire-W4.5):
    //   - `XIncludeExpansionError` subtypes (typed leaves from W3)
    //     propagate untouched.
    //   - Reparse failure of the spliced text throws
    //     `ParseXmlFailed` (D2: reuse `xml/parse`).
    //   - Non-typed `std::exception` from the expander folds into
    //     `XIncludeMalformed` (D3: reuse `xml/xinclude-malformed`).
    //
    // The previous `if (!doc_)` defensive branch was dropped under
    // §wire-W4.5: PugiXMLParser typed-throw (W4 D1-C) never produces
    // a wrapped null doc, so the branch was unreachable.

    // Fast path: documents whose captured source contains no
    // "include" substring bypass the rewrite pipeline. The existing
    // DOM is preserved (already byte-equivalent to the identity
    // map) and the returned PositionMap is identity over the
    // author's source bytes, so subsequent diagnostics resolve to
    // author (file, row, col) without skew. Mirrors the Rust
    // expand fast path in sce-build/src/xinclude.rs and the
    // sibling identity fast path in processSceTemplate.
    if (!sourceText_.empty() &&
        sourceText_.find("include") == std::string::npos) {
        return SCE::parsing::PositionMap::identity(
            std::filesystem::path(sourcePath_), sourceText_);
    }

    try {
        // Coordinate-space input: prefer the captured author bytes
        // when available so PositionMap entries resolve to author
        // file/row/col directly. parseContent (in-memory) leaves
        // sourceText_ empty; in that case fall back to serialising
        // the current DOM through format_raw, accepting that the
        // serialised coordinates differ from any single author file
        // (no in-memory source path exists either).
        std::string content;
        if (!sourceText_.empty()) {
            content = sourceText_;
        } else {
            std::ostringstream serialised;
            doc_->save(serialised, "",
                       pugi::format_raw | pugi::format_no_declaration);
            content = serialised.str();
        }

        SCE::parsing::XIncludeExpandResult expanded =
            SCE::parsing::expandStringX(content, sourcePath_, basePath_);

        // Reparse into the same shared_ptr'd document so every
        // `IXMLElement` the caller has already retrieved continues
        // to see the expanded tree. Mirrors processSceTemplate's
        // reset+load_buffer pattern (PugiXMLParser.cpp:406-413).
        doc_->reset();
        const auto parseResult = doc_->load_buffer(
            expanded.expanded_text.data(), expanded.expanded_text.size());
        if (!parseResult) {
            throw SCE::parsing::ParseXmlFailed(
                "Failed to reparse expanded XInclude: " +
                std::string(parseResult.description()));
        }

        // Threaded buffer: subsequent `processSceTemplate` operates
        // on the post-XInclude bytes that `expanded.positions` is
        // keyed against. Overwriting `sourceText_` here matches the
        // Rust pipeline shape (`let (included, xinclude_map) =
        // xinclude::expand(...); template::expand(&included, ...,
        // &xinclude_map)`) — each stage's input is the previous
        // stage's output, and the PositionMap stays keyed against
        // bytes the consumer can actually inspect.
        // Without this assignment, the next stage would see
        // the original author bytes while the upstream map keys
        // post-XInclude bytes — composition would silently produce
        // wrong origins for fragment regions.
        sourceText_ = expanded.expanded_text;

        SCE_LOG_DEBUG("PugiXMLDocument: XInclude processing successful");
        return std::move(expanded.positions);
    } catch (const SCE::parsing::ParseError &) {
        // Reparse-failure throws (`ParseXmlFailed` from the inline
        // throw above) propagate untouched so the typed leaf reaches
        // `SCXMLParser::parseFile` / `parseContent`'s parser-entry
        // catch arm (§wire-W4.5 D2). The arm is here so the
        // std::exception fallback below does NOT fold a typed
        // ParseError into XIncludeMalformed.
        throw;
    } catch (const SCE::parsing::XIncludeExpansionError &) {
        // §wire-W3: typed XInclude diagnostics propagate to
        // `SCXMLParser::parseFile` / `parseContent`'s typed catch
        // arm so `getDiagnostics()` surfaces the leaf with its
        // `xml/xinclude-*` code(). Re-throw rather than
        // record-and-swallow because the typed object's dynamic
        // type carries the leaf's `code()` override; rebuilding
        // it from the rendered message text would be lossy.
        throw;
    } catch (const std::exception &ex) {
        // §wire-W4.5 D3: non-typed expander failure (e.g.
        // std::bad_alloc propagating through expandStringX) folds
        // into the `xml/xinclude-malformed` family — the catch-all
        // for "expansion failed for an unspecified reason".
        throw SCE::parsing::XIncludeMalformed(
            "XInclude processing failed: " + std::string(ex.what()));
    }
}

SCE::parsing::PositionMap PugiXMLDocument::processSceTemplate(
    const SCE::parsing::PositionMap &upstream) {
    // String-level `<sce:use>` expansion. Operates on the
    // post-XInclude bytes captured in `sourceText_` (see
    // `processXInclude` for how that buffer becomes the threaded
    // post-XInclude content), hands them to
    // `SCE::parsing::expandString` which mirrors
    // `sce-build/src/template.rs::expand`, then reparses the
    // expanded text back into `doc_` so downstream validation sees
    // the expanded tree. `upstream` is the PositionMap describing
    // those input bytes — the expander composes it into the output
    // map so diagnostic byte lookups resolve through both
    // preprocessor stages.
    //
    // Failure surface (§wire-W4.5):
    //   - `TemplateError` subtypes (typed leaves from W1) thrown by
    //     `expandString` propagate untouched.
    //   - Reparse failure of the spliced text throws
    //     `ParseXmlFailed` (D2: reuse `xml/parse`).
    //
    // The previous `if (!doc_)` defensive branch was dropped under
    // §wire-W4.5: PugiXMLParser typed-throw (W4 D1-C) never produces
    // a wrapped null doc, so the branch was unreachable.

    // Fast path: documents whose captured source contains no
    // `<sce:use>` bypass expansion entirely, preserving the existing
    // DOM pointers. Returns `upstream` unchanged so the threaded
    // PositionMap continues to describe every byte (whether it
    // originates in the host document or in an `xi:include`'d
    // fragment).
    if (!sourceText_.empty() &&
        sourceText_.find("sce:use") == std::string::npos) {
        return upstream;
    }

    // Coordinate-space input: prefer the captured (post-XInclude)
    // bytes when available so the upstream PositionMap and
    // `expandString`'s input share the same byte coordinate space —
    // `processXInclude` overwrites `sourceText_` with the spliced
    // bytes on rewrite, so the two stay byte-aligned by
    // construction. parseContent (in-memory) leaves `sourceText_`
    // empty; in that case fall back to serialising the current DOM
    // through `format_raw` (callers are expected to pass an identity
    // upstream over those bytes).
    std::string content;
    if (!sourceText_.empty()) {
        content = sourceText_;
    } else {
        std::ostringstream serialised;
        doc_->save(serialised, "", pugi::format_raw | pugi::format_no_declaration);
        content = serialised.str();
    }

    // Secondary fast path: the captured / serialised post-XInclude
    // text may also lack `<sce:use>` if XInclude neither introduced
    // nor preserved one. Threading `upstream` through keeps any
    // fragment-byte attribution intact.
    if (content.find("sce:use") == std::string::npos) {
        return upstream;
    }

    auto expanded =
        SCE::parsing::expandString(content, sourcePath_, basePath_, upstream);

    // Reparse into the same shared_ptr'd document so every
    // `IXMLElement` the caller has already retrieved continues to
    // see the expanded tree. `xml_document::reset()` clears without
    // deallocating the owning shared_ptr, then load_buffer populates
    // it with the expanded bytes.
    doc_->reset();
    const auto parseResult = doc_->load_buffer(
        expanded.expanded_text.data(), expanded.expanded_text.size());
    if (!parseResult) {
        throw SCE::parsing::ParseXmlFailed(
            "Failed to reparse expanded template: " +
            std::string(parseResult.description()));
    }

    return std::move(expanded.positions);
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

bool PugiXMLDocument::isValid() const {
    return doc_ != nullptr && doc_->document_element();
}

// ============================================================================
// PugiXMLParser implementation
// ============================================================================

// §wire-W4 D1-C: typed-throw replaces the historical nullptr-return
// + lastError_ poll. Callers (SCXMLParser::parseFile / parseContent)
// observe parser-entry failures via typed `SCE::parsing::ParseError`
// subtypes caught at the parser boundary. `lastError_` is no longer
// populated; `getLastError()` returns empty for backward source
// compatibility (the symbol still exists for any out-of-repo direct
// caller, but the typed surface is now the contract).
std::shared_ptr<IXMLDocument> PugiXMLParser::parseFile(const std::string &filename) {
    // Check if file exists
    if (!std::filesystem::exists(filename)) {
        SCE_LOG_ERROR("PugiXMLParser: File not found: {}", filename);
        throw SCE::parsing::ParseFileNotFound("File not found: " + filename);
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
        // Distinct from filesystem::exists==false (ParseFileNotFound):
        // file exists but could not be opened (permission denied,
        // I/O failure). Reuses ParseFileNotFound because no Rust
        // producer distinguishes this case either — the wire still
        // routes through xml/file-not-found with a refined message.
        SCE_LOG_ERROR("PugiXMLParser: Cannot open file: {}", filename);
        throw SCE::parsing::ParseFileNotFound("Cannot open file: " + filename);
    }
    std::ostringstream buffer;
    buffer << in.rdbuf();
    std::string sourceText = buffer.str();

    auto doc = std::make_shared<pugi::xml_document>();
    pugi::xml_parse_result result =
        doc->load_buffer(sourceText.data(), sourceText.size());

    if (!result) {
        const std::string msg =
            "Parse error: " + std::string(result.description());
        SCE_LOG_ERROR("PugiXMLParser: {}", msg);
        throw SCE::parsing::ParseXmlFailed(msg);
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
}

std::shared_ptr<IXMLDocument> PugiXMLParser::parseContent(const std::string &content) {
    SCE_LOG_INFO("PugiXMLParser: Parsing content");

    // Parse from string using pugixml
    auto doc = std::make_shared<pugi::xml_document>();
    pugi::xml_parse_result result = doc->load_string(content.c_str());

    if (!result) {
        const std::string msg =
            "Parse error: " + std::string(result.description());
        SCE_LOG_ERROR("PugiXMLParser: {}", msg);
        throw SCE::parsing::ParseXmlFailed(msg);
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
}

}  // namespace SCE
