// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SSOT byte-equivalence harness for `sce:template` expansion.
//
// Drives one fixture through both producers and asserts their
// canonicalised outputs are byte-identical:
//
//   1. Rust `sce-codegen expand <fixture>` captures post-preprocessor
//      text (XInclude + sce:template expansion per the Rust expander).
//   2. C++ `PugiXMLDocument::processXInclude` +
//      `processSceTemplate` run the runtime equivalents.
//
// Canonicalisation pipes both outputs through pugixml's
// `format_raw` serialisation so any whitespace / attribute-order
// differences between the string-editing Rust expander and the
// DOM-editing C++ runtime normalise away — the comparison becomes
// a test of *DOM shape equivalence*, which is what the SSOT
// contract actually cares about.
//
// Success-path fixtures byte-diff canonicalised outputs; error-path
// fixtures compare typed discriminants instead of message text (see
// `runErrorParityFixture` below).

#include "parsing/PugiXMLParser.h"
#include "parsing/TemplateError.h"
#include "parsing/TemplateExpander.h"
#include <algorithm>
#include <cctype>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <pugixml.hpp>
#include <sstream>
#include <string>
#include <string_view>
#include <typeinfo>
#include <vector>

namespace {

// Deleter for a `popen` stream.
//
// glibc declares `pclose` with function attributes, and naming its type
// through `decltype(&pclose)` as a `unique_ptr` template argument drops
// them — which `-Wignored-attributes` reports and `-Werror` promotes.
// A deleter type that calls `pclose` from an ordinary member body has no
// attributes to lose, so the pointer type stays attribute-free.
struct PipeCloser {
    void operator()(FILE *pipe) const {
        if (pipe != nullptr) {
            pclose(pipe);
        }
    }
};

// Owning handle for a `popen` stream. Call sites that need `pclose`'s
// exit status take it back with `release()` and close it themselves.
using UniquePipe = std::unique_ptr<FILE, PipeCloser>;

// Run `sce-codegen expand <scxml>` as a subprocess and capture its
// stdout. The command binary path is passed via the
// `SCE_CODEGEN_BIN` env var that CMake plumbs in at add_test time,
// so the harness does not hardcode a workspace-relative path and
// works equally well under CI, local cmake --build, or install-tree
// testing.
std::string runSceCodegenExpand(const std::string &sceCodegenBin, const std::string &scxmlPath,
                                const std::vector<std::string> &includeDirs = {}) {
    // Quote both arguments to survive shell metacharacters. The
    // inner paths come from CMake / the fixture tree and are not
    // attacker-controlled; quoting here is hygiene, not a security
    // boundary.
    std::string cmd = "\"" + sceCodegenBin + "\" expand";
    for (const auto &dir : includeDirs) {
        cmd += " --include-dir \"" + dir + "\"";
    }
    cmd += " \"" + scxmlPath + "\" 2>/dev/null";
    UniquePipe pipe(popen(cmd.c_str(), "r"));
    if (!pipe) {
        return {};
    }
    std::string out;
    char buf[4096];
    while (std::fgets(buf, sizeof(buf), pipe.get()) != nullptr) {
        out += buf;
    }
    return out;
}

// Capture of a failing `sce-codegen expand` invocation. Stderr
// carries the NDJSON diagnostic in `--error-format json` mode
// (SCE_ERROR_CONTRACT.md §10), stdout should be empty on the
// error path but is captured for diagnosis if a future regression
// leaks partial output.
struct SceCodegenExpandError {
    int exitCode = 0;
    std::string stdoutText;
    std::string stderrText;
};

// Run `sce-codegen --error-format json expand <scxml>` and capture
// both streams plus the exit code. Used by the error-path
// parity harness to assert the Rust producer fails with a specific
// DiagnosticCode before the C++ side is probed for an equivalent
// typed exception.
SceCodegenExpandError runSceCodegenExpandExpectFail(const std::string &sceCodegenBin, const std::string &scxmlPath) {
    // Merge stderr into stdout via `2>&1` so popen captures both
    // streams; the tests inspect the combined text, and Rust's
    // success-path stdout is empty on error so the NDJSON banner
    // is unambiguously the failure record. Using a temporary stderr
    // redirect file avoids shell-quoting surprises on complex paths.
    const std::string cmd = "\"" + sceCodegenBin + "\" --error-format json expand \"" + scxmlPath + "\" 2>&1";
    SceCodegenExpandError result;
    UniquePipe pipe(popen(cmd.c_str(), "r"));
    if (!pipe) {
        result.exitCode = -1;
        return result;
    }
    char buf[4096];
    while (std::fgets(buf, sizeof(buf), pipe.get()) != nullptr) {
        // Stderr-merged mode — treat all captured bytes as
        // "diagnostic text" for the grep-based assertions below.
        result.stderrText += buf;
    }
    const int raw = pclose(pipe.release());
    result.exitCode = raw;
    return result;
}

// Asserts a substring appears in `haystack`. Keeps the failure
// message compact so GTest prints both the expected code and the
// full captured stream without the helper doing its own formatting.
bool diagnosticCodeAppears(const std::string &haystack, const std::string &code) {
    // DiagnosticCode fields are emitted as `"code":"xml/..."` per
    // SCE_ERROR_CONTRACT.md §10. Matching the quoted form avoids a
    // false positive if the code string appears inside a message
    // body that happens to name it.
    const std::string needle = "\"code\":\"" + code + "\"";
    return haystack.find(needle) != std::string::npos;
}

// Extracted location fields from a Rust `--error-format=json`
// NDJSON diagnostic line. Carries the basename rather than the
// full path so cross-checks against C++ `SourcePos::file` work
// under both absolute-path (C++) and CWD-relative (Rust NDJSON)
// conventions without either side having to canonicalise.
struct RustJsonLocation {
    std::string file;
    std::uint32_t line = 0;
    std::uint32_t col = 0;
};

// Extract the NDJSON `"location":{"file":"...","line":N,"col":M}`
// fragment from `haystack`. Minimal substring parser — the JSON
// emitter (see `forge/error::Located::to_ndjson_line`) always
// orders fields file/line/col, never splits across newlines, and
// only escapes the path's interior quotes with `\"` which a raw
// scan can ignore in practice because test fixture paths never
// contain embedded double quotes. Returns `std::nullopt` when any
// of the three fields is absent or unparseable so the caller can
// surface a pointed assertion message rather than a generic
// "failed to parse" blob.
std::optional<RustJsonLocation> extractRustJsonLocation(const std::string &haystack) {
    const std::size_t locPos = haystack.find("\"location\":{");
    if (locPos == std::string::npos) {
        return std::nullopt;
    }

    const std::string fileKey = "\"file\":\"";
    const std::size_t fileStart = haystack.find(fileKey, locPos);
    if (fileStart == std::string::npos) {
        return std::nullopt;
    }
    const std::size_t fileValueStart = fileStart + fileKey.size();
    const std::size_t fileValueEnd = haystack.find('"', fileValueStart);
    if (fileValueEnd == std::string::npos) {
        return std::nullopt;
    }

    const std::string lineKey = "\"line\":";
    const std::size_t lineKeyPos = haystack.find(lineKey, fileValueEnd);
    if (lineKeyPos == std::string::npos) {
        return std::nullopt;
    }
    const std::size_t lineValueStart = lineKeyPos + lineKey.size();
    std::size_t lineValueEnd = lineValueStart;
    while (lineValueEnd < haystack.size() && std::isdigit(static_cast<unsigned char>(haystack[lineValueEnd]))) {
        ++lineValueEnd;
    }
    if (lineValueEnd == lineValueStart) {
        return std::nullopt;
    }

    const std::string colKey = "\"col\":";
    const std::size_t colKeyPos = haystack.find(colKey, lineValueEnd);
    if (colKeyPos == std::string::npos) {
        return std::nullopt;
    }
    const std::size_t colValueStart = colKeyPos + colKey.size();
    std::size_t colValueEnd = colValueStart;
    while (colValueEnd < haystack.size() && std::isdigit(static_cast<unsigned char>(haystack[colValueEnd]))) {
        ++colValueEnd;
    }
    if (colValueEnd == colValueStart) {
        return std::nullopt;
    }

    RustJsonLocation out;
    std::filesystem::path filePath(haystack.substr(fileValueStart, fileValueEnd - fileValueStart));
    out.file = filePath.filename().string();
    out.line = static_cast<std::uint32_t>(std::stoul(haystack.substr(lineValueStart, lineValueEnd - lineValueStart)));
    out.col = static_cast<std::uint32_t>(std::stoul(haystack.substr(colValueStart, colValueEnd - colValueStart)));
    return out;
}

// Byte-offset → 1-based (row, col) translator mirroring
// `SCE::parsing::rowcol_to_offset`'s inverse. The fixture helpers
// below use this to derive the expected caller-site coordinate
// directly from the fixture source bytes, so a fixture author
// who moves the `<sce:use>` element vertically does not have to
// hand-update a hardcoded expected row.
struct RowCol {
    std::uint32_t row;
    std::uint32_t col;
};

RowCol offsetToRowCol(std::string_view text, std::size_t offset) {
    std::uint32_t row = 1;
    std::uint32_t col = 1;
    const std::size_t end = std::min(offset, text.size());
    for (std::size_t i = 0; i < end; ++i) {
        const unsigned char byte = static_cast<unsigned char>(text[i]);
        if (byte == '\n') {
            ++row;
            col = 1;
        } else if ((byte & 0xC0) != 0x80) {
            // Count Unicode scalar starts (skip continuation bytes)
            // so column precision matches `PositionMap::lookup`'s
            // Unicode-scalar-counting convention — see comment on
            // `SourcePos::col` in parsing/PositionMap.h. `col` is
            // updated eagerly so that after processing every byte
            // below `offset` it equals the column of the byte *at*
            // `offset` (1-based).
            ++col;
        }
    }
    return {row, col};
}

std::string readFileOrEmpty(const std::string &path) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        return {};
    }
    std::ostringstream buf;
    buf << in.rdbuf();
    return buf.str();
}

