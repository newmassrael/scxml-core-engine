// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// §scxml-B-2 — Lua bindings for the host-side DOM tree.
// cpp `LuaDOMBinding.cpp` 1:1 algorithmic mirror.
//
// Lifetime model:
//   • DOMDocument userdata owns the `sce_xml_doc_t` tree; its `__gc`
//     calls `sce_xml_doc_free`.
//   • DOMElement userdata holds a Lua registry reference to the
//     owning DOMDocument userdata, plus a raw `sce_xml_node_t *`
//     pointing inside that tree.  The reference keeps the document
//     reachable for as long as any element survives, mirroring cpp's
//     `shared_ptr<XMLElement>` semantics.

#include <sce/lua_dom_binding.h>

#include <sce/dom.h>

#include <lauxlib.h>
#include <lua.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

static const char SCE_LUA_DOM_DOCUMENT_MT[] = "SCE.DOMDocument";
static const char SCE_LUA_DOM_ELEMENT_MT[] = "SCE.DOMElement";

// Userdata structs mirror cpp LuaDOMDocumentUD / LuaDOMElementUD.
typedef struct {
    sce_xml_doc_t *doc;
    sce_xml_node_t *root;  // convenience pointer into doc, no ownership
} sce_lua_dom_doc_ud_t;

typedef struct {
    sce_xml_node_t *element;  // borrowed from owning doc
    int doc_ref;              // LUA_REGISTRYINDEX ref to DOMDocument userdata
} sce_lua_dom_elem_ud_t;

// Forward decls for the C callbacks installed on the metatables.
static int sce_lua_dom_get_elements_by_tag_name(lua_State *L);
static int sce_lua_dom_get_attribute(lua_State *L);
static int sce_lua_dom_has_attribute(lua_State *L);
static int sce_lua_dom_get_tag_name(lua_State *L);
static int sce_lua_dom_has_child_nodes(lua_State *L);
static int sce_lua_dom_index(lua_State *L);
static int sce_lua_dom_gc_document(lua_State *L);
static int sce_lua_dom_gc_element(lua_State *L);
static int sce_lua_dom_push_element(lua_State *L, sce_xml_node_t *element, int doc_stack_index);

// What the handle at stack index 1 stands for, whichever metatable it
// carries.
//
// A document handle answers the Node interface as the document it is and
// the Element vocabulary for its document element — the delegation
// getAttribute and getTagName have always performed — so one resolver
// serves both and `is_document` is the only thing that differs.
typedef struct {
    sce_xml_node_t *node;
    int is_document;
    // Stack index of the document userdata that owns the tree, or 0 when
    // it could not be resolved. Every node pushed from here anchors
    // against it, which is what keeps the tree alive.
    int owner_index;
} sce_lua_dom_handle_t;

static sce_lua_dom_handle_t sce_lua_dom_handle_at(lua_State *L, int index) {
    sce_lua_dom_handle_t handle;
    handle.node = NULL;
    handle.is_document = 0;
    handle.owner_index = 0;

    sce_lua_dom_doc_ud_t *doc_ud = (sce_lua_dom_doc_ud_t *)luaL_testudata(L, index, SCE_LUA_DOM_DOCUMENT_MT);
    if (doc_ud) {
        handle.node = doc_ud->root;
        handle.is_document = 1;
        handle.owner_index = index;
        return handle;
    }

    sce_lua_dom_elem_ud_t *elem_ud = (sce_lua_dom_elem_ud_t *)luaL_testudata(L, index, SCE_LUA_DOM_ELEMENT_MT);
    if (elem_ud) {
        handle.node = elem_ud->element;
        // Resolve the anchored document onto the stack so nodes pushed
        // from this one inherit the same anchor. The document userdata
        // owns the raw tree, so a second document userdata for the same
        // tree would free it twice — the anchor is pushed, never rebuilt.
        if (elem_ud->doc_ref != LUA_NOREF) {
            lua_rawgeti(L, LUA_REGISTRYINDEX, elem_ud->doc_ref);
            handle.owner_index = lua_gettop(L);
        }
    }
    return handle;
}

