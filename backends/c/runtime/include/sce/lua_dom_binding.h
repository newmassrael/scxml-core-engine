// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// §scxml-B-2 — Lua bindings for the host-side DOM tree.
//
// 1:1 algorithmic mirror of `sce/include/scripting/LuaDOMBinding.h` and
// `sce/src/scripting/LuaDOMBinding.cpp` (cpp ref-backend).  The cpp
// implementation owns its tree through `std::shared_ptr<XMLElement>`;
// element userdata can outlive the document because each element holds
// its own shared_ptr.  In C we model the same lifetime with a Lua
// registry anchor stored on every element userdata: as long as the
// element is reachable, Lua keeps the owning DOMDocument userdata
// reachable too, so the underlying tree stays valid.  When the document
// userdata is finally collected, `__gc` calls `sce_xml_doc_free` and
// releases the whole tree.

#ifndef SCE_C_TESTS_SUPPORT_LUA_DOM_BINDING_H
#define SCE_C_TESTS_SUPPORT_LUA_DOM_BINDING_H

struct lua_State;

#ifdef __cplusplus
extern "C" {
#endif

// cpp LuaDOMBinding::registerMetatable — install metatables for both
// `SCE.DOMDocument` and `SCE.DOMElement` in the registry.  Must be
// called once per `lua_State` before any DOM userdata is created.
void sce_lua_dom_register_metatable(struct lua_State *L);

// cpp LuaDOMBinding::pushDOMObject — parse `xml_content`, push the
// resulting DOM document userdata onto the stack, and return 1.  On
// parse failure, push NOTHING and return 0, so the caller decides what a
// refusal means: a `<data>` element leaves its variable unbound, while an
// arriving `_event.data` falls through to the readings §scxml-B-2-8-1
// names below this one.  Matches the cpp signature.
int sce_lua_dom_push_object(struct lua_State *L, const char *xml_content);

// The same parse for a caller that has ALREADY decided the content is a
// document — `<data>`, `<data src=>` and `<content>`, all read by the SCXML
// parser before codegen emitted the call.  Always leaves exactly one value on
// the stack, nil when the parse refused, so `lua_setglobal` after it is
// balanced.  Separate from the reporting form above because the two callers
// want opposite things from a refusal, and one signature could only give them
// one: an arriving `_event.data` must fall through to the readings
// §scxml-B-2-8-1 names below the DOM one.
void sce_lua_dom_push_object_or_nil(struct lua_State *L, const char *xml_content);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // SCE_C_TESTS_SUPPORT_LUA_DOM_BINDING_H
