// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/TemplateExpander.h"
#include "parsing/TemplateConstants.h"

#include <pugixml.hpp>

#include <algorithm>
#include <cctype>
#include <filesystem>
#include <fstream>
#include <functional>
#include <sstream>
#include <string>
#include <system_error>
#include <unordered_map>
#include <unordered_set>

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

std::filesystem::path resolveTemplatePath(
    std::string_view templateHref, const std::filesystem::path &baseDir,
    std::vector<std::string> &searched) {
    // Mirrors `sce-build/src/template.rs::resolve_template_path`:
    // absolute → base directory → current working directory. Every
    // branch that checks `exists()` appends the candidate to
    // `searched` on miss so the `TemplateNotFound` diagnostic renders
    // the same trail the Rust side emits.
    std::filesystem::path path(std::string{templateHref});
    if (path.is_absolute()) {
        if (std::filesystem::exists(path)) {
            return path;
        }
        searched.push_back(path.string());
        return {};
    }
    if (!baseDir.empty()) {
        const auto candidate = baseDir / path;
        if (std::filesystem::exists(candidate)) {
            return std::filesystem::absolute(candidate);
        }
        searched.push_back(candidate.string());
    }
    if (std::filesystem::exists(path)) {
        return std::filesystem::absolute(path);
    }
    searched.push_back(path.string());
    return {};
}