// Round-trip an XML string through pugixml so both producers are
// compared at the same serialisation convention. `format_raw` omits
// indentation and trailing whitespace; `format_no_declaration`
// strips the `<?xml version="1.0"?>` prologue because the Rust
// producer preserves the original document's declaration bytes
// while the pugixml producer re-emits them with its own
// conventions — stripping both sides leaves DOM content only,
// which is the useful comparison.
std::string canonicalise(const std::string &xmlText) {
    pugi::xml_document doc;
    auto result = doc.load_buffer(xmlText.data(), xmlText.size());
    if (!result) {
        return std::string("<<PARSE_ERROR:") + result.description() + ">>";
    }
    std::ostringstream os;
    doc.save(os, "", pugi::format_raw | pugi::format_no_declaration);
    return os.str();
}

std::string canonicaliseDocument(const pugi::xml_document &doc) {
    std::ostringstream os;
    doc.save(os, "", pugi::format_raw | pugi::format_no_declaration);
    return os.str();
}

// Load a fixture into a pugi::xml_document and run the C++
// preprocessor sequence (processXInclude → processSceTemplate),
// returning the canonicalised post-expansion text. The harness
// owns the raw `pugi::xml_document` (not via IXMLParser) so it
// can re-serialise it after preprocessing without adding a
// serialise() method to the production interface for a single
// test consumer.
std::string runCppPreprocessors(const std::string &scxmlPath, const std::vector<std::string> &includeDirs = {}) {
    // Read the file into a buffer so the `PugiXMLDocument` wrapper
    // can stash the exact bytes pugixml parsed — `processSceTemplate`
    // needs a stable view of the author's source text to build a
    // `SCE::parsing::PositionMap` identity entry with author-source
    // coordinates. Mirrors `PugiXMLParser::parseFile`
    // so the harness sees the same plumbing as production callers.
    const std::string sourceText = readFileOrEmpty(scxmlPath);
    if (sourceText.empty()) {
        return std::string("<<CPP_LOAD_ERROR:could not read source>>");
    }

    auto pugiDoc = std::make_shared<pugi::xml_document>();
    auto loadResult = pugiDoc->load_buffer(sourceText.data(), sourceText.size());
    if (!loadResult) {
        return std::string("<<CPP_LOAD_ERROR:") + loadResult.description() + ">>";
    }

    auto sceDoc = std::make_shared<SCE::PugiXMLDocument>(pugiDoc);
    // Base path governs `<xi:include href="...">` and
    // `<sce:use template="...">` resolution; source path seeds the
    // cycle-detection stack so a fixture that points its top-level
    // `<sce:use template="main.scxml"/>` back at itself is caught
    // symmetrically with Rust (which seeds the stack via the
    // `expand(self_path, ...)` argument in
    // sce-build/src/template.rs). Keeps the test driver aligned with
    // the SCXMLParser.cpp flow so fixtures that exercise xincludes,
    // templates, or nested/cyclic references all see the same
    // plumbing on both producers.
    {
        std::string basePath = scxmlPath;
        auto slash = basePath.find_last_of('/');
        if (slash != std::string::npos) {
            sceDoc->setBasePath(basePath.substr(0, slash));
        }
    }
    sceDoc->setSourcePath(scxmlPath);
    sceDoc->setSourceText(sourceText);
    // Mirror the `--include-dir` search path the Rust side receives so
    // both producers resolve bare `href` / `template` names against the
    // same directory list.
    sceDoc->setIncludeDirs(includeDirs);

    // §wire-W4.5 D1: process*() return PositionMap directly.
    const auto xPositions = sceDoc->processXInclude();
    sceDoc->processSceTemplate(xPositions);

    return canonicaliseDocument(*pugiDoc);
}

