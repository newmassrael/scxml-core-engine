// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/LuaDOMBinding.h"
#include "core/LogMacros.h"

extern "C" {
#include <lauxlib.h>
#include <lua.h>
}

#include <cstring>

namespace SCE {

// Userdata structs stored in Lua
//
// Both carry the document. An XMLElement is a view into the
// XMLDocument's pugixml arena, so a handle that held only the element
// would read freed memory once the document handle was collected — which
// is reachable from a document that assigns an element to one variable
// and overwrites the variable the tree came from.
struct LuaDOMDocumentUD {
    std::shared_ptr<XMLDocument> document;
    std::shared_ptr<XMLElement> rootElement;
};

struct LuaDOMElementUD {
    std::shared_ptr<XMLDocument> document;
    std::shared_ptr<XMLElement> element;
};

namespace {

/// The node a handle wraps, whichever of the two metatables it carries.
///
/// A document handle answers the Node interface as the document it is
/// and the Element vocabulary for its document element — the delegation
/// `getAttribute` and `getTagName` have always performed — so one
/// resolver serves both and `isDocument` is the only thing that differs.
struct Handle {
    std::shared_ptr<XMLDocument> document;
    std::shared_ptr<XMLElement> node;
    bool isDocument = false;

    bool valid() const {
        return static_cast<bool>(node);
    }
};

Handle handleAt(lua_State *L, int index, const char *documentMT, const char *elementMT) {
    Handle handle;
    if (auto *docUD = static_cast<LuaDOMDocumentUD *>(luaL_testudata(L, index, documentMT))) {
        handle.document = docUD->document;
        handle.node = docUD->rootElement;
        handle.isDocument = true;
        return handle;
    }
    if (auto *elemUD = static_cast<LuaDOMElementUD *>(luaL_testudata(L, index, elementMT))) {
        handle.document = elemUD->document;
        handle.node = elemUD->element;
    }
    return handle;
}

}  // namespace

// === Reset Hook ===

void LuaDOMBinding::resetClassId() {
    // §scxml-B-2: No-op for Lua — metatables are per-lua_State and
    // automatically cleaned up on lua_close(). Provided for API consistency
    // with DOMBinding::resetClassId() (QuickJS runtime class IDs).
}

// === Metatable Registration ===

void LuaDOMBinding::registerMetatable(lua_State *L) {
    // Both metatables carry the same members: the document handle
    // answers the Node interface for the document and the Element
    // interface for its document element, so every name resolves on
    // either kind and `lua_index` decides what it means.
    for (const char *name : {DOM_DOCUMENT_MT, DOM_ELEMENT_MT}) {
        if (luaL_newmetatable(L, name)) {
            lua_pushcfunction(L, lua_getElementsByTagName);
            lua_setfield(L, -2, "getElementsByTagName");
            lua_pushcfunction(L, lua_getAttribute);
            lua_setfield(L, -2, "getAttribute");
            lua_pushcfunction(L, lua_hasAttribute);
            lua_setfield(L, -2, "hasAttribute");
            lua_pushcfunction(L, lua_getTagName);
            lua_setfield(L, -2, "getTagName");
            lua_pushcfunction(L, lua_hasChildNodes);
            lua_setfield(L, -2, "hasChildNodes");

            // __index is a function rather than the metatable itself:
            // the read surface is properties, and a property has to be
            // computed. Methods are still reached — `lua_index` falls
            // through to a raw lookup on this same table.
            lua_pushcfunction(L, lua_index);
            lua_setfield(L, -2, "__index");

            lua_pushcfunction(L, std::strcmp(name, DOM_DOCUMENT_MT) == 0 ? lua_gc_document : lua_gc_element);
            lua_setfield(L, -2, "__gc");
        }
        lua_pop(L, 1);
    }
}

// === DOM Object Creation ===

int LuaDOMBinding::pushDOMObject(lua_State *L, const std::string &xmlContent) {
    auto document = std::make_shared<XMLDocument>(xmlContent);
    if (!document->isValid()) {
        // Nothing pushed, and no log line: the caller decides whether a
        // refusal is an error. `setVariableAsDOM` is handed content the SCXML
        // parser already read, so a refusal there is this engine's invariant
        // breaking; an arriving `_event.data` that merely opens with `<` has a
        // reading below this one (§scxml-B-2-8-1) and is perfectly ordinary.
        // Pushing nil served neither — it answered for the caller — and it is
        // what the header has documented as `0 if error` all along.
        SCE_LOG_DEBUG("LuaDOMBinding: content is not a valid XML document - {}", document->getErrorMessage());
        return 0;
    }

    // Create userdata
    auto *ud = static_cast<LuaDOMDocumentUD *>(lua_newuserdata(L, sizeof(LuaDOMDocumentUD)));
    new (ud) LuaDOMDocumentUD();
    ud->document = document;
    ud->rootElement = document->getDocumentElement();

    // Set metatable
    luaL_getmetatable(L, DOM_DOCUMENT_MT);
    lua_setmetatable(L, -2);

    return 1;
}

int LuaDOMBinding::pushElementObject(lua_State *L, std::shared_ptr<XMLDocument> document,
                                     std::shared_ptr<XMLElement> node) {
    if (!node) {
        lua_pushnil(L);
        return 1;
    }

    // pugixml's document node is the parent of a document element, so a
    // climb from the root lands on it. It is pushed as the document
    // handle — the same value the variable holds — rather than as an
    // element whose vocabulary would be a third shape.
    if (node->getNodeType() == DomNodeType::Document) {
        auto *docUD = static_cast<LuaDOMDocumentUD *>(lua_newuserdata(L, sizeof(LuaDOMDocumentUD)));
        new (docUD) LuaDOMDocumentUD();
        docUD->document = std::move(document);
        docUD->rootElement = docUD->document ? docUD->document->getDocumentElement() : nullptr;
        luaL_getmetatable(L, DOM_DOCUMENT_MT);
        lua_setmetatable(L, -2);
        return 1;
    }

    auto *ud = static_cast<LuaDOMElementUD *>(lua_newuserdata(L, sizeof(LuaDOMElementUD)));
    new (ud) LuaDOMElementUD();
    ud->document = std::move(document);
    ud->element = std::move(node);

    luaL_getmetatable(L, DOM_ELEMENT_MT);
    lua_setmetatable(L, -2);

    return 1;
}

// === Lua C Callbacks ===

int LuaDOMBinding::lua_getElementsByTagName(lua_State *L) {
    const char *tagName = luaL_checkstring(L, 2);
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (!handle.valid()) {
        return luaL_error(L, "getElementsByTagName called on invalid DOM object");
    }

    // A document matches its root inclusively, an element only descends
    // into its children — DOM Level 1 Core 1.2's split between
    // Document.getElementsByTagName and Element's.
    std::vector<std::shared_ptr<XMLElement>> elements =
        handle.isDocument ? handle.document->getElementsByTagName(tagName) : handle.node->getElementsByTagName(tagName);

    // Create Lua table with 1-based indexing (Lua convention)
    // ECMAScript [0],[1] are lowered to Lua [1],[2] by sce-build's ECMAScript
    // frontend, which is what shifts the index (`lua.rs`, `Expr::Index`)
    lua_newtable(L);
    for (size_t i = 0; i < elements.size(); ++i) {
        pushElementObject(L, handle.document, elements[i]);
        lua_rawseti(L, -2, static_cast<int>(i + 1));
    }

    return 1;
}

int LuaDOMBinding::lua_getAttribute(lua_State *L) {
    const char *attrName = luaL_checkstring(L, 2);
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (!handle.valid()) {
        return luaL_error(L, "getAttribute called on invalid DOM object");
    }
    lua_pushstring(L, handle.node->getAttribute(attrName).c_str());
    return 1;
}

int LuaDOMBinding::lua_hasAttribute(lua_State *L) {
    const char *attrName = luaL_checkstring(L, 2);
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (!handle.valid()) {
        return luaL_error(L, "hasAttribute called on invalid DOM object");
    }
    lua_pushboolean(L, handle.node->hasAttribute(attrName) ? 1 : 0);
    return 1;
}

int LuaDOMBinding::lua_getTagName(lua_State *L) {
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (!handle.valid()) {
        return luaL_error(L, "getTagName called on invalid DOM object");
    }
    lua_pushstring(L, handle.node->getTagName().c_str());
    return 1;
}

int LuaDOMBinding::lua_hasChildNodes(lua_State *L) {
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (!handle.valid()) {
        return luaL_error(L, "hasChildNodes called on invalid DOM object");
    }
    // A document always has one child: its document element.
    lua_pushboolean(L, (handle.isDocument || handle.node->hasChildNodes()) ? 1 : 0);
    return 1;
}

int LuaDOMBinding::lua_index(lua_State *L) {
    const char *key = lua_tostring(L, 2);
    Handle handle = handleAt(L, 1, DOM_DOCUMENT_MT, DOM_ELEMENT_MT);
    if (key != nullptr && handle.valid()) {
        const std::string name(key);

        // ── The Node interface, as properties ──
        if (name == "nodeType") {
            lua_pushinteger(L, handle.isDocument ? DomNodeType::Document : handle.node->getNodeType());
            return 1;
        }
        if (name == "nodeName") {
            lua_pushstring(L, handle.isDocument ? "#document" : handle.node->getNodeName().c_str());
            return 1;
        }
        // DOM Level 1 Core gives an element and a document a null
        // nodeValue; `data` is the same value under CharacterData's own
        // name for it.
        if (name == "nodeValue" || name == "data") {
            if (handle.isDocument || !handle.node->hasNodeValue()) {
                lua_pushnil(L);
            } else {
                lua_pushstring(L, handle.node->getNodeValue().c_str());
            }
            return 1;
        }
        if (name == "tagName") {
            if (!handle.isDocument && handle.node->hasNodeValue()) {
                lua_pushnil(L);  // character data has no tag name
            } else {
                lua_pushstring(L, handle.node->getTagName().c_str());
            }
            return 1;
        }
        if (name == "textContent") {
            lua_pushstring(L, handle.node->getTextContent().c_str());
            return 1;
        }
        if (name == "childNodes") {
            lua_newtable(L);
            if (handle.isDocument) {
                pushElementObject(L, handle.document, handle.node);
                lua_rawseti(L, -2, 1);
            } else {
                std::vector<std::shared_ptr<XMLElement>> children = handle.node->getChildNodes();
                for (size_t i = 0; i < children.size(); ++i) {
                    pushElementObject(L, handle.document, children[i]);
                    lua_rawseti(L, -2, static_cast<int>(i + 1));
                }
            }
            return 1;
        }
        if (name == "firstChild" || name == "lastChild") {
            if (handle.isDocument) {
                return pushElementObject(L, handle.document, handle.node);
            }
            return pushElementObject(L, handle.document,
                                     name == "firstChild" ? handle.node->getFirstChild() : handle.node->getLastChild());
        }
        if (name == "nextSibling" || name == "previousSibling") {
            if (handle.isDocument) {
                lua_pushnil(L);
                return 1;
            }
            return pushElementObject(L, handle.document,
                                     name == "nextSibling" ? handle.node->getNextSibling()
                                                           : handle.node->getPreviousSibling());
        }
        if (name == "parentNode") {
            if (handle.isDocument) {
                lua_pushnil(L);
                return 1;
            }
            return pushElementObject(L, handle.document, handle.node->getParentNode());
        }
        // Only the document handle carries documentElement, which is how
        // a document can tell the two kinds apart without nodeType.
        if (name == "documentElement") {
            if (!handle.isDocument) {
                lua_pushnil(L);
                return 1;
            }
            return pushElementObject(L, handle.document, handle.node);
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

// === GC Callbacks ===

int LuaDOMBinding::lua_gc_document(lua_State *L) {
    auto *ud = static_cast<LuaDOMDocumentUD *>(luaL_checkudata(L, 1, DOM_DOCUMENT_MT));
    if (ud) {
        ud->~LuaDOMDocumentUD();
    }
    return 0;
}

int LuaDOMBinding::lua_gc_element(lua_State *L) {
    auto *ud = static_cast<LuaDOMElementUD *>(luaL_checkudata(L, 1, DOM_ELEMENT_MT));
    if (ud) {
        ud->~LuaDOMElementUD();
    }
    return 0;
}

}  // namespace SCE
