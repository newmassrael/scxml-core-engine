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
        -- Escaped character by character rather than with `gsub`, because the
        -- Lua behind the six backends is not one Lua: go-lua ships no `gsub`
        -- at all -- its string library comments the whole pattern family out
        -- -- so a pattern-based escape worked on four engines and raised on
        -- the fifth. It surfaced as a structured `<data>` that could not be
        -- read there at all, since a table of strings recurses through here.
        -- Only `sub`, `byte`, `format` and `table.concat` are used below, and
        -- every Lua this file is loaded into has those.
        --
        -- C0 controls are escaped as `\uXXXX` rather than passed through:
        -- RFC 8259 forbids them raw, so emitting one produced JSON that no
        -- parser had to accept -- including this file's own.
        local out = {'"'}
        for i = 1, #v do
            local c = string.sub(v, i, i)
            if c == '\\' then out[#out+1] = '\\\\'
            elseif c == '"' then out[#out+1] = '\\"'
            elseif c == '\n' then out[#out+1] = '\\n'
            elseif c == '\r' then out[#out+1] = '\\r'
            elseif c == '\t' then out[#out+1] = '\\t'
            elseif string.byte(c) < 32 then out[#out+1] = string.format('\\u%04x', string.byte(c))
            else out[#out+1] = c end
        end
        out[#out+1] = '"'
        return table.concat(out)
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

-- ECMA-262 15.12.2 / RFC 8259: JSON.parse, as a parser.
--
-- ⚠ This used to rewrite the text into Lua source and `load` it in an empty
-- environment. Two things followed, and both were silent:
--
--   * it accepted what is not JSON. `2 + 3` is a Lua expression, so the
--     rewrite produced `return 2 + 3` and the "parse" answered 5, where
--     ECMA-262 requires a SyntaxError. That reached far past an author
--     calling `JSON.parse`: the C11 backend decodes an arriving `_event.data`
--     through this shape, so a payload that was an EXPRESSION got evaluated
--     at the receiving boundary and a payload that was a CALL ran.
--     The ECMAScript data model gives an arriving payload three readings --
--     XML, JSON, space-normalized string -- and "whatever Lua makes of it"
--     is not one of them.
--   * `gsub` is not in every Lua this file is loaded into. go-lua comments
--     the whole pattern family out, so this half raised there while
--     `JSON.stringify` above was written specifically to avoid it.
--
-- Only `sub`, `byte`, `char`, `format`, `tonumber` and `table.concat` are
-- used below, which every Lua this file reaches has, and nothing is run.
--
-- Split into `JSON._*` functions rather than one closure because the C11
-- backend loads this file as a sequence of `luaL_dostring` chunks split on
-- BLANK LINES: a local from one chunk does not exist in the next, and a
-- function long enough to be split silently fails to compile and leaves
-- `JSON.parse` nil. Fields of the global table resolve at call time, so
-- splitting between them is harmless. Keep each function blank-line free.
--
-- Returns `value, true` on success and `nil, false` on anything that is not
-- JSON. A caller taking only the first return keeps the old contract; one
-- that must tell JSON `null` from "not JSON" -- the event-payload decoders
-- must -- takes the second.
function JSON._skip_ws(s, i)
    local n = #s
    while i <= n do
        local c = string.sub(s, i, i)
        if c == " " or c == "\t" or c == "\n" or c == "\r" then
            i = i + 1
        else
            return i
        end
    end
    return i
end

-- RFC 8259 §7's `\uXXXX`, as UTF-8 bytes. Encoded by hand rather than with
-- `utf8.char`, which is Lua 5.3+ and this file reaches older interpreters.
function JSON._utf8(cp)
    if cp < 0x80 then return string.char(cp) end
    if cp < 0x800 then
        return string.char(0xC0 + math.floor(cp / 0x40), 0x80 + cp % 0x40)
    end
    if cp < 0x10000 then
        return string.char(0xE0 + math.floor(cp / 0x1000),
                           0x80 + math.floor(cp / 0x40) % 0x40,
                           0x80 + cp % 0x40)
    end
    return string.char(0xF0 + math.floor(cp / 0x40000),
                       0x80 + math.floor(cp / 0x1000) % 0x40,
                       0x80 + math.floor(cp / 0x40) % 0x40,
                       0x80 + cp % 0x40)
end

-- `i` is at the opening quote. Returns value, next index, ok.
function JSON._parse_string(s, i)
    local n = #s
    i = i + 1
    local out = {}
    while i <= n do
        local c = string.sub(s, i, i)
        if c == '"' then
            return table.concat(out), i + 1, true
        elseif c == "\\" then
            local esc = string.sub(s, i + 1, i + 1)
            i = i + 2
            if esc == '"' then out[#out+1] = '"'
            elseif esc == "\\" then out[#out+1] = "\\"
            elseif esc == "/" then out[#out+1] = "/"
            elseif esc == "b" then out[#out+1] = "\b"
            elseif esc == "f" then out[#out+1] = "\f"
            elseif esc == "n" then out[#out+1] = "\n"
            elseif esc == "r" then out[#out+1] = "\r"
            elseif esc == "t" then out[#out+1] = "\t"
            elseif esc == "u" then
                local hex = string.sub(s, i, i + 3)
                local cp = (#hex == 4) and tonumber(hex, 16) or nil
                if cp == nil then return nil, i, false end
                i = i + 4
                -- A character outside the BMP is written as a surrogate
                -- pair, and the two halves are one character.
                if cp >= 0xD800 and cp <= 0xDBFF and string.sub(s, i, i + 1) == "\\u" then
                    local lo = tonumber(string.sub(s, i + 2, i + 5), 16)
                    if lo ~= nil and lo >= 0xDC00 and lo <= 0xDFFF then
                        cp = 0x10000 + (cp - 0xD800) * 0x400 + (lo - 0xDC00)
                        i = i + 6
                    end
                end
                out[#out+1] = JSON._utf8(cp)
            else
                return nil, i, false
            end
        elseif string.byte(c) < 32 then
            -- RFC 8259 §7 forbids a raw control character inside a string.
            return nil, i, false
        else
            out[#out+1] = c
            i = i + 1
        end
    end
    return nil, i, false
end

-- RFC 8259 §6. Deliberately stricter than `tonumber`, which accepts hex,
-- leading `+` and Lua's own spellings -- none of which are JSON.
function JSON._parse_number(s, i)
    local n = #s
    local start = i
    if string.sub(s, i, i) == "-" then i = i + 1 end
    -- RFC 8259 §6's `int` is `0` or `[1-9][0-9]*`: a leading zero is not a
    -- JSON number. `tonumber` takes `01` happily, which is the kind of
    -- difference that makes a reader's accepted set its host language's
    -- rather than the grammar's.
    local first = string.byte(s, i)
    if first == nil or first < 48 or first > 57 then return nil, i, false end
    if first == 48 then
        i = i + 1
        local after = string.byte(s, i)
        if after ~= nil and after >= 48 and after <= 57 then return nil, i, false end
    else
        while i <= n do
            local b = string.byte(s, i)
            if b >= 48 and b <= 57 then i = i + 1 else break end
        end
    end
    if string.sub(s, i, i) == "." then
        i = i + 1
        local frac = 0
        while i <= n do
            local b = string.byte(s, i)
            if b >= 48 and b <= 57 then i = i + 1; frac = frac + 1 else break end
        end
        if frac == 0 then return nil, i, false end
    end
    local e = string.sub(s, i, i)
    if e == "e" or e == "E" then
        i = i + 1
        local sign = string.sub(s, i, i)
        if sign == "+" or sign == "-" then i = i + 1 end
        local expd = 0
        while i <= n do
            local b = string.byte(s, i)
            if b >= 48 and b <= 57 then i = i + 1; expd = expd + 1 else break end
        end
        if expd == 0 then return nil, i, false end
    end
    local num = tonumber(string.sub(s, start, i - 1))
    if num == nil then return nil, i, false end
    return num, i, true
end

-- 1-based, the way every other sequence in this datamodel is; the
-- ECMAScript-to-Lua frontend rewrites `[0]` to `[1]`.
function JSON._parse_array(s, i)
    i = JSON._skip_ws(s, i + 1)
    local arr = {}
    if string.sub(s, i, i) == "]" then return arr, i + 1, true end
    local count = 0
    while true do
        local v, ni, ok = JSON._parse_value(s, i)
        if not ok then return nil, ni, false end
        count = count + 1
        arr[count] = v
        i = JSON._skip_ws(s, ni)
        local c = string.sub(s, i, i)
        if c == "," then
            i = i + 1
        elseif c == "]" then
            return arr, i + 1, true
        else
            return nil, i, false
        end
    end
end

function JSON._parse_object(s, i)
    i = JSON._skip_ws(s, i + 1)
    local obj = {}
    if string.sub(s, i, i) == "}" then return obj, i + 1, true end
    while true do
        i = JSON._skip_ws(s, i)
        if string.sub(s, i, i) ~= '"' then return nil, i, false end
        local key, ki, kok = JSON._parse_string(s, i)
        if not kok then return nil, ki, false end
        i = JSON._skip_ws(s, ki)
        if string.sub(s, i, i) ~= ":" then return nil, i, false end
        local v, ni, ok = JSON._parse_value(s, i + 1)
        if not ok then return nil, ni, false end
        obj[key] = v
        i = JSON._skip_ws(s, ni)
        local c = string.sub(s, i, i)
        if c == "," then
            i = i + 1
        elseif c == "}" then
            return obj, i + 1, true
        else
            return nil, i, false
        end
    end
end

-- JSON `null` has no Lua value of its own; `nil` is what this datamodel
-- reads as absent, and the third return is what tells a caller the parse
-- succeeded rather than refused.
function JSON._parse_value(s, i)
    i = JSON._skip_ws(s, i)
    if i > #s then return nil, i, false end
    local c = string.sub(s, i, i)
    if c == '"' then return JSON._parse_string(s, i) end
    if c == "{" then return JSON._parse_object(s, i) end
    if c == "[" then return JSON._parse_array(s, i) end
    if string.sub(s, i, i + 3) == "true" then return true, i + 4, true end
    if string.sub(s, i, i + 4) == "false" then return false, i + 5, true end
    if string.sub(s, i, i + 3) == "null" then return nil, i + 4, true end
    return JSON._parse_number(s, i)
end

-- The whole-document read, and the only place that answers "was this JSON?"
-- separately from "what was it?". JSON `null` and a refusal are both `nil`,
-- and the event-payload decoders have to tell them apart: one binds
-- `_event.data` to the datamodel's absent value, the other falls through to
-- the data model's string reading of an arriving payload.
function JSON._parse_document(s)
    if type(s) ~= "string" or s == "" then return nil, false end
    local value, i, ok = JSON._parse_value(s, 1)
    if not ok then return nil, false end
    i = JSON._skip_ws(s, i)
    -- RFC 8259: one value, and nothing after it. Trailing text is how
    -- `2 + 3` used to get in.
    if i <= #s then return nil, false end
    return value, true
end

-- ECMA-262 15.12.2 shape: ONE value, `nil` when the text is not JSON.
--
-- ⚠ One value on purpose. Returning the `ok` flag from here too changed what
-- `JSON.parse(x)` means in an expression: a `return JSON.parse(x)` in tail
-- position propagates every return, and the Python backend's lupa bridge
-- surfaced that as a tuple — measured, by the ECMA-262 semantics table, which
-- is the only place in this repository that evaluates the call bare.
function JSON.parse(s)
    local value = JSON._parse_document(s)
    return value
end