std::string resolveFixture(const char *name) {
    const char *root = std::getenv("SCE_TEMPLATE_PARITY_FIXTURES_ROOT");
    EXPECT_NE(root, nullptr) << "SCE_TEMPLATE_PARITY_FIXTURES_ROOT env var must be "
                                "set by CMake add_test ENVIRONMENT";
    if (root == nullptr) {
        return {};
    }
    return std::string(root) + "/" + name + "/main.scxml";
}

}  // namespace

// Single-fixture parity check shared by every harness entry.
// Extracted from TemplateParity.PassthroughNoUse once the second
// fixture arrived — keeps new fixtures to one `TEST(...)` macro
// body plus one CMakeLists line.
void runParityFixture(const std::string &fixtureName) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN env var must be set by CMake "
                               "add_test ENVIRONMENT";

    const std::string fixturePath = resolveFixture(fixtureName.c_str());
    ASSERT_FALSE(fixturePath.empty());

    const std::string rustText = runSceCodegenExpand(bin, fixturePath);
    ASSERT_FALSE(rustText.empty()) << "sce-codegen expand produced no output "
                                      "for fixture: "
                                   << fixturePath;

    const std::string rustCanonical = canonicalise(rustText);
    const std::string cppCanonical = runCppPreprocessors(fixturePath);

    // Textual rather than binary comparison; if bytes differ GTest
    // prints both strings so the developer sees which DOM subtree
    // diverged without rerunning under a debugger.
    ASSERT_EQ(rustCanonical, cppCanonical) << "Template parity violation on fixture '" << fixtureName
                                           << "': Rust-canonical and C++-canonical expansion outputs "
                                              "diverge. This means `sce-codegen expand` and "
                                              "`PugiXMLDocument::processSceTemplate` disagree on the "
                                              "effective post-preprocessor document — the asymmetry "
                                              "this harness exists to close.";
}

