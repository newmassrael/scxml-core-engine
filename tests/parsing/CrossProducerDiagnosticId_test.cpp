// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Cross-producer diagnostic-`id` parity harness.
//
// `SCE_ERROR_CONTRACT.md` §2.1 defines `id` as the dedup / cache key a
// consumer folds repeated reports of one error on. Two producers emit
// records for the same document: the Rust `sce-codegen` CLI (the
// authority) and the C++ runtime parser, which `parsing/Diagnostic.h`
// declares a "second independent conformer" to the same schema. A
// consumer reading both sees one logical error twice unless the two
// ids agree — and reading both is the ordinary case, not a corner: the
// W3C harness generates a document with `sce-codegen` and loads the
// same document through the Interpreter.
//
// What the tree already had was a cross-producer harness for
// *discriminants* (`tests/w3c_template_parity` compares the emitted
// `code` and, opt-in, the location triple). Nothing compared ids, and
// `Diagnostic.h` recorded the divergence in prose — "schema-valid but
// not byte-equivalent to Rust's id for the same logical error" — with
// no test that would go red if it stayed. This suite is that test.
//
// Nothing here is curated:
//   * the fixture set is the directory listing under
//     `SCE_CROSS_PRODUCER_FIXTURES_ROOT`, so a fixture added to the
//     tree is exercised without a second edit;
//   * the covered-leaf set is whatever the C++ producer actually
//     threw, so a leaf no fixture reaches cannot look covered;
//   * the declared-leaf set is derived from
//     `sce/include/parsing/*Error.h`, so a new leaf is red until a
//     fixture reaches it or an exemption states why it cannot.

#include "SourceScan.h"
#include "factory/NodeFactory.h"
#include "parsing/Diagnostic.h"
#include "parsing/SCXMLParser.h"

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <regex>
#include <set>
#include <string>
#include <typeinfo>
#include <unistd.h>
#include <vector>

