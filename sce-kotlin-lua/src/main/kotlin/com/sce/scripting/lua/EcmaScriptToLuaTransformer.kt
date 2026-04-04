// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Lua — ECMAScript to Lua expression transformer
//
// 1:1 port of C++ EcmaScriptToLuaTransformer.cpp.
// Covers all 202 W3C SCXML test expression patterns.

package com.sce.scripting.lua

/**
 * Transforms ECMAScript expressions to Lua equivalents.
 *
 * Pattern-based transformer for the subset of ECMAScript used in W3C SCXML.
 * Applied inside [LuaScriptEngine] before passing expressions to Lua.
 *
 * C++ parity: sce/src/scripting/EcmaScriptToLuaTransformer.cpp
 */
class EcmaScriptToLuaTransformer {

    enum class ExpressionContext { General, Guard }

    private val generalCache = lruCache()
    private val guardCache = lruCache()
    private val scriptCache = lruCache()

    fun clearCache() {
        generalCache.clear()
        guardCache.clear()
        scriptCache.clear()
    }

    fun transform(ecmaScript: String, context: ExpressionContext = ExpressionContext.General): String {
        if (ecmaScript.isEmpty()) return ecmaScript

        val cache = if (context == ExpressionContext.Guard) guardCache else generalCache
        cache[ecmaScript]?.let { return it }

        // Pre-pass: typeof before string protection
        var preProcessed = transformTypeofPatterns(ecmaScript)

        // Stage 1: Protect string literals
        val (processed, literals) = protectStringLiterals(preProcessed)

        // Stage 2: Transformation pipeline (order matters)
        var result = processed
        result = transformCompoundAssignment(result)
        result = transformIncrementDecrement(result)
        result = transformFunctionSyntax(result)
        result = transformInstanceofPatterns(result)
        result = transformArrayLiterals(result)
        result = transformNullUndefined(result)
        result = transformNewExpression(result)
        result = transformArrayMethods(result)
        result = transformArrayIndexing(result)
        result = transformObjectLiterals(result)
        result = transformTernaryOperator(result)
        result = transformOperators(result)
        result = transformStringConcat(result)
        result = transformDOMMethods(result)
        result = transformVarDeclarations(result)
        result = transformSemicolons(result)

        // Stage 3: Restore string literals
        result = restoreStringLiterals(result, literals)

        // Stage 4: Guard truthiness
        if (context == ExpressionContext.Guard) {
            result = wrapTruthiness(result)
        }

        cache[ecmaScript] = result
        return result
    }

    fun transformScript(script: String): String {
        if (script.isEmpty()) return script
        scriptCache[script]?.let { return it }

        var preProcessed = transformTypeofPatterns(script)
        val (processed, literals) = protectStringLiterals(preProcessed)

        var result = processed
        result = transformForLoops(result)
        result = transformCompoundAssignment(result)
        result = transformIncrementDecrement(result)
        result = transformFunctionSyntax(result)
        result = transformInstanceofPatterns(result)
        result = transformArrayLiterals(result)
        result = transformNullUndefined(result)
        result = transformNewExpression(result)
        result = transformArrayMethods(result)
        result = transformArrayIndexing(result)
        result = transformObjectLiterals(result)
        result = transformTernaryOperator(result)
        result = transformOperators(result)
        result = transformStringConcat(result)
        result = transformDOMMethods(result)
        result = transformVarDeclarations(result)
        result = transformSemicolons(result)
        result = transformConditionalBlocks(result)
        result = transformBareExpressions(result)

        result = restoreStringLiterals(result, literals)
        scriptCache[script] = result
        return result
    }

    // === Helpers ===

    private fun Char.isWordChar(): Boolean = isLetterOrDigit() || this == '_'

    private fun findWord(s: String, word: String, startPos: Int = 0): Int {
        var pos = startPos
        while (true) {
            pos = s.indexOf(word, pos)
            if (pos == -1) return -1
            val leftOk = pos == 0 || !s[pos - 1].isWordChar()
            val rightOk = pos + word.length >= s.length || !s[pos + word.length].isWordChar()
            if (leftOk && rightOk) return pos
            pos++
        }
    }

    private fun replaceWord(s: String, word: String, replacement: String): String {
        val sb = StringBuilder(s.length)
        var lastPos = 0
        var pos = 0
        while (true) {
            pos = findWord(s, word, pos)
            if (pos == -1) break
            sb.append(s, lastPos, pos)
            sb.append(replacement)
            lastPos = pos + word.length
            pos = lastPos
        }
        sb.append(s, lastPos, s.length)
        return sb.toString()
    }

    private fun replaceWordAtTopLevel(s: String, word: String, replacement: String): String {
        val sb = StringBuilder(s.length)
        var depth = 0
        var i = 0
        while (i < s.length) {
            val c = s[i]
            if (c == '{' || c == '(' || c == '[') { depth++; sb.append(c); i++; continue }
            if (c == '}' || c == ')' || c == ']') { if (depth > 0) depth--; sb.append(c); i++; continue }
            if (depth == 0 && i + word.length <= s.length &&
                s.regionMatches(i, word, 0, word.length) &&
                (i == 0 || !s[i - 1].isWordChar()) &&
                (i + word.length >= s.length || !s[i + word.length].isWordChar())) {
                sb.append(replacement)
                i += word.length
            } else {
                sb.append(c)
                i++
            }
        }
        return sb.toString()
    }

