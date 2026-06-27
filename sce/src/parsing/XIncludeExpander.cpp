// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/XIncludeExpander.h"
#include "parsing/TemplateExpander.h"

#include <pugixml.hpp>

#include <filesystem>
#include <fstream>
#include <functional>
#include <sstream>
#include <string>
#include <system_error>
#include <vector>

// String-level `<xi:include>` expander. Mirrors
// `sce-build/src/xinclude.rs::expand`. See the header comment for
// the module-level contract.

namespace SCE::parsing {

namespace {

// Re-use the TemplateExpander byte-range type so an `<xi:include>`
// element's source span can compose with the same primitives.
using detail::ByteRange;

// One discovered top-level `<xi:include>` (or bare `<include>`)
// element: its byte span in the caller source, plus the pugixml
// node handle for attribute access. Mirrors Rust `roxmltree::Node`
// returned from `collect_includes`.
struct XIncludeHit {
    ByteRange range;
    pugi::xml_node node;
};

// True when `child` is an `<xi:include>` / `<include>` element.
// Lenient on the namespace prefix to match the local-name match
// of the DOM-mutating pugixml path this expander replaced and
// Rust `collect_includes_into`'s namespace branch
// (sce-build/src/xinclude.rs).
bool isIncludeElement(const pugi::xml_node &child) {
    if (child.type() != pugi::node_element) {
        return false;
    }
    const std::string name = child.name();
    return name == "include" || name == "xi:include";
}

// Resolve `child.offset_debug()` to the byte offset of its leading
// `<`. pugixml occasionally reports the offset one past `<` for
// elements after a quoted-attribute heavy parent; the same
// normalisation is applied in TemplateExpander.cpp:746-749.
std::size_t leadingAngleOffset(const pugi::xml_node &child,
                               std::string_view content) {
    const auto signedOffset = child.offset_debug();
    if (signedOffset < 0) {
        return std::string_view::npos;
    }
    std::size_t start = static_cast<std::size_t>(signedOffset);
    if (start > 0 && start < content.size() && content[start] != '<' &&
        content[start - 1] == '<') {
        start -= 1;
    }
    return start;
}

// Walk `node`'s subtree collecting top-level `<xi:include>` /
// `<include>` byte ranges. "Top-level" matches Rust
// `collect_includes_into`: a child include element terminates the
// recursion at that branch — nested includes inside the included
// fragment are handled by the outer recursion in `expandImpl`.
// Mirrors `sce-build/src/xinclude.rs::collect_includes_into`.
void collectIncludesInto(pugi::xml_node node, std::string_view content,
                         std::vector<XIncludeHit> &out) {
    for (const auto &child : node.children()) {
        if (child.type() != pugi::node_element) {
            continue;
        }
        if (isIncludeElement(child)) {
            const std::size_t start = leadingAngleOffset(child, content);
            if (start == std::string_view::npos) {
                continue;
            }
            const std::string fullName = child.name();
            const std::size_t end =
                detail::findElementEnd(content, start, fullName);
            out.push_back(XIncludeHit{ByteRange{start, end}, child});
            continue;
        }
        collectIncludesInto(child, content, out);
    }
}

// Render the byte slice of `expanded` covered by the children of
// the included document's root element. Mirrors Rust
// `render_root_children` in `sce-build/src/xinclude.rs`. The root
// element wrapper is dropped (the included document is treated as
// a fragment) and every element / text / comment child is preserved
// so SCXML mixed content survives the splice.
//
// Returns `{slice, [start, end)}` so the caller can compose the
// nested `PositionMap` against exactly the bytes spliced into the
// outer output via `append_mapped_substring`.
struct RootChildrenRender {
    std::string text;
    std::size_t start;
    std::size_t end;
};

RootChildrenRender renderRootChildren(std::string_view expanded,
                                      std::string_view href) {
    pugi::xml_document doc;
    const auto parseResult = doc.load_buffer(expanded.data(), expanded.size());
    if (!parseResult) {
        throw XIncludeMalformed(
            "<xi:include href=\"" + std::string(href) +
            "\">: included file is malformed: " + parseResult.description());
    }
    const auto root = doc.document_element();
    if (!root || !root.first_child()) {
        return RootChildrenRender{std::string(), 0, 0};
    }

    // First child's leading `<` (or text-node start) — pugixml
    // text nodes report their content offset directly.
    const auto firstChild = root.first_child();
    std::size_t start;
    if (firstChild.type() == pugi::node_element) {
        start = leadingAngleOffset(firstChild, expanded);
        if (start == std::string_view::npos) {
            return RootChildrenRender{std::string(), 0, 0};
        }
    } else {
        const auto signedOffset = firstChild.offset_debug();
        if (signedOffset < 0) {
            return RootChildrenRender{std::string(), 0, 0};
        }
        start = static_cast<std::size_t>(signedOffset);
    }

    // Last child's end. Element children resolve to the byte past
    // their matching close tag. Text / comment children resolve to
    // the start of the root's closing tag, found by scanning back
    // from the root element span end.
    const auto lastChild = root.last_child();
    std::size_t end;
    if (lastChild.type() == pugi::node_element) {
        const std::size_t lastStart = leadingAngleOffset(lastChild, expanded);
        if (lastStart == std::string_view::npos) {
            return RootChildrenRender{std::string(), 0, 0};
        }
        const std::string lastName = lastChild.name();
        end = detail::findElementEnd(expanded, lastStart, lastName);
    } else {
        // Find the root element's closing tag start by scanning back
        // from the root span. The signed offset points at root's
        // leading `<`; rfind locates `</rootName` in the trailing
        // segment so text-only children do not pull in the close
        // tag bytes.
        const std::size_t rootStart = leadingAngleOffset(root, expanded);
        if (rootStart == std::string_view::npos) {
            return RootChildrenRender{std::string(), 0, 0};
        }
        const std::string rootName = root.name();
        const std::string closeNeedle = "</" + rootName;
        const std::size_t closePos =
            std::string_view(expanded).rfind(closeNeedle, expanded.size());
        if (closePos == std::string_view::npos || closePos < rootStart) {
            return RootChildrenRender{std::string(), 0, 0};
        }
        end = closePos;
    }

    if (end <= start) {
        return RootChildrenRender{std::string(), 0, 0};
    }
    return RootChildrenRender{
        std::string(expanded.substr(start, end - start)), start, end};
}

// Resolve `href` against the same precedence ladder
// `PugiXMLDocument::resolveFilePathInBase` uses (and the replaced
// DOM-mutation path used): absolute → base directory →
// operator-configured include directories → current working
// directory. Mirrors Rust `sce-build/src/xinclude.rs::resolve_href`.
// `includeDirs` is the `--include-dir` search path, tried in
// declaration order after `baseDir`. On miss, populates `searched`
// with the paths tried (so the diagnostic can show where the operator
// should put the file) and returns an empty path.
std::filesystem::path resolveHref(std::string_view href,
                                  const std::filesystem::path &baseDir,
                                  const std::vector<std::string> &includeDirs,
                                  std::vector<std::string> &searched) {
    namespace fs = std::filesystem;
    fs::path hrefPath(std::string{href});

    std::error_code ec;
    if (hrefPath.is_absolute()) {
        if (fs::exists(hrefPath, ec)) {
            return hrefPath;
        }
        searched.push_back(hrefPath.string());
        return {};
    }

    if (!baseDir.empty()) {
        const fs::path candidate = baseDir / hrefPath;
        if (fs::exists(candidate, ec)) {
            return candidate;
        }
        searched.push_back(candidate.string());
    }
    for (const auto &dir : includeDirs) {
        const fs::path candidate = fs::path(dir) / hrefPath;
        if (fs::exists(candidate, ec)) {
            return candidate;
        }
        searched.push_back(candidate.string());
    }
    if (fs::exists(hrefPath, ec)) {
        return hrefPath;
    }
    searched.push_back(hrefPath.string());
    return {};
}

// Render the cycle chain for a `Cycle` diagnostic. Joins the live
// stack plus the offending next entry with U+2192 ` → ` separators
// — same shape as Rust `render_chain`
// (sce-build/src/xinclude.rs:514-518).
std::string renderCycleChain(const std::vector<std::filesystem::path> &stack,
                             const std::filesystem::path &next) {
    std::string out;
    for (const auto &entry : stack) {
        if (!out.empty()) {
            out.append(" → ");
        }
        out.append(entry.string());
    }
    if (!out.empty()) {
        out.append(" → ");
    }
    out.append(next.string());
    return out;
}

// Reject XInclude features the pugixml runtime does not implement.
// Mirrors Rust `reject_unsupported`: `parse="text"`, `xpointer=`,
// and `<xi:fallback>` children. The C++ expander preserves the
// same rejection set so AOT and Interpreter agree on which inputs
// are accepted.
void rejectUnsupportedFeatures(const pugi::xml_node &node,
                               std::string_view href) {
    const auto parseAttr = node.attribute("parse");
    if (parseAttr) {
        const std::string mode = parseAttr.value();
        if (!mode.empty() && mode != "xml") {
            throw XIncludeUnsupported(
                "<xi:include href=\"" + std::string(href) +
                "\">: unsupported feature: parse=\"" + mode +
                "\" (only parse=\"xml\" is supported)");
        }
    }
    if (node.attribute("xpointer")) {
        throw XIncludeUnsupported(
            "<xi:include href=\"" + std::string(href) +
            "\">: unsupported feature: xpointer selection is not implemented");
    }
    for (const auto &child : node.children()) {
        if (child.type() != pugi::node_element) {
            continue;
        }
        const std::string n = child.name();
        if (n == "fallback" || n == "xi:fallback") {
            throw XIncludeUnsupported(
                "<xi:include href=\"" + std::string(href) +
                "\">: unsupported feature: <xi:fallback> alternative content "
                "is not implemented");
        }
    }
}

// Read the full text of `path`. Throws `XIncludeReadError` with
// the offending href baked into the message on read failure.
std::string readFragmentText(const std::filesystem::path &path,
                             std::string_view href) {
    std::ifstream ifs(path, std::ios::binary);
    if (!ifs) {
        throw XIncludeReadError(
            "<xi:include href=\"" + std::string(href) +
            "\">: cannot read file: " + path.string());
    }
    std::ostringstream buf;
    buf << ifs.rdbuf();
    return buf.str();
}

// Recursive expander. Mirrors `sce-build/src/xinclude.rs::expand_impl`
// line-for-line. Public entry is `expandStringX`, which seeds the
// cycle stack and invokes this with `depth = 0`.
XIncludeExpandResult expandImpl(std::string_view content,
                                const std::filesystem::path &contentFile,
                                const std::filesystem::path &baseDir,
                                const std::vector<std::string> &includeDirs,
                                unsigned depth,
                                std::vector<std::filesystem::path> &stack) {
    if (depth >= MAX_XINCLUDE_DEPTH) {
        throw XIncludeTooDeep(
            "<xi:include> nesting exceeds depth limit of " +
            std::to_string(MAX_XINCLUDE_DEPTH));
    }

    pugi::xml_document doc;
    const auto parseResult = doc.load_buffer(content.data(), content.size());
    if (!parseResult) {
        throw XIncludeMalformed(
            std::string("<xi:include> source document is malformed: ") +
            parseResult.description());
    }

    std::vector<XIncludeHit> includes;
    const auto root = doc.document_element();
    if (root) {
        collectIncludesInto(root, content, includes);
    }

    if (includes.empty()) {
        // Identity short-circuit: output is a 1:1 copy of `content`
        // from `contentFile`. Mirrors Rust early return at
        // sce-build/src/xinclude.rs:228-235.
        return XIncludeExpandResult{
            std::string(content),
            PositionMap::identity(contentFile, content)};
    }

    std::string out;
    out.reserve(content.size());
    PositionMap map;
    map.register_file(contentFile, content);
    std::size_t cursor = 0;

    for (const auto &hit : includes) {
        const auto &range = hit.range;
        const auto &node = hit.node;

        // Prefix region: bytes [cursor, range.start) copy
        // unchanged from `contentFile`. Tag with `FileOrigin` so
        // diagnostics resolve to author coords.
        if (cursor < range.start) {
            const std::size_t outStart = out.size();
            out.append(content.substr(cursor, range.start - cursor));
            map.push_entry(outStart, out.size(),
                           FileOrigin{contentFile, cursor});
        }

        // href validation comes before unsupported-feature rejection
        // so the diagnostic can name the href in its message even
        // when the feature itself is what fails. Empty / missing
        // href is its own typed failure.
        const auto hrefAttr = node.attribute("href");
        const std::string href = hrefAttr ? hrefAttr.value() : std::string();
        if (href.empty()) {
            throw XIncludeMissingHref(
                "<xi:include> missing or empty `href` attribute");
        }

        rejectUnsupportedFeatures(node, href);

        std::vector<std::string> searched;
        const auto resolved = resolveHref(href, baseDir, includeDirs, searched);
        if (resolved.empty()) {
            std::string trail;
            for (const auto &entry : searched) {
                if (!trail.empty()) {
                    trail.append(", ");
                }
                trail.append(entry);
            }
            throw XIncludeNotFound(
                "<xi:include href=\"" + href +
                "\">: file not found (searched: " + trail + ")");
        }

        // Cycle detection. Canonicalise so `./foo.xml` and `foo.xml`
        // resolve to the same stack entry. Mirrors Rust
        // canonicalize-or-fallback at sce-build/src/xinclude.rs:283.
        std::error_code canonEc;
        auto canon = std::filesystem::canonical(resolved, canonEc);
        if (canonEc) {
            canon = resolved;
        }
        for (const auto &entry : stack) {
            if (entry == canon) {
                throw XIncludeCycle(
                    "<xi:include href=\"" + href +
                    "\">: cycle detected (" + renderCycleChain(stack, canon) +
                    ")");
            }
        }

        const std::string raw = readFragmentText(resolved, href);

        stack.push_back(canon);
        XIncludeExpandResult nested;
        try {
            const std::filesystem::path nestedBase = resolved.parent_path();
            nested =
                expandImpl(raw, resolved, nestedBase, includeDirs, depth + 1, stack);
        } catch (...) {
            stack.pop_back();
            throw;
        }
        stack.pop_back();

        const auto rendered =
            renderRootChildren(nested.expanded_text, href);
        const std::size_t spliceStart = out.size();
        out.append(rendered.text);
        // Compose nested map for the bytes just spliced in: clip
        // nested entries to `[rendered.start, rendered.end)`, then
        // shift them so they land at `spliceStart` in the outer map.
        map.append_mapped_substring(nested.positions, rendered.start,
                                    rendered.end, spliceStart);

        cursor = range.end;
    }

    // Tail: copy the unchanged suffix after the last include.
    if (cursor < content.size()) {
        const std::size_t outStart = out.size();
        out.append(content.substr(cursor));
        map.push_entry(outStart, out.size(),
                       FileOrigin{contentFile, cursor});
    }

    return XIncludeExpandResult{std::move(out), std::move(map)};
}

}  // namespace

XIncludeExpandResult expandStringX(std::string_view content,
                                   std::string_view selfPath,
                                   std::string_view baseDir,
                                   const std::vector<std::string> &includeDirs) {
    // Fast path: a document with no "include" substring cannot
    // contain `<xi:include>` or `<include>`, so the output is the
    // identity. Mirrors Rust `expand` at
    // sce-build/src/xinclude.rs:181-186.
    if (content.find("include") == std::string_view::npos) {
        return XIncludeExpandResult{
            std::string(content),
            PositionMap::identity(std::filesystem::path(std::string(selfPath)),
                                  content)};
    }

    const std::filesystem::path selfFile{std::string(selfPath)};
    const std::filesystem::path baseFile{std::string(baseDir)};

    std::vector<std::filesystem::path> stack;
    if (!selfPath.empty()) {
        std::error_code ec;
        auto canon = std::filesystem::canonical(selfFile, ec);
        stack.push_back(ec ? selfFile : canon);
    }

    return expandImpl(content, selfFile, baseFile, includeDirs, /*depth=*/0,
                      stack);
}

}  // namespace SCE::parsing