// Parity check for a fixture whose `<xi:include>` / `<sce:use>`
// fragments resolve by bare name against an `--include-dir` search
// path rather than a relative path. `relIncludeDirs` are resolved
// against the fixture's own directory and handed to BOTH producers —
// `sce-codegen expand --include-dir ...` on the Rust side and
// `PugiXMLDocument::setIncludeDirs(...)` on the C++ side — so the
// comparison proves the two engines apply the same search-path
// precedence (absolute -> base -> include dirs -> cwd).
void runParityFixtureWithIncludeDirs(const std::string &fixtureName, const std::vector<std::string> &relIncludeDirs) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN env var must be set by CMake "
                               "add_test ENVIRONMENT";
    const char *root = std::getenv("SCE_TEMPLATE_PARITY_FIXTURES_ROOT");
    ASSERT_NE(root, nullptr) << "SCE_TEMPLATE_PARITY_FIXTURES_ROOT env var must "
                                "be set by CMake add_test ENVIRONMENT";

    const std::string fixturePath = resolveFixture(fixtureName.c_str());
    ASSERT_FALSE(fixturePath.empty());

    const std::string fixtureDir = std::string(root) + "/" + fixtureName;
    std::vector<std::string> includeDirs;
    includeDirs.reserve(relIncludeDirs.size());
    for (const auto &rel : relIncludeDirs) {
        includeDirs.push_back(fixtureDir + "/" + rel);
    }

    const std::string rustText = runSceCodegenExpand(bin, fixturePath, includeDirs);
    ASSERT_FALSE(rustText.empty()) << "sce-codegen expand produced no output "
                                      "for include-dir fixture: "
                                   << fixturePath;

    const std::string rustCanonical = canonicalise(rustText);
    const std::string cppCanonical = runCppPreprocessors(fixturePath, includeDirs);

    ASSERT_EQ(rustCanonical, cppCanonical) << "Template parity violation on include-dir fixture '" << fixtureName
                                           << "': Rust-canonical and C++-canonical expansion outputs diverge. "
                                              "The `--include-dir` search path resolved differently between "
                                              "`sce-codegen expand` and `PugiXMLDocument::setIncludeDirs` + "
                                              "`processXInclude`/`processSceTemplate`.";
}

// Fixture — no `<sce:use>` in the document, both producers
// return the input unchanged.
TEST(TemplateParity, PassthroughNoUse) {
    runParityFixture("passthrough_no_use");
}

// Fixture — `<sce:use>` resolves a template by bare name against an
// `--include-dir` (the template lives in a sibling `_inc/` directory,
// not next to the caller). Proves the search-path precedence is
// byte-identical across both engines.
TEST(TemplateParity, IncludeDirTemplate) {
    runParityFixtureWithIncludeDirs("include_dir_template", {"_inc"});
}

// Fixture — `<xi:include>` resolves a fragment by bare name against an
// `--include-dir`. Same proof for the XInclude resolver.
TEST(TemplateParity, IncludeDirXInclude) {
    runParityFixtureWithIncludeDirs("include_dir_xinclude", {"_inc"});
}

// Fixture — single `<sce:use>` with one required parameter
// (`port="80"`). First harness entry that actually drives the
// expansion loop: the template body must be spliced into the
// caller's `<state id="idle">` and every `{$port}` replaced with
// `80`. Canonicalisation is what normalises away the string-edit
// vs DOM-edit differences between the two producers.
TEST(TemplateParity, WithParams) {
    runParityFixture("with_params");
}

// Fixture — two-level nested `<sce:use>` chain (main → outer →
// inner). Exercises recursive expansion: the outer template's body
// contains `<sce:use template="inner.scxml" endpoint="host:{$port}"/>`,
// so the caller's `port=8080` must be substituted into the nested
// call's `endpoint` argument BEFORE `inner.scxml` loads — proving
// the substitute-then-recurse order matches between the Rust
// string-editing expander and the C++ DOM-editing runtime. Also
// exercises the per-template base-directory resolution for nested
// lookups (the nested `inner.scxml` reference resolves against
// outer.scxml's parent directory, not the caller's).
TEST(TemplateParity, Nested) {
    runParityFixture("nested");
}