    private fun replaceKeywordPrefix(s: String, keyword: String, replacement: String): String {
        val sb = StringBuilder(s.length)
        var lastPos = 0
        var pos = 0
        while (true) {
            pos = findWord(s, keyword, pos)
            if (pos == -1) break
            val afterKw = pos + keyword.length
            if (afterKw < s.length && s[afterKw].isWhitespace()) {
                sb.append(s, lastPos, pos)
                sb.append(replacement)
                var spaceEnd = afterKw
                while (spaceEnd < s.length && s[spaceEnd].isWhitespace()) spaceEnd++
                lastPos = spaceEnd
                pos = lastPos
            } else {
                pos++
            }
        }
        sb.append(s, lastPos, s.length)
        return sb.toString()
    }

    private fun skipSpaces(s: String, pos: Int): Int {
        var p = pos
        while (p < s.length && s[p].isWhitespace()) p++
        return p
    }

    private fun readWord(s: String, pos: Int): Int {
        var p = pos
        while (p < s.length && s[p].isWordChar()) p++
        return p
    }

    private fun String.trimBoth(): String {
        val start = indexOfFirst { it != ' ' && it != '\t' }
        val end = indexOfLast { it != ' ' && it != '\t' }
        return if (start == -1) "" else substring(start, end + 1)
    }

    private fun findMatchingClose(s: String, openPos: Int, open: Char, close: Char): Int {
        var depth = 1
        var pos = openPos + 1
        while (pos < s.length && depth > 0) {
            if (s[pos] == open) depth++
            else if (s[pos] == close) depth--
            if (depth > 0) pos++
        }
        return if (depth == 0) pos else (if (s.isEmpty()) 0 else s.length - 1)
    }

    private fun findPrecedingIdentifier(s: String, dotPos: Int): Int {
        if (dotPos == 0) return -1
        var start = dotPos
        while (start > 0 && s[start - 1].isWordChar()) start--
        return if (start < dotPos) start else -1
    }

    private fun isPureInPredicate(expr: String): Boolean {
        var i = 0
        var foundIn = false
        while (i < expr.length) {
            val c = expr[i]
            if (c.isWhitespace() || c == '(' || c == ')') { i++; continue }
            if (i + 3 <= expr.length && expr.regionMatches(i, "In(", 0, 3)) {
                foundIn = true; i += 3
                while (i < expr.length && expr[i] != ')') i++
                if (i < expr.length) i++
                continue
            }
            if (i + 3 <= expr.length && expr.regionMatches(i, "and", 0, 3) &&
                (i + 3 >= expr.length || !expr[i + 3].isWordChar())) { i += 3; continue }
            if (i + 2 <= expr.length && expr.regionMatches(i, "or", 0, 2) &&
                (i + 2 >= expr.length || !expr[i + 2].isWordChar())) { i += 2; continue }
            return false
        }
        return foundIn
    }

    // === Stage 1: String Literal Protection ===

    private data class ProtectedString(val processed: String, val literals: List<String>)

    private fun protectStringLiterals(input: String): ProtectedString {
        val literals = mutableListOf<String>()
        val output = StringBuilder(input.length)
        var i = 0

        while (i < input.length) {
            val c = input[i]

            // Strip block comments /* ... */
            if (c == '/' && i + 1 < input.length && input[i + 1] == '*') {
                i += 2
                while (i + 1 < input.length && !(input[i] == '*' && input[i + 1] == '/')) i++
                if (i + 1 < input.length) i++
                i++; continue
            }

            // Strip line comments // ...
            if (c == '/' && i + 1 < input.length && input[i + 1] == '/') {
                i += 2
                while (i < input.length && input[i] != '\n') i++
                if (i < input.length) output.append('\n')
                i++; continue
            }

            if (c == '\'' || c == '"') {
                val quote = c
                val literal = StringBuilder()
                literal.append(c)
                i++
                while (i < input.length) {
                    if (input[i] == '\\' && i + 1 < input.length) {
                        literal.append(input[i]); literal.append(input[i + 1])
                        i += 2; continue
                    }
                    if (input[i] == quote) { literal.append(input[i]); break }
                    literal.append(input[i]); i++
                }
                val idx = literals.size
                literals.add(literal.toString())
                output.append("\u0001STR${idx}\u0001")
                i++
            } else {
                output.append(c)
                i++
            }
        }

        return ProtectedString(output.toString(), literals)
    }

    private fun restoreStringLiterals(processed: String, literals: List<String>): String {
        var result = processed
        for (i in literals.indices) {
            val placeholder = "\u0001STR${i}\u0001"
            result = result.replace(placeholder, literals[i])
        }
        return result
    }

    // === Pre-pass: typeof ===

