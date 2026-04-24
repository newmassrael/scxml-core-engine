// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/TemplateExpander.h"
#include "parsing/TemplateConstants.h"

#include <pugixml.hpp>

#include <cctype>
#include <filesystem>
#include <functional>
#include <string>
#include <unordered_map>

// String-level `<sce:use>` / `<sce:template>` expander
// implementation. Mirrors `sce-build/src/template.rs`; see the
// header comment for the module-level contract. Phase C P2 ships
// the low-level scanner primitives (`findElementEnd`,
// `collectTopLevelSceUseRanges`) and the fast-path return from
// `expandString`. The full recursive expansion path lands in
// follow-up Phase C P2 commits that port
// `apply_substitution_with_tracking`,
// `substitute_into_template_with_map`, and `expand_impl` from the
// Rust reference.

namespace SCE::parsing {

namespace detail {

namespace {

bool startsWith(std::string_view source, std::size_t pos,
                std::string_view needle) noexcept {
    if (pos > source.size() || source.size() - pos < needle.size()) {
        return false;
    }
    return source.compare(pos, needle.size(), needle) == 0;
}

// Advance past the closing `terminator` at or after `pos`. Returns
// `source.size()` on no-match so the scanner's outer loop
// terminates rather than spinning.
std::size_t skipUntil(std::string_view source, std::size_t pos,
                      std::string_view terminator) noexcept {
    const std::size_t hit = source.find(terminator, pos);
    if (hit == std::string_view::npos) {
        return source.size();
    }
    return hit + terminator.size();
}

// Skip a quoted attribute value beginning at `pos` (the opening
// quote character). Returns one past the matching close quote; on
// runaway quote (no close found) returns `source.size()` so the
// outer scanner halts deterministically.
std::size_t skipQuotedAttribute(std::string_view source,
                                std::size_t pos) noexcept {
    if (pos >= source.size()) {
        return source.size();
    }
    const char quote = source[pos];
    if (quote != '"' && quote != '\'') {
        return pos;
    }
    const std::size_t end = source.find(quote, pos + 1);
    if (end == std::string_view::npos) {
        return source.size();
    }
    return end + 1;
}

// After `<tagName` has been matched at `start`, scan past the open
// tag's attributes to the first `>` or `/>`. Returns the offset
// one past that terminator. When the terminator is `/>`, sets
// `*selfClosing = true`; otherwise false. On unterminated tag
// (defensive against malformed input) returns `source.size()`.
std::size_t consumeOpenTag(std::string_view source, std::size_t afterTagName,
                           bool &selfClosing) noexcept {
    selfClosing = false;
    std::size_t pos = afterTagName;
    while (pos < source.size()) {
        const char c = source[pos];
        if (c == '"' || c == '\'') {
            pos = skipQuotedAttribute(source, pos);
            continue;
        }
        if (c == '/' && pos + 1 < source.size() && source[pos + 1] == '>') {
            selfClosing = true;
            return pos + 2;
        }
        if (c == '>') {
            return pos + 1;
        }
        ++pos;
    }
    return source.size();
}

}  // namespace

std::size_t findElementEnd(std::string_view source, std::size_t start,
                           std::string_view tagName) {
    // Open-tag scan: skip past attributes to the `>` or `/>`.
    const std::size_t afterTagName = start + 1 + tagName.size();
    bool selfClosing = false;
    std::size_t pos = consumeOpenTag(source, afterTagName, selfClosing);
    if (selfClosing) {
        return pos;
    }
    if (pos >= source.size()) {
        return source.size();
    }

    // Body scan with a depth counter. `depth == 0` means the
    // outermost `<tagName>` has been closed; every nested
    // `<tagName>` bumps it up, every `</tagName>` bumps it down.
    int depth = 1;
    while (pos < source.size()) {
        if (startsWith(source, pos, "<!--")) {
            pos = skipUntil(source, pos + 4, "-->");
            continue;
        }
        if (startsWith(source, pos, "<![CDATA[")) {
            pos = skipUntil(source, pos + 9, "]]>");
            continue;
        }
        if (source[pos] != '<') {
            ++pos;
            continue;
        }
        // `<` candidate — either `</tagName>` or `<tagName…`.
        if (pos + 1 < source.size() && source[pos + 1] == '/') {
            const std::size_t namePos = pos + 2;
            if (startsWith(source, namePos, tagName)) {
                std::size_t after = namePos + tagName.size();
                while (after < source.size() &&
                       std::isspace(static_cast<unsigned char>(source[after]))) {
                    ++after;
                }
                if (after < source.size() && source[after] == '>') {
                    --depth;
                    pos = after + 1;
                    if (depth == 0) {
                        return pos;
                    }
                    continue;
                }
            }
            // `</something-else>` — skip the `<` and continue the
            // scan; tag-boundary ambiguity on well-formed input is
            // impossible (pugi already validated).
            ++pos;
            continue;
        }
        if (startsWith(source, pos + 1, tagName)) {
            const std::size_t after = pos + 1 + tagName.size();
            if (after < source.size()) {
                const char sep = source[after];
                const bool isTagBoundary =
                    std::isspace(static_cast<unsigned char>(sep)) ||
                    sep == '>' || sep == '/';
                if (isTagBoundary) {
                    bool innerSelfClose = false;
                    const std::size_t afterOpen =
                        consumeOpenTag(source, after, innerSelfClose);
                    if (!innerSelfClose) {
                        ++depth;
                    }
                    pos = afterOpen;
                    continue;
                }
            }
        }
        ++pos;
    }
    // Unmatched close — well-formed input cannot produce this, but
    // return EOF rather than assert so the surrounding expander
    // can surface `TemplateMalformed`.
    return source.size();
}

std::vector<ByteRange> collectTopLevelSceUseRanges(std::string_view source) {
    std::vector<ByteRange> result;
    pugi::xml_document doc;
    const auto parseResult = doc.load_buffer(source.data(), source.size());
    if (!parseResult) {
        throw TemplateMalformed(
            std::string("source document is malformed: ") +
            parseResult.description());
    }

    // Walk the tree; push top-level `<sce:use>` byte ranges only.
    // Nested `<sce:use>` inside another `<sce:use>`'s children is
    // not collected here — mirrors Rust `collect_uses_into` which
    // stops descent at `<sce:use>` nodes so the caller's splice
    // loop sees top-level operations in document order.
    std::function<void(pugi::xml_node)> walk;
    walk = [&](pugi::xml_node node) {
        for (const auto &child : node.children()) {
            if (child.type() != pugi::node_element) {
                continue;
            }
            if (std::string(child.name()) == "sce:use") {
                const auto startSigned = child.offset_debug();
                if (startSigned < 0) {
                    // Programmatically-constructed node with no
                    // anchored source offset — skip. pugi sets
                    // offset_debug == -1 for nodes created via
                    // insert_copy_before or append_child.
                    continue;
                }
                std::size_t start = static_cast<std::size_t>(startSigned);
                // pugi `offset_debug` returns the position of the
                // tag-name character (the byte after `<`), not the
                // `<` itself. Back up one if needed so downstream
                // byte-range consumers see the full `<tagname...>`
                // substring. Defensive: only adjust when the byte
                // before is in fact `<`, so a future pugi whose
                // offset points at `<` directly does not
                // double-correct.
                if (start > 0 && source[start] != '<' &&
                    source[start - 1] == '<') {
                    start -= 1;
                }
                const std::size_t end =
                    findElementEnd(source, start, "sce:use");
                result.push_back({start, end});
                // Top-level only: do not recurse into the subtree.
                continue;
            }
            walk(child);
        }
    };
    const auto root = doc.document_element();
    if (root) {
        walk(root);
    }
    return result;
}

SubstitutionResult applySubstitutionWithTracking(
    std::string_view body, std::size_t bodySourceOffset,
    const std::filesystem::path &templatePath,
    const std::unordered_map<std::string, std::string> &params,
    const std::filesystem::path &callerFile, std::uint32_t callerRow,
    std::uint32_t callerCol) {
    SubstitutionResult result;
    result.substituted.reserve(body.size());
    std::size_t pos = 0;
    while (pos < body.size()) {
        const std::size_t startRel = body.substr(pos).find("{$");
        if (startRel == std::string_view::npos) {
            break;
        }
        const std::size_t start = pos + startRel;
        // Flush non-substituted prefix [pos, start) as template-file
        // bytes. Mirrors Rust's prefix-flush in
        // `apply_substitution_with_tracking`.
        if (start > pos) {
            const std::size_t outStart = result.substituted.size();
            result.substituted.append(body.substr(pos, start - pos));
            result.entries.push_back(SubstitutionEntry{
                outStart, result.substituted.size(),
                FileOrigin{templatePath, bodySourceOffset + pos}});
        }
        const std::size_t after = start + 2;
        const std::size_t closeRel =
            (after < body.size()) ? body.substr(after).find('}')
                                  : std::string_view::npos;
        if (closeRel != std::string_view::npos) {
            const std::string_view name = body.substr(after, closeRel);
            if (is_valid_param_name(name)) {
                const auto it = params.find(std::string(name));
                if (it != params.end()) {
                    if (!it->second.empty()) {
                        const std::size_t outStart = result.substituted.size();
                        result.substituted.append(it->second);
                        result.entries.push_back(SubstitutionEntry{
                            outStart, result.substituted.size(),
                            CallSiteOrigin{callerFile, callerRow, callerCol}});
                    }
                    pos = after + closeRel + 1;
                    continue;
                }
            }
        }
        // Not a valid `{$name}` token — emit `{$` literally as
        // template-file bytes and resume the scan immediately
        // after. Mirrors Rust's literal-`{$` emission path.
        const std::size_t outStart = result.substituted.size();
        result.substituted.append("{$");
        result.entries.push_back(SubstitutionEntry{
            outStart, result.substituted.size(),
            FileOrigin{templatePath, bodySourceOffset + start}});
        pos = after;
    }
    // Tail — any bytes past the last `{$`.
    if (pos < body.size()) {
        const std::size_t outStart = result.substituted.size();
        result.substituted.append(body.substr(pos));
        result.entries.push_back(SubstitutionEntry{
            outStart, result.substituted.size(),
            FileOrigin{templatePath, bodySourceOffset + pos}});
    }
    return result;
}

ParamDecl parseParamDecl(pugi::xml_node node, std::string_view templateHref) {
    // Mirrors `sce-build/src/template.rs::parse_param_decl`. Every
    // malformed shape surfaces as `TemplateMalformed` with a
    // message naming the offending template so agent-side repair
    // heuristics can dispatch without re-reading the body.
    const auto nameAttr = node.attribute("name");
    if (!nameAttr) {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: <sce:param> missing required `name` "
            "attribute");
    }
    std::string paramName = nameAttr.value();
    if (!is_valid_param_name(paramName)) {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: <sce:param name=\"" + paramName +
            "\"> name must match " + std::string(PARAM_NAME_PATTERN));
    }