// ── Rust-expander success-path replays ──────────────────────────
//
// Every Rust-expander success-path unit fixture replays here at
// 100% byte-equivalence. The entries above cover the
// passthrough, single-parameter, and nested-multi-file paths;
// the entries below close the remaining success scenarios from
// `sce-build/src/template.rs::tests`: substring false-positives,
// defaulted parameters, sibling independence, single-pass
// semantics on cascading defaults, and undeclared-brace
// passthrough. Each uses the shared `runParityFixture` driver,
// so the check collapses to one TEST macro body per scenario.

// Rust-test replay — "sce:use" appears in a comment and an
// attribute value but no element. Both producers must reach the
// same "no expansion needed" conclusion via element scan, not
// substring search. Source: Rust unit test
// `passthrough_when_substring_but_no_element`.
TEST(TemplateParity, PassthroughSubstringInComment) {
    runParityFixture("passthrough_substring_in_comment");
}

// Rust-test replay — `<sce:param default="TCP"/>` fallback when
// the caller omits the parameter. First harness entry that
// exercises the default-value splice path. Source: Rust unit test
// `default_value_substitutes_when_call_site_omits`.
TEST(TemplateParity, DefaultValue) {
    runParityFixture("default_value");
}

// Rust-test replay — two sibling `<sce:use>` with different
// `n` bindings. Each splice must carry its own parameter map;
// a producer that caches across siblings would leak the second
// binding into the first expansion. Source: Rust unit test
// `sibling_uses_are_independent`.
TEST(TemplateParity, SiblingUses) {
    runParityFixture("sibling_uses");
}

// Rust-test replay — `<sce:param b default="{$a}"/>` emits the
// literal `{$a}` when `b` is omitted (single-pass, literal-only
// substitution). A producer that
// iterated to fixed point would substitute `HIT` here — both
// must stop after one pass. Source: Rust unit test
// `substitution_is_single_pass`.
TEST(TemplateParity, SinglePassSubstitution) {
    runParityFixture("single_pass_substitution");
}

// Rust-test replay — `{$undeclared}` passes through unchanged
// while the declared `{$a}` is substituted. Preserves authored
// literal content that happens to contain brace-dollar tokens.
// Source: Rust unit test `undeclared_braces_pass_through`.
TEST(TemplateParity, UndeclaredBraces) {
    runParityFixture("undeclared_braces");
}

// ── Error-path harness ──────────────────────────────────────────
//
// Introduced with the `cycle_detected` fixture: the first harness
// entry where both producers must FAIL rather than succeed. The
// assertion contract is intentionally looser than the byte-diff
// success path — error messages are UX surface (operator-facing
// text, subject to rewording across releases) while discriminants
// are API surface (DiagnosticCode on the Rust side maps 1:1 to a
// C++ TemplateError subtype). Comparing messages
// would either let the two sides drift without failing or force
// message text to be locked cross-language; the discriminant
// match captures the useful invariant without either problem.

