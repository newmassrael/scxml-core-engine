// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Regression test for `PugiXMLElement::serializeChildContent` namespace
// propagation across the serialization boundary. Pinned by the
// W3C SCXML Test_338 / Test_347 / Test_530 interpreter failures
// surfaced when `a46d2c27` flipped `ParsingCommon::isScxmlNamespace`
// to strict mode:
//
//   * Tests 338/347/530 carry `<invoke><content><scxml ...>...</scxml>`
//     payloads where the inner `<scxml>` inherits the default
//     `xmlns="http://www.w3.org/2005/07/scxml"` from the outer document.
//   * `XmlSerializationHelper::serializeContent` is the boundary
//     between the parent document tree and the string that the child
//     invoke session re-parses via `loadSCXMLFromString`. Without
//     namespace propagation pugixml drops the inherited xmlns from
//     the serialized fragment, the re-parse sees an xmlns-less
//     `<scxml>` root, and strict `isScxmlNamespace` rejects it as
//     `ParseWrongRootElement` ("Root element is not 'scxml', found:
//     scxml"). This unit test pins the propagation directly so a
//     future serializer refactor cannot silently bring the
//     regression back without tripping here.

#include "parsing/PugiXMLParser.h"
#include "parsing/XmlSerializationHelper.h"

#include <gtest/gtest.h>

#include <functional>
#include <memory>
#include <string>

namespace {

constexpr const char *SCXML_NS = "http://www.w3.org/2005/07/scxml";

// ── Helper: parse `xml` and return the named child element ─────────

std::shared_ptr<SCE::IXMLElement> parseAndFindChild(const std::string &xml,
                                                   const std::string &childName) {
    SCE::PugiXMLParser parser;
    auto doc = parser.parseContent(xml);
    EXPECT_TRUE(doc) << "parser must accept the fixture document";
    if (!doc) {
        return nullptr;
    }
    auto root = doc->getRootElement();
    EXPECT_TRUE(root) << "fixture document must have a root element";
    if (!root) {
        return nullptr;
    }
    // DFS descent — the W3C test fixtures place `<content>` at
    // root→state→invoke→content, so a fixed-depth scan misses it.
    std::function<std::shared_ptr<SCE::IXMLElement>(const std::shared_ptr<SCE::IXMLElement> &)>
        find = [&](const std::shared_ptr<SCE::IXMLElement> &node)
        -> std::shared_ptr<SCE::IXMLElement> {
        if (!node) {
            return nullptr;
        }
        if (node->getName() == childName) {
            return node;
        }
        for (auto &c : node->getChildren()) {
            if (auto hit = find(c)) {
                return hit;
            }
        }
        return nullptr;
    };
    return find(root);
}

// ── Tests ──────────────────────────────────────────────────────────

// Mirrors the W3C Test_338 inline-invoke shape: the outer `<scxml>`
// declares the namespace, the inner `<scxml>` (under `<content>`)
// inherits it without its own xmlns attribute. The serializer must
// re-introduce the binding so the re-parse sees a namespaced root.
TEST(SerializeChildContentXmlns, InvokeContentInlineScxmlInheritsXmlns) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml version="1.0" initial="sub0">
          <final id="sub0"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement) << "fixture must expose <content>";

    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    // The first thing after `<content>` should be `<scxml ` carrying
    // the inherited default xmlns. Without the propagation patch the
    // serialized string would say `<scxml version="1.0" ...>` with no
    // xmlns attribute — and the strict re-parse path would reject it.
    EXPECT_NE(serialized.find("<scxml"), std::string::npos)
        << "serialized fragment must include the inner <scxml> root";
    EXPECT_NE(serialized.find("xmlns=\"http://www.w3.org/2005/07/scxml\""), std::string::npos)
        << "serialized fragment must carry the inherited default xmlns so "
           "round-trip parsing succeeds; got:\n"
        << serialized;
}

