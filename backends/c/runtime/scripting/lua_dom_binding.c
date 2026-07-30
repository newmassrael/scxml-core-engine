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
static int sce_lua_dom_get_tag_name(lua_State *L);
static int sce_lua_dom_gc_document(lua_State *L);
static int sce_lua_dom_gc_element(lua_State *L);

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

        // __index = metatable itself (methods accessible via dot syntax)
        lua_pushvalue(L, -1);
        lua_setfield(L, -2, "__index");

        lua_pushcfunction(L, sce_lua_dom_gc_document);
        lua_setfield(L, -2, "__gc");
    }
    lua_pop(L, 1);

    // DOMElement metatable
    if (luaL_newmetatable(L, SCE_LUA_DOM_ELEMENT_MT)) {
        lua_pushcfunction(L, sce_lua_dom_get_elements_by_tag_name);
        lua_setfield(L, -2, "getElementsByTagName");
        lua_pushcfunction(L, sce_lua_dom_get_attribute);
        lua_setfield(L, -2, "getAttribute");
        lua_pushcfunction(L, sce_lua_dom_get_tag_name);
        lua_setfield(L, -2, "getTagName");

        lua_pushvalue(L, -1);
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
        lua_pushnil(L);
        return 1;
    }

    sce_lua_dom_doc_ud_t *ud = (sce_lua_dom_doc_ud_t *)lua_newuserdata(L, sizeof(*ud));
    ud->doc = doc;
    ud->root = sce_xml_doc_root(doc);

    luaL_getmetatable(L, SCE_LUA_DOM_DOCUMENT_MT);
    lua_setmetatable(L, -2);
    return 1;
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