namespace {

// ── Producer drivers ───────────────────────────────────────────────

// One producer's verdict on one document.
struct ProducerRecord {
    bool emitted = false;
    std::string code;
    std::string id;
    // Full record, for failure messages: a mismatch is only
    // actionable next to the two payloads that produced it.
    std::string raw;
    // C++ only — the dynamic type that was thrown, used to build the
    // covered-leaf set at runtime.
    std::string leaf;
};

struct PipeCloser {
    void operator()(FILE *pipe) const {
        if (pipe != nullptr) {
            pclose(pipe);
        }
    }
};

using UniquePipe = std::unique_ptr<FILE, PipeCloser>;

// Rust producer: `sce-codegen check <doc> --error-format=json`.
// Diagnostics land on stderr per `SCE_ERROR_CONTRACT.md` §10, so
// stdout is discarded and stderr captured — the reverse of the
// redirect, which would capture the (empty) success channel.
ProducerRecord runRustProducer(const std::string &bin, const std::string &document) {
    const std::string cmd = "\"" + bin + "\" check \"" + document + "\" --error-format=json 2>&1 >/dev/null";
    ProducerRecord out;
    UniquePipe pipe(popen(cmd.c_str(), "r"));
    if (!pipe) {
        return out;
    }
    std::string captured;
    char buf[4096];
    while (std::fgets(buf, sizeof(buf), pipe.get()) != nullptr) {
        captured += buf;
    }

    // NDJSON: one record per line. The first record is the one both
    // producers are compared on — the C++ parser aborts at its first
    // typed throw, so a later Rust record has no counterpart to
    // compare against.
    const auto eol = captured.find('\n');
    const std::string first = captured.substr(0, eol == std::string::npos ? captured.size() : eol);
    out.raw = first;
    if (first.empty()) {
        return out;
    }
    nlohmann::json parsed = nlohmann::json::parse(first, nullptr, false);
    if (parsed.is_discarded() || !parsed.is_object()) {
        return out;
    }
    out.emitted = true;
    out.code = parsed.value("code", std::string{});
    out.id = parsed.value("id", std::string{});
    return out;
}

// C++ producer: `SCXMLParser::parseFile` — the entry point the
// Interpreter itself uses, so the comparison is against what
// production emits rather than against a test-only assembly of the
// same leaf.
ProducerRecord runCppProducer(const std::string &document) {
    auto nodeFactory = std::make_shared<SCE::NodeFactory>();
    SCE::SCXMLParser parser(nodeFactory);
    const auto model = parser.parseFile(document);

    ProducerRecord out;
    const auto &diagnostics = parser.getDiagnostics();
    if (diagnostics.empty()) {
        out.raw = model ? "(parse succeeded, no diagnostic)" : "(parse failed with no typed diagnostic)";
        return out;
    }

    const auto &diag = *diagnostics.front();
    const auto record = diag.to_json();
    out.emitted = true;
    out.code = record.value("code", std::string{});
    out.id = record.value("id", std::string{});
    out.raw = diag.to_canonical_json_string();
    out.leaf = typeid(diag).name();
    return out;
}

// ── Declared leaves ────────────────────────────────────────────────

// Every concrete `Diagnostic` leaf, derived from the headers that
// declare them. The four family bases are named rather than matched
// loosely so a helper struct in the same header cannot inflate the
// set.
std::set<std::string> declaredLeaves(std::size_t minLeaves) {
    static const std::regex kLeaf{
        R"rx(class\s+(\w+)\s*:\s*public\s+(?:XIncludeExpansionError|TemplateError|SemanticError|ParseError)\b)rx"};
    std::set<std::string> leaves;
    std::size_t headers = 0;
    for (const auto &entry : std::filesystem::directory_iterator{SCE_PARSING_HEADER_DIR}) {
        const auto name = entry.path().filename().string();
        if (name.size() < 7 || name.compare(name.size() - 7, 7, "Error.h") != 0) {
            continue;
        }
        ++headers;
        const auto body = SCE::TestSupport::stripComments(SCE::TestSupport::readFile(entry.path().string()));
        const auto found = SCE::TestSupport::matchAll(body, kLeaf);
        leaves.insert(found.begin(), found.end());
    }
    EXPECT_GE(headers, 4u) << "found only " << headers << " *Error.h header(s) under " << SCE_PARSING_HEADER_DIR
                           << " — the directory scan is broken, not the headers";
    // Floor, for the reason every derived set in this tree carries
    // one: a scanner that stopped matching returns the empty set and
    // makes the coverage comparison below vacuously true.
    EXPECT_GE(leaves.size(), minLeaves) << "derived only " << leaves.size() << " diagnostic leaf class(es)";
    return leaves;
}

// ── Leaves no fixture can reach, and why ───────────────────────────
//
// An exemption is a claim, so each carries the measurement that
// backs it. The harness checks that every name here is a leaf that
// actually exists, so an exemption cannot outlive the class it
// excuses.
const std::map<std::string, std::string> &exemptLeaves() {
    static const std::map<std::string, std::string> kExempt = {
        {"ParseFileNotFound", "Measured, not inferred: `check` and `expand` on a missing path both answer "
                              "`cli/read-input` from the CLI boundary, and the one library entry that raises "
                              "`XmlError::FileNotFound` (`load_forge_source`) is called only by fixture generation. "
                              "No CLI invocation puts the two producers on the same input."},
        {"ParseWrongRootElement",
         "Stage composition, measured: the Rust pipeline validates against the XSD first and "
         "answers `xml/schema-validation` for a non-`<scxml>` root. Its own root check is "
         "defence-in-depth for a build with no schema to validate against, and this build always "
         "has one — `find_schema_path` falls back to the compiled-in repository path, so even "
         "`SCE_SCHEMAS_DIR` pointed at an empty directory still validates (probed). The C++ "
         "runtime parser has no schema stage at all, which is why it reaches its root check."},
        {"ParseNoRootElement", "Same measured stage difference, on a comment-only document: Rust answers "
                               "`xml/schema-validation` (\"The document has no document element.\") before its own "
                               "root check runs."},
        {"ParseException", "Wrapper for a non-typed `std::exception` escaping the parser (bad_alloc, third-party "
                           "throw). No document input raises it deterministically, so no fixture can pin it."},
        {"SemanticWrongPipeline",
         "Stage composition, measured 2026-08-12: a root that declares a forge `sce:kind` is refused by both "
         "producers, but never through the same stage, so no fixture can put it in the code-agreeing set. Rust "
         "routes the document to Forge per SCE_ERROR_CONTRACT.md §4.1 and answers whatever Forge finds there "
         "(`xml/schema-validation` for an unknown kind, `validation/missing-element` for a known one whose body "
         "is a statechart); the C++ engine has no forge pipeline to route into and answers "
         "`validation/wrong-pipeline`. What matters across the two — that neither engine RUNS the document — is "
         "pinned by `BothProducersAgreeOnWhichPipelineOwnsTheDocument` below, which is the property this leaf was "
         "added for: before it existed the C++ parser accepted such documents and ran them as statecharts."},
        {"SemanticTopLevelScriptUnloaded",
         "Both producers reject what §scxml-5.8 forbids — measured on all three shapes — but "
         "through different surfaces: the C++ parser raises this leaf, while the Rust pipeline "
         "records the rejection in its stdout manifest (`rejected`) and emits the stub W3C test "
         "301 expects. This harness reads diagnostics, so it cannot compare a manifest field. The "
         "agreement is pinned instead by "
         "`SCXMLParserBoundary.TopLevelScriptMustSpecifyExactlyOneOfSrcOrBody`."},
    };
    return kExempt;
}

std::string fixtureRoot() {
    const char *root = std::getenv("SCE_CROSS_PRODUCER_FIXTURES_ROOT");
    EXPECT_NE(root, nullptr) << "SCE_CROSS_PRODUCER_FIXTURES_ROOT must be set by CMake add_test ENVIRONMENT";
    return root == nullptr ? std::string{} : std::string{root};
}

std::vector<std::string> fixtureNames(const std::string &root) {
    std::vector<std::string> names;
    for (const auto &entry : std::filesystem::directory_iterator{root}) {
        if (entry.is_directory() && std::filesystem::exists(entry.path() / "main.scxml")) {
            names.push_back(entry.path().filename().string());
        }
    }
    std::sort(names.begin(), names.end());
    return names;
}

}  // namespace