    private fun transformTypeofPatterns(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0

        while (i < input.length) {
            val typeofPos = input.indexOf("typeof", i)
            if (typeofPos == -1) { result.append(input, i, input.length); break }
            if (typeofPos > 0 && input[typeofPos - 1].isWordChar()) {
                result.append(input, i, typeofPos + 6); i = typeofPos + 6; continue
            }
            result.append(input, i, typeofPos)

            var j = skipSpaces(input, typeofPos + 6)
            val hasParen = j < input.length && input[j] == '('
            if (hasParen) j = skipSpaces(input, j + 1)

            val varStart = j
            val varEnd = readWord(input, j)
            if (varEnd == varStart) { result.append("typeof"); i = typeofPos + 6; continue }
            val varName = input.substring(varStart, varEnd)

            var afterTypeofExpr = skipSpaces(input, varEnd)
            if (hasParen && afterTypeofExpr < input.length && input[afterTypeofExpr] == ')') afterTypeofExpr++

            var k = skipSpaces(input, afterTypeofExpr)
            var opLen = 0; var isNeg = false
            if (k + 3 <= input.length && input.substring(k, k + 3) == "!==") { opLen = 3; isNeg = true }
            else if (k + 3 <= input.length && input.substring(k, k + 3) == "===") { opLen = 3 }
            else if (k + 2 <= input.length && input.substring(k, k + 2) == "!=") { opLen = 2; isNeg = true }
            else if (k + 2 <= input.length && input.substring(k, k + 2) == "==") { opLen = 2 }

            if (opLen > 0) {
                var m = skipSpaces(input, k + opLen)
                if (m < input.length && (input[m] == '\'' || input[m] == '"')) {
                    val quote = input[m]; m++
                    val typeStart = m
                    while (m < input.length && input[m] != quote) m++
                    val typeStr = input.substring(typeStart, m)
                    if (m < input.length) m++
                    val luaOp = if (isNeg) "~=" else "=="
                    if (typeStr == "undefined") {
                        result.append("$varName $luaOp nil")
                    } else {
                        val luaType = if (typeStr == "object") "table" else typeStr
                        result.append("type($varName) $luaOp '$luaType'")
                    }
                    i = m; continue
                }
            }

            result.append("_typeof($varName)")
            i = afterTypeofExpr
        }

        return result.toString()
    }

    // === Compound Assignment ===

    private fun transformCompoundAssignment(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0

        while (i < input.length) {
            val eqPos = input.indexOf('=', i)
            if (eqPos == -1 || eqPos == 0) { result.append(input, i, input.length); break }
            val prev = input[eqPos - 1]
            if (prev == '=' || prev == '!' || prev == '<' || prev == '>') {
                result.append(input, i, eqPos + 1); i = eqPos + 1; continue
            }
            if (prev != '+' && prev != '-' && prev != '*' && prev != '/' && prev != '%') {
                result.append(input, i, eqPos + 1); i = eqPos + 1; continue
            }

            val opPos = eqPos - 1
            var varEnd = opPos
            while (varEnd > i && input[varEnd - 1].isWhitespace()) varEnd--
            var varStart = varEnd
            while (varStart > i && input[varStart - 1].isWordChar()) varStart--
            if (varStart == varEnd) { result.append(input, i, eqPos + 1); i = eqPos + 1; continue }

            val varName = input.substring(varStart, varEnd)
            val op = prev
            var rhsStart = eqPos + 1
            while (rhsStart < input.length && input[rhsStart].isWhitespace()) rhsStart++
            var rhsEnd = rhsStart
            while (rhsEnd < input.length && input[rhsEnd] != ';' && input[rhsEnd] != '\n') rhsEnd++
            val rhs = input.substring(rhsStart, rhsEnd)

            result.append(input, i, varStart)
            result.append("$varName = $varName $op ($rhs)")
            i = rhsEnd
        }

        return result.toString()
    }

    // === Increment/Decrement ===

    private fun transformIncrementDecrement(input: String): String {
        val result = StringBuilder(input.length * 2)
        var i = 0

        while (i < input.length) {
            if (input[i] == '+' && i + 1 < input.length && input[i + 1] == '+') {
                var varEnd = i; var varStart = i
                while (varStart > 0 && input[varStart - 1].isWordChar()) varStart--
                if (varStart < varEnd) {
                    result.delete(result.length - (varEnd - varStart), result.length)
                    val v = input.substring(varStart, varEnd)
                    result.append("(function() local _t = $v $v = $v + 1 return _t end)()")
                    i += 2; continue
                }
                val afterOp = i + 2
                val wordEnd = readWord(input, afterOp)
                if (wordEnd > afterOp) {
                    val v = input.substring(afterOp, wordEnd)
                    result.append("(function() $v = $v + 1 return $v end)()")
                    i = wordEnd; continue
                }
            }
            if (input[i] == '-' && i + 1 < input.length && input[i + 1] == '-') {
                var varEnd = i; var varStart = i
                while (varStart > 0 && input[varStart - 1].isWordChar()) varStart--
                if (varStart < varEnd) {
                    result.delete(result.length - (varEnd - varStart), result.length)
                    val v = input.substring(varStart, varEnd)
                    result.append("(function() local _t = $v $v = $v - 1 return _t end)()")
                    i += 2; continue
                }
                val afterOp = i + 2
                val wordEnd = readWord(input, afterOp)
                if (wordEnd > afterOp) {
                    val v = input.substring(afterOp, wordEnd)
                    result.append("(function() $v = $v - 1 return $v end)()")
                    i = wordEnd; continue
                }
            }
            result.append(input[i]); i++
        }

        return result.toString()
    }

