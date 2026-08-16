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
-- one native implementation per engine -- is six chances for the semantics to
-- drift, and the semantics are the whole point. Do NOT reimplement these in an
-- engine.
--
-- That sentence used to carry an exception: `_scxml_truthy`, `_typeof`,
-- `_isArray`, `_indexOf`, `_concat`, `parseInt` and `parseFloat` were written
-- once per engine, in the engine's own language. The drift arrived exactly
-- where it was predicted to. Measured 2026-08-16, the day the shared table
-- gained a reader on every Lua backend: Go answered ten of eighty-four cases
-- differently -- its `_indexOf` and `_concat` had no Array implementation at
-- all, returning -1 and "" for every array a document passed them -- Python
-- called `typeof [1,2,3]` "function", and Rust dropped `indexOf`'s second
-- argument. Every one of those backends was green on the W3C suite
-- throughout, because no fixture in it asks `[1,2,3].indexOf(2)`.
--
-- So they live here now, and an engine's remaining job is to load this file.
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

-- ECMA-262 7.1.2 ToBoolean -- the `!` operator, every `cond=` attribute, and
-- the guard the emitter wraps around `&&` and `||`.
--
-- Lua's only falsy values are `nil` and `false`, so this is the whole reason
-- `cond="Var1"` cannot be emitted as `if Var1 then`. NaN is the case a native
-- implementation keeps getting wrong: `n ~= 0` calls NaN true, because NaN
-- compares equal to nothing, including zero.
function _scxml_truthy(v)
    if v == nil or v == false then return false end
    -- An engine that has to keep `null` distinct from absence INSIDE an array
    -- binds it to a sentinel (C++ a lightuserdata, Kotlin a table), because
    -- a Lua sequence with a nil in it has no length. Both sentinels are
    -- falsy. Where an engine defines neither, `_NULL` is itself nil and these
    -- compare false against every value that reaches here.
    if rawequal(v, _NULL) or rawequal(v, _UNDEFINED) then return false end
    local t = type(v)
    if t == "number" then return v == v and v ~= 0 end
    if t == "string" then return v ~= "" end
    return true
end

-- ECMA-262 13.5.3 the `typeof` operator.
--
-- Every value this datamodel can hold, named the way ECMAScript names it.
-- `nil` answers "undefined" rather than "object": SCE binds both `null` and
-- `undefined` to Lua's one empty value, and an unassigned `<data>` is the
-- reachable case, so "undefined" is the answer a document is asking for.
--
-- `type(v)` is what makes this correct where a native was not: Lua's C-level
-- `lua_isnumber` answers true for a STRING that parses as a number, so an
-- engine that asks it before asking about strings calls a text payload a
-- number -- and `typeof` is precisely how a document tells those apart.
function _typeof(v)
    if v == nil then return "undefined" end
    -- The sentinels `_scxml_truthy` describes: `typeof null` is "object" and
    -- `typeof undefined` is "undefined", which is the one place the two
    -- values SCE otherwise collapses can still be told apart.
    if rawequal(v, _NULL) then return "object" end
    if rawequal(v, _UNDEFINED) then return "undefined" end
    local t = type(v)
    if t == "boolean" then return "boolean" end
    if t == "number" then return "number" end
    if t == "string" then return "string" end
    if t == "function" then return "function" end
    return "object"
end

-- ECMA-262 13.10.2 `x instanceof Array`, and the array/object question every
-- helper below has to answer.
--
-- SCE stores an ECMAScript Array as a 1-based Lua sequence and an Object as a
-- keyed table, so the two differ only in their keys -- and an EMPTY array and
-- an EMPTY object have no keys at all. A table with no keys answers true,
-- because `[]` is the value a document constructs and `{}` is the one it does
-- not ask this question about; the ambiguity itself is a limit of the
-- representation rather than a choice.
function _isArray(v)
    if type(v) ~= "table" then return false end
    if #v > 0 then return true end
    return next(v) == nil
end

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

