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
//
// The default namespace was the only binding carried across until
// 2026-09-24: a fragment using a prefix its document declared above it came
// out naming an unbound prefix, and so did an in-line child `<scxml>`. A
// fragment now declares every binding its names use and inherit, the rule
// sce-build's `inherited_bindings` states for the generated machines, and
// the cases that call `undeclaredNames` read it back as a reader of the
// fragment alone would.

#include "parsing/PugiXMLParser.h"
#include "parsing/XmlSerializationHelper.h"

#include <gtest/gtest.h>

#include <cstring>
#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace {

constexpr const char *SCXML_NS = "http://www.w3.org/2005/07/scxml";

// ── Helper: the names a serialized fragment uses but does not declare ──
//
// A fragment is read on its own — by the invoked session's parser, by a
// backend's DOM reader — so the binding each of its names uses has to be
// declared inside it. Reads `fragment` under a wrapper that declares
// nothing, and names each element or prefixed attribute whose binding no
// element from it up to the wrapper declares. An unprefixed element counts
// only when `defaultNamespace` is given: it must then resolve to that URI.
std::vector<std::string> undeclaredNames(const std::string &fragment, const char *defaultNamespace = nullptr) {
    pugi::xml_document document;
    const std::string wrapped = "<fragment>" + fragment + "</fragment>";
    EXPECT_TRUE(document.load_string(wrapped.c_str())) << "the fragment must be XML; got:\n" << fragment;
    pugi::xml_node wrapper = document.document_element();

    auto boundTo = [&](pugi::xml_node element, const std::string &declaration) -> const char * {
        for (pugi::xml_node at = element; at && at != wrapper; at = at.parent()) {
            if (pugi::xml_attribute declared = at.attribute(declaration.c_str())) {
                return declared.value();
            }
        }
        return nullptr;
    };

    std::vector<std::string> undeclared;
    std::function<void(pugi::xml_node)> visit = [&](pugi::xml_node element) {
        const char *name = element.name();
        if (const char *colon = std::strchr(name, ':')) {
            if (!boundTo(element, "xmlns:" + std::string(name, colon))) {
                undeclared.push_back(name);
            }
        } else if (defaultNamespace) {
            const char *uri = boundTo(element, "xmlns");
            if (!uri || std::strcmp(uri, defaultNamespace) != 0) {
                undeclared.push_back(name);
            }
        }
        for (pugi::xml_attribute attribute : element.attributes()) {
            const std::string attributeName = attribute.name();
            const auto colon = attributeName.find(':');
            if (colon == std::string::npos || attributeName.compare(0, colon, "xmlns") == 0 ||
                attributeName.compare(0, colon, "xml") == 0) {
                continue;
            }
            if (!boundTo(element, "xmlns:" + attributeName.substr(0, colon))) {
                undeclared.push_back(attributeName);
            }
        }
        for (pugi::xml_node child : element.children()) {
            if (child.type() == pugi::node_element) {
                visit(child);
            }
        }
    };
    for (pugi::xml_node child : wrapper.children()) {
        if (child.type() == pugi::node_element) {
            visit(child);
        }
    }
    return undeclared;
}

std::string joined(const std::vector<std::string> &names) {
    std::string out;
    for (const auto &name : names) {
        out += (out.empty() ? "" : ", ") + name;
    }
    return out;
}

// ── Helper: parse `xml` and return the named child element ─────────

std::shared_ptr<SCE::IXMLElement> parseAndFindChild(const std::string &xml, const std::string &childName) {
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
    std::function<std::shared_ptr<SCE::IXMLElement>(const std::shared_ptr<SCE::IXMLElement> &)> find =
        [&](const std::shared_ptr<SCE::IXMLElement> &node) -> std::shared_ptr<SCE::IXMLElement> {
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
    EXPECT_EQ(count, 1u) << "child element that already declares xmlns must not get a "
                            "second injection; got "
                         << count << " in:\n"
                         << serialized;
}

// A prefixed child takes its namespace from its prefix, so it is given that
// prefix's declaration — the document declared it above the fragment — and
// no default namespace, which nothing in it uses. (It was "left alone"
// until 2026-09-24, which wrote the prefix unbound.)
TEST(SerializeChildContentXmlns, APrefixedChildDeclaresItsPrefixAndNoDefault) {
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
    // No default namespace: nothing in `widget` takes it, and a default
    // declared where nothing uses it re-binds nothing and only adds noise.
    // Verify the serialized framework:widget tag has no `xmlns=`.
    auto open_start = serialized.find("<framework:widget");
    ASSERT_NE(open_start, std::string::npos);
    auto open_end = serialized.find('>', open_start);
    ASSERT_NE(open_end, std::string::npos);
    auto open_tag = serialized.substr(open_start, open_end - open_start + 1);
    EXPECT_EQ(open_tag.find("xmlns="), std::string::npos)
        << "prefixed child must not receive a default-xmlns injection; got: " << open_tag;
    EXPECT_NE(open_tag.find("xmlns:framework=\"http://example.com/framework\""), std::string::npos)
        << "the prefix the document declared above the fragment must be declared on it; got: " << open_tag;
    const auto undeclared = undeclaredNames(serialized);
    EXPECT_TRUE(undeclared.empty()) << "names the fragment does not declare: " << joined(undeclared) << "\nin:\n"
                                    << serialized;
}

// W3C SCXML 5.4: in-line `<data>` content using a prefix its document binds
// above it reads, on its own, as what it read in place.
TEST(SerializeChildContentXmlns, APrefixItsDocumentBindsIsDeclaredOnTheFragment) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" version="1.0" initial="s0">
  <datamodel>
    <data id="d"><root ext:note="x"><ext:item/><plain/></root></data>
  </datamodel>
  <state id="s0"/>
</scxml>)";

    auto dataElement = parseAndFindChild(xml, "data");
    ASSERT_TRUE(dataElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(dataElement);

    const auto undeclared = undeclaredNames(serialized, SCXML_NS);
    EXPECT_TRUE(undeclared.empty()) << "names the fragment does not declare: " << joined(undeclared) << "\nin:\n"
                                    << serialized;
    EXPECT_NE(serialized.find("<root xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:ext=\"urn:example:ext\""),
              std::string::npos)
        << "the fragment's root declares what its names inherit, in the order they first use it; got:\n"
        << serialized;
}