    // === instanceof ===

    private fun transformInstanceofPatterns(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0

        while (i < input.length) {
            val pos = findWord(input, "instanceof", i)
            if (pos == -1) { result.append(input, i, input.length); break }
            val afterInst = skipSpaces(input, pos + 10)
            val arrayEnd = readWord(input, afterInst)
            if (arrayEnd > afterInst && input.substring(afterInst, arrayEnd) == "Array") {
                var exprEnd = pos
                while (exprEnd > i && input[exprEnd - 1].isWhitespace()) exprEnd--
                var exprStart = exprEnd
                if (exprEnd > i && input[exprEnd - 1] == ')') {
                    var depth = 1; var k = exprEnd - 2
                    while (k > i && depth > 0) {
                        if (input[k] == ')') depth++ else if (input[k] == '(') depth--
                        if (depth > 0) k--
                    }
                    exprStart = k
                } else {
                    while (exprStart > i && input[exprStart - 1].isWordChar()) exprStart--
                }
                val expr = input.substring(exprStart, exprEnd)
                result.append(input, i, exprStart)
                result.append("_isArray($expr)")
                i = arrayEnd
            } else {
                result.append(input, i, pos + 10); i = pos + 10
            }
        }

        return result.toString()
    }

    // === null/undefined → nil ===

    private fun transformNullUndefined(input: String): String {
        return replaceWord(replaceWord(input, "undefined", "nil"), "null", "nil")
    }

    // === Operators ===

    private fun transformOperators(input: String): String {
        var result = input

        // !== → ~=
        result = result.replace("!==", "~=")
        // === → ==
        result = result.replace("===", "==")

        // != → ~= (avoid re-matching ~=)
        run {
            val sb = StringBuilder(result.length)
            var j = 0
            while (j < result.length) {
                if (result[j] == '!' && j + 1 < result.length && result[j + 1] == '=') {
                    sb.append("~="); j += 2
                } else { sb.append(result[j]); j++ }
            }
            result = sb.toString()
        }

        // && → and
        result = result.replace("&&", " and ")
        // || → or
        result = result.replace("||", " or ")

        // !expr → not expr (skip ~=)
        run {
            val sb = StringBuilder(result.length)
            var j = 0
            while (j < result.length) {
                if (result[j] == '!') {
                    if (j + 1 < result.length && result[j + 1] == '=') sb.append(result[j])
                    else sb.append("not ")
                } else { sb.append(result[j]) }
                j++
            }
            result = sb.toString()
        }

        result = parenthesizeBitwiseOperands(result)
        return result
    }

    private fun parenthesizeBitwiseOperands(input: String): String {
        if (input.none { it == '|' || it == '&' || it == '^' }) return input
        if ("==" !in input && "~=" !in input && "<=" !in input && ">=" !in input &&
            input.none { (it == '<' || it == '>') }) return input

        val sb = StringBuilder(input.length + 16)
        var start = 0
        for (i in input.indices) {
            val c = input[i]
            if (c == '|' || c == '&' || c == '^') {
                sb.append("(").append(input.substring(start, i).trimBoth()).append(") $c ")
                start = i + 1
            }
        }
        sb.append("(").append(input.substring(start).trimBoth()).append(")")
        return sb.toString()
    }

    // === Array Literals: [] → {} ===

    private fun transformArrayLiterals(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0
        while (i < input.length) {
            if (input[i] == '[') {
                var isPropertyAccess = false
                if (i > 0) {
                    val prev = input[i - 1]
                    if (prev.isLetterOrDigit() || prev == '_' || prev == ')' || prev == ']' || prev == '\u0001') {
                        isPropertyAccess = true
                    }
                }
                if (!isPropertyAccess) {
                    val closePos = findMatchingClose(input, i, '[', ']')
                    var contents = input.substring(i + 1, closePos)
                    contents = transformArrayLiterals(contents)
                    contents = replaceWordAtTopLevel(contents, "null", "_NULL")
                    contents = replaceWordAtTopLevel(contents, "undefined", "_UNDEFINED")
                    result.append("{").append(contents).append("}")
                    i = closePos + 1
                } else { result.append(input[i]); i++ }
            } else { result.append(input[i]); i++ }
        }
        return result.toString()
    }

    // === Array Indexing: arr[0] → arr[0+1] ===