-- W3C SCXML B.2 binds `datamodel="ecmascript"` to ECMAScript 3rd edition, and
-- 3rd edition is a language AND a standard library. Everything above this line
-- is the language; everything below is the part of the library an ordinary
-- SCXML document reaches for — a bound on a counter, a slice of an event's
-- payload, the pieces of a delimited string.
--
-- Lua's own string library is not a substitute for it. The names differ
-- (`sub` vs `substring`), the indices are 1-based and inclusive where
-- ECMAScript's are 0-based and half-open, and `"abc".charAt(1)` is not even
-- valid Lua syntax. So the frontend emits these as plain calls with the
-- receiver first, the way `_indexOf` already works, and they live HERE rather
-- than as another native per engine: `_typeof` and `_isArray` are six
-- implementations of one meaning, and that is the shape this file exists to
-- avoid.
--
-- `string.find` is called with an explicit `true` for plain matching
-- everywhere below, and the explicitness is load-bearing: go-lua reads a
-- MISSING fourth argument as plain and a present-but-false one as a pattern it
-- does not implement — where it asserts rather than returns — which is the
-- reverse of Lua 5.4's default. Naming it is what makes the two agree. The
-- pattern-matching functions are not available at all there (`gsub`, `gmatch`
-- and `match` are commented out of go-lua's string library), so nothing below
-- may use them.

-- ECMA-262 15.5.4.15 String.prototype.substring.
--
-- Both indices are clamped to the string, and a start after the end is
-- SWAPPED rather than empty — that is the clause, not a convenience.
function _scxml_substring(s, from, to)
    s = _scxml_tostring(s)
    local len = #s
    local a = (from == nil) and 0 or _scxml_tonumber(from)
    local b = (to == nil) and len or _scxml_tonumber(to)
    if a ~= a then a = 0 end
    if b ~= b then b = 0 end
    a = math.floor(a)
    b = math.floor(b)
    if a < 0 then a = 0 elseif a > len then a = len end
    if b < 0 then b = 0 elseif b > len then b = len end
    if a > b then a, b = b, a end
    return string.sub(s, a + 1, b)
end

-- ECMA-262 15.5.4.4 String.prototype.charAt: the empty string when the
-- position is outside the string, never nil.
function _scxml_charat(s, position)
    s = _scxml_tostring(s)
    local i = (position == nil) and 0 or _scxml_tonumber(position)
    if i ~= i then i = 0 end
    i = math.floor(i)
    if i < 0 or i >= #s then return "" end
    return string.sub(s, i + 1, i + 1)
end

-- ECMA-262 15.5.4.16 / 15.5.4.17.
function _scxml_tolowercase(s)
    return string.lower(_scxml_tostring(s))
end

function _scxml_touppercase(s)
    return string.upper(_scxml_tostring(s))
end

-- ECMA-262 15.5.4.14 String.prototype.split, for the separators this
-- datamodel can express.
--
-- A RegExp separator is not among them: the accepted subset has no regular
-- expression literal and no `RegExp`, so a separator here is a string or
-- absent. The three clause behaviours that are reachable are all implemented —
-- an absent separator yields the whole string as one element, an empty one
-- yields the characters, and `limit` truncates.
function _scxml_split(s, separator, limit)
    s = _scxml_tostring(s)
    local out = {}
    local lim = (limit == nil) and -1 or math.floor(_scxml_tonumber(limit))
    if lim == 0 then return out end
    if separator == nil then
        out[1] = s
        return out
    end
    separator = _scxml_tostring(separator)
    if separator == "" then
        for i = 1, #s do
            if lim >= 0 and #out >= lim then return out end
            out[#out + 1] = string.sub(s, i, i)
        end
        return out
    end
    local pos = 1
    while true do
        local a, b = string.find(s, separator, pos, true)
        if a == nil then break end
        if lim >= 0 and #out >= lim then return out end
        out[#out + 1] = string.sub(s, pos, a - 1)
        pos = b + 1
    end
    if lim >= 0 and #out >= lim then return out end
    out[#out + 1] = string.sub(s, pos)
    return out
end

-- ECMA-262 15.4.4.14 Array.prototype.indexOf / 15.5.4.7
-- String.prototype.indexOf, which the emitter spells as one receiver-first
-- call because it cannot always know which of the two it is looking at.
--
-- Two clauses do the work a native version kept dropping. The search starts
-- at `from`, so a caller walking occurrences is not handed the one it has
-- already seen; and an Array compares with `===`, so `[1,2,3].indexOf('2')`
-- is -1 rather than the coercion `==` would have performed. Lua's `==` is
-- strict across types, which is what makes the second one a one-liner.
function _indexOf(subject, search, from)
    local start = (from == nil) and 0 or _scxml_tonumber(from)
    if start ~= start then start = 0 end
    start = (start >= 0) and math.floor(start) or -math.floor(-start)
    if type(subject) == "string" then
        local needle = _scxml_tostring(search)
        local len = #subject
        if start < 0 then start = 0 elseif start > len then start = len end
        -- Plain search, named explicitly: go-lua reads a MISSING fourth
        -- argument as plain and a present-but-false one as a pattern it does
        -- not implement, the reverse of Lua 5.4's default.
        local at = string.find(subject, needle, start + 1, true)
        if at == nil then return -1 end
        return at - 1
    end
    if type(subject) == "table" then
        local len = #subject
        -- A negative `from` counts back from the end (15.4.4.14 step 5).
        if start < 0 then
            start = len + start
            if start < 0 then start = 0 end
        end
        for i = start + 1, len do
            if subject[i] == search then return i - 1 end
        end
        return -1
    end
    return -1
end

-- ECMA-262 15.4.4.4 Array.prototype.concat / 15.5.4.6 String.prototype.concat.
--
-- Binary, because the emitter folds `[].concat(a, b)` into
-- `_concat(_concat({}, a), b)` -- spelling the fold out there is what keeps a
-- third argument from being silently dropped, which is what the rewriter this
-- replaced did.
--
-- The receiver is always spread and an argument is spread only when it is an
-- Array, which is the clause: `[1].concat([2,3])` is three elements and
-- `[1].concat(2)` is two. An empty table is an Array by `_isArray`, so an
-- empty Object argument spreads to nothing instead of appending itself --
-- the same representational limit documented there, at the same one value.
function _concat(subject, other)
    if type(subject) == "string" then
        return subject .. _scxml_tostring(other)
    end
    local out = {}
    if type(subject) == "table" then
        for i = 1, #subject do out[#out + 1] = subject[i] end
    elseif subject ~= nil then
        out[#out + 1] = subject
    end
    if _isArray(other) then
        for i = 1, #other do out[#out + 1] = other[i] end
    elseif other ~= nil then
        out[#out + 1] = other
    end
    return out
end

-- ECMA-262 15.1.2.2 parseInt / 15.1.2.3 parseFloat: the two global functions
-- that read a PREFIX.
--
-- That is the whole difference between `parseInt(s)` and `Number(s)`, and it
-- is what an engine delegating to its host's string-to-number routine loses:
-- a strict parser refuses "42abc" and answers 0, a number the document cannot
-- tell from a real one. Written out here rather than delegated for that
-- reason.
-- The value of one digit in `base`, or nil for anything that is not one --
-- including the empty string a scan past the end returns.
--
-- Global for the same reason everything else here is: the C11 backend loads
-- this file as several independent chunks, and a file-local would not survive
-- the split.
function _scxml_digit(c, base)
    if c == "" then return nil end
    local b = string.byte(c)
    local v
    if b >= 48 and b <= 57 then
        v = b - 48
    elseif b >= 97 and b <= 122 then
        v = b - 87
    elseif b >= 65 and b <= 90 then
        v = b - 55
    else
        return nil
    end
    if v >= base then return nil end
    return v
end

-- Leading whitespace, per 15.1.2.2 step 2 (StrWhiteSpaceChar).
function _scxml_skip_space(s, i)
    while i <= #s do
        local c = string.sub(s, i, i)
        if c ~= " " and c ~= "\t" and c ~= "\n" and c ~= "\r" and c ~= "\f" and c ~= "\v" then
            break
        end
        i = i + 1
    end
    return i
end

function parseInt(value, radix)
    local s = _scxml_tostring(value)
    local i = _scxml_skip_space(s, 1)
    local sign = 1
    local c = string.sub(s, i, i)
    if c == "-" then
        sign = -1
        i = i + 1
    elseif c == "+" then
        i = i + 1
    end
    local base = (radix == nil) and 0 or _scxml_tonumber(radix)
    if base ~= base then base = 0 end
    base = math.floor(base)
    -- `0x` picks hexadecimal when no radix was named, and is accepted (not
    -- required) when 16 was.
    if base == 0 or base == 16 then
        if string.lower(string.sub(s, i, i + 1)) == "0x" then
            base = 16
            i = i + 2
        elseif base == 0 then
            base = 10
        end
    end
    if base < 2 or base > 36 then return 0 / 0 end
    local n, digits = 0, 0
    while i <= #s do
        local v = _scxml_digit(string.sub(s, i, i), base)
        if v == nil then break end
        n = n * base + v
        digits = digits + 1
        i = i + 1
    end
    -- No digit at all is NaN, not zero: "the string is not a number" and "the
    -- string is zero" are different answers and a document can branch on it.
    if digits == 0 then return 0 / 0 end
    return sign * n
end

function parseFloat(value)
    local s = _scxml_tostring(value)
    local i = _scxml_skip_space(s, 1)
    local sign = 1
    local c = string.sub(s, i, i)
    if c == "-" then
        sign = -1
        i = i + 1
    elseif c == "+" then
        i = i + 1
    end
    -- StrDecimalLiteral admits `Infinity`, and it is the one member of the
    -- grammar that is a word rather than digits.
    if string.sub(s, i, i + 7) == "Infinity" then
        return sign * math.huge
    end
    local from = i
    local digits = 0
    while _scxml_digit(string.sub(s, i, i), 10) ~= nil do
        i = i + 1
        digits = digits + 1
    end
    if string.sub(s, i, i) == "." then
        i = i + 1
        while _scxml_digit(string.sub(s, i, i), 10) ~= nil do
            i = i + 1
            digits = digits + 1
        end
    end
    if digits == 0 then return 0 / 0 end
    -- An exponent counts only when it is complete: "1e" is the number 1
    -- followed by text, not a malformed number (15.1.2.3 reads the longest
    -- prefix that satisfies StrDecimalLiteral).
    local mantissa = i
    if string.lower(string.sub(s, i, i)) == "e" then
        i = i + 1
        local c2 = string.sub(s, i, i)
        if c2 == "-" or c2 == "+" then i = i + 1 end
        local exponent = 0
        while _scxml_digit(string.sub(s, i, i), 10) ~= nil do
            i = i + 1
            exponent = exponent + 1
        end
        if exponent == 0 then i = mantissa end
    end
    -- The sign is applied here rather than handed to `tonumber`: whether a
    -- leading `+` is accepted is an interpreter detail, and go-lua and Lua 5.4
    -- do not have to agree on it.
    local n = tonumber(string.sub(s, from, i - 1))
    if n == nil then return 0 / 0 end
    return sign * n
end

-- ECMA-262 15.8.2.15 Math.round.
--
-- Lua has no rounding function, and the habit that fills the gap
-- (`math.floor(x + 0.5)`) is right only because the clause sends a half toward
-- +Infinity — `Math.round(-2.5)` is -2, not -3, which is what a "round half
-- away from zero" implementation would answer. NaN and the infinities are
-- handed back rather than floored, since `math.floor` has no answer for them.
function _scxml_round(v)
    local n = _scxml_tonumber(v)
    if n ~= n or n == math.huge or n == -math.huge then return n end
    return math.floor(n + 0.5)
end

-- ECMA-262 15.5.4.11 String.prototype.replace, for the searchValue this
-- datamodel can express.
--
-- A RegExp is not among them (the accepted subset has no regular expression
-- literal), so the search is a string — and a string searchValue replaces the
-- FIRST match and only that one. That is the clause and it is the half a
-- `gsub`-shaped implementation gets wrong; `gsub` is also absent from go-lua,
-- so the plain-search spelling is what every backend can run.
function _scxml_replace(s, search, replacement)
    s = _scxml_tostring(s)
    local needle = _scxml_tostring(search)
    local rep = _scxml_tostring(replacement)
    if needle == "" then return rep .. s end
    local a, b = string.find(s, needle, 1, true)
    if a == nil then return s end
    return string.sub(s, 1, a - 1) .. rep .. string.sub(s, b + 1)
end

-- ECMA-262 15.5.4.13 String.prototype.slice / 15.4.4.10 Array.prototype.slice.
--
-- One helper for both, because the emitter cannot tell which receiver it has —
-- the same reason `_indexOf` takes both. A negative index counts from the end
-- and an out-of-range one clamps, which is what separates `slice` from
-- `substring`: that one SWAPS a reversed pair, this one yields nothing.
function _scxml_slice(subject, from, to)
    local is_text = (type(subject) == "string")
    if not is_text and type(subject) ~= "table" then return subject end
    local len = #subject
    local a = (from == nil) and 0 or _scxml_tonumber(from)
    if a ~= a then a = 0 end
    a = (a >= 0) and math.floor(a) or -math.floor(-a)
    if a < 0 then a = len + a end
    if a < 0 then a = 0 elseif a > len then a = len end
    local b = (to == nil) and len or _scxml_tonumber(to)
    if b ~= b then b = 0 end
    b = (b >= 0) and math.floor(b) or -math.floor(-b)
    if b < 0 then b = len + b end
    if b < 0 then b = 0 elseif b > len then b = len end
    if is_text then
        if b <= a then return "" end
        return string.sub(subject, a + 1, b)
    end
    local out = {}
    for i = a + 1, b do out[#out + 1] = subject[i] end
    return out
end

-- ECMA-262 15.4.4.11 Array.prototype.sort.
--
-- The default comparator compares ToString of the elements, so `[10, 9]` sorts
-- to `[10, 9]` and not `[9, 10]`. That surprises everyone once and is the
-- clause; an implementation that sorts numerically is answering a different
-- language. Sorting is IN PLACE and the array itself comes back, which is what
-- lets `xs.sort()` and `xs` be the same value afterwards.
function _scxml_sort(arr, comparator)
    if type(arr) ~= "table" then return arr end
    if comparator == nil then
        table.sort(arr, function(x, y) return _scxml_tostring(x) < _scxml_tostring(y) end)
    else
        table.sort(arr, function(x, y) return _scxml_tonumber(comparator(x, y)) < 0 end)
    end
    return arr
end

-- ECMA-262 15.4.4.8 Array.prototype.reverse — in place, and the array itself
-- is the result.
function _scxml_reverse(arr)
    if type(arr) ~= "table" then return arr end
    local i, j = 1, #arr
    while i < j do
        arr[i], arr[j] = arr[j], arr[i]
        i = i + 1
        j = j - 1
    end
    return arr
end

-- W3C SCXML 6.2: a `<send>`'s `<param>` list, as the object `_event.data` is.
--
-- A name may repeat — W3C's own test178 sends `<param name="1" expr="2"/>`
-- twice with different values and requires BOTH pairs to be delivered — and an
-- object cannot hold one name twice, so a repeated name becomes an Array of
-- its values in document order. A single name is the value itself, because
-- wrapping every param in a one-element Array would change what every existing
-- document reads.
--
-- This exists because the emitted table literal cannot express it. Backends
-- built `{["n"]=7, ["d"]=1, ["d"]=2}`, where Lua's last key wins and the first
-- value is simply gone — silently, on five backends; the sixth kept both and
-- lost their types instead (`EventDataHelper::mergeParams` publishes the
-- STRING vector for a repeated name). test178 is a MANUAL test in the IRP, so
-- no channel had ever asked. Measured 2026-08-16 by the third phase of
-- `integration_resources/send_param_payload`.
--
-- Taking the pairs as arguments rather than a table is what makes the rule
-- expressible at all: a table constructor has already collapsed the duplicate
-- by the time anything could look at it.
function _scxml_params(...)
    local out = {}
    local counts = {}
    for i = 1, select("#", ...) do
        local pair = select(i, ...)
        local name, value = pair[1], pair[2]
        local seen = counts[name]
        if seen == nil then
            counts[name] = 1
            out[name] = value
        elseif seen == 1 then
            counts[name] = 2
            out[name] = {out[name], value}
        else
            counts[name] = seen + 1
            local held = out[name]
            held[#held + 1] = value
        end
    end
    return out
end

-- ECMA-262 15.2.3.14 Object.keys, the one way this datamodel can walk an
-- object whose shape arrived with the payload.
--
-- Sorted, which the clause does not ask for and this file has to decide
-- anyway: ECMAScript gives the order of a `for-in` enumeration, and a Lua
-- table has no such order to give -- `pairs` returns whatever the hash layout
-- produces, which differs between interpreters and between two runs of one.
-- So the choice is a normal form or an answer that cannot be relied on, and
-- five backends previously made it five ways. The sixth, Go, did not define
-- `Object` at all: a document calling `Object.keys` reached a nil there and
-- nowhere else.
Object = Object or {}
function Object.keys(t)
    if type(t) ~= "table" then return {} end
    local keys = {}
    for k in pairs(t) do keys[#keys + 1] = _scxml_tostring(k) end
    table.sort(keys)
    return keys
end

-- ECMA-262 15.5.1.1 / 15.7.1.1 / 15.6.1.1: the three constructors CALLED AS
-- FUNCTIONS, which is how an SCXML author converts a payload that arrived as
-- text. Each is exactly the corresponding abstract operation, so they are the
-- conversions this file already defines rather than new ones.
function String(value)
    return _scxml_tostring(value)
end

function Number(value)
    return _scxml_tonumber(value)
end

function Boolean(value)
    return _scxml_truthy(value)
end
