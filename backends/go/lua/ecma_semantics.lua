-- SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
-- SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
--
-- W3C SCXML B.2: the ECMAScript operators Lua does not share.
--
-- SCE runs the ECMAScript datamodel on a Lua interpreter, so `sce-build`'s
-- ECMAScript frontend (sce-build/src/ecmascript/) emits Lua. Most operators
-- map straight across; these do not, because ECMAScript coerces where Lua
-- either refuses or answers differently:
--
--   `+`         concatenates when either side is a string, adds otherwise
--   `==` `!=`   compare across types after coercion
--   `& | ^ ~`   operate on ToInt32 of their operands, not on integers
--   `<< >> >>>` likewise, and `>>>` is unsigned where `>>` is not
--
-- Single Source of Truth: every backend loads THIS file. The alternative --
-- one native implementation per engine, as `_scxml_truthy` and `_typeof`
-- have -- is six chances for the semantics to drift, and the semantics are
-- the whole point. Do NOT reimplement these in an engine.
--
-- Lua 5.2 compatible on purpose: the Go backend embeds go-lua (5.2), which
-- has no `//` operator, no integer subtype and no bitwise operators at all.
-- The bit helpers below are arithmetic for that reason, not for want of
-- `&`.
--
-- C++:    #include via CMake-generated string literal
-- Rust:   include_str!() at compile time
-- Go:     //go:embed of the copy in backends/go/lua/
-- Kotlin: resource copied by the Gradle build
-- Python: read from the repository path
-- C11:    emitted into the generated engine bootstrap

-- ECMA-262 7.1.4 ToNumber, over the value domain the SCXML datamodel
-- carries. `nil` is ECMAScript `undefined` here: SCE binds both `_NULL` and
-- `_UNDEFINED` to Lua's one empty value, so the two cannot be told apart
-- downstream, and `undefined` is what an unassigned `<data>` reads as.
function _scxml_tonumber(v)
    local t = type(v)
    if t == "number" then return v end
    if t == "boolean" then return v and 1 or 0 end
    if t == "nil" then return 0 / 0 end
    if t == "string" then
        -- ECMA-262 7.1.4.1: a string that is empty or all whitespace is 0,
        -- and anything else that does not parse is NaN. Lua's `tonumber`
        -- already ignores surrounding whitespace, so the scan below only has
        -- to tell "blank" from "not a number".
        --
        -- It is a byte loop rather than `v:match("^%s*(.-)%s*$")` because
        -- go-lua ships neither `string.match` nor `string.gsub` — measured,
        -- not assumed: the pattern version raised "attempt to call local 's'
        -- (a nil value)" on every `==` between a string and a number in the
        -- Go backend, and Lua 5.4 accepted it happily.
        local blank = true
        for i = 1, #v do
            local c = string.sub(v, i, i)
            if c ~= " " and c ~= "\t" and c ~= "\n" and c ~= "\r" then
                blank = false
                break
            end
        end
        if blank then return 0 end
        local n = tonumber(v)
        if n == nil then return 0 / 0 end
        return n
    end
    return 0 / 0
end

-- ECMA-262 7.1.17 ToString. Lua 5.4 renders an integral float as "1.0" and
-- go-lua renders every number as a float, so `1 + ''` would answer "1.0" on
-- one engine and "1" on another; ECMAScript says "1" on both.
function _scxml_tostring(v)
    local t = type(v)
    if t == "string" then return v end
    if t == "nil" then return "undefined" end
    if t == "boolean" then return v and "true" or "false" end
    if t == "number" then
        if v ~= v then return "NaN" end
        if v == math.huge then return "Infinity" end
        if v == -math.huge then return "-Infinity" end
        if v == math.floor(v) and math.abs(v) < 1e15 then
            return string.format("%d", math.floor(v))
        end
        return tostring(v)
    end
    if t == "table" then
        -- ECMA-262 7.1.1 ToPrimitive: an Array joins with commas, any other
        -- object is "[object Object]".
        if _isArray(v) then
            local parts = {}
            for i = 1, #v do
                local e = v[i]
                parts[i] = (e == nil) and "" or _scxml_tostring(e)
            end
            return table.concat(parts, ",")
        end
        return "[object Object]"
    end
    return tostring(v)
end

