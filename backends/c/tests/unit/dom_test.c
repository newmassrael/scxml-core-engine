// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Unit tests for backends/c/tests/support/dom.{c,h} — verifies the recursive-
// descent parser's coverage of cpp pugixml's `parse_default` feature set.
// W3C SCXML B.2 conformance fixtures (test557 / test561) only exercise a
// minimal subset (paired/self-close tags + dual-quote attrs); these unit
// tests force the rest of the surface so corpus-absent regressions
// (DOCTYPE / CDATA / entity decoding / mixed text) are caught.
//
// Each test is a standalone function returning 0 on PASS, non-zero on
// FAIL. main() runs all and aggregates verdicts. CTest wires this as a
// single binary returning 0 / non-zero based on the aggregate.

#include <sce/dom.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ASSERT_TRUE(cond, msg)                                                                                         \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            fprintf(stderr, "  FAIL: %s\n", msg);                                                                      \
            return 1;                                                                                                  \
        }                                                                                                              \
    } while (0)

#define ASSERT_STREQ(actual, expected, msg)                                                                            \
    do {                                                                                                               \
        if (strcmp((actual), (expected)) != 0) {                                                                       \
            fprintf(stderr, "  FAIL: %s — got %s, want %s\n", msg, (actual), (expected));                              \
            return 1;                                                                                                  \
        }                                                                                                              \
    } while (0)

// ─── Test cases ─────────────────────────────────────────────────────

// Baseline — already exercised by W3C corpus, included as a sanity
// floor so a regression in the recursive collector surfaces here too.
static int test_paired_and_self_close(void) {
    const char *xml = "<root><a/><b>x</b></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    size_t count = 0;
    sce_xml_node_t **nodes = sce_xml_doc_get_elements_by_tag_name(doc, "a", &count);
    ASSERT_TRUE(count == 1, "single <a/> match");
    sce_xml_free_node_array(nodes);
    nodes = sce_xml_doc_get_elements_by_tag_name(doc, "b", &count);
    ASSERT_TRUE(count == 1, "single <b> match");
    sce_xml_free_node_array(nodes);
    sce_xml_doc_free(doc);
    return 0;
}

// DOCTYPE prologue should be skipped, the rest parses normally.  Mirrors
// pugixml `parse_default`: DOCTYPE drops on load.
static int test_doctype_prologue(void) {
    const char *xml = "<?xml version=\"1.0\"?>"
                      "<!DOCTYPE root SYSTEM \"root.dtd\">"
                      "<root><leaf/></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid after DOCTYPE skip");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_STREQ(sce_xml_get_tag_name(root), "root", "root tag");
    sce_xml_doc_free(doc);
    return 0;
}

// DOCTYPE with internal subset `[ ... ]` — must balance brackets so the
// terminating `>` inside the subset is not mistaken for the DOCTYPE end.
static int test_doctype_internal_subset(void) {
    const char *xml = "<!DOCTYPE root [ <!ELEMENT root (leaf*)> <!ATTLIST leaf id ID #REQUIRED> ]>"
                      "<root><leaf/></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc with internal subset");
    sce_xml_doc_free(doc);
    return 0;
}

// CDATA section — body bytes preserved verbatim, even XML-significant
// characters like `<` and `&`.  pugixml exposes these as `node_cdata`
// children; sce_xml_collect skips them (element-only).
static int test_cdata_section(void) {
    const char *xml = "<root><leaf><![CDATA[ <not-a-tag> & </not-a-tag> ]]></leaf></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "CDATA-bearing doc");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_TRUE(root && root->first_child, "root has children");
    sce_xml_node_t *leaf = root->first_child;
    ASSERT_STREQ(sce_xml_get_tag_name(leaf), "leaf", "leaf tag");
    // First (only) child of <leaf> is the CDATA node.
    ASSERT_TRUE(leaf->first_child, "leaf has CDATA child");
    ASSERT_TRUE(leaf->first_child->type == SCE_XML_NODE_CDATA, "CDATA type");
    ASSERT_STREQ(leaf->first_child->text, " <not-a-tag> & </not-a-tag> ", "CDATA verbatim");
    sce_xml_doc_free(doc);
    return 0;
}