// Run an error-path fixture through both producers and assert
// discriminant equivalence. `rustCode` is the expected
// DiagnosticCode emitted on stderr in `--error-format json` mode;
// `cppMatches(const std::exception&)` is a predicate that returns
// true iff the caught exception's dynamic type is the C++ mirror
// subtype of `rustCode`. Splitting the C++ side into a predicate
// (instead of a class-template type parameter) keeps the helper
// callable from a non-template TEST macro body.
//
// `assertCoordinateAgreement` is the coordinate-agreement opt-in:
// when true, the harness additionally compares the Rust NDJSON
// `location.{file, line, col}` against the C++ `TemplateError::
// location()->{file, row, col}` and fails on mismatch. The
// discriminant-only fixtures (cycle_detected, not_found, malformed,
// missing_attribute) keep the default-false
// contract — coordinate agreement is opt-in per fixture, not
// retroactive.
template <typename CppCheck>
void runErrorParityFixture(const std::string &fixtureName, const std::string &rustCode, const char *cppSubtypeName,
                           CppCheck cppMatches, bool assertCoordinateAgreement = false) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN env var must be set by CMake "
                               "add_test ENVIRONMENT";

    const std::string fixturePath = resolveFixture(fixtureName.c_str());
    ASSERT_FALSE(fixturePath.empty());

    // Rust side: must fail with non-zero exit and emit the
    // expected DiagnosticCode on stderr (merged into our pipe
    // above).
    const auto rustResult = runSceCodegenExpandExpectFail(bin, fixturePath);
    ASSERT_NE(rustResult.exitCode, 0) << "sce-codegen expand unexpectedly succeeded on error-path fixture '"
                                      << fixtureName << "'.\nCaptured output:\n"
                                      << rustResult.stderrText;
    ASSERT_TRUE(diagnosticCodeAppears(rustResult.stderrText, rustCode))
        << "sce-codegen expand on fixture '" << fixtureName << "' did not emit DiagnosticCode '" << rustCode
        << "'. Captured output:\n"
        << rustResult.stderrText;

    // C++ side: running the preprocessors MUST throw a
    // TemplateError subtype whose dynamic type satisfies the
    // `cppMatches` predicate. Anything else (no throw, wrong
    // subtype, unrelated exception) is a parity violation.
    bool threwExpected = false;
    std::string actualType;
    std::string actualMessage;
    std::optional<SCE::parsing::SourcePos> cppLocation;
    try {
        const std::string sourceText = readFileOrEmpty(fixturePath);
        ASSERT_FALSE(sourceText.empty()) << "Failed to read fixture source text: " << fixturePath;
        auto pugiDoc = std::make_shared<pugi::xml_document>();
        auto loadResult = pugiDoc->load_buffer(sourceText.data(), sourceText.size());
        ASSERT_TRUE(loadResult) << "Failed to load C++ fixture: " << loadResult.description();
        auto sceDoc = std::make_shared<SCE::PugiXMLDocument>(pugiDoc);
        {
            std::string basePath = fixturePath;
            auto slash = basePath.find_last_of('/');
            if (slash != std::string::npos) {
                sceDoc->setBasePath(basePath.substr(0, slash));
            }
        }
        sceDoc->setSourcePath(fixturePath);
        sceDoc->setSourceText(sourceText);
        const auto xPositions = sceDoc->processXInclude();
        sceDoc->processSceTemplate(xPositions);
    } catch (const SCE::parsing::TemplateError &ex) {
        actualType = typeid(ex).name();
        actualMessage = ex.what();
        threwExpected = cppMatches(ex);
        cppLocation = ex.location();
    } catch (const std::exception &ex) {
        actualType = typeid(ex).name();
        actualMessage = ex.what();
    }

    ASSERT_TRUE(threwExpected) << "Template parity violation on error-path fixture '" << fixtureName
                               << "': C++ preprocessors did not throw `" << cppSubtypeName
                               << "`. Actual type: " << actualType << ". Actual message: " << actualMessage
                               << ". Rust emitted: " << rustCode
                               << ". Cross-language discriminant parity is the error-path "
                                  "contract.";

    if (!assertCoordinateAgreement) {
        return;
    }

    // Coordinate-agreement check. Reached only for
    // fixtures that opt in via the fourth argument — the
    // discriminant-only fixtures keep their contract
    // unchanged. Compares the Rust NDJSON `location.{file, line,
    // col}` against the C++ `TemplateError::location()` so any
    // drift between the two sides (e.g. one side reporting a
    // post-expansion offset instead of an author-source offset)
    // surfaces as a pointed diff rather than a silent disagreement.
    const auto rustLoc = extractRustJsonLocation(rustResult.stderrText);
    ASSERT_TRUE(rustLoc.has_value()) << "Coordinate-agreement on '" << fixtureName
                                     << "': Rust NDJSON did not carry a parseable "
                                        "`location.{file,line,col}` triple.\n"
                                        "Captured output:\n"
                                     << rustResult.stderrText;
    ASSERT_TRUE(cppLocation.has_value()) << "Coordinate-agreement on '" << fixtureName << "': C++ `" << cppSubtypeName
                                         << "` threw with `TemplateError::location() == std::nullopt`. "
                                            "P2 wires location() at every throw site inside the "
                                            "`<sce:use>` processing loop — see TemplateExpander.cpp "
                                            "`useLocation` stamping.";

    const std::string cppFile = cppLocation->file.filename().string();
    ASSERT_EQ(rustLoc->file, cppFile) << "Coordinate-agreement on '" << fixtureName << "': Rust file='" << rustLoc->file
                                      << "', C++ file='" << cppFile << "'.";
    ASSERT_EQ(rustLoc->line, cppLocation->row)
        << "Coordinate-agreement on '" << fixtureName << "': Rust line=" << rustLoc->line
        << ", C++ row=" << cppLocation->row << ".";
    ASSERT_EQ(rustLoc->col, cppLocation->col)
        << "Coordinate-agreement on '" << fixtureName << "': Rust col=" << rustLoc->col
        << ", C++ col=" << cppLocation->col << ".";
}