namespace {

// Canonicalise a resolved template path for cycle-stack membership.
// Falls back to the uncanonical form on error so self-references
// still trip when the resolver and the canonicaliser disagree
// (e.g. a transient filesystem state during canonical()).
std::filesystem::path canonicaliseForCycle(
    const std::filesystem::path &resolved) {
    std::error_code ec;
    auto canon = std::filesystem::canonical(resolved, ec);
    if (ec) {
        return resolved;
    }
    return canon;
}

// Render a cycle chain as `outer → inner → next` using the U+2192
// arrow. Mirrors Rust `sce-build/src/template.rs::render_chain` so
// cross-language diagnostic consumers key on the same separator.
std::string renderChain(const std::vector<std::filesystem::path> &stack,
                        const std::filesystem::path &next) {
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

// Read the full text of `path` into an owned string. Throws
// `TemplateReadError` labelled with `templateHref` on I/O failure
// so the expander's call site can surface the correct typed
// subtype without re-classifying.
std::string readTemplateText(const std::filesystem::path &path,
                             std::string_view templateHref) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        throw TemplateReadError(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: cannot read: open failed");
    }
    std::ostringstream buffer;
    buffer << in.rdbuf();
    if (in.bad()) {
        throw TemplateReadError(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: cannot read: I/O failure during read");
    }
    return buffer.str();
}

}  // namespace

SubstituteIntoTemplateResult substituteIntoTemplateWithMap(
    std::string_view templateRaw,
    const std::filesystem::path &templatePath,
    std::string_view templateHref,
    const std::unordered_map<std::string, std::string> &bound,
    const std::filesystem::path &callerFile, std::uint32_t callerRow,
    std::uint32_t callerCol) {
    pugi::xml_document doc;
    const auto parseResult =
        doc.load_buffer(templateRaw.data(), templateRaw.size());
    if (!parseResult) {
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: " + parseResult.description());
    }
    const auto root = doc.document_element();
    if (!root || std::string(root.name()) != "sce:template") {
        const std::string rootName = root ? root.name() : "<none>";
        throw TemplateMalformed(
            "<sce:use template=\"" + std::string(templateHref) +
            "\">: template is malformed: root element must be <sce:template>, "
            "got <" +
            rootName + ">");
    }

    // Collect `<sce:param>` declarations + compute body byte span.
    std::vector<ParamDecl> decls;
    std::unordered_set<std::string> seenNames;
    bool haveBodyStart = false;
    std::size_t bodyStart = 0;
    std::size_t bodyEnd = 0;

    for (const auto child : root.children()) {
        if (child.type() == pugi::node_element &&
            std::string(child.name()) == "sce:param") {
            ParamDecl decl = parseParamDecl(child, templateHref);
            if (!seenNames.insert(decl.name).second) {
                throw TemplateMalformed(
                    "<sce:use template=\"" + std::string(templateHref) +
                    "\">: template is malformed: duplicate <sce:param "
                    "name=\"" +
                    decl.name + "\"> declaration");
            }
            decls.push_back(std::move(decl));
            continue;
        }
        if (child.type() != pugi::node_element &&
            child.type() != pugi::node_pcdata &&
            child.type() != pugi::node_cdata &&
            child.type() != pugi::node_comment) {
            continue;
        }
        const auto childStartSigned = child.offset_debug();
        if (childStartSigned < 0) {
            continue;
        }
        std::size_t childStart =
            static_cast<std::size_t>(childStartSigned);
        if (child.type() == pugi::node_element && childStart > 0 &&
            templateRaw[childStart] != '<' &&
            templateRaw[childStart - 1] == '<') {
            childStart -= 1;
        }
        std::size_t childEnd = childStart;
        if (child.type() == pugi::node_element) {
            childEnd =
                findElementEnd(templateRaw, childStart,
                                std::string(child.name()));
        } else if (child.type() == pugi::node_comment) {
            const std::size_t closer = templateRaw.find("-->", childStart);
            childEnd = (closer == std::string_view::npos)
                           ? templateRaw.size()
                           : closer + 3;
        } else {
            // pcdata / cdata: run to the next `<` (next sibling or
            // the `</sce:template>` closing tag).
            const std::size_t nextOpen = templateRaw.find('<', childStart);
            childEnd = (nextOpen == std::string_view::npos)
                           ? templateRaw.size()
                           : nextOpen;
        }
        if (!haveBodyStart) {
            bodyStart = childStart;
            haveBodyStart = true;
        }
        bodyEnd = childEnd;
    }

    // Bindings validation mirrors Rust `substitute_into_template_with_map`:
    // every bound name must be declared, every required param must be
    // bound, missing non-required params get their default or empty.
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
    for (const auto &kv : bound) {
        const bool known = std::any_of(decls.begin(), decls.end(),
                                        [&](const ParamDecl &d) {
                                            return d.name == kv.first;
                                        });
        if (!known) {
            throw TemplateUnknownParam(
                "<sce:use template=\"" + std::string(templateHref) +
                "\">: unknown parameter '" + kv.first + "' (declared: " +
                declaredList + ")");
        }
    }

    std::unordered_map<std::string, std::string> params;
    params.reserve(decls.size());
    for (const auto &d : decls) {
        const auto it = bound.find(d.name);
        if (it != bound.end()) {
            params.emplace(d.name, it->second);
            continue;
        }
        if (d.required) {
            throw TemplateMissingParam(
                "<sce:use template=\"" + std::string(templateHref) +
                "\">: missing required parameter '" + d.name + "'");
        }
        params.emplace(d.name, d.hasDefault ? d.defaultValue : std::string());
    }

    SubstituteIntoTemplateResult result;
    result.positions.register_file(templatePath,
                                    std::string_view{templateRaw});

    if (!haveBodyStart) {
        // No body — the whole template file is a single File-origin
        // passthrough (mirrors Rust's "template has no body" branch).
        result.substituted.assign(templateRaw);
        result.positions.push_entry(
            0, templateRaw.size(),
            FileOrigin{templatePath, 0});
        return result;
    }

    // Reassemble: prefix [0, bodyStart) + substituted body
    // + suffix [bodyEnd, templateRaw.size()).
    result.substituted.reserve(templateRaw.size() + 32);
    if (bodyStart > 0) {
        result.substituted.append(templateRaw.substr(0, bodyStart));
        result.positions.push_entry(0, bodyStart,
                                     FileOrigin{templatePath, 0});
    }
    const std::size_t bodyBase = result.substituted.size();
    const auto substitution = applySubstitutionWithTracking(
        templateRaw.substr(bodyStart, bodyEnd - bodyStart), bodyStart,
        templatePath, params, callerFile, callerRow, callerCol);
    result.substituted.append(substitution.substituted);
    for (const auto &entry : substitution.entries) {
        result.positions.push_entry(bodyBase + entry.out_start,
                                     bodyBase + entry.out_end,
                                     entry.origin);
    }
    if (bodyEnd < templateRaw.size()) {
        const std::size_t suffixStart = result.substituted.size();
        result.substituted.append(templateRaw.substr(bodyEnd));
        result.positions.push_entry(
            suffixStart, result.substituted.size(),
            FileOrigin{templatePath, bodyEnd});
    }
    return result;
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

namespace {

// Recursive string-level expander. Mirrors
// `sce-build/src/template.rs::expand_impl`. Public entry point is
// `SCE::parsing::expandString`, which seeds the cycle stack and the
// identity input map before calling into this function.
TemplateExpandResult expandImpl(std::string_view content,
                                const std::filesystem::path &contentFile,
                                const std::filesystem::path &baseDir,
                                std::uint32_t depth,
                                std::vector<std::filesystem::path> &stack,
                                const PositionMap &inputMap) {
    if (depth >= static_cast<std::uint32_t>(MAX_TEMPLATE_DEPTH)) {
        throw TemplateTooDeep(
            "<sce:use> template nesting exceeds depth limit of " +
            std::to_string(MAX_TEMPLATE_DEPTH));
    }

    pugi::xml_document doc;
    const auto parseResult = doc.load_buffer(content.data(), content.size());
    if (!parseResult) {
        throw TemplateMalformed(
            std::string("source document is malformed: ") +
            parseResult.description());
    }

    struct SceUseHit {
        ByteRange range;
        pugi::xml_node node;
    };
    std::vector<SceUseHit> uses;
    std::function<void(pugi::xml_node)> walk;
    walk = [&](pugi::xml_node node) {
        for (const auto &child : node.children()) {
            if (child.type() != pugi::node_element) {
                continue;
            }
            if (std::string(child.name()) == "sce:use") {
                const auto startSigned = child.offset_debug();
                if (startSigned < 0) {
                    continue;
                }
                std::size_t start = static_cast<std::size_t>(startSigned);
                if (start > 0 && content[start] != '<' &&
                    content[start - 1] == '<') {
                    start -= 1;
                }
                const std::size_t end =
                    findElementEnd(content, start, "sce:use");
                uses.push_back(SceUseHit{ByteRange{start, end}, child});
                continue;
            }
            walk(child);
        }
    };
    const auto rootNode = doc.document_element();
    if (rootNode) {
        walk(rootNode);
    }

    if (uses.empty()) {
        // No `<sce:use>` in this content — output is 1:1 copy of
        // `content`, so the upstream map already describes every
        // emitted byte. Mirrors Rust's early-return identity.
        return TemplateExpandResult{std::string(content), inputMap};
    }

    // Identity over `content` provides content-local (row, col)
    // for `<sce:use>` call-site metadata (Rust `doc_loc` equivalent).
    const auto identity = PositionMap::identity(contentFile, content);

    std::string out;
    out.reserve(content.size());
    PositionMap outMap;
    std::size_t cursor = 0;

    for (const auto &hit : uses) {
        const auto useRange = hit.range;
        const auto &useNode = hit.node;

        // Prefix [cursor, useRange.start) — bytes unchanged from
        // `content`, so compose from `inputMap`.
        if (cursor < useRange.start) {
            const std::size_t outStart = out.size();
            out.append(content.substr(cursor, useRange.start - cursor));
            outMap.append_mapped_substring(inputMap, cursor, useRange.start,
                                            outStart);
        }

        const auto callerPos = identity.lookup(useRange.start);
        const std::uint32_t callerRow = callerPos.row;
        const std::uint32_t callerCol = callerPos.col;

        // Caller-file SourcePos for the `<sce:use>` element, used
        // to populate `TemplateError::location()` at the throw sites
        // below. Every failure mode that keys to this particular
        // `<sce:use>` (missing attr, not-found, cycle, read-error,
        // substitution failure) gets its location stamped to the
        // same author-source coordinate so diagnostic consumers can
        // present one pointed line in the caller file.
        const SourcePos useLocation{contentFile, callerRow, callerCol};

        const auto templateAttr = useNode.attribute("template");
        const std::string templateHref =
            templateAttr ? templateAttr.value() : std::string();
        if (templateHref.empty()) {
            TemplateMissingAttribute err(
                "<sce:use> missing required `template` attribute");
            err.setLocation(useLocation);
            throw err;
        }

        std::vector<std::string> tried;
        const auto resolvedPath =
            resolveTemplatePath(templateHref, baseDir, tried);
        if (resolvedPath.empty()) {
            std::string trail;
            for (const auto &entry : tried) {
                if (!trail.empty()) {
                    trail.append(", ");
                }
                trail.append(entry);
            }
            TemplateNotFound err(
                "<sce:use template=\"" + templateHref +
                "\">: file not found (searched: " + trail + ")");
            err.setLocation(useLocation);
            throw err;
        }

        const auto canon = canonicaliseForCycle(resolvedPath);
        for (const auto &entry : stack) {
            if (entry == canon) {
                TemplateCycle err(
                    "<sce:use template=\"" + templateHref +
                    "\">: cycle detected (" + renderChain(stack, canon) + ")");
                err.setLocation(useLocation);
                throw err;
            }
        }

        std::string templateText;
        try {
            templateText = readTemplateText(resolvedPath, templateHref);
        } catch (TemplateReadError &e) {
            e.setLocation(useLocation);
            throw;
        }
        const auto bindings = collectUseBindings(useNode);
        SubstituteIntoTemplateResult substitution;
        try {
            substitution = substituteIntoTemplateWithMap(
                templateText, resolvedPath, templateHref, bindings,
                contentFile, callerRow, callerCol);
        } catch (TemplateError &e) {
            // The `<sce:use>` failure surfaces at the caller site —
            // even when the root cause (malformed template,
            // unknown/missing param) sits inside the template file.
            // Author-facing diagnostics point at the call site per
            // RFC §6.3 Q3 depth-1 rule so the operator sees which
            // `<sce:use>` to fix first.
            if (!e.location().has_value()) {
                e.setLocation(useLocation);
            }
            throw;
        }

        stack.push_back(canon);
        const std::filesystem::path nestedBase = resolvedPath.parent_path();
        TemplateExpandResult nested;
        try {
            nested = expandImpl(substitution.substituted, resolvedPath,
                                 nestedBase, depth + 1, stack,
                                 substitution.positions);
        } catch (TemplateError &e) {
            stack.pop_back();
            // Nested failures already carry an inner location when
            // available; fall back to the caller's `<sce:use>` only
            // when nothing downstream set one.
            if (!e.location().has_value()) {
                e.setLocation(useLocation);
            }
            throw;
        } catch (...) {
            stack.pop_back();
            throw;
        }
        stack.pop_back();

        const auto bodyRanges =
            extractTemplateBodyRanges(nested.expanded_text, templateHref);
        for (const auto &range : bodyRanges) {
            const std::size_t segStart = out.size();
            out.append(nested.expanded_text.substr(
                range.start, range.end - range.start));
            outMap.append_mapped_substring(nested.positions, range.start,
                                            range.end, segStart);
        }

        cursor = useRange.end;
    }

    // Tail [cursor, content.size()) — bytes unchanged from `content`.
    if (cursor < content.size()) {
        const std::size_t outStart = out.size();
        out.append(content.substr(cursor));
        outMap.append_mapped_substring(inputMap, cursor, content.size(),
                                        outStart);
    }

    return TemplateExpandResult{std::move(out), std::move(outMap)};
}

}  // namespace

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

    const std::filesystem::path selfFile{std::string{selfPath}};
    const std::filesystem::path baseFile{std::string{baseDir}};

    std::vector<std::filesystem::path> stack;
    if (!selfPath.empty()) {
        std::error_code ec;
        auto canon = std::filesystem::canonical(selfFile, ec);
        stack.push_back(ec ? selfFile : canon);
    }

    const auto inputMap = PositionMap::identity(selfFile, content);
    return detail::expandImpl(content, selfFile, baseFile, /*depth=*/0, stack,
                               inputMap);
}

}  // namespace SCE::parsing
