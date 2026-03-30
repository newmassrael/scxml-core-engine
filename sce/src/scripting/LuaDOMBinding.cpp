#include "scripting/LuaDOMBinding.h"
#include "core/LogMacros.h"

extern "C" {
#include <lauxlib.h>
#include <lua.h>
}

namespace SCE {

// Userdata structs stored in Lua
struct LuaDOMDocumentUD {
    std::shared_ptr<XMLDocument> document;
    std::shared_ptr<XMLElement> rootElement;
};

struct LuaDOMElementUD {
    std::shared_ptr<XMLElement> element;
};

// === Metatable Registration ===

void LuaDOMBinding::registerMetatable(lua_State *L) {
    // DOMDocument metatable
    if (luaL_newmetatable(L, DOM_DOCUMENT_MT)) {
        lua_pushcfunction(L, lua_getElementsByTagName);
        lua_setfield(L, -2, "getElementsByTagName");
        lua_pushcfunction(L, lua_getAttribute);
        lua_setfield(L, -2, "getAttribute");
        lua_pushcfunction(L, lua_getTagName);
        lua_setfield(L, -2, "getTagName");

        // __index = metatable itself (methods accessible via .)
        lua_pushvalue(L, -1);
        lua_setfield(L, -2, "__index");

        // __gc for cleanup
        lua_pushcfunction(L, lua_gc_document);
        lua_setfield(L, -2, "__gc");
    }
    lua_pop(L, 1);

    // DOMElement metatable
    if (luaL_newmetatable(L, DOM_ELEMENT_MT)) {
        lua_pushcfunction(L, lua_getElementsByTagName);
        lua_setfield(L, -2, "getElementsByTagName");
        lua_pushcfunction(L, lua_getAttribute);
        lua_setfield(L, -2, "getAttribute");
        lua_pushcfunction(L, lua_getTagName);
        lua_setfield(L, -2, "getTagName");

        lua_pushvalue(L, -1);
        lua_setfield(L, -2, "__index");

        lua_pushcfunction(L, lua_gc_element);
        lua_setfield(L, -2, "__gc");
    }
    lua_pop(L, 1);
}

// === DOM Object Creation ===

int LuaDOMBinding::pushDOMObject(lua_State *L, const std::string &xmlContent) {
    auto document = std::make_shared<XMLDocument>(xmlContent);
    if (!document->isValid()) {
        SCE_LOG_ERROR("LuaDOMBinding: Failed to parse XML - {}", document->getErrorMessage());
        lua_pushnil(L);
        return 1;
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

int LuaDOMBinding::pushElementObject(lua_State *L, std::shared_ptr<XMLElement> element) {
    auto *ud = static_cast<LuaDOMElementUD *>(lua_newuserdata(L, sizeof(LuaDOMElementUD)));
    new (ud) LuaDOMElementUD();
    ud->element = std::move(element);

    luaL_getmetatable(L, DOM_ELEMENT_MT);
    lua_setmetatable(L, -2);

    return 1;
}

// === Lua C Callbacks ===

int LuaDOMBinding::lua_getElementsByTagName(lua_State *L) {
    const char *tagName = luaL_checkstring(L, 2);

    std::vector<std::shared_ptr<XMLElement>> elements;

    // Try as DOMDocument first
    auto *docUD = static_cast<LuaDOMDocumentUD *>(luaL_testudata(L, 1, DOM_DOCUMENT_MT));
    if (docUD) {
        if (docUD->document) {
            elements = docUD->document->getElementsByTagName(tagName);
        }
    } else {
        // Try as DOMElement
        auto *elemUD = static_cast<LuaDOMElementUD *>(luaL_testudata(L, 1, DOM_ELEMENT_MT));
        if (elemUD && elemUD->element) {
            elements = elemUD->element->getElementsByTagName(tagName);
        } else {
            return luaL_error(L, "getElementsByTagName called on invalid DOM object");
        }
    }

    // Create Lua table (1-based array)
    lua_newtable(L);
    for (size_t i = 0; i < elements.size(); ++i) {
        pushElementObject(L, elements[i]);
        lua_rawseti(L, -2, static_cast<int>(i + 1));
    }

    return 1;
}

int LuaDOMBinding::lua_getAttribute(lua_State *L) {
    const char *attrName = luaL_checkstring(L, 2);

    // Try as DOMDocument (uses root element)
    auto *docUD = static_cast<LuaDOMDocumentUD *>(luaL_testudata(L, 1, DOM_DOCUMENT_MT));
    if (docUD && docUD->rootElement) {
        std::string val = docUD->rootElement->getAttribute(attrName);
        lua_pushstring(L, val.c_str());
        return 1;
    }

    // Try as DOMElement
    auto *elemUD = static_cast<LuaDOMElementUD *>(luaL_testudata(L, 1, DOM_ELEMENT_MT));
    if (elemUD && elemUD->element) {
        std::string val = elemUD->element->getAttribute(attrName);
        lua_pushstring(L, val.c_str());
        return 1;
    }

    return luaL_error(L, "getAttribute called on invalid DOM object");
}

int LuaDOMBinding::lua_getTagName(lua_State *L) {
    auto *docUD = static_cast<LuaDOMDocumentUD *>(luaL_testudata(L, 1, DOM_DOCUMENT_MT));
    if (docUD && docUD->rootElement) {
        lua_pushstring(L, docUD->rootElement->getTagName().c_str());
        return 1;
    }

    auto *elemUD = static_cast<LuaDOMElementUD *>(luaL_testudata(L, 1, DOM_ELEMENT_MT));
    if (elemUD && elemUD->element) {
        lua_pushstring(L, elemUD->element->getTagName().c_str());
        return 1;
    }

    return luaL_error(L, "getTagName called on invalid DOM object");
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
