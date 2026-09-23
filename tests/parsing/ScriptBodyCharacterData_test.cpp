// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A <script> body is all of its character data (W3C SCXML 5.8).
//
// `PugiXMLElement::getTextContent` answered the first text child alone —
// pugixml's `child_value()` — and the default parse keeps the text on either
// side of a comment, and a CDATA section beside text, as separate nodes. So a
// comment in a script body cut the script off there, and so did a CDATA
// section written after a statement (measured 2026-09-24). The generator's
// parser read the same bodies short too; the expected strings below are the
// ones `sce-build/tests/a_script_body_is_all_of_its_character_data.rs` pins
// for it, so the interpreter and the generated machine run one body.

#include "actions/ScriptAction.h"
#include "factory/NodeFactory.h"
#include "model/SCXMLModel.h"
#include "parsing/SCXMLParser.h"

#include <gtest/gtest.h>

#include <memory>
#include <string>
#include <vector>

namespace {

constexpr const char *kScxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="x" expr="0"/>
    <data id="y" expr="0"/>
    <data id="z" expr="0"/>
  </datamodel>
  <script><!-- set up before anything runs -->z = 7;</script>
  <state id="a">
    <onentry>
      <script>x = 2; <!-- the rest of the body --> y = 3;</script>
      <script>x = 4; <![CDATA[ y = y < 5 ? 5 : y; ]]> z = 8;</script>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
)";

std::string bodyOf(const std::shared_ptr<SCE::IActionNode> &node) {
    auto script = std::dynamic_pointer_cast<SCE::ScriptAction>(node);
    return script ? script->getContent() : std::string("<not a script>");
}

}  // namespace

TEST(ScriptBodyCharacterDataTest, AGlobalScriptThatOpensWithACommentKeepsItsBody) {
    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    const auto &scripts = model->getTopLevelScripts();
    ASSERT_EQ(scripts.size(), 1u);
    EXPECT_EQ(bodyOf(scripts[0]), "z = 7;");
}

TEST(ScriptBodyCharacterDataTest, ACommentOrACdataSectionInsideAScriptDoesNotCutItOff) {
    SCE::SCXMLParser parser(std::make_shared<SCE::NodeFactory>());
    auto model = parser.parseContent(kScxml);
    ASSERT_NE(model, nullptr);

    auto *state = model->findStateById("a");
    ASSERT_NE(state, nullptr);
    std::vector<std::string> bodies;
    for (const auto &block : state->getEntryActionBlocks()) {
        for (const auto &action : block) {
            bodies.push_back(bodyOf(action));
        }
    }
    EXPECT_EQ(bodies, (std::vector<std::string>{"x = 2;  y = 3;", "x = 4;  y = y < 5 ? 5 : y;  z = 8;"}));
}