    private fun transformArrayIndexing(input: String): String {
        val result = StringBuilder(input.length + 32)
        var i = 0
        while (i < input.length) {
            if (input[i] == '[') {
                var isPropertyAccess = false
                if (i > 0) {
                    val prev = input[i - 1]
                    if (prev.isLetterOrDigit() || prev == '_' || prev == ')' || prev == ']' || prev == '\u0001') {
                        isPropertyAccess = true
                    }
                }
                if (isPropertyAccess) {
                    val end = findMatchingClose(input, i, '[', ']')
                    val indexExpr = input.substring(i + 1, end).trimBoth()
                    val isIntLiteral = indexExpr.isNotEmpty() && indexExpr.length <= 9 &&
                            indexExpr.all { it.isDigit() }
                    if (isIntLiteral) {
                        result.append("[").append(indexExpr.toInt() + 1).append("]")
                        i = end + 1
                    } else { result.append(input[i]); i++ }
                } else { result.append(input[i]); i++ }
            } else { result.append(input[i]); i++ }
        }
        return result.toString()
    }

    // === Array Methods ===

    private fun transformArrayMethods(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0
        while (i < input.length) {
            if (input[i] == '.') {
                // .length
                if (i + 7 <= input.length && input.regionMatches(i + 1, "length", 0, 6) &&
                    (i + 7 >= input.length || !input[i + 7].isWordChar())) {
                    val idStart = findPrecedingIdentifier(result.toString(), result.length)
                    if (idStart >= 0) {
                        val varName = result.substring(idStart)
                        result.delete(idStart, result.length)
                        result.append("#").append(varName)
                        i += 7; continue
                    }
                }
                // .indexOf(
                if (i + 9 <= input.length && input.regionMatches(i + 1, "indexOf(", 0, 8)) {
                    val idStart = findPrecedingIdentifier(result.toString(), result.length)
                    if (idStart >= 0) {
                        val varName = result.substring(idStart)
                        result.delete(idStart, result.length)
                        val parenOpen = i + 8
                        val argEnd = findMatchingClose(input, parenOpen, '(', ')')
                        val args = input.substring(parenOpen + 1, argEnd)
                        result.append("_indexOf($varName, $args)")
                        i = argEnd + 1; continue
                    }
                }
                // .concat(
                if (i + 8 <= input.length && input.regionMatches(i + 1, "concat(", 0, 7)) {
                    val idStart = findPrecedingIdentifier(result.toString(), result.length)
                    if (idStart >= 0) {
                        val varName = result.substring(idStart)
                        result.delete(idStart, result.length)
                        result.append("_concat($varName, ")
                        i += 8; continue
                    }
                    if (result.length >= 2) {
                        val last2 = result.substring(result.length - 2)
                        if (last2 == "{}" || last2 == "[]") {
                            result.delete(result.length - 2, result.length)
                            result.append("_concat($last2, ")
                            i += 8; continue
                        }
                    }
                }
                // .push(
                if (i + 6 <= input.length && input.regionMatches(i + 1, "push(", 0, 5)) {
                    val idStart = findPrecedingIdentifier(result.toString(), result.length)
                    if (idStart >= 0) {
                        val varName = result.substring(idStart)
                        result.delete(idStart, result.length)
                        val parenOpen = i + 5
                        val argEnd = findMatchingClose(input, parenOpen, '(', ')')
                        val args = input.substring(parenOpen + 1, argEnd)
                        result.append("table.insert($varName, $args)")
                        i = argEnd + 1; continue
                    }
                }
                // .join(
                if (i + 6 <= input.length && input.regionMatches(i + 1, "join(", 0, 5)) {
                    val idStart = findPrecedingIdentifier(result.toString(), result.length)
                    if (idStart >= 0) {
                        val varName = result.substring(idStart)
                        result.delete(idStart, result.length)
                        val parenOpen = i + 5
                        val argEnd = findMatchingClose(input, parenOpen, '(', ')')
                        var args = input.substring(parenOpen + 1, argEnd)
                        if (args.isEmpty()) args = "','"
                        result.append("table.concat($varName, $args)")
                        i = argEnd + 1; continue
                    }
                }
            }
            result.append(input[i]); i++
        }
        return result.toString()
    }

    // === String Concat (no-op: Lua metatable handles it) ===

    private fun transformStringConcat(input: String): String = input

    // === Function Syntax ===

