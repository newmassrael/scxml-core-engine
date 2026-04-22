// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Phase B SSOT byte-equivalence harness for `sce:template` expansion.
//
// Drives one fixture through both producers and asserts their
// canonicalised outputs are byte-identical:
//
//   1. Rust `sce-codegen expand <fixture>` captures post-preprocessor
//      text (XInclude + sce:template expansion per Phase A expander).
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
// See claudedocs/rfc-sce-template-phase-b.md §1 Q1 for the
// canonicalisation rationale and the error-path asymmetry carve-out
// (this harness tests success paths only; error-path comparison
// arrives in M3).
//
// M1 covers the no-use passthrough fixture; M2 will exercise the
// real expansion loop with a `with_params` fixture.

#include "parsing/PugiXMLParser.h"
#include "parsing/TemplateError.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <gtest/gtest.h>
#include <memory>
#include <pugixml.hpp>
#include <sstream>
#include <string>
#include <typeinfo>

namespace {

// Run `sce-codegen expand <scxml>` as a subprocess and capture its
// stdout. The command binary path is passed via the
// `SCE_CODEGEN_BIN` env var that CMake plumbs in at add_test time,
// so the harness does not hardcode a workspace-relative path and
// works equally well under CI, local cmake --build, or install-tree
// testing.
std::string runSceCodegenExpand(const std::string &sceCodegenBin, const std::string &scxmlPath) {
    // Quote both arguments to survive shell metacharacters. The
    // inner paths come from CMake / the fixture tree and are not
    // attacker-controlled; quoting here is hygiene, not a security
    // boundary.
    std::string cmd = "\"" + sceCodegenBin + "\" expand \"" + scxmlPath + "\" 2>/dev/null";
    std::unique_ptr<FILE, decltype(&pclose)> pipe(popen(cmd.c_str(), "r"), pclose);
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
// both streams plus the exit code. Used by the M3 error-path
// parity harness to assert the Rust producer fails with a specific
// DiagnosticCode before the C++ side is probed for an equivalent
// typed exception.
SceCodegenExpandError runSceCodegenExpandExpectFail(const std::string &sceCodegenBin,
                                                    const std::string &scxmlPath) {
    // Merge stderr into stdout via `2>&1` so popen captures both
    // streams; the tests inspect the combined text, and Rust's
    // success-path stdout is empty on error so the NDJSON banner
    // is unambiguously the failure record. Using a temporary stderr
    // redirect file avoids shell-quoting surprises on complex paths.
    const std::string cmd =
        "\"" + sceCodegenBin + "\" --error-format json expand \"" + scxmlPath + "\" 2>&1";
    SceCodegenExpandError result;
    std::unique_ptr<FILE, decltype(&pclose)> pipe(popen(cmd.c_str(), "r"), pclose);
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

// Load a fixture into a pugi::xml_document and run the Phase B
// preprocessor sequence (processXInclude → processSceTemplate),
// returning the canonicalised post-expansion text. The harness
// owns the raw `pugi::xml_document` (not via IXMLParser) so it
// can re-serialise it after preprocessing without adding a
// serialise() method to the production interface for a single
// test consumer.
std::string runCppPreprocessors(const std::string &scxmlPath) {
    auto pugiDoc = std::make_shared<pugi::xml_document>();
    auto loadResult = pugiDoc->load_file(scxmlPath.c_str());
    if (!loadResult) {
        return std::string("<<CPP_LOAD_ERROR:") + loadResult.description() + ">>";
    }

    auto sceDoc = std::make_shared<SCE::PugiXMLDocument>(pugiDoc);
    // Base path governs `<xi:include href="...">` and
    // `<sce:use template="...">` resolution; source path seeds the
    // M3 cycle-detection stack so a fixture that points its top-level
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

    sceDoc->processXInclude();
    sceDoc->processSceTemplate();

    return canonicaliseDocument(*pugiDoc);
}

std::string resolveFixture(const char *name) {
    const char *root = std::getenv("SCE_PHASE_B_FIXTURES_ROOT");
    EXPECT_NE(root, nullptr) << "SCE_PHASE_B_FIXTURES_ROOT env var must be "
                                "set by CMake add_test ENVIRONMENT";
    if (root == nullptr) {
        return {};
    }
    return std::string(root) + "/" + name + "/main.scxml";
}

}  // namespace

// Single-fixture parity check shared by every harness entry.
// Extracted from PhaseBParity.PassthroughNoUse once M2 added the
// second fixture — keeps new fixtures to one `TEST(...)` macro
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
    ASSERT_EQ(rustCanonical, cppCanonical)
        << "Phase B parity violation on fixture '" << fixtureName
        << "': Rust-canonical and C++-canonical expansion outputs "
           "diverge. This means `sce-codegen expand` and "
           "`PugiXMLDocument::processSceTemplate` disagree on the "
           "effective post-preprocessor document — the asymmetry Phase "
           "B exists to close. See "
           "claudedocs/rfc-sce-template-phase-b.md §1 Q1.";
}

// M1 fixture — no `<sce:use>` in the document, both producers
// return the input unchanged.
TEST(PhaseBParity, PassthroughNoUse) {
    runParityFixture("passthrough_no_use");
}

// M2 fixture — single `<sce:use>` with one required parameter
// (`port="80"`). First harness entry that actually drives the
// expansion loop: the template body must be spliced into the
// caller's `<state id="idle">` and every `{$port}` replaced with
// `80`. Canonicalisation is what normalises away the string-edit
// vs DOM-edit differences between the two producers.
TEST(PhaseBParity, WithParams) {
    runParityFixture("with_params");
}

// M3 fixture — two-level nested `<sce:use>` chain (main → outer →
// inner). Exercises recursive expansion: the outer template's body
// contains `<sce:use template="inner.scxml" endpoint="host:{$port}"/>`,
// so the caller's `port=8080` must be substituted into the nested
// call's `endpoint` argument BEFORE `inner.scxml` loads — proving
// the substitute-then-recurse order matches between the Rust
// string-editing expander and the C++ DOM-editing runtime. Also
// exercises the per-template base-directory resolution for nested
// lookups (the nested `inner.scxml` reference resolves against
// outer.scxml's parent directory, not the caller's).
TEST(PhaseBParity, Nested) {
    runParityFixture("nested");
}

// ── M3 error-path harness ───────────────────────────────────────
//
// Introduced with the `cycle_detected` fixture: the first harness
// entry where both producers must FAIL rather than succeed. The
// assertion contract is intentionally looser than the byte-diff
// success path — error messages are UX surface (operator-facing
// text, subject to rewording across releases) while discriminants
// are API surface (DiagnosticCode on the Rust side maps 1:1 to a
// C++ TemplateError subtype by RFC §1 Q4). Comparing messages
// would either let the two sides drift without failing or force
// message text to be locked cross-language; the discriminant
// match captures the useful invariant without either problem.
//
// See claudedocs/rfc-sce-template-phase-b.md §3 M3 "Does not
// compare error messages".

// Run an error-path fixture through both producers and assert
// discriminant equivalence. `rustCode` is the expected
// DiagnosticCode emitted on stderr in `--error-format json` mode;
// `cppMatches(const std::exception&)` is a predicate that returns
// true iff the caught exception's dynamic type is the C++ mirror
// subtype of `rustCode`. Splitting the C++ side into a predicate
// (instead of a class-template type parameter) keeps the helper
// callable from a non-template TEST macro body.
template <typename CppCheck>
void runErrorParityFixture(const std::string &fixtureName,
                           const std::string &rustCode,
                           const char *cppSubtypeName,
                           CppCheck cppMatches) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN env var must be set by CMake "
                               "add_test ENVIRONMENT";

