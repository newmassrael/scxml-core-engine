# EmbedLuaScript.cmake — embed a .lua file as a C++ raw string literal header.
#
# Usage (from CMakeLists.txt):
#   cmake -D LUA_SRC=path/to/file.lua -D HDR_OUT=path/to/output.h -P EmbedLuaScript.cmake
#
# Generates a header containing:
#   static constexpr const char* JSON_BUILTINS_LUA = R"LUA(...)LUA";

file(READ "${LUA_SRC}" _lua_content)

# Derive variable name from filename: json_builtins.lua -> JSON_BUILTINS_LUA
get_filename_component(_basename "${LUA_SRC}" NAME_WE)
string(TOUPPER "${_basename}" _varname)
string(REPLACE "-" "_" _varname "${_varname}")
set(_varname "${_varname}_LUA")

set(_header "// Auto-generated from ${_basename}.lua — DO NOT EDIT\n")
string(APPEND _header "// Single Source of Truth: shared with Rust sce-rust-lua\n")
string(APPEND _header "static constexpr const char* ${_varname} = R\"LUA(\n")
string(APPEND _header "${_lua_content}")
string(APPEND _header ")LUA\";\n")

file(WRITE "${HDR_OUT}" "${_header}")
