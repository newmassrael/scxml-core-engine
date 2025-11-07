#include "model/DataModelItem.h"
#include "parsing/IXMLElement.h"
#include "parsing/IXMLParser.h"
#include <gtest/gtest.h>
#include <memory>

/**
 * @brief Test suite for DataModelItem XML DOM functionality
 *
 * Verifies platform-independent XML parsing using IXMLDocument interface.
 * Tests the refactored implementation that removed #ifdef __EMSCRIPTEN__ conditionals.
 */
class DataModelItemTest : public ::testing::Test {
protected:
    void SetUp() override {
        // No setup needed - DataModelItem is standalone
    }

    void TearDown() override {
        // Cleanup managed by smart pointers
    }

    std::unique_ptr<SCE::DataModelItem> createItem(const std::string &id, const std::string &type = "") {
        auto item = std::make_unique<SCE::DataModelItem>(id);
        if (!type.empty()) {
            item->setType(type);
        }
        return item;
    }
};

/**
 * @brief Test XML content parsing with type="xpath"
 *
 * W3C SCXML B.1: XPath data model support
 * Verifies that XML content is parsed when type is "xpath"
 */
TEST_F(DataModelItemTest, XmlContentParsing_XPathType) {
    auto item = createItem("data1", "xpath");

    const std::string xmlContent = "<root><child>value</child></root>";
    item->setContent(xmlContent);

    EXPECT_TRUE(item->isXmlContent()) << "XML content should be parsed for xpath type";
    EXPECT_NE(item->getXmlContent(), nullptr) << "Should return valid XML element";

    auto rootElement = item->getXmlContent();
    ASSERT_NE(rootElement, nullptr);
    EXPECT_EQ(rootElement->getName(), "root") << "Root element name should match";
}

/**
 * @brief Test XML content parsing with type="xml"
 *
 * Verifies that XML content is parsed when type is "xml"
 */
TEST_F(DataModelItemTest, XmlContentParsing_XmlType) {
    auto item = createItem("data2", "xml");

    const std::string xmlContent = "<element attr='value'/>";
    item->setContent(xmlContent);

    EXPECT_TRUE(item->isXmlContent()) << "XML content should be parsed for xml type";

    auto rootElement = item->getXmlContent();
    ASSERT_NE(rootElement, nullptr);
    EXPECT_EQ(rootElement->getName(), "element");
}

/**
 * @brief Test non-XML content storage for ECMAScript datamodel
 *
 * W3C SCXML B.2: ECMAScript data model stores content as string
 * XML parsing should NOT occur for non-xpath/xml types
 */
TEST_F(DataModelItemTest, NonXmlContent_StringStorage) {
    auto item = createItem("data3", "ecmascript");

    const std::string content = "<not-parsed-xml>";
    item->setContent(content);

    EXPECT_FALSE(item->isXmlContent()) << "Content should not be parsed as XML for ecmascript type";
    EXPECT_EQ(item->getContent(), content) << "Content should be stored as string";
    EXPECT_EQ(item->getXmlContent(), nullptr) << "Should not return XML element";
}

/**
 * @brief Test non-XML content with default type
 *
 * When type is not specified, content should be stored as string
 */
TEST_F(DataModelItemTest, NonXmlContent_DefaultType) {
    auto item = createItem("data4");  // No type specified

    const std::string content = "<some-content>";
    item->setContent(content);

    EXPECT_FALSE(item->isXmlContent());
    EXPECT_EQ(item->getContent(), content);
}

/**
 * @brief Test invalid XML handling
 *
 * When XML parsing fails, content should be stored as string fallback
 */
TEST_F(DataModelItemTest, InvalidXmlContent_FallbackToString) {
    auto item = createItem("data5", "xpath");

    const std::string invalidXml = "<root><unclosed>";
    item->setContent(invalidXml);

    // Implementation should fallback to string storage on parse failure
    EXPECT_FALSE(item->isXmlContent()) << "Invalid XML should not be parsed";
    EXPECT_EQ(item->getContent(), invalidXml) << "Invalid XML should be stored as string";
    EXPECT_EQ(item->getXmlContent(), nullptr);
}

/**
 * @brief Test XML content with attributes
 *
 * Verifies that XML attributes are accessible through IXMLElement interface
 */
TEST_F(DataModelItemTest, XmlContentWithAttributes) {
    auto item = createItem("data6", "xpath");

    const std::string xmlContent = "<root id='123' name='test'/>";
    item->setContent(xmlContent);

    EXPECT_TRUE(item->isXmlContent());

    auto rootElement = item->getXmlContent();
    ASSERT_NE(rootElement, nullptr);
    EXPECT_EQ(rootElement->getAttribute("id"), "123");
    EXPECT_EQ(rootElement->getAttribute("name"), "test");
}

/**
 * @brief Test nested XML structure
 *
 * Verifies parsing of complex nested XML documents
 */
TEST_F(DataModelItemTest, NestedXmlStructure) {
    auto item = createItem("data7", "xpath");

    const std::string xmlContent = "<root>"
                                   "  <parent>"
                                   "    <child1>value1</child1>"
                                   "    <child2>value2</child2>"
                                   "  </parent>"
                                   "</root>";
    item->setContent(xmlContent);

    EXPECT_TRUE(item->isXmlContent());

    auto rootElement = item->getXmlContent();
    ASSERT_NE(rootElement, nullptr);
    EXPECT_EQ(rootElement->getName(), "root");

    // Verify child elements accessible
    auto children = rootElement->getChildrenByTagName("parent");
    EXPECT_FALSE(children.empty()) << "Should find parent element";
}