    const std::string fixturePath = resolveFixture(fixtureName.c_str());
    ASSERT_FALSE(fixturePath.empty());

    // Rust side: must fail with non-zero exit and emit the
    // expected DiagnosticCode on stderr (merged into our pipe
    // above).
    const auto rustResult = runSceCodegenExpandExpectFail(bin, fixturePath);
    ASSERT_NE(rustResult.exitCode, 0)
        << "sce-codegen expand unexpectedly succeeded on error-path fixture '"
        << fixtureName << "'.\nCaptured output:\n" << rustResult.stderrText;
    ASSERT_TRUE(diagnosticCodeAppears(rustResult.stderrText, rustCode))
        << "sce-codegen expand on fixture '" << fixtureName
        << "' did not emit DiagnosticCode '" << rustCode
        << "'. Captured output:\n" << rustResult.stderrText;

    // C++ side: running the preprocessors MUST throw a
    // TemplateError subtype whose dynamic type satisfies the
    // `cppMatches` predicate. Anything else (no throw, wrong
    // subtype, unrelated exception) is a parity violation.
    bool threwExpected = false;
    std::string actualType;
    std::string actualMessage;
    try {
        auto pugiDoc = std::make_shared<pugi::xml_document>();
        auto loadResult = pugiDoc->load_file(fixturePath.c_str());
        ASSERT_TRUE(loadResult) << "Failed to load C++ fixture: "
                                << loadResult.description();
        auto sceDoc = std::make_shared<SCE::PugiXMLDocument>(pugiDoc);
        {
            std::string basePath = fixturePath;
            auto slash = basePath.find_last_of('/');
            if (slash != std::string::npos) {
                sceDoc->setBasePath(basePath.substr(0, slash));
            }
        }
        sceDoc->setSourcePath(fixturePath);
        sceDoc->processXInclude();
        sceDoc->processSceTemplate();
    } catch (const SCE::parsing::TemplateError &ex) {
        actualType = typeid(ex).name();
        actualMessage = ex.what();
        threwExpected = cppMatches(ex);
    } catch (const std::exception &ex) {
        actualType = typeid(ex).name();
        actualMessage = ex.what();
    }

    ASSERT_TRUE(threwExpected)
        << "Phase B parity violation on error-path fixture '" << fixtureName
        << "': C++ preprocessors did not throw `" << cppSubtypeName
        << "`. Actual type: " << actualType
        << ". Actual message: " << actualMessage
        << ". Rust emitted: " << rustCode
        << ". Cross-language discriminant parity is the M3 error-path "
           "contract — see claudedocs/rfc-sce-template-phase-b.md §3 M3.";
}

// M3 error-path fixture — a.scxml uses b.scxml which uses a.scxml,
// completing a cycle through `main.scxml`. Both producers must
// raise the cycle diagnostic discriminant. First harness entry
// that asserts FAILURE equivalence rather than success bytes.
TEST(PhaseBParity, CycleDetected) {
    runErrorParityFixture(
        "cycle_detected",
        "xml/template-cycle",
        "SCE::parsing::TemplateCycle",
        [](const SCE::parsing::TemplateError &ex) -> bool {
            return dynamic_cast<const SCE::parsing::TemplateCycle *>(&ex) != nullptr;
        });
}