// Success-path coordinate-lookup driver. Drives the
// expander directly (not through the full parser) so the assertion
// pins the PositionMap primitive rather than the
// `PugiXMLDocument::processSceTemplate` reparse boundary, which
// discards the expanded text bytes that `PositionMap::lookup`
// keys against. Loads `main.scxml`, runs `expandString`, locates
// the needle in the expanded output, and asserts the resulting
// `SourcePos` matches the coordinate of the caller's `<sce:use>`
// element in the source bytes.
void runCoordinateLookupFixture(const std::string &fixtureName, std::string_view callerNeedle,
                                std::string_view expandedNeedle) {
    const std::string fixturePath = resolveFixture(fixtureName.c_str());
    ASSERT_FALSE(fixturePath.empty());

    const std::string source = readFileOrEmpty(fixturePath);
    ASSERT_FALSE(source.empty()) << "Failed to read success-path fixture: " << fixturePath;

    const std::size_t callerOffset = source.find(callerNeedle);
    ASSERT_NE(callerOffset, std::string::npos) << "Fixture '" << fixtureName
                                               << "' does not contain the caller "
                                                  "needle '"
                                               << callerNeedle
                                               << "' the harness scans for to "
                                                  "derive the expected row/col. Either update the fixture or "
                                                  "the needle to keep them in sync.";
    const RowCol expected = offsetToRowCol(source, callerOffset);

    std::string basePath = fixturePath;
    const auto slash = basePath.find_last_of('/');
    const std::string baseDir = (slash == std::string::npos) ? std::string{} : basePath.substr(0, slash);

    SCE::parsing::TemplateExpandResult result;
    try {
        const auto upstream = SCE::parsing::PositionMap::identity(fixturePath, source);
        result = SCE::parsing::expandString(source, fixturePath, baseDir, {}, upstream);
    } catch (const std::exception &ex) {
        FAIL() << "Fixture '" << fixtureName << "' unexpectedly failed expansion: " << ex.what();
        return;
    }

    const std::size_t expandedOffset = result.expanded_text.find(expandedNeedle);
    ASSERT_NE(expandedOffset, std::string::npos) << "Fixture '" << fixtureName
                                                 << "' expanded output does not "
                                                    "contain the substituted needle '"
                                                 << expandedNeedle << "'. Expander output:\n"
                                                 << result.expanded_text;

    const auto resolved = result.positions.lookup(expandedOffset);
    const std::string resolvedFile = resolved.file.filename().string();
    ASSERT_EQ(resolvedFile, std::filesystem::path(fixturePath).filename().string())
        << "coord_lookup on '" << fixtureName
        << "': depth-1 collapse expected `lookup()` to resolve to the "
           "caller file, got file='"
        << resolvedFile << "'.";
    ASSERT_EQ(resolved.row, expected.row)
        << "coord_lookup on '" << fixtureName << "': `lookup()` row=" << resolved.row
        << " but the `<sce:use>` caller sits at row=" << expected.row << " in the fixture source.";
    ASSERT_EQ(resolved.col, expected.col)
        << "coord_lookup on '" << fixtureName << "': `lookup()` col=" << resolved.col
        << " but the `<sce:use>` caller sits at col=" << expected.col << " in the fixture source.";
}

// Error-path fixture — a.scxml uses b.scxml which uses a.scxml,
// completing a cycle through `main.scxml`. Both producers must
// raise the cycle diagnostic discriminant. First harness entry
// that asserts FAILURE equivalence rather than success bytes.
TEST(TemplateParity, CycleDetected) {
    runErrorParityFixture("cycle_detected", "xml/template-cycle", "SCE::parsing::TemplateCycle",
                          [](const SCE::parsing::TemplateError &ex) -> bool {
                              return dynamic_cast<const SCE::parsing::TemplateCycle *>(&ex) != nullptr;
                          });
}

// Error-path fixture — `<sce:use template="ghost.scxml"/>`
// references a file that does not exist. Rust emits
// `xml/template-not-found`; C++ throws
// `SCE::parsing::TemplateNotFound`. The search trail rendered in
// the diagnostic message is subject to path-format drift
// (absolute vs display form) and is not compared — the
// discriminant match is the API-surface invariant.
TEST(TemplateParity, NotFound) {
    runErrorParityFixture("not_found", "xml/template-not-found", "SCE::parsing::TemplateNotFound",
                          [](const SCE::parsing::TemplateError &ex) -> bool {
                              return dynamic_cast<const SCE::parsing::TemplateNotFound *>(&ex) != nullptr;
                          });
}

// Error-path fixture — template file has the wrong root
// element (`<not-a-template>` instead of `<sce:template>`). Rust
// emits `xml/template-malformed`; C++ throws
// `SCE::parsing::TemplateMalformed`. Malformed is a shared bucket
// for three repair surfaces (wrong root, invalid XML, ill-formed
// `<sce:param>`); this fixture covers wrong-root — the other two
// are exercised by Rust unit tests in `template.rs::tests`.
TEST(TemplateParity, Malformed) {
    runErrorParityFixture("malformed", "xml/template-malformed", "SCE::parsing::TemplateMalformed",
                          [](const SCE::parsing::TemplateError &ex) -> bool {
                              return dynamic_cast<const SCE::parsing::TemplateMalformed *>(&ex) != nullptr;
                          });
}

// Error-path fixture — `<sce:use/>` omits the required
// `template` attribute. Rust emits
// `xml/template-missing-attribute` (with a deterministic
// `add_attribute` fix payload); C++ throws
// `SCE::parsing::TemplateMissingAttribute`. The fix payload is
// not inspected by the discriminant-only contract, but the
// DiagnosticCode assertion pins the wire name so agent dispatch
// keys on the code without having to parse the payload.
TEST(TemplateParity, MissingAttribute) {
    runErrorParityFixture("missing_attribute", "xml/template-missing-attribute",
                          "SCE::parsing::TemplateMissingAttribute", [](const SCE::parsing::TemplateError &ex) -> bool {
                              return dynamic_cast<const SCE::parsing::TemplateMissingAttribute *>(&ex) != nullptr;
                          });
}