// Named entity references in attribute values: &amp; / &lt; / &gt; /
// &quot; / &apos; round-trip to their literal characters.
static int test_named_entities_in_attribute(void) {
    const char *xml = "<root attr=\"&amp;&lt;&gt;&quot;&apos;\"/>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_STREQ(sce_xml_get_attribute(root, "attr"), "&<>\"'", "named entities decoded");
    sce_xml_doc_free(doc);
    return 0;
}

// Numeric character references — both decimal `&#N;` and hex `&#xN;`.
// Must UTF-8 encode the resulting codepoint.
static int test_numeric_entities_in_attribute(void) {
    // 'A' = 65, 'B' = 0x42, '€' = U+20AC (3-byte UTF-8: 0xE2 0x82 0xAC).
    const char *xml = "<root attr=\"&#65;&#x42;&#x20AC;\"/>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    const char *attr = sce_xml_get_attribute(root, "attr");
    // "AB€" in UTF-8: A B 0xE2 0x82 0xAC
    ASSERT_TRUE(attr[0] == 'A', "decimal &#65; → 'A'");
    ASSERT_TRUE(attr[1] == 'B', "hex &#x42; → 'B'");
    ASSERT_TRUE((unsigned char)attr[2] == 0xE2u, "UTF-8 byte 0");
    ASSERT_TRUE((unsigned char)attr[3] == 0x82u, "UTF-8 byte 1");
    ASSERT_TRUE((unsigned char)attr[4] == 0xACu, "UTF-8 byte 2");
    ASSERT_TRUE(attr[5] == '\0', "string terminator");
    sce_xml_doc_free(doc);
    return 0;
}

// Mixed text content — PCDATA child between elements, with entity
// references decoded.
static int test_mixed_text_pcdata(void) {
    const char *xml = "<root>before<inner/>after &amp; tail</root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_TRUE(root->first_child, "first child = 'before' PCDATA");
    ASSERT_TRUE(root->first_child->type == SCE_XML_NODE_PCDATA, "first is PCDATA");
    ASSERT_STREQ(root->first_child->text, "before", "PCDATA content 1");
    sce_xml_node_t *inner = root->first_child->next_sibling;
    ASSERT_TRUE(inner && inner->type == SCE_XML_NODE_ELEMENT, "second is element");
    ASSERT_STREQ(inner->tag, "inner", "inner tag");
    sce_xml_node_t *trailing = inner->next_sibling;
    ASSERT_TRUE(trailing && trailing->type == SCE_XML_NODE_PCDATA, "trailing PCDATA");
    ASSERT_STREQ(trailing->text, "after & tail", "trailing PCDATA decoded");
    sce_xml_doc_free(doc);
    return 0;
}

// Comment in element body — must be silently dropped.  Children of
// element are unaffected by the comment.
static int test_comment_in_element_body(void) {
    const char *xml = "<root><a/><!-- ignore --><b/></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    size_t count = 0;
    sce_xml_node_t **nodes = sce_xml_doc_get_elements_by_tag_name(doc, "a", &count);
    ASSERT_TRUE(count == 1, "<a/> still found");
    sce_xml_free_node_array(nodes);
    nodes = sce_xml_doc_get_elements_by_tag_name(doc, "b", &count);
    ASSERT_TRUE(count == 1, "<b/> still found");
    sce_xml_free_node_array(nodes);
    sce_xml_doc_free(doc);
    return 0;
}

// getElementsByTagName must skip non-element children (PCDATA + CDATA)
// and only collect element matches recursively.
static int test_get_elements_skips_text_nodes(void) {
    const char *xml = "<root>text1<book title=\"a\"/>text2<![CDATA[raw]]><book title=\"b\"/></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    size_t count = 0;
    sce_xml_node_t **books = sce_xml_doc_get_elements_by_tag_name(doc, "book", &count);
    ASSERT_TRUE(count == 2, "exactly two <book>");
    ASSERT_STREQ(sce_xml_get_attribute(books[0], "title"), "a", "first book title");
    ASSERT_STREQ(sce_xml_get_attribute(books[1], "title"), "b", "second book title");
    sce_xml_free_node_array(books);
    sce_xml_doc_free(doc);
    return 0;
}

