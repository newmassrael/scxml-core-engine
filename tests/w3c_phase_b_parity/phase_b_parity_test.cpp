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
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <gtest/gtest.h>
#include <memory>
#include <pugixml.hpp>
#include <sstream>
#include <string>

namespace {

// Run `sce-codegen expand <scxml>` as a subprocess and capture its
// stdout. The command binary path is passed via the
// `SCE_CODEGEN_BIN` env var that CMake plumbs in at add_test time,
// so the harness does not hardcode a workspace-relative path and
// works equally well under CI, local cmake --build, or install-tree
// testing.
std::string runSceCodegenExpand(const std::string &sceCodegenBin, const std::string &scxmlPath) {
    // Quote both arguments to survive spaces / shell metacharacters.
    // The inner paths come from CMake / the fixture tree and are
    // not attacker-controlled; quoting here is hygiene, not a
    // security boundary.
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
    // Base path governs `<xi:include href="...">` resolution; for
    // M1's no-use fixture this has no effect but the call keeps
    // the test driver aligned with the SCXMLParser.cpp flow so
    // future fixtures that do include fragments work the same way.
    {
        std::string basePath = scxmlPath;
        auto slash = basePath.find_last_of('/');
        if (slash != std::string::npos) {
            sceDoc->setBasePath(basePath.substr(0, slash));
        }
    }

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

// M1 fixture — no `<sce:use>` in the document, both producers
// return the input unchanged.
TEST(PhaseBParity, PassthroughNoUse) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN env var must be set by CMake "
                               "add_test ENVIRONMENT";

    const std::string fixturePath = resolveFixture("passthrough_no_use");
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
        << "Phase B parity violation: Rust-canonical and C++-canonical "
           "expansion outputs diverge. This means `sce-codegen expand` "
           "and `PugiXMLDocument::processSceTemplate` disagree on the "
           "effective post-preprocessor document — the asymmetry Phase "
           "B exists to close. See "
           "claudedocs/rfc-sce-template-phase-b.md §1 Q1.";
}