// Push one node, or nil, anchored against the handle's owning document.
static int sce_lua_dom_push_node_or_nil(lua_State *L, sce_xml_node_t *node, const sce_lua_dom_handle_t *handle) {
    if (!node) {
        lua_pushnil(L);
        return 1;
    }
    if (handle->owner_index == 0) {
        lua_pushnil(L);
        return 1;
    }
    return sce_lua_dom_push_element(L, node, handle->owner_index);
}

// cpp LuaDOMBinding::pushElementObject — push an element userdata that
// borrows from the document at `doc_stack_index`.  The element retains a
// registry ref to that doc userdata so the tree outlives any element.
static int sce_lua_dom_push_element(lua_State *L, sce_xml_node_t *element, int doc_stack_index) {
    sce_lua_dom_elem_ud_t *ud = (sce_lua_dom_elem_ud_t *)lua_newuserdata(L, sizeof(*ud));
    ud->element = element;
    ud->doc_ref = LUA_NOREF;

    // Anchor the owning document so its tree cannot be collected while
    // any element referencing it is still reachable.
    int abs_doc = (doc_stack_index < 0) ? lua_gettop(L) + doc_stack_index : doc_stack_index;
    if (abs_doc >= 1 && abs_doc <= lua_gettop(L) - 1) {
        lua_pushvalue(L, abs_doc);
        ud->doc_ref = luaL_ref(L, LUA_REGISTRYINDEX);
    }

    luaL_getmetatable(L, SCE_LUA_DOM_ELEMENT_MT);
    lua_setmetatable(L, -2);
    return 1;
}

// ─── Public API ─────────────────────────────────────────────────────

void sce_lua_dom_register_metatable(lua_State *L) {
    // DOMDocument metatable
    if (luaL_newmetatable(L, SCE_LUA_DOM_DOCUMENT_MT)) {
        lua_pushcfunction(L, sce_lua_dom_get_elements_by_tag_name);
        lua_setfield(L, -2, "getElementsByTagName");
        lua_pushcfunction(L, sce_lua_dom_get_attribute);
        lua_setfield(L, -2, "getAttribute");
        lua_pushcfunction(L, sce_lua_dom_get_tag_name);
        lua_setfield(L, -2, "getTagName");
        lua_pushcfunction(L, sce_lua_dom_has_attribute);
        lua_setfield(L, -2, "hasAttribute");
        lua_pushcfunction(L, sce_lua_dom_has_child_nodes);
        lua_setfield(L, -2, "hasChildNodes");

        // __index is a function, not the metatable: the DOM read surface
        // is properties, and a property has to be computed. Methods are
        // still reached — the dispatcher falls through to a raw lookup on
        // this same table.
        lua_pushcfunction(L, sce_lua_dom_index);
        lua_setfield(L, -2, "__index");

        lua_pushcfunction(L, sce_lua_dom_gc_document);
        lua_setfield(L, -2, "__gc");
    }
    lua_pop(L, 1);

    // DOMElement metatable. It carries the same members as the document's:
    // a node reached by traversal answers the whole surface, and the two
    // metatables differ only in which `__gc` frees what.
    if (luaL_newmetatable(L, SCE_LUA_DOM_ELEMENT_MT)) {
        lua_pushcfunction(L, sce_lua_dom_get_elements_by_tag_name);
        lua_setfield(L, -2, "getElementsByTagName");
        lua_pushcfunction(L, sce_lua_dom_get_attribute);
        lua_setfield(L, -2, "getAttribute");
        lua_pushcfunction(L, sce_lua_dom_get_tag_name);
        lua_setfield(L, -2, "getTagName");
        lua_pushcfunction(L, sce_lua_dom_has_attribute);
        lua_setfield(L, -2, "hasAttribute");
        lua_pushcfunction(L, sce_lua_dom_has_child_nodes);
        lua_setfield(L, -2, "hasChildNodes");

        lua_pushcfunction(L, sce_lua_dom_index);
        lua_setfield(L, -2, "__index");

        lua_pushcfunction(L, sce_lua_dom_gc_element);
        lua_setfield(L, -2, "__gc");
    }
    lua_pop(L, 1);
}