// ─── DOM Level 1 Core read surface ──────────────────────────────────

// The tree an author walks has no whitespace-only text in it, so
// `firstChild` of a pretty-printed document is its first element.
//
// This is the pugixml `parse_default` alignment: while
// getElementsByTagName was the only reader the difference could not be
// seen — that call collects elements — and it decides every traversal
// once `childNodes` and `firstChild` are readable.
static int test_whitespace_between_elements_is_not_a_node(void) {
    const char *xml = "<books xmlns=\"\">\n  <book title=\"t1\"/>\n</books>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_TRUE(root != NULL, "root exists");
    ASSERT_TRUE(root->first_child != NULL, "root has a child");
    ASSERT_STREQ(sce_xml_node_name(root->first_child), "book", "first child is the element");
    ASSERT_TRUE(root->first_child->next_sibling == NULL, "and it is the only child");
    char *text = sce_xml_text_content(root);
    ASSERT_TRUE(text != NULL, "textContent allocates");
    ASSERT_STREQ(text, "", "a pretty-printed element has no text content");
    free(text);
    sce_xml_doc_free(doc);
    return 0;
}

// Character data reports itself the way DOM Level 1 Core does, and the
// two kinds stay distinguishable — which is what nodeType is for.
static int test_character_data_reports_its_own_kind(void) {
    const char *xml = "<p>plain<b>bold</b><![CDATA[raw & <kept>]]></p>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *p = sce_xml_doc_root(doc);

    sce_xml_node_t *first = p->first_child;
    ASSERT_TRUE(first != NULL, "<p> has a first child");
    ASSERT_TRUE(sce_xml_node_type(first) == SCE_XML_DOM_TYPE_TEXT, "first child is text");
    ASSERT_STREQ(sce_xml_node_name(first), "#text", "text node name");
    ASSERT_STREQ(sce_xml_node_value(first), "plain", "text node value");
    ASSERT_TRUE(sce_xml_has_node_value(first), "text has a nodeValue");

    sce_xml_node_t *bold = first->next_sibling;
    ASSERT_TRUE(bold != NULL, "<b> follows the text");
    ASSERT_TRUE(sce_xml_node_type(bold) == SCE_XML_DOM_TYPE_ELEMENT, "<b> is an element");
    ASSERT_TRUE(!sce_xml_has_node_value(bold), "an element has no nodeValue");
    ASSERT_TRUE(sce_xml_previous_sibling(bold) == first, "previousSibling walks back");

    sce_xml_node_t *cdata = sce_xml_last_child(p);
    ASSERT_TRUE(cdata != NULL, "<p> has a last child");
    ASSERT_TRUE(sce_xml_node_type(cdata) == SCE_XML_DOM_TYPE_CDATA_SECTION, "last child is CDATA");
    ASSERT_STREQ(sce_xml_node_name(cdata), "#cdata-section", "CDATA node name");
    ASSERT_STREQ(sce_xml_node_value(cdata), "raw & <kept>", "CDATA value");
    ASSERT_TRUE(sce_xml_previous_sibling(p->first_child) == NULL, "the first child has none");

    char *text = sce_xml_text_content(p);
    ASSERT_TRUE(text != NULL, "textContent allocates");
    ASSERT_STREQ(text, "plainboldraw & <kept>", "textContent is every descendant's data");
    free(text);

    ASSERT_TRUE(sce_xml_has_attribute(p, "missing") == 0, "hasAttribute on an absent name");
    sce_xml_doc_free(doc);
    return 0;
}