    ParamDecl decl;
    decl.name = std::move(paramName);
    if (const auto reqAttr = node.attribute("required")) {
        const std::string reqVal = reqAttr.value();
        if (reqVal == "true") {
            decl.required = true;
        } else if (reqVal != "false") {
            throw TemplateMalformed(
                "<sce:use template=\"" + std::string(templateHref) +
                "\">: template is malformed: <sce:param name=\"" + decl.name +
                "\"> `required` must be \"true\" or \"false\", got \"" +
                reqVal + "\"");
        }
    }
    if (const auto defAttr = node.attribute("default")) {
        decl.hasDefault = true;
        decl.defaultValue = defAttr.value();
    }
    if (decl.required && decl.hasDefault) {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: <sce:param name=\"" + decl.name +
            "\"> declares both `required=\"true\"` and `default=\"...\"` — "
            "mutually exclusive");
    }
    return decl;
}

std::unordered_map<std::string, std::string> collectUseBindings(
    pugi::xml_node useNode) {
    std::unordered_map<std::string, std::string> bindings;
    for (const auto attr : useNode.attributes()) {
        const std::string attrName = attr.name();
        if (attrName == "template") {
            continue;
        }
        // pugixml exposes XML namespace declarations as regular
        // attributes; filter them to match Rust roxmltree's
        // `attribute()` iterator shape which hides namespaces.
        if (attrName == "xmlns" ||
            (attrName.size() > 6 && attrName.compare(0, 6, "xmlns:") == 0)) {
            continue;
        }
        bindings.emplace(attrName, attr.value());
    }
    return bindings;
}