int sce_lua_dom_push_object(lua_State *L, const char *xml_content) {
    sce_xml_doc_t *doc = sce_xml_parse(xml_content ? xml_content : "");
    if (!doc || !sce_xml_doc_is_valid(doc)) {
        sce_xml_doc_free(doc);
        /* Nothing pushed, so the caller decides what a refusal means. It used
           to push nil and report 1, which could only serve one of the two
           callers: a `<data>` element wants the variable left unbound, and an
           arriving `_event.data` has a reading below this one — §scxml-B-2-8-1
           conditions the DOM reading on the content being a valid document and
           closes with "Otherwise, the Processor MUST treat the content as a
           space-normalized string literal". The cpp sibling
           `LuaDOMBinding::pushDOMObject` reports the same way. */
        return 0;
    }

    sce_lua_dom_doc_ud_t *ud = (sce_lua_dom_doc_ud_t *)lua_newuserdata(L, sizeof(*ud));
    ud->doc = doc;
    ud->root = sce_xml_doc_root(doc);

    luaL_getmetatable(L, SCE_LUA_DOM_DOCUMENT_MT);
    lua_setmetatable(L, -2);
    return 1;
}

void sce_lua_dom_push_object_or_nil(lua_State *L, const char *xml_content) {
    if (sce_lua_dom_push_object(L, xml_content) == 0) {
        lua_pushnil(L);
    }
}

// ─── Method callbacks ───────────────────────────────────────────────

// cpp LuaDOMBinding::lua_getElementsByTagName 1:1 — dispatches on
// userdata type, recursive search delegated to dom.c.
static int sce_lua_dom_get_elements_by_tag_name(lua_State *L) {
    const char *tag = luaL_checkstring(L, 2);

    sce_xml_node_t **elements = NULL;
    size_t count = 0u;

    sce_lua_dom_doc_ud_t *doc_ud = (sce_lua_dom_doc_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_DOCUMENT_MT);
    int self_idx = 1;
    int element_owning_doc_idx = 1;

    if (doc_ud) {
        if (doc_ud->doc) {
            elements = sce_xml_doc_get_elements_by_tag_name(doc_ud->doc, tag, &count);
        }
        // For elements pushed below, anchor against the doc userdata at
        // index 1 (self).
    } else {
        sce_lua_dom_elem_ud_t *elem_ud = (sce_lua_dom_elem_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_ELEMENT_MT);
        if (!elem_ud || !elem_ud->element) {
            return luaL_error(L, "getElementsByTagName called on invalid DOM object");
        }
        elements = sce_xml_node_get_elements_by_tag_name(elem_ud->element, tag, &count);

        // Element userdata at index 1 anchors against its own owning
        // document.  Resolve that doc userdata onto the stack so the
        // pushed children inherit the same anchor.
        if (elem_ud->doc_ref != LUA_NOREF) {
            lua_rawgeti(L, LUA_REGISTRYINDEX, elem_ud->doc_ref);
            element_owning_doc_idx = lua_gettop(L);
        }
        (void)self_idx;
    }

    // Build 1-based Lua array (cpp pushes 1-based; ECMAScript [0]/[1]
    // are lowered to Lua [1]/[2] by EcmaScriptToLuaTransformer).
    lua_newtable(L);
    for (size_t i = 0; i < count; ++i) {
        sce_lua_dom_push_element(L, elements[i], element_owning_doc_idx);
        lua_rawseti(L, -2, (int)(i + 1u));
    }
    sce_xml_free_node_array(elements);
    return 1;
}

