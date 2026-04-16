// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "scripting/XMLDOMWrapper.h"
#include <memory>
#include <string>

// Forward declaration
struct lua_State;

namespace SCE {

/**
 * @brief Lua bindings for W3C SCXML B.2 XML DOM API
 *
 * Creates Lua userdata objects wrapping XMLDocument/XMLElement with
 * getElementsByTagName() and getAttribute() methods exposed via metatables.
 * Reuses the same XMLDOMWrapper (pugixml-based) as the QuickJS DOMBinding.
 */
class LuaDOMBinding {
public:
    /**
     * @brief Register DOM metatable in a Lua state
     * Must be called once per lua_State before creating DOM objects.
     */
    static void registerMetatable(lua_State *L);

    /**
     * @brief Reset DOM binding state for engine reset/shutdown
     *
     * W3C SCXML B.2: Mirrors DOMBinding::resetClassId() for API consistency.
     * Lua metatables are per-lua_State and auto-cleaned on lua_close(),
     * so this is currently a no-op.
     */
    static void resetClassId();

    /**
     * @brief Create a DOM userdata from XML content string and push onto Lua stack
     * @param L Lua state
     * @param xmlContent XML string to parse
     * @return 1 if DOM object pushed, 0 if error (error message pushed instead)
     */
    static int pushDOMObject(lua_State *L, const std::string &xmlContent);

    /**
     * @brief Create a DOM element userdata and push onto Lua stack
     * @param L Lua state
     * @param element XMLElement to wrap
     * @return 1 (always pushes one value)
     */
    static int pushElementObject(lua_State *L, std::shared_ptr<XMLElement> element);

private:
    static constexpr const char *DOM_DOCUMENT_MT = "SCE.DOMDocument";
    static constexpr const char *DOM_ELEMENT_MT = "SCE.DOMElement";

    // Lua C function callbacks
    static int lua_getElementsByTagName(lua_State *L);
    static int lua_getAttribute(lua_State *L);
    static int lua_getTagName(lua_State *L);
    static int lua_gc_document(lua_State *L);
    static int lua_gc_element(lua_State *L);
};

}  // namespace SCE