// W3C SCXML 6.4: the in-line child document an `<invoke>` hands its session.
// A prefix only the parent declares has to come with it, or the session's
// parser meets an unbound prefix.
TEST(SerializeChildContentXmlns, AnInlineChildKeepsAPrefixItsParentDeclares) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml version="1.0" initial="c">
          <final id="c" ext:note="x"/>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    const auto undeclared = undeclaredNames(serialized, SCXML_NS);
    EXPECT_TRUE(undeclared.empty()) << "names the child document does not declare: " << joined(undeclared) << "\nin:\n"
                                    << serialized;

    SCE::PugiXMLParser reparser;
    auto reparsed = reparser.parseContent(serialized);
    ASSERT_TRUE(reparsed) << "the child document must re-parse; got:\n" << serialized;
    auto reroot = reparsed->getRootElement();
    ASSERT_TRUE(reroot);
    EXPECT_EQ(reroot->getNamespace(), std::string(SCXML_NS));
}

// An unprefixed descendant of a prefixed child takes the default namespace
// in place. The copy declares it on the child, or the descendant falls out
// of every namespace.
TEST(SerializeChildContentXmlns, AnUnprefixedDescendantOfAPrefixedChildKeepsTheDefault) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:framework="http://example.com/framework"
       version="1.0" initial="s0">
  <state id="s0">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <framework:widget><part/></framework:widget>
      </content>
    </invoke>
  </state>
</scxml>)";

    auto contentElement = parseAndFindChild(xml, "content");
    ASSERT_TRUE(contentElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(contentElement);

    const auto undeclared = undeclaredNames(serialized, SCXML_NS);
    EXPECT_TRUE(undeclared.empty()) << "names the fragment does not declare: " << joined(undeclared) << "\nin:\n"
                                    << serialized;
}

// A binding the fragment declares itself, above the name that uses it, is
// not declared a second time — and a binding one branch declares does not
// stand in for the one another branch inherits.
TEST(SerializeChildContentXmlns, ABindingTheFragmentDeclaresIsNotDeclaredAgain) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" version="1.0" initial="s0">
  <datamodel>
    <data id="d"><root><outer xmlns:deep="urn:example:deep"><deep:leaf/></outer><relay xmlns:ext="urn:example:other"><back xmlns:ext="urn:example:ext" ext:y="1"/></relay><ext:tail/></root></data>
  </datamodel>
  <state id="s0"/>
</scxml>)";

    auto dataElement = parseAndFindChild(xml, "data");
    ASSERT_TRUE(dataElement);
    const std::string serialized = SCE::XmlSerializationHelper::serializeContent(dataElement);

    const auto undeclared = undeclaredNames(serialized, SCXML_NS);
    EXPECT_TRUE(undeclared.empty()) << "names the fragment does not declare: " << joined(undeclared) << "\nin:\n"
                                    << serialized;
    size_t deep = 0;
    for (size_t at = serialized.find("xmlns:deep="); at != std::string::npos;
         at = serialized.find("xmlns:deep=", at + 1)) {
        ++deep;
    }
    EXPECT_EQ(deep, 1u) << "the author's own declaration is the only one; got:\n" << serialized;
}

// The declarations go on a copy: the document read afterwards is the one
// that was parsed, and serializing twice writes the same text.
TEST(SerializeChildContentXmlns, SerializingLeavesTheDocumentAsItWas) {
    const std::string xml = R"(<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:ext="urn:example:ext" version="1.0" initial="s0">
  <datamodel>
    <data id="d"><root ext:note="x"/></data>
  </datamodel>
  <state id="s0"/>
</scxml>)";

    auto dataElement = parseAndFindChild(xml, "data");
    ASSERT_TRUE(dataElement);
    const std::string first = SCE::XmlSerializationHelper::serializeContent(dataElement);
    const std::string second = SCE::XmlSerializationHelper::serializeContent(dataElement);
    EXPECT_EQ(first, second);

    auto children = dataElement->getChildren();
    ASSERT_EQ(children.size(), 1u);
    const auto attributes = children.front()->getAttributes();
    EXPECT_EQ(attributes.count("xmlns"), 0u);
    EXPECT_EQ(attributes.count("xmlns:ext"), 0u);
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
    std::string trimmed = (start == std::string::npos) ? std::string() : serialized.substr(start, end - start + 1);
    EXPECT_EQ(trimmed, std::string("hello world"))
        << "text-only content must round-trip verbatim; got: '" << serialized << "'";
    EXPECT_EQ(serialized.find("xmlns="), std::string::npos)
        << "text-only content must not gain a phantom xmlns attribute";
}

}  // namespace