// cpp LuaDOMBinding::lua_getAttribute 1:1.
static int sce_lua_dom_get_attribute(lua_State *L) {
    const char *attr = luaL_checkstring(L, 2);

    sce_lua_dom_doc_ud_t *doc_ud = (sce_lua_dom_doc_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_DOCUMENT_MT);
    if (doc_ud && doc_ud->root) {
        const char *val = sce_xml_get_attribute(doc_ud->root, attr);
        lua_pushstring(L, val);
        return 1;
    }

    sce_lua_dom_elem_ud_t *elem_ud = (sce_lua_dom_elem_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_ELEMENT_MT);
    if (elem_ud && elem_ud->element) {
        const char *val = sce_xml_get_attribute(elem_ud->element, attr);
        lua_pushstring(L, val);
        return 1;
    }

    return luaL_error(L, "getAttribute called on invalid DOM object");
}

// DOM Level 2 Core hasAttribute — the answer getAttribute's "" cannot
// give.
static int sce_lua_dom_has_attribute(lua_State *L) {
    const char *attr = luaL_checkstring(L, 2);
    sce_lua_dom_handle_t handle = sce_lua_dom_handle_at(L, 1);
    if (!handle.node) {
        return luaL_error(L, "hasAttribute called on invalid DOM object");
    }
    lua_pushboolean(L, sce_xml_has_attribute(handle.node, attr));
    return 1;
}

static int sce_lua_dom_has_child_nodes(lua_State *L) {
    sce_lua_dom_handle_t handle = sce_lua_dom_handle_at(L, 1);
    if (!handle.node) {
        return luaL_error(L, "hasChildNodes called on invalid DOM object");
    }
    // A document always has one child: its document element.
    lua_pushboolean(L, handle.is_document || handle.node->first_child != NULL);
    return 1;
}

// The DOM read surface, served as properties.
static int sce_lua_dom_index(lua_State *L) {
    const char *key = lua_tostring(L, 2);
    sce_lua_dom_handle_t handle = sce_lua_dom_handle_at(L, 1);

    if (key && handle.node) {
        if (strcmp(key, "nodeType") == 0) {
            lua_pushinteger(L, handle.is_document ? SCE_XML_DOM_TYPE_DOCUMENT : sce_xml_node_type(handle.node));
            return 1;
        }
        if (strcmp(key, "nodeName") == 0) {
            lua_pushstring(L, handle.is_document ? "#document" : sce_xml_node_name(handle.node));
            return 1;
        }
        // DOM Level 1 Core gives an element and a document a null
        // nodeValue; `data` is CharacterData's own name for the value.
        if (strcmp(key, "nodeValue") == 0 || strcmp(key, "data") == 0) {
            if (handle.is_document || !sce_xml_has_node_value(handle.node)) {
                lua_pushnil(L);
            } else {
                lua_pushstring(L, sce_xml_node_value(handle.node));
            }
            return 1;
        }
        if (strcmp(key, "tagName") == 0) {
            if (!handle.is_document && sce_xml_has_node_value(handle.node)) {
                lua_pushnil(L);  // character data has no tag name
            } else {
                lua_pushstring(L, sce_xml_get_tag_name(handle.node));
            }
            return 1;
        }
        if (strcmp(key, "textContent") == 0) {
            char *text = sce_xml_text_content(handle.node);
            if (!text) {
                return luaL_error(L, "out of memory reading textContent");
            }
            lua_pushstring(L, text);
            free(text);
            return 1;
        }
        if (strcmp(key, "childNodes") == 0) {
            lua_newtable(L);
            if (handle.is_document) {
                sce_lua_dom_push_node_or_nil(L, handle.node, &handle);
                lua_rawseti(L, -2, 1);
            } else {
                int position = 1;
                for (sce_xml_node_t *child = handle.node->first_child; child; child = child->next_sibling) {
                    sce_lua_dom_push_node_or_nil(L, child, &handle);
                    lua_rawseti(L, -2, position++);
                }
            }
            return 1;
        }
        if (strcmp(key, "firstChild") == 0) {
            return sce_lua_dom_push_node_or_nil(L, handle.is_document ? handle.node : handle.node->first_child,
                                                &handle);
        }
        if (strcmp(key, "lastChild") == 0) {
            return sce_lua_dom_push_node_or_nil(L, handle.is_document ? handle.node : sce_xml_last_child(handle.node),
                                                &handle);
        }
        if (strcmp(key, "nextSibling") == 0) {
            return sce_lua_dom_push_node_or_nil(L, handle.is_document ? NULL : handle.node->next_sibling, &handle);
        }
        if (strcmp(key, "previousSibling") == 0) {
            return sce_lua_dom_push_node_or_nil(L, handle.is_document ? NULL : sce_xml_previous_sibling(handle.node),
                                                &handle);
        }
        if (strcmp(key, "parentNode") == 0) {
            if (handle.is_document) {
                lua_pushnil(L);
                return 1;
            }
            if (!handle.node->parent) {
                // The document element's parent is the document — DOM
                // Level 1 Core 1.3 — and the handle for it is the
                // anchored userdata itself, because that one owns the
                // tree.
                if (handle.owner_index == 0) {
                    lua_pushnil(L);
                } else {
                    lua_pushvalue(L, handle.owner_index);
                }
                return 1;
            }
            return sce_lua_dom_push_node_or_nil(L, handle.node->parent, &handle);
        }
        // Only the document handle carries documentElement, which is how
        // a document can tell the two kinds apart without nodeType.
        if (strcmp(key, "documentElement") == 0) {
            if (!handle.is_document) {
                lua_pushnil(L);
                return 1;
            }
            return sce_lua_dom_push_node_or_nil(L, handle.node, &handle);
        }
    }

    // Not a property: whatever the metatable holds under this key, which
    // is where the methods live.
    if (lua_getmetatable(L, 1) == 0) {
        lua_pushnil(L);
        return 1;
    }
    lua_pushvalue(L, 2);
    lua_rawget(L, -2);
    return 1;
}