// The rule is "whitespace-ONLY runs are not nodes", and the two halves of
// that are easy to conflate: a run of `a ` keeps its trailing space and a
// run of ` c` keeps its leading one.
//
// This is the case the parser used to lose. Its element-body loop called
// the misc skipper, which begins by skipping whitespace, so a run's
// leading whitespace was consumed before anything decided text followed —
// ` c` became `c` and textContent came back a character short of the cpp
// reference backend's.
static int test_leading_whitespace_belongs_to_its_text_run(void) {
    sce_xml_doc_t *doc = sce_xml_parse("<p>a <b/> c</p>");
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *p = sce_xml_doc_root(doc);

    sce_xml_node_t *first = p->first_child;
    ASSERT_TRUE(first != NULL, "<p> has a first child");
    ASSERT_STREQ(sce_xml_node_value(first), "a ", "trailing space kept");

    sce_xml_node_t *element = first->next_sibling;
    ASSERT_TRUE(element != NULL, "<b/> follows it");
    ASSERT_STREQ(sce_xml_node_name(element), "b", "and it is the element");

    sce_xml_node_t *last = sce_xml_last_child(p);
    ASSERT_TRUE(last != NULL && last != element, "a third child follows");
    ASSERT_STREQ(sce_xml_node_value(last), " c", "leading space kept");

    char *text = sce_xml_text_content(p);
    ASSERT_TRUE(text != NULL, "textContent allocates");
    ASSERT_STREQ(text, "a  c", "textContent is the document's text, markup removed");
    free(text);
    sce_xml_doc_free(doc);
    return 0;
}

// hasAttribute answers what getAttribute's "" cannot: an attribute that
// is present and empty.
static int test_has_attribute_separates_absent_from_empty(void) {
    sce_xml_doc_t *doc = sce_xml_parse("<node empty=\"\" set=\"v\"/>");
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "doc must be valid");
    sce_xml_node_t *node = sce_xml_doc_root(doc);
    ASSERT_TRUE(sce_xml_has_attribute(node, "empty") == 1, "present and empty");
    ASSERT_STREQ(sce_xml_get_attribute(node, "empty"), "", "reads as empty");
    ASSERT_TRUE(sce_xml_has_attribute(node, "set") == 1, "present");
    ASSERT_TRUE(sce_xml_has_attribute(node, "absent") == 0, "absent");
    ASSERT_STREQ(sce_xml_get_attribute(node, "absent"), "", "also reads as empty");
    sce_xml_doc_free(doc);
    return 0;
}

// ─── Driver ─────────────────────────────────────────────────────────

typedef int (*test_fn_t)(void);

typedef struct {
    const char *name;
    test_fn_t fn;
} test_entry_t;

int main(void) {
    static const test_entry_t tests[] = {
        {"paired_and_self_close", test_paired_and_self_close},
        {"doctype_prologue", test_doctype_prologue},
        {"doctype_internal_subset", test_doctype_internal_subset},
        {"cdata_section", test_cdata_section},
        {"named_entities_in_attribute", test_named_entities_in_attribute},
        {"numeric_entities_in_attribute", test_numeric_entities_in_attribute},
        {"mixed_text_pcdata", test_mixed_text_pcdata},
        {"comment_in_element_body", test_comment_in_element_body},
        {"get_elements_skips_text_nodes", test_get_elements_skips_text_nodes},
        {"whitespace_between_elements_is_not_a_node", test_whitespace_between_elements_is_not_a_node},
        {"character_data_reports_its_own_kind", test_character_data_reports_its_own_kind},
        {"leading_whitespace_belongs_to_its_text_run", test_leading_whitespace_belongs_to_its_text_run},
        {"has_attribute_separates_absent_from_empty", test_has_attribute_separates_absent_from_empty},
    };
    const size_t n = sizeof(tests) / sizeof(tests[0]);
    size_t failed = 0;
    for (size_t i = 0; i < n; ++i) {
        printf("[%2zu/%zu] %s ... ", i + 1u, n, tests[i].name);
        fflush(stdout);
        int rc = tests[i].fn();
        if (rc == 0) {
            printf("PASS\n");
        } else {
            printf("FAIL\n");
            failed++;
        }
    }
    if (failed != 0u) {
        fprintf(stderr, "FAILED: %zu/%zu tests\n", failed, n);
        return 1;
    }
    printf("OK: %zu/%zu tests\n", n, n);
    return 0;
}
