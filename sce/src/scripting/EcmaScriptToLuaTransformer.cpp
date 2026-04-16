// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "scripting/EcmaScriptToLuaTransformer.h"
#include <algorithm>
#include <cstring>
#include <sstream>

namespace {

// Word character check: equivalent to regex \w (alphanumeric + underscore)
inline bool isWordChar(char c) {
    return std::isalnum(static_cast<unsigned char>(c)) || c == '_';
}

// Find a keyword with word boundaries (equivalent to \bword\b)
size_t findWord(const std::string &s, const char *word, size_t wordLen, size_t startPos = 0) {
    size_t pos = startPos;
    while ((pos = s.find(word, pos)) != std::string::npos) {
        bool leftOk = (pos == 0 || !isWordChar(s[pos - 1]));
        bool rightOk = (pos + wordLen >= s.size() || !isWordChar(s[pos + wordLen]));
        if (leftOk && rightOk) return pos;
        ++pos;
    }
    return std::string::npos;
}

// Replace all word-bounded occurrences: \bword\b -> replacement
std::string replaceWord(const std::string &s, const char *word, const char *replacement) {
    size_t wordLen = std::strlen(word);
    size_t replLen = std::strlen(replacement);
    std::string result;
    result.reserve(s.size());
    size_t lastPos = 0;
    size_t pos = 0;
    while ((pos = findWord(s, word, wordLen, pos)) != std::string::npos) {
        result.append(s, lastPos, pos - lastPos);
        result.append(replacement, replLen);
        lastPos = pos + wordLen;
        pos = lastPos;
    }
    result.append(s, lastPos, s.size() - lastPos);
    return result;
}

// Replace word-bounded occurrences only at bracket depth 0.
// Skips matches inside nested {}, (), [] — used for array-element-level sentinel replacement.
std::string replaceWordAtTopLevel(const std::string &s, const char *word, const char *replacement) {
    size_t wordLen = std::strlen(word);
    size_t replLen = std::strlen(replacement);
    std::string result;
    result.reserve(s.size());
    int depth = 0;
    size_t i = 0;

    while (i < s.size()) {
        char c = s[i];
        if (c == '{' || c == '(' || c == '[') {
            ++depth;
            result += c;
            ++i;
        } else if (c == '}' || c == ')' || c == ']') {
            if (depth > 0) --depth;
            result += c;
            ++i;
        } else if (depth == 0 &&
                   i + wordLen <= s.size() &&
                   s.compare(i, wordLen, word) == 0 &&
                   (i == 0 || !isWordChar(s[i - 1])) &&
                   (i + wordLen >= s.size() || !isWordChar(s[i + wordLen]))) {
            result.append(replacement, replLen);
            i += wordLen;
        } else {
            result += c;
            ++i;
        }
    }

    return result;
}

// Replace word-bounded keyword followed by whitespace: \bword\s+ -> replacement
// Used for var/let/const -> local
std::string replaceKeywordPrefix(const std::string &s, const char *keyword, const char *replacement) {
    size_t kwLen = std::strlen(keyword);
    std::string result;
    result.reserve(s.size());
    size_t lastPos = 0;
    size_t pos = 0;
    while ((pos = findWord(s, keyword, kwLen, pos)) != std::string::npos) {
        size_t afterKw = pos + kwLen;
        if (afterKw < s.size() && std::isspace(static_cast<unsigned char>(s[afterKw]))) {
            result.append(s, lastPos, pos - lastPos);
            result.append(replacement);
            // Consume all whitespace after keyword (matches \bword\s+ semantics)
            size_t spaceEnd = afterKw;
            while (spaceEnd < s.size() && std::isspace(static_cast<unsigned char>(s[spaceEnd]))) ++spaceEnd;
            lastPos = spaceEnd;
            pos = lastPos;
        } else {
            ++pos;
        }
    }
    result.append(s, lastPos, s.size() - lastPos);
    return result;
}

// Skip whitespace, return new position
inline size_t skipSpaces(const std::string &s, size_t pos) {
    while (pos < s.size() && std::isspace(static_cast<unsigned char>(s[pos]))) ++pos;
    return pos;
}

// Read a word (\w+) starting at pos, return end position
inline size_t readWord(const std::string &s, size_t pos) {
    while (pos < s.size() && isWordChar(s[pos])) ++pos;
    return pos;
}

// Trim whitespace from both ends
std::string trim(const std::string &s) {
    size_t start = s.find_first_not_of(" \t");
    size_t end = s.find_last_not_of(" \t");
    return (start == std::string::npos) ? "" : s.substr(start, end - start + 1);
}

// Find matching close bracket/paren/brace starting after the opening character.
// Returns position of the matching closer. If unbalanced, returns the last position
// in the string (safe fallback — callers can extract truncated content without UB).
size_t findMatchingClose(const std::string &s, size_t openPos, char open, char close) {
    int depth = 1;
    size_t pos = openPos + 1;
    while (pos < s.size() && depth > 0) {
        if (s[pos] == open) ++depth;
        else if (s[pos] == close) --depth;
        if (depth > 0) ++pos;
    }
    return (depth == 0) ? pos : (s.empty() ? 0 : s.size() - 1);
}

// Check if position is preceded only by \w characters going back to a word-char
// Used to extract a preceding identifier for methods like .length, .indexOf, etc.
// Returns start position of the identifier, or npos if none found.
size_t findPrecedingIdentifier(const std::string &s, size_t dotPos) {
    if (dotPos == 0) return std::string::npos;
    size_t end = dotPos;
    size_t start = dotPos;
    while (start > 0 && isWordChar(s[start - 1])) --start;
    return (start < end) ? start : std::string::npos;
}

// Check if a pure In() predicate chain (no regex)
// Matches patterns like: In('s1'), In('s1') and In('s2'), (In('s1') or In('s2'))
bool isPureInPredicate(const std::string &expr) {
    size_t i = 0;
    size_t len = expr.size();
    bool foundIn = false;

    while (i < len) {
        char c = expr[i];
        // Skip whitespace and parentheses
        if (std::isspace(static_cast<unsigned char>(c)) || c == '(' || c == ')') {
            ++i;
            continue;
        }
        // Check for "In("
        if (i + 3 <= len && expr.compare(i, 3, "In(") == 0) {
            foundIn = true;
            i += 3;
            // Skip to closing )
            while (i < len && expr[i] != ')') ++i;
            if (i < len) ++i;  // skip ')'
            continue;
        }
        // Check for "and" or "or" keywords
        if (i + 3 <= len && expr.compare(i, 3, "and") == 0 &&
            (i + 3 >= len || !isWordChar(expr[i + 3]))) {
            i += 3;
            continue;
        }
        if (i + 2 <= len && expr.compare(i, 2, "or") == 0 &&
            (i + 2 >= len || !isWordChar(expr[i + 2]))) {
            i += 2;
            continue;
        }
        // Any other character means this is not a pure In() predicate
        return false;
    }
    return foundIn;
}

}  // anonymous namespace