// Which pipeline owns the document, asserted across both producers.
//
// The suite above compares HOW the two reject, over a fixture set
// chosen so they reject through the same stage. It therefore cannot
// see the prior question: whether they agree the document is theirs to
// run at all. Measured 2026-08-12, that agreement did not hold — a
// root declaring `sce:kind="lookup"` was refused by `sce-codegen` and
// ACCEPTED by the C++ parser, which ran it as a plain statechart. The
// two engines' accepted sets differed on a document an author writes
// by typo, and no test in the tree asked.
//
// `SCE_ERROR_CONTRACT.md` §4.1 defines the routing and calls its last
// row a contract guarantee, so the cases here are that table, not a
// sample: absent and `"statechart"` belong to the SCXML engine, every
// other value belongs to Forge. Documents are written at run time
// because the class is closed and enumerable — a fixture directory
// would invite it to be read as a sample of an open set.
TEST(CrossProducerDiagnosticId, BothProducersAgreeOnWhichPipelineOwnsTheDocument) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";

    struct Case {
        const char *label;
        const char *kindAttr;
        bool ownedByThisEngine;
    };

    // A statechart body throughout, so the only thing that moves
    // between cases is the routing attribute.
    const std::vector<Case> cases = {
        {"kind absent", "", true},
        {"kind statechart", R"( sce:kind="statechart")", true},
        {"kind known forge", R"( sce:kind="lookup")", false},
        {"kind unknown", R"( sce:kind="bogus")", false},
    };

    const auto dir = std::filesystem::temp_directory_path() / ("sce-pipeline-ownership-" + std::to_string(::getpid()));
    std::filesystem::create_directories(dir);

    struct Cleanup {
        std::filesystem::path dir;

        ~Cleanup() {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        }
    } cleanup{dir};

    std::vector<std::string> violations;
    for (const auto &c : cases) {
        const auto path = dir / (std::string{"case-"} + std::to_string(&c - cases.data()) + ".scxml");
        {
            std::ofstream out{path};
            out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                << "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\""
                << " version=\"1.0\" initial=\"s1\"" << c.kindAttr << ">\n"
                << "  <state id=\"s1\"><transition target=\"s2\"/></state>\n"
                << "  <final id=\"s2\"/>\n"
                << "</scxml>\n";
        }
        const auto rust = runRustProducer(bin, path.string());
        const auto cpp = runCppProducer(path.string());

        if (rust.emitted == c.ownedByThisEngine) {
            violations.push_back(std::string{c.label} + ": Rust " + (rust.emitted ? "rejected" : "accepted") +
                                 " a document §4.1 routes to " + (c.ownedByThisEngine ? "SCXML" : "Forge") +
                                 ". Record: " + rust.raw);
        }
        if (cpp.emitted == c.ownedByThisEngine) {
            violations.push_back(std::string{c.label} + ": C++ " + (cpp.emitted ? "rejected" : "accepted") +
                                 " a document §4.1 routes to " + (c.ownedByThisEngine ? "SCXML" : "Forge") +
                                 ". Record: " + cpp.raw);
        }
    }

    EXPECT_TRUE(violations.empty())
        << "the two producers disagree about which pipeline owns a document:\n  - " <<
        [&] {
            std::string joined;
            for (const auto &v : violations) {
                if (!joined.empty()) {
                    joined += "\n  - ";
                }
                joined += v;
            }
            return joined;
        }()
        << "\nA document one engine refuses and the other runs is the same source producing two behaviours; "
           "SCE_ERROR_CONTRACT.md §4.1 makes the routing a contract guarantee for exactly that reason.";
}