// Direct round-trip: serialize, re-parse, assert the resulting root
// element is namespaced. Mirrors what `SCXMLInvokeHandler::start
// InvokeInternal → loadSCXMLFromString` does at runtime so a failure
// here precisely names the broken contract.
TEST(SerializeChildContentXmlns, SerializedFragmentRoundTripsThroughParser) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml version="1.0" initial="sub0">
          <final id="sub0"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    SCE::PugiXMLParser reparser;
    auto reparsed = reparser.parseContent(serialized);
    ASSERT_TRUE(reparsed) << "serialized fragment must re-parse cleanly; got:\n" << serialized;

    auto reroot = reparsed->getRootElement();
    ASSERT_TRUE(reroot) << "re-parsed document must expose a root";
    EXPECT_EQ(reroot->getName(), std::string("scxml"));
    EXPECT_EQ(reroot->getNamespace(), std::string(SCXML_NS))
        << "re-parsed root must carry the SCXML namespace so strict "
           "isScxmlNamespace accepts it";
}

// A child that already declares its own default xmlns must NOT get a
// second xmlns attribute injected. The injection logic skips the
// element in that case so the byte-shape of pre-existing fixtures
// stays stable.
TEST(SerializeChildContentXmlns, ChildWithOwnXmlnsIsLeftAlone) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="sub0">
          <final id="sub0"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    // Exactly one occurrence of the xmlns binding — no duplicate
    // injection on top of the child's own declaration.
    size_t count = 0;
    const std::string needle = "xmlns=\"http://www.w3.org/2005/07/scxml\"";
    size_t pos = 0;
    while ((pos = serialized.find(needle, pos)) != std::string::npos) {
        ++count;
        pos += needle.size();
    }
    EXPECT_EQ(count, 1u)
        << "child element that already declares xmlns must not get a "
           "second injection; got " << count << " in:\n" << serialized;
}

// Foreign-namespace prefixed children must also be left alone — the
// `xmlns:<prefix>` resolution is the binding mechanism, and the
// strict-isScxmlNamespace policy treats prefixed elements as foreign
// regardless. Injecting a default xmlns onto a prefixed element would
// not affect its namespace but would inflate the serialization shape;
// the propagation logic must skip them.
TEST(SerializeChildContentXmlns, PrefixedChildIsLeftAlone) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:framework="http://example.com/framework"
       version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <framework:widget kind="ignored"/>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    EXPECT_NE(serialized.find("<framework:widget"), std::string::npos)
        << "prefixed child must round-trip with its prefix intact";
    // Prefixed element must NOT receive a default xmlns injection —
    // its namespace is bound via the prefix on the ancestor, and
    // injecting `xmlns=` on a prefixed element would be a semantic
    // change (re-binding the default for unprefixed descendants).
    // Verify the serialized framework:widget tag has no `xmlns=`.
    auto open_start = serialized.find("<framework:widget");
    ASSERT_NE(open_start, std::string::npos);
    auto open_end = serialized.find('>', open_start);
    ASSERT_NE(open_end, std::string::npos);
    auto open_tag = serialized.substr(open_start, open_end - open_start + 1);
    EXPECT_EQ(open_tag.find("xmlns="), std::string::npos)
        << "prefixed child must not receive a default-xmlns injection; got: " << open_tag;
}

// Text-only `<data>` content must not gain a phantom xmlns attribute.
// This is what protects DataModelParser callers (RFC W3C SCXML B.2):
// the propagation is element-targeted, so text / CDATA payloads
// round-trip verbatim.
TEST(SerializeChildContentXmlns, TextOnlyDataContentDoesNotMutate) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <datamodel>
    <data id="message">hello world</data>
  </datamodel>
  <state id="s0">
    <final id="done"/>
  </state>
</scxml>)";

    auto dataElement = parseAndFindChild(xml, "data");
    ASSERT_TRUE(dataElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(dataElement);

    // Trim and compare — pugixml may keep surrounding whitespace.
    auto start = serialized.find_first_not_of(" \t\n\r");
    auto end = serialized.find_last_not_of(" \t\n\r");
    std::string trimmed = (start == std::string::npos)
                              ? std::string()
                              : serialized.substr(start, end - start + 1);
    EXPECT_EQ(trimmed, std::string("hello world"))
        << "text-only content must round-trip verbatim; got: '" << serialized << "'";
    EXPECT_EQ(serialized.find("xmlns="), std::string::npos)
        << "text-only content must not gain a phantom xmlns attribute";
}

}  // namespace