/**
 * @brief Test addContent() with XML merging
 *
 * Verifies that addContent() properly tracks multiple content additions
 */
TEST_F(DataModelItemTest, AddContent_MultipleItems) {
    auto item = createItem("data8", "xpath");

    const std::string content1 = "<first/>";
    const std::string content2 = "<second/>";

    item->setContent(content1);
    item->addContent(content2);

    const auto &contentItems = item->getContentItems();
    EXPECT_EQ(contentItems.size(), 2);
    EXPECT_EQ(contentItems[0], content1);
    EXPECT_EQ(contentItems[1], content2);
}

/**
 * @brief Test XML content reset
 *
 * Verifies that setting non-XML content clears XML document
 */
TEST_F(DataModelItemTest, XmlContentReset_WhenTypeChanges) {
    auto item = createItem("data9", "xpath");

    // First set XML content
    item->setContent("<root/>");
    EXPECT_TRUE(item->isXmlContent());

    // Change to non-XML type and set new content
    item->setType("ecmascript");
    item->setContent("plain text");

    EXPECT_FALSE(item->isXmlContent()) << "XML content should be cleared";
    EXPECT_EQ(item->getContent(), "plain text");
}

/**
 * @brief Test empty content handling
 *
 * Verifies behavior with empty strings
 */
TEST_F(DataModelItemTest, EmptyContentHandling) {
    auto item = createItem("data10", "xpath");

    item->setContent("");

    EXPECT_FALSE(item->isXmlContent()) << "Empty string should not be parsed as XML";
    EXPECT_EQ(item->getContent(), "");
}

/**
 * @brief Test IXMLDocument interface usage (Zero Duplication verification)
 *
 * Verifies that both Native and WASM use the same IXMLDocument interface
 * This test ensures the refactoring successfully removed platform conditionals
 */
TEST_F(DataModelItemTest, InterfaceUsage_PlatformIndependent) {
    auto item = createItem("data11", "xpath");

    const std::string xmlContent = "<test>content</test>";
    item->setContent(xmlContent);

    // getXmlContent() should return IXMLElement interface on all platforms
    auto element = item->getXmlContent();
    ASSERT_NE(element, nullptr);

    // Interface methods should work identically on Native and WASM
    EXPECT_EQ(element->getName(), "test");

#ifdef __EMSCRIPTEN__
    // This test should pass on WASM without any platform-specific code
    SUCCEED() << "WASM platform: IXMLDocument interface working correctly";
#else
    // This test should pass on Native without any platform-specific code
    SUCCEED() << "Native platform: IXMLDocument interface working correctly";
#endif
}

/**
 * @brief Test expr and src attributes
 *
 * Verifies that expr and src are properly stored/retrieved
 */
TEST_F(DataModelItemTest, ExprAndSrcAttributes) {
    auto item = createItem("data12", "xpath");

    item->setExpr("someExpression");
    item->setSrc("http://example.com/data.xml");

    EXPECT_EQ(item->getExpr(), "someExpression");
    EXPECT_EQ(item->getSrc(), "http://example.com/data.xml");
}

/**
 * @brief Test custom attributes
 *
 * Verifies setAttribute/getAttribute functionality
 */
TEST_F(DataModelItemTest, CustomAttributes) {
    auto item = createItem("data13");

    item->setAttribute("custom1", "value1");
    item->setAttribute("custom2", "value2");

    EXPECT_EQ(item->getAttribute("custom1"), "value1");
    EXPECT_EQ(item->getAttribute("custom2"), "value2");
    EXPECT_EQ(item->getAttribute("nonexistent"), "");

    const auto &attrs = item->getAttributes();
    EXPECT_EQ(attrs.size(), 2);
}

/**
 * @brief Test serializeChildContent() - Platform difference verification
 *
 * Verifies whether Native and WASM both preserve XML structure or only text content.
 * According to IXMLElement.h comments:
 * - All platforms (pugixml): Full XML structure preservation
 *
 * However, actual implementation shows both use full structure preservation.
 */
TEST_F(DataModelItemTest, SerializeChildContent_StructurePreservation) {
    auto item = createItem("data14", "xpath");

    const std::string xmlContent = "<root><child attr='value'>text content</child><sibling>data</sibling></root>";
    item->setContent(xmlContent);

    ASSERT_TRUE(item->isXmlContent());

    auto rootElement = item->getXmlContent();
    ASSERT_NE(rootElement, nullptr);

    std::string serialized = rootElement->serializeChildContent();
    EXPECT_FALSE(serialized.empty()) << "Serialized content should not be empty";

    // Check if XML structure is preserved (both Native and WASM should preserve structure)
    bool hasChildTag = serialized.find("<child") != std::string::npos;
    bool hasAttribute = serialized.find("attr") != std::string::npos;
    bool hasSiblingTag = serialized.find("<sibling>") != std::string::npos;

    // Verify full XML structure is preserved on both platforms
    EXPECT_TRUE(hasChildTag) << "Missing <child> tag in: " << serialized;
    EXPECT_TRUE(hasAttribute) << "Missing attr attribute in: " << serialized;
    EXPECT_TRUE(hasSiblingTag) << "Missing <sibling> tag in: " << serialized;
}