    private fun transformFunctionSyntax(input: String): String {
        val output = StringBuilder(input.length * 2)
        var i = 0
        data class FuncInfo(val depth: Int, val isConstructor: Boolean)
        val funcStack = mutableListOf<FuncInfo>()
        var braceDepth = 0

        while (i < input.length) {
            if (i + 8 <= input.length && input.regionMatches(i, "function", 0, 8) &&
                (i == 0 || !input[i - 1].isWordChar())) {
                val funcStart = i; var j = i + 8
                while (j < input.length && (input[j].isWhitespace() || input[j].isWordChar())) j++
                if (j < input.length && input[j] == '(') {
                    var parenDepth = 1; j++
                    while (j < input.length && parenDepth > 0) {
                        if (input[j] == '(') parenDepth++ else if (input[j] == ')') parenDepth--
                        j++
                    }
                    while (j < input.length && input[j].isWhitespace()) j++
                    if (j < input.length && input[j] == '{') {
                        var isConstructor = false
                        var checkDepth = 1
                        var k = j + 1
                        while (k < input.length && checkDepth > 0) {
                            if (input[k] == '{') checkDepth++ else if (input[k] == '}') checkDepth--
                            if (checkDepth == 1 && k + 5 <= input.length &&
                                input.regionMatches(k, "this.", 0, 5)) { isConstructor = true; break }
                            k++
                        }
                        output.append(input.substring(funcStart, j))
                        if (isConstructor) output.append(" local self = {} ") else output.append(' ')
                        funcStack.add(FuncInfo(braceDepth, isConstructor))
                        braceDepth++; i = j + 1; continue
                    }
                }
            }
            if (input[i] == '{') { braceDepth++; output.append(input[i]) }
            else if (input[i] == '}') {
                braceDepth--
                if (funcStack.isNotEmpty() && braceDepth == funcStack.last().depth) {
                    if (funcStack.last().isConstructor) output.append(" return self end")
                    else output.append(" end")
                    funcStack.removeLast()
                } else { output.append(input[i]) }
            } else { output.append(input[i]) }
            i++
        }

        var result = output.toString()
        // this.prop → self.prop
        var pos = 0
        while (true) {
            pos = findWord(result, "this", pos)
            if (pos == -1) break
            if (pos + 4 < result.length && result[pos + 4] == '.') {
                result = result.substring(0, pos) + "self" + result.substring(pos + 4)
            }
            pos += 4
        }
        return result
    }

    // === Var Declarations ===

    private fun transformVarDeclarations(input: String): String {
        var result = replaceKeywordPrefix(input, "var", "")
        result = replaceKeywordPrefix(result, "let", "")
        return replaceKeywordPrefix(result, "const", "")
    }

    // === new Expression ===

    private fun transformNewExpression(input: String): String {
        val result = StringBuilder(input.length)
        var i = 0
        while (i < input.length) {
            val pos = findWord(input, "new", i)
            if (pos == -1) { result.append(input, i, input.length); break }
            result.append(input, i, pos)
            val j = skipSpaces(input, pos + 3)
            val nameEnd = readWord(input, j)
            if (nameEnd > j) {
                val k = skipSpaces(input, nameEnd)
                if (k < input.length && input[k] == '(') {
                    result.append(input.substring(j, nameEnd)).append('(')
                    i = k + 1; continue
                }
            }
            result.append("new"); i = pos + 3
        }
        return result.toString()
    }

    // === Ternary Operator ===

    private fun transformTernaryOperator(input: String): String {
        val qPos = input.indexOf('?')
        if (qPos == -1) return input
        val cPos = input.indexOf(':', qPos + 1)
        if (cPos == -1) return input
        val cond = input.substring(0, qPos).trimBoth()
        val trueVal = input.substring(qPos + 1, cPos).trimBoth()
        val falseVal = input.substring(cPos + 1).trimBoth()
        if (cond.isEmpty() || trueVal.isEmpty() || falseVal.isEmpty()) return input
        return "($cond and $trueVal or $falseVal)"
    }

    // === Object Literals: {key: value} → {key = value} ===

    private fun transformObjectLiterals(input: String): String {
        val result = StringBuilder(input.length)
        var braceDepth = 0; var ternaryCount = 0
        val ternaryStack = mutableListOf<Int>()

        for (i in input.indices) {
            when (input[i]) {
                '{' -> { braceDepth++; ternaryStack.add(ternaryCount); ternaryCount = 0; result.append(input[i]) }
                '}' -> {
                    braceDepth--
                    if (ternaryStack.isNotEmpty()) { ternaryCount = ternaryStack.removeLast() }
                    result.append(input[i])
                }
                '?' -> { if (braceDepth > 0) ternaryCount++; result.append(input[i]) }
                ':' -> {
                    if (braceDepth > 0) {
                        if (ternaryCount > 0) { ternaryCount--; result.append(input[i]) }
                        else {
                            var keyEnd = i
                            while (keyEnd > 0 && input[keyEnd - 1].isWhitespace()) keyEnd--
                            if (keyEnd > 0 && (input[keyEnd - 1].isLetterOrDigit() || input[keyEnd - 1] == '_')) {
                                result.append(" =")
                            } else if (keyEnd > 0 && input[keyEnd - 1] == '\u0001') {
                                var placeholderStart = keyEnd - 1
                                while (placeholderStart > 0 && input[placeholderStart - 1] != '\u0001') placeholderStart--
                                if (placeholderStart > 0) placeholderStart--
                                val placeholder = input.substring(placeholderStart, keyEnd)
                                val resultPlaceholderPos = result.lastIndexOf(placeholder)
                                if (resultPlaceholderPos >= 0) {
                                    result.replace(resultPlaceholderPos, resultPlaceholderPos + placeholder.length,
                                        "[$placeholder]")
                                }
                                result.append(" =")
                            } else { result.append(input[i]) }
                        }
                    } else { result.append(input[i]) }
                }
                else -> result.append(input[i])
            }
        }
        return result.toString()
    }