std::vector<ByteRange> extractTemplateBodyRanges(
    std::string_view expandedTemplate, std::string_view templateHref) {
    pugi::xml_document doc;
    const auto parseResult =
        doc.load_buffer(expandedTemplate.data(), expandedTemplate.size());
    if (!parseResult) {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: expanded template is malformed: " +
            parseResult.description());
    }
    const auto root = doc.document_element();
    if (!root || std::string(root.name()) != "sce:template") {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: expanded template root is not "
            "<sce:template>");
    }
    std::vector<ByteRange> ranges;
    for (const auto child : root.children()) {
        if (child.type() == pugi::node_element &&
            std::string(child.name()) == "sce:param") {
            continue;
        }
        const auto startSigned = child.offset_debug();
        if (startSigned < 0) {
            continue;
        }
        std::size_t start = static_cast<std::size_t>(startSigned);
        if (start > 0 && expandedTemplate[start] != '<' &&
            expandedTemplate[start - 1] == '<') {
            start -= 1;
        }
        std::size_t end = start;
        if (child.type() == pugi::node_element) {
            end = findElementEnd(expandedTemplate, start,
                                  std::string(child.name()));
        } else if (child.type() == pugi::node_pcdata ||
                   child.type() == pugi::node_cdata) {
            // Text node: span runs from `start` to the next `<` or
            // EOF. pugi `offset_debug` on text nodes points at the
            // first character of the data.
            end = expandedTemplate.find('<', start);
            if (end == std::string_view::npos) {
                end = expandedTemplate.size();
            }
        } else if (child.type() == pugi::node_comment) {
            const std::size_t closer =
                expandedTemplate.find("-->", start);
            end = (closer == std::string_view::npos)
                      ? expandedTemplate.size()
                      : closer + 3;
        } else {
            continue;
        }
        ranges.push_back(ByteRange{start, end});
    }
    return ranges;
}

}  // namespace detail

TemplateExpandResult expandString(std::string_view content,
                                  std::string_view selfPath,
                                  std::string_view baseDir) {
    // Fast path — mirrors Rust `expand`'s `if !content.contains("sce:use")`.
    if (content.find("sce:use") == std::string_view::npos) {
        TemplateExpandResult result;
        result.expanded_text.assign(content);
        result.positions =
            PositionMap::identity(std::filesystem::path(selfPath), content);
        return result;
    }

    // Full recursive expansion is not yet implemented in this
    // commit; the next Phase C P2 commits port
    // `substitute_into_template_with_map` and `expand_impl`. The
    // scanner primitives above stand as the consumer for the
    // current commit — unit tests in
    // `tests/parsing/TemplateExpander_test.cpp` exercise them.
    (void)baseDir;
    TemplateExpandResult result;
    result.expanded_text.assign(content);
    result.positions =
        PositionMap::identity(std::filesystem::path(selfPath), content);
    return result;
}

}  // namespace SCE::parsing