-- ECMA-262 7.2.15 IsLooselyEqual -- the `==` operator.
function _scxml_eq(a, b)
    local ta, tb = type(a), type(b)
    if ta == tb then
        if ta == "number" and a ~= a then return false end  -- NaN
        return a == b
    end
    -- null == undefined, and neither equals anything else. Both are `nil`
    -- here, so this arm is reached only when exactly one side is empty.
    if a == nil or b == nil then return false end
    if ta == "boolean" then return _scxml_eq(a and 1 or 0, b) end
    if tb == "boolean" then return _scxml_eq(a, b and 1 or 0) end
    if ta == "number" and tb == "string" then
        local n = _scxml_tonumber(b)
        return n == n and a == n
    end
    if ta == "string" and tb == "number" then
        local n = _scxml_tonumber(a)
        return n == n and n == b
    end
    if ta == "table" then return _scxml_eq(_scxml_tostring(a), b) end
    if tb == "table" then return _scxml_eq(a, _scxml_tostring(b)) end
    return false
end

-- ECMA-262 13.15.3 ApplyStringOrNumericBinaryOperator for `+`.
function _scxml_add(a, b)
    local pa = (type(a) == "table") and _scxml_tostring(a) or a
    local pb = (type(b) == "table") and _scxml_tostring(b) or b
    if type(pa) == "string" or type(pb) == "string" then
        return _scxml_tostring(pa) .. _scxml_tostring(pb)
    end
    return _scxml_tonumber(pa) + _scxml_tonumber(pb)
end

-- ECMA-262 13.3.3 computed member access.
--
-- SCE stores an ECMAScript Array as a 1-based Lua sequence — that is what
-- `_isArray`, `_concat`, `#` and the JSON decoder all agree on — so a
-- numeric index has to move by one, while a string key addresses an object
-- property and must not. Which of the two a computed index is cannot be
-- decided at codegen time; a literal one can, and the emitter does it there.
function _scxml_index(obj, key)
    if type(key) == "number" and _isArray(obj) then
        return obj[key + 1]
    end
    return obj[key]
end

-- ECMA-262 7.1.6 ToInt32 / 7.1.7 ToUint32.
--
-- Global rather than `local` so each definition in this file is an
-- independent chunk. The C11 backend cannot load a file at runtime and
-- embeds this source in the generated translation unit; ISO C99 guarantees
-- only 4095 characters in a string literal (and GCC's `-Woverlength-strings`
-- counts adjacent literals *after* concatenation), so the embed splits the
-- source into several `luaL_dostring` calls. A file-local would not survive
-- that split.
function _scxml_touint32(v)
    local n = _scxml_tonumber(v)
    if n ~= n or n == math.huge or n == -math.huge then return 0 end
    n = (n >= 0) and math.floor(n) or -math.floor(-n)
    n = n % 4294967296
    if n < 0 then n = n + 4294967296 end
    return n
end

function _scxml_toint32(v)
    local n = _scxml_touint32(v)
    if n >= 2147483648 then n = n - 4294967296 end
    return n
end

-- Bitwise over ToUint32 operands, by arithmetic so go-lua (5.2, no bitwise
-- operators) answers what Lua 5.4 answers. `op` picks the per-bit rule.
function _scxml_bitwise(a, b, op)
    local x, y = _scxml_touint32(a), _scxml_touint32(b)
    local result, bit = 0, 1
    for _ = 1, 32 do
        local xb, yb = x % 2, y % 2
        local rb
        if op == "and" then
            rb = (xb == 1 and yb == 1) and 1 or 0
        elseif op == "or" then
            rb = (xb == 1 or yb == 1) and 1 or 0
        else
            rb = (xb ~= yb) and 1 or 0
        end
        result = result + rb * bit
        x, y, bit = (x - xb) / 2, (y - yb) / 2, bit * 2
    end
    if result >= 2147483648 then result = result - 4294967296 end
    return result
end

function _scxml_bitand(a, b) return _scxml_bitwise(a, b, "and") end
function _scxml_bitor(a, b) return _scxml_bitwise(a, b, "or") end
function _scxml_bitxor(a, b) return _scxml_bitwise(a, b, "xor") end
function _scxml_bitnot(a) return -_scxml_toint32(a) - 1 end

function _scxml_shl(a, b)
    local n = _scxml_touint32(a) * (2 ^ (_scxml_touint32(b) % 32))
    n = n % 4294967296
    if n >= 2147483648 then n = n - 4294967296 end
    return n
end

-- `>>` keeps the sign, `>>>` does not.
function _scxml_shr(a, b)
    local n, shift = _scxml_toint32(a), _scxml_touint32(b) % 32
    if n < 0 then
        return -math.floor(-n / (2 ^ shift)) - ((n % (2 ^ shift) ~= 0) and 1 or 0)
    end
    return math.floor(n / (2 ^ shift))
end

function _scxml_ushr(a, b)
    return math.floor(_scxml_touint32(a) / (2 ^ (_scxml_touint32(b) % 32)))
end