    // === DOM Methods ===

    private fun transformDOMMethods(input: String): String {
        return input.replace(".getElementsByTagName(", ":getElementsByTagName(")
            .replace(".getAttribute(", ":getAttribute(")
            .replace(".getTagName(", ":getTagName(")
    }

    // === Semicolons ===

    private fun transformSemicolons(input: String): String {
        var result = input.trimEnd(';')
        return result.replace(';', '\n')
    }

    // === For Loops ===

    private fun transformForLoops(input: String): String {
        val result = StringBuilder(input.length + 64)
        var i = 0
        while (i < input.length) {
            val forPos = findWord(input, "for", i)
            if (forPos == -1) { result.append(input, i, input.length); break }
            result.append(input, i, forPos)
            i = forPos + 3

            val parenStart = skipSpaces(input, i)
            if (parenStart >= input.length || input[parenStart] != '(') {
                result.append("for"); continue
            }

            var parenDepth = 1; var pos = parenStart + 1
            val semiPositions = mutableListOf<Int>()
            while (pos < input.length && parenDepth > 0) {
                if (input[pos] == '(') parenDepth++
                else if (input[pos] == ')') parenDepth--
                else if (input[pos] == ';' && parenDepth == 1) semiPositions.add(pos)
                if (parenDepth > 0) pos++
            }

            if (semiPositions.size != 2) { result.append("for"); i = forPos + 3; continue }

            val init = input.substring(parenStart + 1, semiPositions[0]).trimBoth()
            val cond = input.substring(semiPositions[0] + 1, semiPositions[1]).trimBoth()
            var incr = input.substring(semiPositions[1] + 1, pos).trimBoth()
            i = pos + 1

            // Simplify increment patterns
            if (incr.length >= 3) {
                if (incr.endsWith("++")) { val v = incr.dropLast(2).trimBoth(); incr = "$v = $v + 1" }
                else if (incr.startsWith("++")) { val v = incr.drop(2).trimBoth(); incr = "$v = $v + 1" }
                else if (incr.endsWith("--")) { val v = incr.dropLast(2).trimBoth(); incr = "$v = $v - 1" }
                else if (incr.startsWith("--")) { val v = incr.drop(2).trimBoth(); incr = "$v = $v - 1" }
            }

            val bodyStart = skipSpaces(input, i)
            if (bodyStart >= input.length || input[bodyStart] != '{') {
                result.append(input.substring(forPos, i)); continue
            }
            val bodyEnd = findMatchingClose(input, bodyStart, '{', '}')
            val body = input.substring(bodyStart + 1, bodyEnd).trimBoth()
            i = bodyEnd + 1

            // Detect for (var i = 0; i < arr.length; i++) → for i = 1, #arr do
            var usedNumericFor = false
            run {
                var initTrimmed = init
                for (kw in listOf("let ", "var ", "const ")) {
                    if (initTrimmed.startsWith(kw)) { initTrimmed = initTrimmed.removePrefix(kw); break }
                }
                initTrimmed = initTrimmed.trimBoth()
                val eqPos = initTrimmed.indexOf('=')
                if (eqPos != -1) {
                    val loopVar = initTrimmed.substring(0, eqPos).trimBoth()
                    val startVal = initTrimmed.substring(eqPos + 1).trimBoth()
                    if (startVal == "0" && loopVar.isNotEmpty()) {
                        val condPattern = "$loopVar < "
                        if (cond.startsWith(condPattern)) {
                            val boundExpr = cond.removePrefix(condPattern).trimBoth()
                            if (boundExpr.length > 7 && boundExpr.endsWith(".length")) {
                                val arrName = boundExpr.dropLast(7)
                                result.append("for $loopVar = 1, #$arrName do\n$body\nend\n")
                                usedNumericFor = true
                            }
                        }
                    }
                }
            }

            if (!usedNumericFor) {
                result.append("$init\nwhile $cond do\n$body\n$incr\nend\n")
            }
        }
        return result.toString()
    }

    // === Conditional Blocks ===

