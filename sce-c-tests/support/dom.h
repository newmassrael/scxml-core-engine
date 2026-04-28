// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML B.2 — XML DOM tree for the C11 backend's host-side helper.
//
// 1:1 algorithmic mirror of `sce/include/scripting/XMLDOMWrapper.h` and
// `sce/src/scripting/XMLDOMWrapper.cpp` (cpp ref-backend, pugixml-based),
// reimplemented in pure C (no pugixml — that header is C++ only).
// Mini recursive-descent parser covers the corpus subset forced by
// test557 / test561: paired `<tag>...</tag>` + self-close `<tag/>`,
// `attr="value"` and `attr='value'` both quote styles, `xmlns=""` as a
// regular attribute (no namespace prefix processing), whitespace skip,
// optional `<?xml ?>` PI prologue, optional `<!-- comment -->` skip.
// DOCTYPE / CDATA / mixed text content / entity references are not
// covered — the parser stores an error message and reports invalid.

#ifndef SCE_C_TESTS_SUPPORT_DOM_H
#define SCE_C_TESTS_SUPPORT_DOM_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sce_xml_attr_s {
    char *name;
    char *value;
    struct sce_xml_attr_s *next;
} sce_xml_attr_t;

typedef struct sce_xml_node_s {
    char *tag;
    sce_xml_attr_t *attrs;
    struct sce_xml_node_s *first_child;
    struct sce_xml_node_s *next_sibling;
    struct sce_xml_node_s *parent;
} sce_xml_node_t;

typedef struct sce_xml_doc_s sce_xml_doc_t;

// cpp XMLDocument 1:1 mirror.
sce_xml_doc_t *sce_xml_parse(const char *src);
void sce_xml_doc_free(sce_xml_doc_t *doc);
int sce_xml_doc_is_valid(const sce_xml_doc_t *doc);
const char *sce_xml_doc_error(const sce_xml_doc_t *doc);
sce_xml_node_t *sce_xml_doc_root(sce_xml_doc_t *doc);

// cpp XMLDocument::getElementsByTagName — recurses from root (root itself
// is matched, then its descendants via DFS).
sce_xml_node_t **sce_xml_doc_get_elements_by_tag_name(
    sce_xml_doc_t *doc, const char *tag, size_t *out_count);

// cpp XMLElement 1:1 mirror.
const char *sce_xml_get_tag_name(const sce_xml_node_t *node);
const char *sce_xml_get_attribute(const sce_xml_node_t *node, const char *attr);

// cpp XMLElement::getElementsByTagName — descends children (self not
// matched), recursive DFS via findElementsByTagNameStatic on each child.
sce_xml_node_t **sce_xml_node_get_elements_by_tag_name(
    sce_xml_node_t *node, const char *tag, size_t *out_count);

// Free the heap array returned by the two get_elements_by_tag_name
// functions. The element pointers it holds remain owned by the document
// and must not be freed by the caller; only the array itself is released.
void sce_xml_free_node_array(sce_xml_node_t **arr);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // SCE_C_TESTS_SUPPORT_DOM_H
