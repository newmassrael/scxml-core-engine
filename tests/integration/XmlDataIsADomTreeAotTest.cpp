// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2: a `<data>` element's XML content is a DOM structure a
// document can walk — C++ AOT path.
//
// The appendix obliges the Processor to create "the corresponding DOM
// structure". Measured 2026-08-18, what every backend created was an object
// carrying three methods — `getElementsByTagName`, `getAttribute` and a
// non-standard `getTagName`, which are the two names the W3C IRP suite reads
// plus one — so `doc.tagName`, `doc.firstChild` and `doc.childNodes.length`
// answered nil on all seven channels with 204/204 W3C fixtures green.
//
// Sibling of `XmlDataIsADomTreeTest.cpp` (Interpreter). This one compiles the
// fixture's `<data>` initializer and its guards into C++ ahead of time, so what
// it asks is whether the GENERATED code reaches the binding — the seam
// `tests/engine/DomReadSurfaceTest.cpp` cannot see, because that test calls the
// engine directly.
//
// Fixture:
// integration_resources/xml_data_is_a_dom_tree/xml_data_is_a_dom_tree.scxml
// The generated machine is produced by `sce_generate_static_integration_test`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "xml_data_is_a_dom_tree_sm.h"

#include <gtest/gtest.h>
#include <memory>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::xml_data_is_a_dom_tree::xml_data_is_a_dom_tree;

/// The fixture is a flat machine, so its configuration IS the current state —
/// `getActiveStates` is emitted only for machines that carry a `<parallel>`.
bool isActive(SM &sm, SM::State state) {
    return sm.getCurrentState() == state;
}

/// Rendered into every failure message: the fixture lands each way of failing
/// in a `<final>` of its own, so the state names which claim broke.
std::string describe(SM &sm) {
    return std::string("[") + sm.getPolicy().getStateName(sm.getCurrentState()) + "]";
}

}  // namespace

TEST(XmlDataIsADomTreeAotTest, ADataElementsXmlIsADomTreeTheDocumentCanWalk) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    // Every transition is eventless, so initialization runs the machine to its
    // verdict; no event is needed to ask the question.
    sm.initialize();

    EXPECT_FALSE(isActive(sm, SM::State::NotADocument))
        << "the variable did not hold a document: `doc.nodeType === 9`, `doc.nodeName === "
           "'#document'`, `doc.documentElement.tagName === 'books'` or `doc.hasAttribute('count')` "
           "did not hold. active: "
        << describe(sm);
    EXPECT_FALSE(isActive(sm, SM::State::WrongTree))
        << "the document element's children are not the two `<book>` elements in document order — "
           "the whitespace between them may have become nodes, or a sibling/parent link is "
           "missing. active: "
        << describe(sm);
    EXPECT_FALSE(isActive(sm, SM::State::NoText))
        << "character data did not report itself as a text node, or `textContent` did not read the "
           "text below the element. active: "
        << describe(sm);
    EXPECT_TRUE(isActive(sm, SM::State::Settled))
        << "the machine reached none of its four verdicts, so the guards did not evaluate at all. "
           "active: "
        << describe(sm);
}

}  // namespace SCE::Tests
