// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML B.2 — XML DOM tree for the C11 backend's host-side helper.
//
// 1:1 algorithmic mirror of `sce/include/scripting/XMLDOMWrapper.h` and
// `sce/src/scripting/XMLDOMWrapper.cpp` (cpp ref-backend, pugixml-based),
// reimplemented in pure C (no pugixml — that header is C++ only).
// Recursive-descent parser covers the cpp pugixml feature set required
// by W3C SCXML 1.0:
//   * paired `<tag>...</tag>` + self-close `<tag/>`
//   * `attr="value"` and `attr='value'` both quote styles
//   * `xmlns=""` / `xmlns:x=""` as regular attributes (no namespace
//     prefix processing — pugixml's default `parse_default` flag set)
//   * named entity refs `&amp;` / `&lt;` / `&gt;` / `&quot;` / `&apos;`
//     and numeric refs `&#N;` / `&#xN;` (UTF-8 encoded), in attribute
//     values and text content
//   * `<?xml ?>` PI prologue + `<!-- comment -->` skip (anywhere)
//   * `<!DOCTYPE ...>` skip (with optional internal subset `[...]`)
//   * `<![CDATA[...]]>` content as a CDATA node child
//   * mixed text content as PCDATA node children
// DTD validation, xml:space="preserve", and namespace-aware tag lookup
// are deliberately out of scope (cpp pugixml also runs without them on
// `parse_default`).

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

// Mirrors a subset of pugi::xml_node_type — only the kinds needed for
// W3C SCXML B.2 corpus and getElementsByTagName semantics.
typedef enum {
    SCE_XML_NODE_ELEMENT,
    SCE_XML_NODE_PCDATA,  // mixed text content (#PCDATA)
    SCE_XML_NODE_CDATA    // <![CDATA[...]]> section
} sce_xml_node_type_t;

typedef struct sce_xml_node_s {
    sce_xml_node_type_t type;
    char *tag;   // element: tag name; pcdata/cdata: NULL
    char *text;  // pcdata/cdata: content; element: NULL
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
sce_xml_node_t **sce_xml_doc_get_elements_by_tag_name(sce_xml_doc_t *doc, const char *tag, size_t *out_count);

// cpp XMLElement 1:1 mirror.
const char *sce_xml_get_tag_name(const sce_xml_node_t *node);
const char *sce_xml_get_attribute(const sce_xml_node_t *node, const char *attr);

// cpp XMLElement::getElementsByTagName — descends children (self not
// matched), recursive DFS via findElementsByTagNameStatic on each child.
sce_xml_node_t **sce_xml_node_get_elements_by_tag_name(sce_xml_node_t *node, const char *tag, size_t *out_count);

// Free the heap array returned by the two get_elements_by_tag_name
// functions. The element pointers it holds remain owned by the document
// and must not be freed by the caller; only the array itself is released.
void sce_xml_free_node_array(sce_xml_node_t **arr);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // SCE_C_TESTS_SUPPORT_DOM_H