    private fun transformConditionalBlocks(input: String): String {
        val result = StringBuilder(input.length + 32)
        var i = 0
        while (i < input.length) {
            val ifPos = findWord(input, "if", i)
            if (ifPos == -1) { result.append(input, i, input.length); break }
            result.append(input, i, ifPos)
            i = ifPos + 2

            val condStart = skipSpaces(input, i)
            if (condStart >= input.length || input[condStart] != '(') { result.append("if"); continue }
            val condEnd = findMatchingClose(input, condStart, '(', ')')
            val condition = input.substring(condStart + 1, condEnd).trimBoth()
            i = condEnd + 1

            val bodyStart = skipSpaces(input, i)
            if (bodyStart >= input.length || input[bodyStart] != '{') {
                result.append("if ($condition)"); i = bodyStart; continue
            }
            val bodyEnd = findMatchingClose(input, bodyStart, '{', '}')
            val body = input.substring(bodyStart + 1, bodyEnd).trimBoth()
            i = bodyEnd + 1

            result.append("if $condition then\n$body\n")

            // Check for else/else if
            while (i < input.length) {
                val elsePos = skipSpaces(input, i)
                if (elsePos + 4 <= input.length && input.regionMatches(elsePos, "else", 0, 4) &&
                    (elsePos + 4 >= input.length || !input[elsePos + 4].isWordChar())) {
                    i = elsePos + 4
                    val afterElse = skipSpaces(input, i)
                    if (afterElse + 2 <= input.length && input.regionMatches(afterElse, "if", 0, 2) &&
                        (afterElse + 2 >= input.length || !input[afterElse + 2].isWordChar())) {
                        i = afterElse + 2
                        val eiCondStart = skipSpaces(input, i)
                        if (eiCondStart < input.length && input[eiCondStart] == '(') {
                            val eiCondEnd = findMatchingClose(input, eiCondStart, '(', ')')
                            val eiCond = input.substring(eiCondStart + 1, eiCondEnd).trimBoth()
                            i = eiCondEnd + 1
                            val eiBodyStart = skipSpaces(input, i)
                            if (eiBodyStart < input.length && input[eiBodyStart] == '{') {
                                val eiBodyEnd = findMatchingClose(input, eiBodyStart, '{', '}')
                                val eiBody = input.substring(eiBodyStart + 1, eiBodyEnd).trimBoth()
                                i = eiBodyEnd + 1
                                result.append("elseif $eiCond then\n$eiBody\n")
                                continue
                            }
                        }
                        i = elsePos + 4
                    }
                    // Plain else
                    val elseBodyStart = skipSpaces(input, i)
                    if (elseBodyStart < input.length && input[elseBodyStart] == '{') {
                        val elseBodyEnd = findMatchingClose(input, elseBodyStart, '{', '}')
                        val elseBody = input.substring(elseBodyStart + 1, elseBodyEnd).trimBoth()
                        i = elseBodyEnd + 1
                        result.append("else\n$elseBody\n")
                    }
                }
                break
            }
            result.append("end\n")
        }
        return result.toString()
    }

    // === Bare Expressions ===

    private fun transformBareExpressions(input: String): String {
        val lines = input.split('\n').toMutableList()
        var lastNonEmpty = -1
        for (j in lines.indices.reversed()) {
            if (lines[j].trimBoth().isNotEmpty()) { lastNonEmpty = j; break }
        }

        val luaKeywords = arrayOf("local", "return", "if", "for", "while", "repeat",
            "do", "end", "else", "elseif", "then", "function", "break")

        val result = StringBuilder()
        for (j in lines.indices) {
            val trimmed = lines[j].trimBoth()
            if (trimmed.isEmpty() || trimmed == "end") { result.append(lines[j]).append('\n'); continue }

            var isStatement = false
            // Check for assignment (not ==, ~=, <=, >=)
            for (k in trimmed.indices) {
                if (trimmed[k] == '=' && k > 0 && trimmed[k - 1] != '~' && trimmed[k - 1] != '<' &&
                    trimmed[k - 1] != '>' && trimmed[k - 1] != '!' &&
                    (k + 1 >= trimmed.length || trimmed[k + 1] != '=')) {
                    isStatement = true; break
                }
            }
            if (!isStatement) {
                for (kw in luaKeywords) {
                    if (trimmed.length >= kw.length && trimmed.startsWith(kw) &&
                        (trimmed.length == kw.length || !trimmed[kw.length].isWordChar())) {
                        isStatement = true; break
                    }
                }
            }
            if (!isStatement && '(' in trimmed) isStatement = true

            if (!isStatement) {
                lines[j] = if (j == lastNonEmpty) "return $trimmed" else "_ = $trimmed"
            }
            result.append(lines[j]).append('\n')
        }
        // Remove trailing newlines
        while (result.isNotEmpty() && result.last() == '\n') result.deleteCharAt(result.length - 1)
        return result.toString()
    }

    // === Truthiness Wrapping ===

    private fun needsTruthinessWrapping(expr: String): Boolean {
        if ("==" in expr || "~=" in expr || ">=" in expr || "<=" in expr ||
            "_scxml_truthy" in expr || "_isArray" in expr) return false
        for (i in expr.indices) {
            if ((expr[i] == '<' || expr[i] == '>') && (i + 1 >= expr.length || expr[i + 1] != '='))
                return false
        }
        val trimmed = expr.trimBoth()
        if (trimmed == "true" || trimmed == "false") return false
        if ("In(" in trimmed && isPureInPredicate(trimmed)) return false
        if (trimmed.length >= 4 && trimmed.startsWith("not ")) return false
        return true
    }

    private fun wrapTruthiness(input: String): String {
        return if (needsTruthinessWrapping(input)) "_scxml_truthy($input)" else input
    }

    companion object {
        private const val MAX_CACHE_SIZE = 1024

        private fun lruCache(): LinkedHashMap<String, String> =
            object : LinkedHashMap<String, String>(64, 0.75f, true) {
                override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, String>?) =
                    size > MAX_CACHE_SIZE
            }
    }
}
