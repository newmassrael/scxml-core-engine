#include "scripting/EcmaScriptToLuaTransformer.h"
#include <algorithm>
#include <cstring>

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
    // Compound assignment and increment/decrement must run before operator transforms
    processed = transformCompoundAssignment(processed);
    processed = transformIncrementDecrement(processed);
    processed = transformInstanceofPatterns(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayLiterals(processed);
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

    // Stage 2: Apply transformations
    // Compound assignment and increment/decrement must run before operator transforms
    processed = transformCompoundAssignment(processed);
    processed = transformIncrementDecrement(processed);
    processed = transformFunctionSyntax(processed);
    processed = transformInstanceofPatterns(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayLiterals(processed);
    processed = transformObjectLiterals(processed);
    processed = transformTernaryOperator(processed);
    processed = transformOperators(processed);
    processed = transformStringConcat(processed);
    processed = transformDOMMethods(processed);
    processed = transformVarDeclarations(processed);
    processed = transformSemicolons(processed);

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
                int depth = 1;
                size_t start = i;
                ++i;
                while (i < input.size() && depth > 0) {
                    if (input[i] == '[') ++depth;
                    else if (input[i] == ']') --depth;
                    if (depth > 0) ++i;
                }
                std::string contents = input.substr(start + 1, i - start - 1);
                result += "{" + contents + "}";
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
                    // Find closing )
                    size_t argStart = i + 9;
                    size_t argEnd = argStart;
                    int depth = 1;
                    while (argEnd < input.size() && depth > 0) {
                        if (input[argEnd] == '(') ++depth;
                        else if (input[argEnd] == ')') --depth;
                        if (depth > 0) ++argEnd;
                    }
                    std::string args = input.substr(argStart, argEnd - argStart);
                    result += "_indexOf(" + varName + ", " + args + ")";
                    i = argEnd + 1;  // skip past )
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
                    size_t argStart = i + 6;
                    size_t argEnd = argStart;
                    int depth = 1;
                    while (argEnd < input.size() && depth > 0) {
                        if (input[argEnd] == '(') ++depth;
                        else if (input[argEnd] == ')') --depth;
                        if (depth > 0) ++argEnd;
                    }
                    std::string args = input.substr(argStart, argEnd - argStart);
                    result += "table.insert(" + varName + ", " + args + ")";
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
    // Leave + as-is; Lua runtime _plus() helper handles type dispatch
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
    // Only transform colon after identifier keys inside braces (not in ternary, labels, etc.)
    std::string result;
    result.reserve(input.size());
    int braceDepth = 0;

    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '{') {
            ++braceDepth;
            result += input[i];
        } else if (input[i] == '}') {
            --braceDepth;
            result += input[i];
        } else if (input[i] == ':' && braceDepth > 0) {
            // Check if preceded by an identifier (object key)
            size_t keyEnd = i;
            while (keyEnd > 0 && std::isspace(static_cast<unsigned char>(input[keyEnd - 1]))) --keyEnd;
            if (keyEnd > 0 && (std::isalnum(static_cast<unsigned char>(input[keyEnd - 1])) ||
                               input[keyEnd - 1] == '_')) {
                result += " =";
            } else {
                result += input[i];
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

// === Stage 3: Truthiness Wrapping ===

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