// ── Coordinate-parity fixtures ──────────────────────────────────
//
// Three fixtures that opt in to `assertCoordinateAgreement` on the
// error-path harness (or use the success-path `runCoordinateLookup
// Fixture` driver) to pin end-to-end `location.{file, line, col}`
// agreement between the Rust NDJSON and the C++ `TemplateError::
// location()`. They cover the template half of the
// coordinate-remap surface; the coord_xinclude_* fixtures below
// cover the XInclude half.

// Error-path fixture — caller passes `bogus="x"` which the
// template does not declare. Both producers must raise the
// unknown-parameter diagnostic and both must pin its
// `location.{file, line, col}` at the caller's `<sce:use>` in
// main.scxml (depth-1 collapse, RFC §6.3 Q3).
TEST(TemplateParity, CoordCallsiteParam) {
    runErrorParityFixture(
        "coord_callsite_param", "xml/template-unknown-param", "SCE::parsing::TemplateUnknownParam",
        [](const SCE::parsing::TemplateError &ex) -> bool {
            return dynamic_cast<const SCE::parsing::TemplateUnknownParam *>(&ex) != nullptr;
        },
        /*assertCoordinateAgreement=*/true);
}

// P3 error-path fixture — template declares `<sce:param
// required="maybe"/>` (only "true"/"false" are valid). Both
// producers raise Malformed; both pin the coordinate at the
// caller's `<sce:use>` in main.scxml even though the offending
// bytes sit inside the template file — depth-1 collapse pulls
// the diagnostic to the author's call site per RFC §6.3 Q3 so
// repair agents see one pointed line in the caller.
TEST(TemplateParity, CoordTemplateBody) {
    runErrorParityFixture(
        "coord_template_body", "xml/template-malformed", "SCE::parsing::TemplateMalformed",
        [](const SCE::parsing::TemplateError &ex) -> bool {
            return dynamic_cast<const SCE::parsing::TemplateMalformed *>(&ex) != nullptr;
        },
        /*assertCoordinateAgreement=*/true);
}

// P3 success-path fixture — template splices a bound parameter
// into a `<state id="s_{$id}"/>` attribute; a byte inside the
// substituted id value must resolve via `PositionMap::lookup` to
// the caller's `<sce:use>` element, not to the template file.
// No Rust NDJSON counterpart because `--error-format=json` is
// error-only; `runCoordinateLookupFixture` drives the C++
// expander directly and asserts the depth-1 invariant.
TEST(TemplateParity, CoordNestedCollapse) {
    runCoordinateLookupFixture("coord_nested_collapse",
                               /*callerNeedle=*/"<sce:use template=\"inner.scxml\"",
                               /*expandedNeedle=*/"ZQ42");
}

// XInclude error-path fixture — `<sce:use template="ghost.scxml"/>`
// sits inside an `<xi:include>`'d fragment. Both producers raise
// `xml/template-not-found` / `TemplateNotFound` and both must pin
// the diagnostic's `location.{file, line, col}` at the fragment
// file (`frag.xml`), not at `main.scxml`. The PositionMap
// composition threading through XInclude → template stages
// resolves the diagnostic byte to its fragment-file origin.
TEST(TemplateParity, CoordXIncludeFragment) {
    runErrorParityFixture(
        "coord_xinclude_fragment", "xml/template-not-found", "SCE::parsing::TemplateNotFound",
        [](const SCE::parsing::TemplateError &ex) -> bool {
            return dynamic_cast<const SCE::parsing::TemplateNotFound *>(&ex) != nullptr;
        },
        /*assertCoordinateAgreement=*/true);
}

// XInclude+template error-path fixture — chains XInclude and
// template expansion: `<xi:include>` splices `frag.xml`, which
// contains `<sce:use template="t.scxml"/>`, whose body contains a
// nested `<sce:use template="ghost.scxml"/>`. The diagnostic
// surfaces at the OUTER callsite (frag.xml's `<sce:use>` row/col)
// per depth-1 collapse: nested-template failures restamp to
// the call site the operator can see. The fixture pins the
// composition chain (xinclude_map → substitution.positions →
// inner expandImpl) AND the depth-1 restamp at the recursive-
// expandImpl catch — both must agree across producers.
TEST(TemplateParity, CoordXIncludeTemplateCombined) {
    runErrorParityFixture(
        "coord_xinclude_template_combined", "xml/template-not-found", "SCE::parsing::TemplateNotFound",
        [](const SCE::parsing::TemplateError &ex) -> bool {
            return dynamic_cast<const SCE::parsing::TemplateNotFound *>(&ex) != nullptr;
        },
        /*assertCoordinateAgreement=*/true);
}
