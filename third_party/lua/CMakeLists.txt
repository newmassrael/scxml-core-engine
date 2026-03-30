# Lua 5.4 Integration
# Header-only interface target with system library linking

add_library(lua54 INTERFACE)

target_include_directories(lua54 INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Link against system Lua 5.4 shared library
# The runtime library (.so.0) is used since dev package may not be installed
find_library(LUA_LIBRARY
    NAMES lua5.4 lua54 lua
    PATHS /usr/lib/x86_64-linux-gnu /usr/lib /usr/local/lib
)

if(NOT LUA_LIBRARY)
    # Dev package not installed — link against runtime .so directly
    find_file(LUA_SO_FILE
        NAMES liblua5.4.so.0
        PATHS /usr/lib/x86_64-linux-gnu /usr/lib
    )
    if(LUA_SO_FILE)
        set(LUA_LIBRARY ${LUA_SO_FILE})
    endif()
endif()

if(LUA_LIBRARY)
    target_link_libraries(lua54 INTERFACE ${LUA_LIBRARY})
    message(STATUS "Lua 5.4: Found library at ${LUA_LIBRARY}")
else()
    message(WARNING "Lua 5.4: Library not found, LuaEngine will not be available")
endif()
