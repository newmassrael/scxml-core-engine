-- SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
-- SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
--
-- W3C SCXML B.2: JSON.stringify / JSON.parse (ECMAScript standard)
--
-- Single Source of Truth for JSON support in both C++ LuaEngine and Rust
-- sce-rust-lua. Do NOT duplicate this logic elsewhere.
--
-- C++:  #include via CMake-generated string literal
-- Rust: include_str!() at compile time

JSON = {}

function JSON.stringify(v, indent)
    local t = type(v)
    if v == nil then return "null"
    elseif t == "boolean" then return v and "true" or "false"
    elseif t == "number" then
        if v ~= v then return "null" end
        if v == math.huge or v == -math.huge then return "null" end
        if v == math.floor(v) and math.abs(v) < 1e15 then return string.format("%d", v) end
        return tostring(v)
    elseif t == "string" then
        return '"' .. v:gsub('[\\"]', '\\%0'):gsub('\n','\\n'):gsub('\r','\\r'):gsub('\t','\\t') .. '"'
    elseif t == "table" then
        -- Detect arrays: all keys must be positive integers.
        -- Handles sparse tables (gaps emit "null") by iterating to max index.
        -- Guard: if max_idx > count * 2, treat as object to prevent memory explosion
        -- from extreme sparse tables like {[1000000]=true}.
        local max_idx = 0
        local count = 0
        local is_arr = true
        for k, _ in pairs(v) do
            if type(k) == "number" and k > 0 and k == math.floor(k) then
                count = count + 1
                if k > max_idx then max_idx = k end
            else
                is_arr = false
                break
            end
        end
        is_arr = is_arr and count > 0 and max_idx <= count * 2
        if is_arr then
            local parts = {}
            for i = 1, max_idx do
                parts[i] = (v[i] ~= nil) and JSON.stringify(v[i]) or "null"
            end
            return "[" .. table.concat(parts, ",") .. "]"
        else
            local keys = {}
            for k, _ in pairs(v) do
                keys[#keys+1] = k
            end
            table.sort(keys, function(a, b)
                return tostring(a) < tostring(b)
            end)
            local parts = {}
            for _, k in ipairs(keys) do
                parts[#parts+1] = JSON.stringify(tostring(k)) .. ":" .. JSON.stringify(v[k])
            end
            return "{" .. table.concat(parts, ",") .. "}"
        end
    end
    return "null"
end

-- W3C SCXML: JSON input originates from trusted SCXML documents.
-- Sandboxed load() with empty environment prevents access to globals
-- (os, io, require, etc.) even if malformed input is supplied.
-- String literals are extracted first to prevent bracket/colon transformations
-- from corrupting values like "array is [1,2]" or "prefix:true".
function JSON.parse(s)
    if s == nil or s == "" then return nil end
    -- Stage 1: Extract string literals (handles escape sequences)
    local strs = {}
    local buf = {}
    local i = 1
    while i <= #s do
        if s:sub(i, i) == '"' then
            local j = i + 1
            while j <= #s do
                if s:sub(j, j) == '\\' then j = j + 2
                elseif s:sub(j, j) == '"' then break
                else j = j + 1 end
            end
            strs[#strs+1] = s:sub(i+1, j-1)
            buf[#buf+1] = '\x01' .. #strs .. '\x01'
            i = j + 1
        else
            buf[#buf+1] = s:sub(i, i)
            i = i + 1
        end
    end
    -- Stage 2: Transform JSON syntax to Lua (string contents protected)
    local str = table.concat(buf)
    str = str:gsub('%[', '{'):gsub('%]', '}')
    str = str:gsub('\x01(%d+)\x01%s*:', function(idx)
        return '["' .. strs[tonumber(idx)] .. '"]=';
    end)
    str = str:gsub(':true', '=true'):gsub(':false', '=false'):gsub(':null', '=nil')
    str = str:gsub('([=,{])%s*(%d)', '%1%2')
    -- Stage 3: Restore non-key string values
    str = str:gsub('\x01(%d+)\x01', function(idx)
        return '"' .. strs[tonumber(idx)] .. '"'
    end)
    local fn = load("return " .. str, "json", "t", {})
    if fn then return fn() end
    return nil
end
