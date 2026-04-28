// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Unit tests for sce-c-tests/support/dom.{c,h} — verifies the recursive-
// descent parser's coverage of cpp pugixml's `parse_default` feature set.
// W3C SCXML B.2 conformance fixtures (test557 / test561) only exercise a
// minimal subset (paired/self-close tags + dual-quote attrs); these unit
// tests force the rest of the surface so corpus-absent regressions
// (DOCTYPE / CDATA / entity decoding / mixed text) are caught.
//
// Each test is a standalone function returning 0 on PASS, non-zero on
// FAIL. main() runs all and aggregates verdicts. CTest wires this as a
// single binary returning 0 / non-zero based on the aggregate.

#include "dom.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ASSERT_TRUE(cond, msg) \
    do { if (!(cond)) { fprintf(stderr, "  FAIL: %s\n", msg); return 1; } } while (0)

#define ASSERT_STREQ(actual, expected, msg) \
    do { \
        if (strcmp((actual), (expected)) != 0) { \
            fprintf(stderr, "  FAIL: %s — got %s, want %s\n", msg, (actual), (expected)); \
            return 1; \
        } \
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
    const char *xml =
        "<?xml version=\"1.0\"?>"
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
    const char *xml =
        "<!DOCTYPE root [ <!ELEMENT root (leaf*)> <!ATTLIST leaf id ID #REQUIRED> ]>"
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
    const char *xml =
        "<root><leaf><![CDATA[ <not-a-tag> & </not-a-tag> ]]></leaf></root>";
    sce_xml_doc_t *doc = sce_xml_parse(xml);
    ASSERT_TRUE(sce_xml_doc_is_valid(doc), "CDATA-bearing doc");
    sce_xml_node_t *root = sce_xml_doc_root(doc);
    ASSERT_TRUE(root && root->first_child, "root has children");
    sce_xml_node_t *leaf = root->first_child;
    ASSERT_STREQ(sce_xml_get_tag_name(leaf), "leaf", "leaf tag");
    // First (only) child of <leaf> is the CDATA node.
    ASSERT_TRUE(leaf->first_child, "leaf has CDATA child");
    ASSERT_TRUE(leaf->first_child->type == SCE_XML_NODE_CDATA, "CDATA type");
    ASSERT_STREQ(leaf->first_child->text,
                 " <not-a-tag> & </not-a-tag> ", "CDATA verbatim");
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
    ASSERT_STREQ(sce_xml_get_attribute(root, "attr"),
                 "&<>\"'", "named entities decoded");
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
    const char *xml =
        "<root>text1<book title=\"a\"/>text2<![CDATA[raw]]><book title=\"b\"/></root>";
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