namespace SCE {

// === Public API ===

void EcmaScriptToLuaTransformer::clearCache() {
    generalCache_.clear();
    guardCache_.clear();
    scriptCache_.clear();
}

std::string EcmaScriptToLuaTransformer::transform(const std::string &ecmaScript,
                                                    ExpressionContext context) const {
    if (ecmaScript.empty()) {
        return ecmaScript;
    }

    // Cache lookup: General and Guard use separate maps to avoid key-suffix allocation
    auto &cache = (context == ExpressionContext::Guard) ? guardCache_ : generalCache_;
    auto cacheIt = cache.find(ecmaScript);
    if (cacheIt != cache.end()) {
        return cacheIt->second;
    }

    // Pre-pass: typeof patterns must be transformed BEFORE string protection
    // because we need to inspect string literal content ('undefined' vs 'number')
    std::string preProcessed = transformTypeofPatterns(ecmaScript);

    // Stage 1: Protect string literals from transformation
    auto [processed, literals] = protectStringLiterals(preProcessed);

    // Stage 2: Apply transformation pipeline (order matters)
    // Math builtins must run early (before operator transforms alter the expressions)
    processed = transformMathBuiltins(processed);
    // Compound assignment and increment/decrement must run before operator transforms
    processed = transformCompoundAssignment(processed);
    processed = transformIncrementDecrement(processed);
    processed = transformInstanceofPatterns(processed);
    processed = transformArrayLiterals(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayIndexing(processed);
    processed = transformObjectLiterals(processed);
    processed = transformTernaryOperator(processed);
    processed = transformOperators(processed);
    processed = transformStringConcat(processed);
    processed = transformDOMMethods(processed);
    processed = transformVarDeclarations(processed);
    processed = transformSemicolons(processed);

    // Stage 3: Restore string literals
    std::string result = restoreStringLiterals(processed, literals);

    // Stage 4: Guard-specific truthiness wrapping
    if (context == ExpressionContext::Guard) {
        result = wrapTruthiness(result);
    }

    cache[ecmaScript] = result;
    return result;
}

std::string EcmaScriptToLuaTransformer::transformScript(const std::string &script) const {
    if (script.empty()) {
        return script;
    }

    auto cacheIt = scriptCache_.find(script);
    if (cacheIt != scriptCache_.end()) {
        return cacheIt->second;
    }

    // Pre-pass: typeof before string protection
    std::string preProcessed = transformTypeofPatterns(script);

    // Stage 1: Protect string literals
    auto [processed, literals] = protectStringLiterals(preProcessed);

    // Stage 2: Structural transforms (must see original JS syntax with semicolons)
    // For-in must be extracted before for-loops (which expect C-style for headers)
    processed = transformForInLoops(processed);
    // For-loops must be converted before semicolons are removed
    processed = transformForLoops(processed);

    // Stage 3: Apply transformations
    // Math builtins must run early (before operator transforms alter the expressions)
    processed = transformMathBuiltins(processed);
    // Compound assignment and increment/decrement must run before operator transforms
    processed = transformCompoundAssignment(processed);
    processed = transformIncrementDecrement(processed);
    processed = transformFunctionSyntax(processed);
    processed = transformInstanceofPatterns(processed);
    processed = transformArrayLiterals(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayIndexing(processed);
    processed = transformObjectLiterals(processed);
    processed = transformTernaryOperator(processed);
    processed = transformOperators(processed);
    processed = transformStringConcat(processed);
    processed = transformDOMMethods(processed);
    processed = transformVarDeclarations(processed);
    processed = transformSemicolons(processed);
    processed = transformConditionalBlocks(processed);
    processed = transformBareExpressions(processed);

    // Stage 3: Restore string literals
    std::string result = restoreStringLiterals(processed, literals);
    scriptCache_[script] = result;
    return result;
}

// === Stage 1: String Literal Protection ===

EcmaScriptToLuaTransformer::ProtectedString
EcmaScriptToLuaTransformer::protectStringLiterals(const std::string &input) const {
    ProtectedString result;
    std::vector<std::string> &literals = result.literals;
    std::string &output = result.processed;
    output.reserve(input.size());

    for (size_t i = 0; i < input.size(); ++i) {
        char c = input[i];

        // Strip JS block comments /* ... */
        if (c == '/' && i + 1 < input.size() && input[i + 1] == '*') {
            i += 2;
            while (i + 1 < input.size() && !(input[i] == '*' && input[i + 1] == '/')) ++i;
            if (i + 1 < input.size()) ++i;  // skip closing '/'
            continue;
        }

        // Strip JS line comments // ...
        if (c == '/' && i + 1 < input.size() && input[i + 1] == '/') {
            i += 2;
            while (i < input.size() && input[i] != '\n') ++i;
            if (i < input.size()) output += '\n';  // preserve newline for statement separation
            continue;
        }

        if (c == '\'' || c == '"') {
            char quote = c;
            std::string literal;
            literal += c;
            ++i;
            while (i < input.size()) {
                if (input[i] == '\\' && i + 1 < input.size()) {
                    literal += input[i];
                    literal += input[i + 1];
                    i += 2;
                    continue;
                }
                if (input[i] == quote) {
                    literal += input[i];
                    break;
                }
                literal += input[i];
                ++i;
            }

            size_t idx = literals.size();
            literals.push_back(literal);
            output += "\x01STR" + std::to_string(idx) + "\x01";
        } else {
            output += c;
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::restoreStringLiterals(
    const std::string &processed, const std::vector<std::string> &literals) const {
    std::string result = processed;

    for (size_t i = 0; i < literals.size(); ++i) {
        std::string placeholder = "\x01STR" + std::to_string(i) + "\x01";
        size_t pos = result.find(placeholder);
        while (pos != std::string::npos) {
            result.replace(pos, placeholder.size(), literals[i]);
            pos = result.find(placeholder, pos + literals[i].size());
        }
    }

    return result;
}

// === Pre-pass: typeof Transformation (before string protection) ===

std::string EcmaScriptToLuaTransformer::transformTypeofPatterns(const std::string &input) const {
    // typeof VAR OP 'TYPE' → Lua type comparison
    // typeof VAR (standalone) → _typeof(VAR)
    std::string result;
    result.reserve(input.size());
    size_t i = 0;

    while (i < input.size()) {
        size_t typeofPos = input.find("typeof", i);
        if (typeofPos == std::string::npos) {
            result.append(input, i, input.size() - i);
            break;
        }

        // Check word boundary before "typeof"
        if (typeofPos > 0 && isWordChar(input[typeofPos - 1])) {
            result.append(input, i, typeofPos + 6 - i);
            i = typeofPos + 6;
            continue;
        }

        result.append(input, i, typeofPos - i);

        // Parse: typeof\s*\(?\s*(\w+)\s*\)?
        size_t j = typeofPos + 6;
        j = skipSpaces(input, j);

        bool hasParen = (j < input.size() && input[j] == '(');
        if (hasParen) j = skipSpaces(input, j + 1);

        size_t varStart = j;
        size_t varEnd = readWord(input, j);

        if (varEnd == varStart) {
            result.append("typeof");
            i = typeofPos + 6;
            continue;
        }
        std::string varName = input.substr(varStart, varEnd - varStart);

        // Compute position after typeof expr (past optional closing paren)
        size_t afterTypeofExpr = skipSpaces(input, varEnd);
        if (hasParen && afterTypeofExpr < input.size() && input[afterTypeofExpr] == ')') {
            ++afterTypeofExpr;
        }

        // Check for comparison operator: !==, ===, !=, ==
        size_t k = skipSpaces(input, afterTypeofExpr);
        size_t opLen = 0;
        bool isNeg = false;
        if (k + 2 < input.size() && (input.compare(k, 3, "!==") == 0)) {
            opLen = 3; isNeg = true;
        } else if (k + 2 < input.size() && input.compare(k, 3, "===") == 0) {
            opLen = 3; isNeg = false;
        } else if (k + 1 < input.size() && input.compare(k, 2, "!=") == 0) {
            opLen = 2; isNeg = true;
        } else if (k + 1 < input.size() && input.compare(k, 2, "==") == 0) {
            opLen = 2; isNeg = false;
        }

        if (opLen > 0) {
            size_t m = skipSpaces(input, k + opLen);

            if (m < input.size() && (input[m] == '\'' || input[m] == '"')) {
                // Full typeof comparison pattern
                char quote = input[m];
                ++m;
                size_t typeStart = m;
                while (m < input.size() && input[m] != quote) ++m;
                std::string typeStr = input.substr(typeStart, m - typeStart);
                if (m < input.size()) ++m;

                const char *luaOp = isNeg ? "~=" : "==";

                if (typeStr == "undefined") {
                    result += varName + " " + luaOp + " nil";
                } else {
                    const char *luaType = (typeStr == "object") ? "table" : typeStr.c_str();
                    result += "type(" + varName + ") " + luaOp + " '" + luaType + "'";
                }
                i = m;
                continue;
            }
        }

        // Standalone typeof → _typeof(varName)
        result += "_typeof(" + varName + ")";
        i = afterTypeofExpr;
    }

    return result;
}

// === Stage 2: Pattern Transformations ===

std::string EcmaScriptToLuaTransformer::transformCompoundAssignment(const std::string &input) const {
    // ECMAScript compound assignment: Var1+=1 → Var1 = Var1 + (1)
    // Operators: +=, -=, *=, /=, %=
    std::string result;
    result.reserve(input.size());
    size_t i = 0;

    while (i < input.size()) {
        // Look for compound assignment operators: +=, -=, *=, /=, %=
        // Scan for = preceded by +, -, *, /, %
        size_t eqPos = input.find('=', i);
        if (eqPos == std::string::npos || eqPos == 0) {
            result.append(input, i, input.size() - i);
            break;
        }

        char prev = input[eqPos - 1];
        // Skip ==, !=, <=, >=, !== , ===
        if (prev == '=' || prev == '!' || prev == '<' || prev == '>') {
            result.append(input, i, eqPos + 1 - i);
            i = eqPos + 1;
            continue;
        }

        if (prev != '+' && prev != '-' && prev != '*' && prev != '/' && prev != '%') {
            result.append(input, i, eqPos + 1 - i);
            i = eqPos + 1;
            continue;
        }

        // Found potential compound assignment: extract variable name before operator
        size_t opPos = eqPos - 1;  // position of +, -, *, /, %

        // Read variable name backwards from opPos
        size_t varEnd = opPos;
        // Skip whitespace between var and operator
        while (varEnd > i && std::isspace(static_cast<unsigned char>(input[varEnd - 1]))) --varEnd;
        size_t varStart = varEnd;
        while (varStart > i && isWordChar(input[varStart - 1])) --varStart;

        if (varStart == varEnd) {
            // No identifier before operator
            result.append(input, i, eqPos + 1 - i);
            i = eqPos + 1;
            continue;
        }

        std::string varName = input.substr(varStart, varEnd - varStart);
        char op = prev;

        // Read RHS: everything until ; or newline
        size_t rhsStart = eqPos + 1;
        while (rhsStart < input.size() && std::isspace(static_cast<unsigned char>(input[rhsStart]))) ++rhsStart;
        size_t rhsEnd = rhsStart;
        while (rhsEnd < input.size() && input[rhsEnd] != ';' && input[rhsEnd] != '\n') ++rhsEnd;

        std::string rhs = input.substr(rhsStart, rhsEnd - rhsStart);

        // Emit: text before var + var = var op (rhs)
        result.append(input, i, varStart - i);
        result += varName + " = " + varName + " " + op + " (" + rhs + ")";
        i = rhsEnd;
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformIncrementDecrement(const std::string &input) const {
    // Postfix operators must be matched BEFORE prefix operators.
    std::string result;
    result.reserve(input.size() * 2);
    size_t i = 0;

    while (i < input.size()) {
        // Check for postfix/prefix increment: \w++ or ++\w
        if (input[i] == '+' && i + 1 < input.size() && input[i + 1] == '+') {
            size_t varEnd = i;
            size_t varStart = i;
            while (varStart > 0 && isWordChar(input[varStart - 1])) --varStart;

            if (varStart < varEnd) {
                result.erase(result.size() - (varEnd - varStart));
                std::string var = input.substr(varStart, varEnd - varStart);
                result += "(function() local _t = " + var + " " + var + " = " + var + " + 1 return _t end)()";
                i += 2;
                continue;
            }

            size_t afterOp = i + 2;
            size_t wordEnd = readWord(input, afterOp);
            if (wordEnd > afterOp) {
                std::string var = input.substr(afterOp, wordEnd - afterOp);
                result += "(function() " + var + " = " + var + " + 1 return " + var + " end)()";
                i = wordEnd;
                continue;
            }
        }

        // Check for postfix/prefix decrement: \w-- or --\w
        if (input[i] == '-' && i + 1 < input.size() && input[i + 1] == '-') {
            size_t varEnd = i;
            size_t varStart = i;
            while (varStart > 0 && isWordChar(input[varStart - 1])) --varStart;

            if (varStart < varEnd) {
                result.erase(result.size() - (varEnd - varStart));
                std::string var = input.substr(varStart, varEnd - varStart);
                result += "(function() local _t = " + var + " " + var + " = " + var + " - 1 return _t end)()";
                i += 2;
                continue;
            }

            size_t afterOp = i + 2;
            size_t wordEnd = readWord(input, afterOp);
            if (wordEnd > afterOp) {
                std::string var = input.substr(afterOp, wordEnd - afterOp);
                result += "(function() " + var + " = " + var + " - 1 return " + var + " end)()";
                i = wordEnd;
                continue;
            }
        }

        result += input[i];
        ++i;
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformInstanceofPatterns(const std::string &input) const {
    // expr instanceof Array → _isArray(expr)
    // expr can be a variable or (parenthesized expression)
    std::string result;
    result.reserve(input.size());
    size_t i = 0;

    while (i < input.size()) {
        size_t pos = findWord(input, "instanceof", 10, i);
        if (pos == std::string::npos) {
            result.append(input, i, input.size() - i);
            break;
        }

        // Check if followed by whitespace + "Array"
        size_t afterInst = skipSpaces(input, pos + 10);
        size_t arrayEnd = readWord(input, afterInst);
        if (arrayEnd > afterInst && input.compare(afterInst, arrayEnd - afterInst, "Array") == 0) {
            // Find the preceding expression (variable or parenthesized)
            size_t exprEnd = pos;
            while (exprEnd > i && std::isspace(static_cast<unsigned char>(input[exprEnd - 1]))) --exprEnd;

            std::string expr;
            size_t exprStart = exprEnd;

            if (exprEnd > i && input[exprEnd - 1] == ')') {
                // Parenthesized expression: find matching (
                int depth = 1;
                size_t k = exprEnd - 2;
                while (k > i && depth > 0) {
                    if (input[k] == ')') ++depth;
                    else if (input[k] == '(') --depth;
                    if (depth > 0) --k;
                }
                exprStart = k;
            } else {
                // Variable name
                while (exprStart > i && isWordChar(input[exprStart - 1])) --exprStart;
            }

            expr = input.substr(exprStart, exprEnd - exprStart);
            result.append(input, i, exprStart - i);
            result += "_isArray(" + expr + ")";
            i = arrayEnd;
        } else {
            result.append(input, i, pos + 10 - i);
            i = pos + 10;
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformNullUndefined(const std::string &input) const {
    std::string result = replaceWord(input, "undefined", "nil");
    return replaceWord(result, "null", "nil");
}

std::string EcmaScriptToLuaTransformer::transformOperators(const std::string &input) const {
    std::string result = input;

    // Order matters: !== before !=, === before ==

    // !== -> ~=
    {
        size_t pos = 0;
        while ((pos = result.find("!==", pos)) != std::string::npos) {
            result.replace(pos, 3, "~=");
            pos += 2;
        }
    }

    // === -> ==
    {
        size_t pos = 0;
        while ((pos = result.find("===", pos)) != std::string::npos) {
            result.replace(pos, 3, "==");
            pos += 2;
        }
    }

    // != -> ~= (must not re-match already-converted ~=)
    {
        std::string temp;
        temp.reserve(result.size());
        for (size_t i = 0; i < result.size(); ++i) {
            if (result[i] == '!' && i + 1 < result.size() && result[i + 1] == '=') {
                temp += "~=";
                ++i;
            } else {
                temp += result[i];
            }
        }
        result = std::move(temp);
    }

    // && -> and
    {
        size_t pos = 0;
        while ((pos = result.find("&&", pos)) != std::string::npos) {
            result.replace(pos, 2, " and ");
            pos += 5;
        }
    }

    // || -> or
    {
        size_t pos = 0;
        while ((pos = result.find("||", pos)) != std::string::npos) {
            result.replace(pos, 2, " or ");
            pos += 4;
        }
    }

    // Logical NOT: !expr -> not expr
    // Must not match already-converted ~=
    {
        std::string temp;
        temp.reserve(result.size());
        for (size_t i = 0; i < result.size(); ++i) {
            if (result[i] == '!') {
                if (i + 1 < result.size() && result[i + 1] == '=') {
                    temp += result[i];
                } else {
                    temp += "not ";
                }
            } else {
                temp += result[i];
            }
        }
        result = std::move(temp);
    }

    // W3C SCXML B.2 (test 459): Parenthesize operands of bitwise OR/AND/XOR
    result = parenthesizeBitwiseOperands(result);

    return result;
}

std::string EcmaScriptToLuaTransformer::parenthesizeBitwiseOperands(const std::string &input) const {
    bool hasBitwise = false;
    for (size_t i = 0; i < input.size(); ++i) {
        char c = input[i];
        if (c == '|' || c == '&' || c == '^') {
            hasBitwise = true;
            break;
        }
    }
    if (!hasBitwise) return input;

    bool hasComparison = (input.find("==") != std::string::npos ||
                          input.find("~=") != std::string::npos ||
                          input.find("<=") != std::string::npos ||
                          input.find(">=") != std::string::npos);
    if (!hasComparison) {
        for (size_t i = 0; i < input.size(); ++i) {
            if ((input[i] == '<' || input[i] == '>') &&
                (i + 1 >= input.size() || input[i + 1] != '=')) {
                hasComparison = true;
                break;
            }
        }
    }
    if (!hasComparison) return input;

    std::string result;
    result.reserve(input.size() + 16);
    size_t start = 0;
    for (size_t i = 0; i < input.size(); ++i) {
        char c = input[i];
        if (c == '|' || c == '&' || c == '^') {
            std::string operand = trim(input.substr(start, i - start));
            result += "(" + operand + ") " + c + " ";
            start = i + 1;
        }
    }
    std::string lastOp = trim(input.substr(start));
    result += "(" + lastOp + ")";

    return result;
}

std::string EcmaScriptToLuaTransformer::transformArrayLiterals(const std::string &input) const {
    std::string result;
    result.reserve(input.size());

    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '[') {
            bool isPropertyAccess = false;
            if (i > 0) {
                char prev = input[i - 1];
                if (std::isalnum(prev) || prev == '_' || prev == ')' || prev == ']' || prev == '\x01') {
                    isPropertyAccess = true;
                }
            }

            if (!isPropertyAccess) {
                size_t closePos = findMatchingClose(input, i, '[', ']');
                std::string contents = input.substr(i + 1, closePos - i - 1);
                contents = transformArrayLiterals(contents);
                // W3C SCXML 4.6: Replace null/undefined with sentinels at array element level only.
                // Nested structures ({key: null}, function calls) are left unchanged.
                contents = replaceWordAtTopLevel(contents, "null", "_NULL");
                contents = replaceWordAtTopLevel(contents, "undefined", "_UNDEFINED");
                result += "{" + contents + "}";
                i = closePos;
            } else {
                result += input[i];
            }
        } else {
            result += input[i];
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformArrayIndexing(const std::string &input) const {
    // ECMAScript arrays are 0-based; Lua tables are 1-based.
    // Convert property-access brackets with integer literal index: arr[0] → arr[0 + 1]
    // Only applies to numeric integer literal indices (not variables or string keys).
    std::string result;
    result.reserve(input.size() + 32);

    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '[') {
            // Check if this is a property access (preceded by identifier/)/])
            bool isPropertyAccess = false;
            if (i > 0) {
                char prev = input[i - 1];
                if (std::isalnum(static_cast<unsigned char>(prev)) || prev == '_' ||
                    prev == ')' || prev == ']' || prev == '\x01') {
                    isPropertyAccess = true;
                }
            }

            if (isPropertyAccess) {
                size_t end = findMatchingClose(input, i, '[', ']');
                std::string indexExpr = trim(input.substr(i + 1, end - i - 1));

                // Check if pure integer literal (0, 1, 2, etc.) with safe range
                bool isIntLiteral = !indexExpr.empty() && indexExpr.size() <= 9;
                for (char c : indexExpr) {
                    if (!std::isdigit(static_cast<unsigned char>(c))) {
                        isIntLiteral = false;
                        break;
                    }
                }

                if (isIntLiteral) {
                    int idx = std::stoi(indexExpr);
                    result += "[" + std::to_string(idx + 1) + "]";
                    i = end;  // skip past ']'
                } else {
                    result += input[i];
                }
            } else {
                result += input[i];
            }
        } else {
            result += input[i];
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformArrayMethods(const std::string &input) const {
    std::string result;
    result.reserve(input.size());
    size_t i = 0;

    while (i < input.size()) {
        // Look for dot-method patterns
        if (input[i] == '.') {
            // .length\b → # prefix
            if (i + 7 <= input.size() && input.compare(i + 1, 6, "length") == 0 &&
                (i + 7 >= input.size() || !isWordChar(input[i + 7]))) {
                // Find preceding identifier
                size_t idStart = findPrecedingIdentifier(result, result.size());
                if (idStart != std::string::npos) {
                    std::string varName = result.substr(idStart);
                    result.erase(idStart);
                    result += "#" + varName;
                    i += 7;  // skip .length
                    continue;
                }
            }

            // .indexOf( → _indexOf(var,
            if (i + 9 <= input.size() && input.compare(i + 1, 8, "indexOf(") == 0) {
                size_t idStart = findPrecedingIdentifier(result, result.size());
                if (idStart != std::string::npos) {
                    std::string varName = result.substr(idStart);
                    result.erase(idStart);
                    size_t parenOpen = i + 8;  // position of '('
                    size_t argEnd = findMatchingClose(input, parenOpen, '(', ')');
                    std::string args = input.substr(parenOpen + 1, argEnd - parenOpen - 1);
                    result += "_indexOf(" + varName + ", " + args + ")";
                    i = argEnd + 1;
                    continue;
                }
            }

            // .concat( → _concat(var,
            if (i + 8 <= input.size() && input.compare(i + 1, 7, "concat(") == 0) {
                size_t idStart = findPrecedingIdentifier(result, result.size());
                if (idStart != std::string::npos) {
                    std::string varName = result.substr(idStart);
                    result.erase(idStart);
                    result += "_concat(" + varName + ", ";
                    i += 8;  // skip .concat(
                    continue;
                }
                // Also handle {}.concat( or [].concat(
                if (result.size() >= 2) {
                    std::string last2 = result.substr(result.size() - 2);
                    if (last2 == "{}" || last2 == "[]") {
                        std::string obj = last2;
                        result.erase(result.size() - 2);
                        result += "_concat(" + obj + ", ";
                        i += 8;
                        continue;
                    }
                }
            }

            // .push( → table.insert(var,
            if (i + 6 <= input.size() && input.compare(i + 1, 5, "push(") == 0) {
                size_t idStart = findPrecedingIdentifier(result, result.size());
                if (idStart != std::string::npos) {
                    std::string varName = result.substr(idStart);
                    result.erase(idStart);
                    size_t parenOpen = i + 5;  // position of '('
                    size_t argEnd = findMatchingClose(input, parenOpen, '(', ')');
                    std::string args = input.substr(parenOpen + 1, argEnd - parenOpen - 1);
                    result += "table.insert(" + varName + ", " + args + ")";
                    i = argEnd + 1;
                    continue;
                }
            }

            // .join( → table.concat(var,
            if (i + 6 <= input.size() && input.compare(i + 1, 5, "join(") == 0) {
                size_t idStart = findPrecedingIdentifier(result, result.size());
                if (idStart != std::string::npos) {
                    std::string varName = result.substr(idStart);
                    result.erase(idStart);
                    size_t parenOpen = i + 5;  // position of '('
                    size_t argEnd = findMatchingClose(input, parenOpen, '(', ')');
                    std::string args = input.substr(parenOpen + 1, argEnd - parenOpen - 1);
                    if (args.empty()) args = "','";  // JS default: .join() uses ',' separator
                    result += "table.concat(" + varName + ", " + args + ")";
                    i = argEnd + 1;
                    continue;
                }
            }
        }

        result += input[i];
        ++i;
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformStringConcat(const std::string &input) const {
    // Leave + as-is; Lua string metatable __add handles type dispatch at runtime.
    // Limitation: Lua auto-coerces numeric strings before invoking __add, so
    // "5" + "3" yields 8 (Lua native) instead of "53" (ECMAScript). This does not
    // affect W3C SCXML conformance as no test relies on numeric-string concatenation.
    return input;
}

std::string EcmaScriptToLuaTransformer::transformFunctionSyntax(const std::string &input) const {
    std::string result = input;

    // Track brace depth to replace } -> end at the correct level
    std::string output;
    output.reserve(result.size() * 2);

    size_t i = 0;
    struct FuncInfo {
        int depth;
        bool isConstructor;
    };
    std::vector<FuncInfo> funcStack;
    int braceDepth = 0;

    while (i < result.size()) {
        // Detect function keyword
        if (i + 8 <= result.size() && result.compare(i, 8, "function") == 0 &&
            (i == 0 || !isWordChar(result[i - 1]))) {
            size_t funcStart = i;
            size_t j = i + 8;
            while (j < result.size() && (std::isspace(static_cast<unsigned char>(result[j])) ||
                                          isWordChar(result[j])))
                ++j;

            if (j < result.size() && result[j] == '(') {
                int parenDepth = 1;
                ++j;
                while (j < result.size() && parenDepth > 0) {
                    if (result[j] == '(') ++parenDepth;
                    else if (result[j] == ')') --parenDepth;
                    ++j;
                }

                while (j < result.size() && std::isspace(static_cast<unsigned char>(result[j]))) ++j;

                if (j < result.size() && result[j] == '{') {
                    // W3C SCXML 5.3: Detect JS constructor pattern
                    bool isConstructor = false;
                    int checkDepth = 1;
                    for (size_t k = j + 1; k < result.size() && checkDepth > 0; ++k) {
                        if (result[k] == '{') ++checkDepth;
                        else if (result[k] == '}') --checkDepth;
                        if (checkDepth == 1 && k + 5 <= result.size() &&
                            result.compare(k, 5, "this.") == 0) {
                            isConstructor = true;
                            break;
                        }
                    }

                    output += result.substr(funcStart, j - funcStart);
                    if (isConstructor) {
                        output += " local self = {} ";
                    } else {
                        output += ' ';
                    }
                    funcStack.push_back({braceDepth, isConstructor});
                    braceDepth++;
                    i = j + 1;
                    continue;
                }
            }
        }

        if (result[i] == '{') {
            braceDepth++;
            output += result[i];
        } else if (result[i] == '}') {
            braceDepth--;
            if (!funcStack.empty() && braceDepth == funcStack.back().depth) {
                if (funcStack.back().isConstructor) {
                    output += " return self end";
                } else {
                    output += " end";
                }
                funcStack.pop_back();
            } else {
                output += result[i];
            }
        } else {
            output += result[i];
        }
        ++i;
    }

    result = std::move(output);

    // Transform this.prop -> self.prop
    // Only replace "this." (with dot) to match original behavior.
    {
        size_t pos = 0;
        while (true) {
            pos = findWord(result, "this", 4, pos);
            if (pos == std::string::npos) break;
            if (pos + 4 < result.size() && result[pos + 4] == '.') {
                result.replace(pos, 4, "self");
            }
            pos += 4;
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformVarDeclarations(const std::string &input) const {
    // W3C SCXML: All datamodel variables are session-global.
    // Strip var/let/const entirely to produce global assignments in Lua.
    // (Lua's 'local' is chunk-scoped and would make variables invisible to subsequent evaluations)
    std::string result = replaceKeywordPrefix(input, "var", "");
    result = replaceKeywordPrefix(result, "let", "");
    return replaceKeywordPrefix(result, "const", "");
}

std::string EcmaScriptToLuaTransformer::transformNewExpression(const std::string &input) const {
    // \bnew\s+(\w+)\s*\( → $1(
    std::string result;
    result.reserve(input.size());
    size_t i = 0;

    while (i < input.size()) {
        size_t pos = findWord(input, "new", 3, i);
        if (pos == std::string::npos) {
            result.append(input, i, input.size() - i);
            break;
        }

        result.append(input, i, pos - i);

        // Skip "new" + whitespace
        size_t j = skipSpaces(input, pos + 3);
        // Read constructor name
        size_t nameEnd = readWord(input, j);
        if (nameEnd > j) {
            size_t k = skipSpaces(input, nameEnd);
            if (k < input.size() && input[k] == '(') {
                // Matched: new ConstructorName(
                result.append(input, j, nameEnd - j);
                result += '(';
                i = k + 1;
                continue;
            }
        }
        // Not a match — keep "new"
        result.append("new");
        i = pos + 3;
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformTernaryOperator(const std::string &input) const {
    // cond ? a : b -> (cond and a or b)
    // Only match if the entire expression is a single ternary
    size_t qPos = input.find('?');
    if (qPos == std::string::npos) return input;

    size_t cPos = input.find(':', qPos + 1);
    if (cPos == std::string::npos) return input;

    std::string cond = trim(input.substr(0, qPos));
    std::string trueVal = trim(input.substr(qPos + 1, cPos - qPos - 1));
    std::string falseVal = trim(input.substr(cPos + 1));

    if (cond.empty() || trueVal.empty() || falseVal.empty()) return input;

    return "(" + cond + " and " + trueVal + " or " + falseVal + ")";
}

std::string EcmaScriptToLuaTransformer::transformDOMMethods(const std::string &input) const {
    std::string result = input;

    {
        size_t pos = 0;
        while ((pos = result.find(".getElementsByTagName(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 21;
        }
    }
    {
        size_t pos = 0;
        while ((pos = result.find(".getAttribute(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 14;
        }
    }
    {
        size_t pos = 0;
        while ((pos = result.find(".getTagName(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 12;
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformObjectLiterals(const std::string &input) const {
    // ECMAScript {key: value} → Lua {key = value}
    // Track ternary '?' operators per brace scope so their ':' colons are not
    // mistaken for object-key separators (runs before transformTernaryOperator).
    std::string result;
    result.reserve(input.size());
    int braceDepth = 0;
    int ternaryCount = 0;
    std::vector<int> ternaryStack;

    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '{') {
            ++braceDepth;
            ternaryStack.push_back(ternaryCount);
            ternaryCount = 0;
            result += input[i];
        } else if (input[i] == '}') {
            --braceDepth;
            if (!ternaryStack.empty()) {
                ternaryCount = ternaryStack.back();
                ternaryStack.pop_back();
            }
            result += input[i];
        } else if (input[i] == '?' && braceDepth > 0) {
            ++ternaryCount;
            result += input[i];
        } else if (input[i] == ':' && braceDepth > 0) {
            if (ternaryCount > 0) {
                // Ternary colon — leave as-is for transformTernaryOperator()
                --ternaryCount;
                result += input[i];
            } else {
                // Check if preceded by an identifier or string placeholder (object key)
                size_t keyEnd = i;
                while (keyEnd > 0 && std::isspace(static_cast<unsigned char>(input[keyEnd - 1]))) --keyEnd;
                if (keyEnd > 0 && (std::isalnum(static_cast<unsigned char>(input[keyEnd - 1])) ||
                                   input[keyEnd - 1] == '_')) {
                    result += " =";
                } else if (keyEnd > 0 && input[keyEnd - 1] == '\x01') {
                    // JSON string key placeholder: \x01STR<n>\x01: value → [\x01STR<n>\x01] = value
                    size_t placeholderEnd = keyEnd;
                    size_t placeholderStart = keyEnd - 1;
                    while (placeholderStart > 0 && input[placeholderStart - 1] != '\x01') --placeholderStart;
                    if (placeholderStart > 0) --placeholderStart;
                    std::string placeholder = input.substr(placeholderStart, placeholderEnd - placeholderStart);
                    size_t resultPlaceholderPos = result.rfind(placeholder);
                    if (resultPlaceholderPos != std::string::npos) {
                        result.replace(resultPlaceholderPos, placeholder.size(), "[" + placeholder + "]");
                    }
                    result += " =";
                } else {
                    result += input[i];
                }
            }
        } else {
            result += input[i];
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformSemicolons(const std::string &input) const {
    std::string result = input;

    while (!result.empty() && result.back() == ';') {
        result.pop_back();
    }

    {
        std::string temp;
        temp.reserve(result.size());
        for (size_t i = 0; i < result.size(); ++i) {
            if (result[i] == ';') {
                temp += '\n';
            } else {
                temp += result[i];
            }
        }
        result = std::move(temp);
    }

    return result;
}

// === Math Builtins ===
// W3C SCXML: Math.sqrt(x) → math.sqrt(x), Math.pow(x,y) → (x)^(y), etc.

std::string EcmaScriptToLuaTransformer::transformMathBuiltins(const std::string &input) const {
    std::string result = input;

    // Math.pow(a, b) → (a)^(b) — must be done before simple replacements
    // Find and replace all occurrences of Math.pow(...)
    {
        std::string temp;
        temp.reserve(result.size());
        size_t i = 0;
        while (i < result.size()) {
            size_t pos = result.find("Math.pow(", i);
            if (pos == std::string::npos) {
                temp.append(result, i, result.size() - i);
                break;
            }
            temp.append(result, i, pos - i);

            // Find the matching ')'
            size_t argStart = pos + 9; // after "Math.pow("
            int depth = 1;
            size_t commaPos = std::string::npos;
            size_t j = argStart;
            while (j < result.size() && depth > 0) {
                if (result[j] == '(') ++depth;
                else if (result[j] == ')') { --depth; if (depth == 0) break; }
                else if (result[j] == ',' && depth == 1 && commaPos == std::string::npos) commaPos = j;
                ++j;
            }
            if (depth == 0 && commaPos != std::string::npos) {
                std::string a = trim(result.substr(argStart, commaPos - argStart));
                std::string b = trim(result.substr(commaPos + 1, j - commaPos - 1));
                temp += "(" + a + ")^(" + b + ")";
                i = j + 1;
            } else {
                temp += "Math.pow(";
                i = argStart;
            }
        }
        result = std::move(temp);
    }

    // Direct mappings: Math.func → math.func / Lua equivalent
    auto replaceAll = [](std::string &s, const std::string &from, const std::string &to) {
        size_t pos = 0;
        while ((pos = s.find(from, pos)) != std::string::npos) {
            s.replace(pos, from.size(), to);
            pos += to.size();
        }
    };

    replaceAll(result, "Math.sqrt", "math.sqrt");
    replaceAll(result, "Math.abs", "math.abs");
    replaceAll(result, "Math.floor", "math.floor");
    replaceAll(result, "Math.ceil", "math.ceil");
    replaceAll(result, "Math.max", "math.max");
    replaceAll(result, "Math.min", "math.min");
    replaceAll(result, "Math.random", "math.random");
    replaceAll(result, "Math.log", "math.log");
    replaceAll(result, "Math.exp", "math.exp");
    replaceAll(result, "Math.sin", "math.sin");
    replaceAll(result, "Math.cos", "math.cos");
    replaceAll(result, "Math.tan", "math.tan");
    replaceAll(result, "Math.PI", "math.pi");
    replaceAll(result, "Math.E", "math.exp(1)");

    return result;
}

// === For-In Loops ===
// W3C SCXML: for (var k in obj) { body } → for k, _ in pairs(obj) do body end

std::string EcmaScriptToLuaTransformer::transformForInLoops(const std::string &input) const {
    std::string result;
    result.reserve(input.size() + 64);
    size_t i = 0;
    size_t len = input.size();

    while (i < len) {
        size_t forPos = findWord(input, "for", 3, i);
        if (forPos == std::string::npos) {
            result.append(input, i, len - i);
            break;
        }
        result.append(input, i, forPos - i);
        i = forPos + 3;

        size_t parenStart = skipSpaces(input, i);
        if (parenStart >= len || input[parenStart] != '(') {
            result += "for";
            continue;
        }

        // Find matching ')'
        size_t parenEnd = findMatchingClose(input, parenStart, '(', ')');

        // Extract header content
        std::string header = trim(input.substr(parenStart + 1, parenEnd - parenStart - 1));

        // Check for "var/let/const IDENT in EXPR" or "IDENT in EXPR" pattern
        std::string loopVar;
        std::string objExpr;
        bool isForIn = false;

        // Strip var/let/const prefix
        std::string headerStripped = header;
        for (const char *kw : {"var ", "let ", "const "}) {
            if (headerStripped.find(kw) == 0) {
                headerStripped = headerStripped.substr(std::strlen(kw));
                break;
            }
        }
        headerStripped = trim(headerStripped);

        // Find " in " keyword (word boundary)
        size_t inPos = findWord(headerStripped, "in", 2, 0);
        if (inPos != std::string::npos) {
            std::string beforeIn = trim(headerStripped.substr(0, inPos));
            std::string afterIn = trim(headerStripped.substr(inPos + 2));
            // beforeIn should be a single identifier
            bool isSingleIdent = !beforeIn.empty();
            for (char c : beforeIn) {
                if (!isWordChar(c)) { isSingleIdent = false; break; }
            }
            if (isSingleIdent && !afterIn.empty()) {
                loopVar = beforeIn;
                objExpr = afterIn;
                isForIn = true;
            }
        }

        if (!isForIn) {
            // Not a for-in, put back for transformForLoops
            result += "for";
            i = forPos + 3;
            continue;
        }

        i = parenEnd + 1;

        // Find body block
        size_t bodyStart = skipSpaces(input, i);
        if (bodyStart >= len || input[bodyStart] != '{') {
            result += "for " + loopVar + ", _ in pairs(" + objExpr + ") do end\n";
            continue;
        }
        size_t bodyEnd = findMatchingClose(input, bodyStart, '{', '}');
        std::string body = trim(input.substr(bodyStart + 1, bodyEnd - bodyStart - 1));
        i = bodyEnd + 1;

        result += "for " + loopVar + ", _ in pairs(" + objExpr + ") do\n" + body + "\nend\n";
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformForLoops(const std::string &input) const {
    // Convert JS for (init; cond; incr) { body } → Lua do init while cond do body incr end end
    // Must run BEFORE transformSemicolons (semicolons in for-header are structural)
    std::string result;
    result.reserve(input.size() + 64);
    size_t i = 0;
    size_t len = input.size();

    while (i < len) {
        size_t forPos = findWord(input, "for", 3, i);
        if (forPos == std::string::npos) {
            result.append(input, i, len - i);
            break;
        }
        result.append(input, i, forPos - i);
        i = forPos + 3;

        // Skip whitespace to find '('
        size_t parenStart = skipSpaces(input, i);
        if (parenStart >= len || input[parenStart] != '(') {
            result += "for";
            continue;
        }

        // Find matching ')' — parse three semicolon-separated parts
        int parenDepth = 1;
        size_t pos = parenStart + 1;
        std::vector<size_t> semiPositions;
        while (pos < len && parenDepth > 0) {
            if (input[pos] == '(') ++parenDepth;
            else if (input[pos] == ')') --parenDepth;
            else if (input[pos] == ';' && parenDepth == 1) semiPositions.push_back(pos);
            if (parenDepth > 0) ++pos;
        }

        // Must have exactly 2 semicolons for C-style for loop
        if (semiPositions.size() != 2) {
            result += "for";
            i = forPos + 3;
            continue;
        }

        std::string init = trim(input.substr(parenStart + 1, semiPositions[0] - parenStart - 1));
        std::string cond = trim(input.substr(semiPositions[0] + 1, semiPositions[1] - semiPositions[0] - 1));
        std::string incr = trim(input.substr(semiPositions[1] + 1, pos - semiPositions[1] - 1));
        i = pos + 1;  // skip ')'

        // Simplify common increment patterns to avoid IIFE (prevents Lua call ambiguity)
        // i++ / ++i → i = i + 1;  |  i-- / --i → i = i - 1;
        if (incr.size() >= 3) {
            std::string varName;
            bool isIncr = false, isDecr = false;
            if (incr.size() >= 3 && incr.substr(incr.size() - 2) == "++") {
                varName = trim(incr.substr(0, incr.size() - 2));
                isIncr = true;
            } else if (incr.size() >= 3 && incr.substr(0, 2) == "++") {
                varName = trim(incr.substr(2));
                isIncr = true;
            } else if (incr.size() >= 3 && incr.substr(incr.size() - 2) == "--") {
                varName = trim(incr.substr(0, incr.size() - 2));
                isDecr = true;
            } else if (incr.size() >= 3 && incr.substr(0, 2) == "--") {
                varName = trim(incr.substr(2));
                isDecr = true;
            }
            if (isIncr) incr = varName + " = " + varName + " + 1";
            else if (isDecr) incr = varName + " = " + varName + " - 1";
        }

        // Find '{' for body
        size_t bodyStart = skipSpaces(input, i);
        if (bodyStart >= len || input[bodyStart] != '{') {
            // No block body — pass through original text (let other transforms handle it)
            // Append everything from 'for' keyword through the closing ')' as-is
            result += input.substr(forPos, i - forPos);
            continue;
        }

        // Find matching '}'
        size_t bodyEnd = findMatchingClose(input, bodyStart, '{', '}');
        std::string body = trim(input.substr(bodyStart + 1, bodyEnd - bodyStart - 1));
        i = bodyEnd + 1;

        // Detect common pattern: for (var/let i = 0; i < arr.length; i++) → for i = 1, #arr do
        // This converts JS 0-based iteration to Lua 1-based for-loop with adjusted body indices
        bool usedNumericFor = false;
        {
            // Check init: "var/let/const IDENT = 0" or "IDENT = 0"
            std::string initTrimmed = init;
            // Strip var/let/const prefix
            for (const char *kw : {"let ", "var ", "const "}) {
                if (initTrimmed.find(kw) == 0) {
                    initTrimmed = initTrimmed.substr(std::strlen(kw));
                    break;
                }
            }
            initTrimmed = trim(initTrimmed);
            // Check pattern: IDENT = 0
            size_t eqPos = initTrimmed.find('=');
            if (eqPos != std::string::npos) {
                std::string loopVar = trim(initTrimmed.substr(0, eqPos));
                std::string startVal = trim(initTrimmed.substr(eqPos + 1));
                if (startVal == "0" && !loopVar.empty()) {
                    // Check cond: IDENT < EXPR.length (already transformed to IDENT < #EXPR by pipeline)
                    // At this point, .length hasn't been transformed yet, so check for "i < EXPR.length"
                    std::string condPattern = loopVar + " < ";
                    if (cond.find(condPattern) == 0) {
                        std::string boundExpr = trim(cond.substr(condPattern.size()));
                        // Check if bound ends with .length
                        if (boundExpr.size() > 7 && boundExpr.substr(boundExpr.size() - 7) == ".length") {
                            std::string arrName = boundExpr.substr(0, boundExpr.size() - 7);
                            // Emit Lua numeric for with 1-based indexing
                            result += "for " + loopVar + " = 1, #" + arrName + " do\n" + body + "\nend\n";
                            usedNumericFor = true;
                        }
                    }
                }
            }
        }

        if (!usedNumericFor) {
            // Emit Lua equivalent: init; while cond do body; incr end
            result += init + "\nwhile " + cond + " do\n" + body + "\n" + incr + "\nend\n";
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformConditionalBlocks(const std::string &input) const {
    // Convert JS if/else blocks to Lua syntax:
    //   if (cond) { body } else if (cond2) { body2 } else { body3 }
    //   → if cond then body elseif cond2 then body2 else body3 end
    std::string result;
    result.reserve(input.size() + 32);
    size_t i = 0;
    size_t len = input.size();

    while (i < len) {
        // Look for 'if' keyword with word boundary
        size_t ifPos = findWord(input, "if", 2, i);
        if (ifPos == std::string::npos) {
            result.append(input, i, len - i);
            break;
        }
        result.append(input, i, ifPos - i);
        i = ifPos + 2;

        // Skip whitespace to find '('
        size_t condStart = skipSpaces(input, i);
        if (condStart >= len || input[condStart] != '(') {
            result += "if";
            continue;
        }

        // Find matching ')' for condition
        size_t condEnd = findMatchingClose(input, condStart, '(', ')');
        std::string condition = trim(input.substr(condStart + 1, condEnd - condStart - 1));
        i = condEnd + 1;

        // Skip whitespace to find '{'
        size_t bodyStart = skipSpaces(input, i);
        if (bodyStart >= len || input[bodyStart] != '{') {
            result += "if (" + condition + ")";
            i = bodyStart;
            continue;
        }

        // Find matching '}'
        size_t bodyEnd = findMatchingClose(input, bodyStart, '{', '}');
        std::string body = trim(input.substr(bodyStart + 1, bodyEnd - bodyStart - 1));
        i = bodyEnd + 1;

        result += "if " + condition + " then\n" + body + "\n";

        // Check for else/else if
        while (i < len) {
            size_t elsePos = skipSpaces(input, i);
            if (elsePos + 4 <= len && input.compare(elsePos, 4, "else") == 0 &&
                (elsePos + 4 >= len || !isWordChar(input[elsePos + 4]))) {
                i = elsePos + 4;
                size_t afterElse = skipSpaces(input, i);

                // "else if" → "elseif"
                if (afterElse + 2 <= len && input.compare(afterElse, 2, "if") == 0 &&
                    (afterElse + 2 >= len || !isWordChar(input[afterElse + 2]))) {
                    i = afterElse + 2;
                    size_t eiCondStart = skipSpaces(input, i);
                    if (eiCondStart < len && input[eiCondStart] == '(') {
                        condEnd = findMatchingClose(input, eiCondStart, '(', ')');
                        condition = trim(input.substr(eiCondStart + 1, condEnd - eiCondStart - 1));
                        i = condEnd + 1;

                        bodyStart = skipSpaces(input, i);
                        if (bodyStart < len && input[bodyStart] == '{') {
                            bodyEnd = findMatchingClose(input, bodyStart, '{', '}');
                            body = trim(input.substr(bodyStart + 1, bodyEnd - bodyStart - 1));
                            i = bodyEnd + 1;
                            result += "elseif " + condition + " then\n" + body + "\n";
                            continue;
                        }
                    }
                    // "else if" without valid condition/body — rewind to treat as plain else
                    i = elsePos + 4;
                }

                // Plain "else { ... }"
                bodyStart = skipSpaces(input, i);
                if (bodyStart < len && input[bodyStart] == '{') {
                    bodyEnd = findMatchingClose(input, bodyStart, '{', '}');
                    body = trim(input.substr(bodyStart + 1, bodyEnd - bodyStart - 1));
                    i = bodyEnd + 1;
                    result += "else\n" + body + "\n";
                }
            }
            break;
        }
        result += "end\n";
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformBareExpressions(const std::string &input) const {
    // In Lua, bare expression statements (e.g., `x` or `a.b`) are syntax errors.
    // Only function calls are valid expression statements.
    // Convert bare expression lines to `_ = expr` for non-last lines, or `return expr` for last line.
    std::string result;
    std::vector<std::string> lines;

    // Split by newlines
    std::istringstream stream(input);
    std::string line;
    while (std::getline(stream, line)) {
        lines.push_back(line);
    }

    // Find last non-empty line index
    int lastNonEmpty = -1;
    for (int j = static_cast<int>(lines.size()) - 1; j >= 0; --j) {
        if (!trim(lines[j]).empty()) {
            lastNonEmpty = j;
            break;
        }
    }

    for (int j = 0; j < static_cast<int>(lines.size()); ++j) {
        std::string trimmed = trim(lines[j]);
        if (trimmed.empty() || trimmed == "end") {
            result += lines[j] + "\n";
            continue;
        }

        // Check if this line is a valid Lua statement:
        // - Contains '=' (assignment)
        // - Starts with a Lua keyword
        // - Contains '(' (function call)
        // - Starts with 'return'
        bool isStatement = false;

        // Check for assignment (but not ==, ~=, <=, >=)
        for (size_t k = 0; k < trimmed.size(); ++k) {
            if (trimmed[k] == '=' && k > 0 && trimmed[k - 1] != '~' && trimmed[k - 1] != '<' &&
                trimmed[k - 1] != '>' && trimmed[k - 1] != '!' &&
                (k + 1 >= trimmed.size() || trimmed[k + 1] != '=')) {
                isStatement = true;
                break;
            }
        }

        if (!isStatement) {
            // Check for Lua keywords
            static const char *keywords[] = {"local",  "return", "if",   "for",     "while",  "repeat",
                                             "do",     "end",    "else", "elseif",  "then",   "function",
                                             "break",  nullptr};
            for (int k = 0; keywords[k]; ++k) {
                size_t kwLen = std::strlen(keywords[k]);
                if (trimmed.size() >= kwLen && trimmed.compare(0, kwLen, keywords[k]) == 0 &&
                    (trimmed.size() == kwLen || !isWordChar(trimmed[kwLen]))) {
                    isStatement = true;
                    break;
                }
            }
        }

        if (!isStatement) {
            // Check for function call (contains open paren not inside string)
            if (trimmed.find('(') != std::string::npos) {
                isStatement = true;
            }
        }

        if (!isStatement) {
            // This is a bare expression — wrap it
            if (j == lastNonEmpty) {
                lines[j] = "return " + trimmed;
            } else {
                lines[j] = "_ = " + trimmed;
            }
        }

        result += lines[j] + "\n";
    }

    // Remove trailing newline
    while (!result.empty() && result.back() == '\n') result.pop_back();

    return result;
}

bool EcmaScriptToLuaTransformer::needsTruthinessWrapping(const std::string &expr) const {
    if (expr.find("==") != std::string::npos ||
        expr.find("~=") != std::string::npos ||
        expr.find(">=") != std::string::npos ||
        expr.find("<=") != std::string::npos ||
        expr.find("_scxml_truthy") != std::string::npos ||
        expr.find("_isArray") != std::string::npos) {
        return false;
    }

    for (size_t i = 0; i < expr.size(); ++i) {
        if ((expr[i] == '<' || expr[i] == '>') &&
            (i + 1 >= expr.size() || expr[i + 1] != '=')) {
            return false;
        }
    }

    std::string trimmed = trim(expr);
    if (trimmed == "true" || trimmed == "false") {
        return false;
    }

    // Pure In() predicate calls are already boolean
    if (trimmed.find("In(") != std::string::npos) {
        if (isPureInPredicate(trimmed)) {
            return false;
        }
    }

    // 'not' prefix produces boolean
    if (trimmed.size() >= 4 && trimmed.compare(0, 4, "not ") == 0) {
        return false;
    }

    return true;
}

std::string EcmaScriptToLuaTransformer::wrapTruthiness(const std::string &input) const {
    if (!needsTruthinessWrapping(input)) {
        return input;
    }
    return "_scxml_truthy(" + input + ")";
}

}  // namespace SCE