// cpp LuaDOMBinding::lua_getTagName 1:1.
static int sce_lua_dom_get_tag_name(lua_State *L) {
    sce_lua_dom_doc_ud_t *doc_ud = (sce_lua_dom_doc_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_DOCUMENT_MT);
    if (doc_ud && doc_ud->root) {
        lua_pushstring(L, sce_xml_get_tag_name(doc_ud->root));
        return 1;
    }

    sce_lua_dom_elem_ud_t *elem_ud = (sce_lua_dom_elem_ud_t *)luaL_testudata(L, 1, SCE_LUA_DOM_ELEMENT_MT);
    if (elem_ud && elem_ud->element) {
        lua_pushstring(L, sce_xml_get_tag_name(elem_ud->element));
        return 1;
    }

    return luaL_error(L, "getTagName called on invalid DOM object");
}

// cpp LuaDOMBinding::lua_gc_document — releases the parsed tree.
static int sce_lua_dom_gc_document(lua_State *L) {
    sce_lua_dom_doc_ud_t *ud = (sce_lua_dom_doc_ud_t *)luaL_checkudata(L, 1, SCE_LUA_DOM_DOCUMENT_MT);
    if (ud) {
        sce_xml_doc_free(ud->doc);
        ud->doc = NULL;
        ud->root = NULL;
    }
    return 0;
}

// cpp LuaDOMBinding::lua_gc_element — releases the registry ref so the
// owning document becomes eligible for collection too.  The element
// pointer itself is borrowed and must not be freed here.
static int sce_lua_dom_gc_element(lua_State *L) {
    sce_lua_dom_elem_ud_t *ud = (sce_lua_dom_elem_ud_t *)luaL_checkudata(L, 1, SCE_LUA_DOM_ELEMENT_MT);
    if (ud) {
        if (ud->doc_ref != LUA_NOREF) {
            luaL_unref(L, LUA_REGISTRYINDEX, ud->doc_ref);
            ud->doc_ref = LUA_NOREF;
        }
        ud->element = NULL;
    }
    return 0;
}