// Both producers, every fixture, one assertion per contract clause.
//
// Violations are collected rather than asserted one at a time: a
// harness that stops at the first mismatch leaves every later fixture
// unproven, and the useful output of a cross-producer run is the
// whole disagreement, not its alphabetically first member.
TEST(CrossProducerDiagnosticId, EveryFixtureAgreesOnCodeAndId) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";

    const std::string root = fixtureRoot();
    ASSERT_FALSE(root.empty());
    const auto fixtures = fixtureNames(root);
    // Floor: an empty fixture directory would satisfy every
    // comparison below without proving anything.
    ASSERT_GE(fixtures.size(), 22u) << "found only " << fixtures.size() << " fixture(s) under " << root;

    std::vector<std::string> violations;
    std::set<std::string> coveredLeaves;

    for (const auto &name : fixtures) {
        const std::string document = root + "/" + name + "/main.scxml";

        const auto rust = runRustProducer(bin, document);
        const auto cpp = runCppProducer(document);

        if (!rust.emitted) {
            violations.push_back(name + ": Rust producer emitted no parseable diagnostic. Captured: " + rust.raw);
            continue;
        }
        if (!cpp.emitted) {
            violations.push_back(name + ": C++ producer emitted no diagnostic. " + cpp.raw +
                                 " Rust emitted: " + rust.code);
            continue;
        }

        coveredLeaves.insert(cpp.leaf);

        if (rust.code != cpp.code) {
            violations.push_back(name + ": code disagreement — Rust '" + rust.code + "' vs C++ '" + cpp.code +
                                 "'. A fixture belongs here only when both producers classify the document the "
                                 "same way; if the stage composition genuinely differs, the C++ leaf belongs in "
                                 "the exemption table with that reason instead.");
            continue;
        }

        if (rust.id != cpp.id) {
            violations.push_back(name + ": id disagreement on code '" + rust.code + "' — Rust '" + rust.id +
                                 "' vs C++ '" + cpp.id + "'.\n    Rust record: " + rust.raw +
                                 "\n    C++ record:  " + cpp.raw);
        }
    }

    EXPECT_TRUE(violations.empty()) << "cross-producer diagnostic parity violation(s):\n  - " <<
        [&] {
            std::string joined;
            for (const auto &v : violations) {
                if (!joined.empty()) {
                    joined += "\n  - ";
                }
                joined += v;
            }
            return joined;
        }()
                                    << "\n`id` is the dedup key (SCE_ERROR_CONTRACT.md §2.1): two producers that "
                                       "disagree make one logical error count twice for any consumer reading both.";

    // Coverage, measured from the run rather than declared: a leaf is
    // covered only if a fixture actually made the C++ producer throw
    // it.
    const auto declared = declaredLeaves(20);
    const auto &exempt = exemptLeaves();

    std::vector<std::string> uncovered;
    for (const auto &leaf : declared) {
        if (exempt.count(leaf) != 0) {
            continue;
        }
        const bool covered = std::any_of(coveredLeaves.begin(), coveredLeaves.end(), [&](const std::string &thrown) {
            return thrown.find(leaf) != std::string::npos;
        });
        if (!covered) {
            uncovered.push_back(leaf);
        }
    }
    EXPECT_TRUE(uncovered.empty()) << "diagnostic leaf class(es) no fixture reaches: "
                                   << ::testing::PrintToString(uncovered)
                                   << "\nAdd a fixture directory whose main.scxml makes both producers report it, "
                                      "or register it in `exemptLeaves()` with the measurement that shows why no "
                                      "document can. Silent partial coverage is what this check exists to stop.";

    std::vector<std::string> phantomExemptions;
    for (const auto &[leaf, reason] : exempt) {
        if (declared.count(leaf) == 0) {
            phantomExemptions.push_back(leaf);
        }
        EXPECT_FALSE(reason.empty()) << "exemption for '" << leaf << "' carries no reason";
    }
    EXPECT_TRUE(phantomExemptions.empty())
        << "exemption(s) for leaf class(es) no header declares: " << ::testing::PrintToString(phantomExemptions)
        << "\nThe class was renamed or deleted and its excuse outlived it.";
}
