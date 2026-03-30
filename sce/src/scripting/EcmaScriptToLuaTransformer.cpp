#include "scripting/EcmaScriptToLuaTransformer.h"
#include <algorithm>
#include <regex>
#include <sstream>

namespace SCE {

// === Public API ===

std::string EcmaScriptToLuaTransformer::transform(const std::string &ecmaScript,
                                                    ExpressionContext context) const {
    if (ecmaScript.empty()) {
        return ecmaScript;
    }

    // Pre-pass: typeof patterns must be transformed BEFORE string protection
    // because we need to inspect string literal content ('undefined' vs 'number')
    std::string preProcessed = transformTypeofPatterns(ecmaScript);

    // Stage 1: Protect string literals from transformation
    auto [processed, literals] = protectStringLiterals(preProcessed);

    // Stage 2: Apply transformation pipeline (order matters)
    processed = transformInstanceofPatterns(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayLiterals(processed);
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

    return result;
}

std::string EcmaScriptToLuaTransformer::transformScript(const std::string &script) const {
    if (script.empty()) {
        return script;
    }

    // Pre-pass: typeof before string protection
    std::string preProcessed = transformTypeofPatterns(script);

    // Stage 1: Protect string literals
    auto [processed, literals] = protectStringLiterals(preProcessed);

    // Stage 2: Apply transformations
    processed = transformFunctionSyntax(processed);
    processed = transformInstanceofPatterns(processed);
    processed = transformNullUndefined(processed);
    processed = transformNewExpression(processed);
    processed = transformArrayMethods(processed);
    processed = transformArrayLiterals(processed);
    processed = transformTernaryOperator(processed);
    processed = transformOperators(processed);
    processed = transformStringConcat(processed);
    processed = transformDOMMethods(processed);
    processed = transformVarDeclarations(processed);
    processed = transformSemicolons(processed);

    // Stage 3: Restore string literals
    return restoreStringLiterals(processed, literals);
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

// Helper: extract the unquoted content of a JS string literal ('foo' or "foo" -> foo)
static std::string unquote(const std::string &s) {
    if (s.size() >= 2 && ((s.front() == '\'' && s.back() == '\'') ||
                           (s.front() == '"' && s.back() == '"'))) {
        return s.substr(1, s.size() - 2);
    }
    return s;
}

std::string EcmaScriptToLuaTransformer::transformTypeofPatterns(const std::string &input) const {
    std::string result = input;

    // Pattern: typeof VAR OP 'TYPE'   where OP is ===, ==, !==, !=
    // Must handle: typeof x === 'undefined', typeof(x) !== 'undefined',
    //              typeof x === 'number', typeof x === 'string', etc.
    //
    // Lua mapping:
    //   typeof x === 'undefined'  ->  x == nil
    //   typeof x !== 'undefined'  ->  x ~= nil
    //   typeof x === 'number'     ->  type(x) == 'number'
    //   typeof x === 'string'     ->  type(x) == 'string'
    //   typeof x === 'boolean'    ->  type(x) == 'boolean'
    //   typeof x === 'function'   ->  type(x) == 'function'
    //   typeof x === 'object'     ->  type(x) == 'table'
    //   standalone typeof x       ->  _typeof(x)

    // Match typeof with comparison: typeof VAR OP 'string_literal'
    // Capture groups: (1)varname, (2)operator, (3)quoted_type_string
    std::regex typeofCmp(
        R"(typeof\s*\(?\s*(\w+)\s*\)?\s*(!==?|===?)\s*('[^']*'|"[^"]*"))");

    std::string temp;
    std::sregex_iterator it(result.begin(), result.end(), typeofCmp);
    std::sregex_iterator end;
    size_t lastPos = 0;

    for (; it != end; ++it) {
        auto &match = *it;
        temp += result.substr(lastPos, match.position() - lastPos);

        std::string varName = match[1].str();
        std::string op = match[2].str();
        std::string typeStr = unquote(match[3].str());

        // Determine Lua operator
        std::string luaOp;
        if (op == "!==" || op == "!=") {
            luaOp = "~=";
        } else {
            luaOp = "==";
        }

        if (typeStr == "undefined") {
            // typeof x === 'undefined' -> x == nil
            temp += varName + " " + luaOp + " nil";
        } else {
            // Map JS type names to Lua type() return values
            std::string luaType = typeStr;
            if (typeStr == "object") {
                luaType = "table";
            }
            // typeof x === 'number' -> type(x) == 'number'
            temp += "type(" + varName + ") " + luaOp + " '" + luaType + "'";
        }

        lastPos = match.position() + match.length();
    }
    temp += result.substr(lastPos);
    result = std::move(temp);

    // Remaining standalone typeof x -> _typeof(x)
    {
        std::regex remainingTypeof(R"(\btypeof\s*\(?\s*(\w+)\s*\)?)");
        result = std::regex_replace(result, remainingTypeof, "_typeof($1)");
    }

    return result;
}

// === Stage 2: Pattern Transformations ===

std::string EcmaScriptToLuaTransformer::transformInstanceofPatterns(const std::string &input) const {
    // Match: expr instanceof Array — where expr can be a variable, (expr), or complex expression
    std::regex instanceofArray(R"(((?:\([^)]+\)|\w+))\s+instanceof\s+Array)");
    return std::regex_replace(input, instanceofArray, "_isArray($1)");
}

std::string EcmaScriptToLuaTransformer::transformNullUndefined(const std::string &input) const {
    std::string result = input;

    // Standalone undefined -> nil (word boundary)
    std::regex undefRegex(R"(\bundefined\b)");
    result = std::regex_replace(result, undefRegex, "nil");

    // null -> nil
    std::regex nullRegex(R"(\bnull\b)");
    result = std::regex_replace(result, nullRegex, "nil");

    return result;
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
                // Skip if next char is '=' (already handled above as !=)
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

    return result;
}

std::string EcmaScriptToLuaTransformer::transformArrayLiterals(const std::string &input) const {
    std::string result;
    result.reserve(input.size());

    for (size_t i = 0; i < input.size(); ++i) {
        if (input[i] == '[') {
            // Determine if array literal or property access
            // Property access: preceded by identifier char, ), ]
            bool isPropertyAccess = false;
            if (i > 0) {
                char prev = input[i - 1];
                if (std::isalnum(prev) || prev == '_' || prev == ')' || prev == ']' || prev == '\x01') {
                    isPropertyAccess = true;
                }
            }

            if (!isPropertyAccess) {
                // Array literal: find matching ] and replace with {}
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
    std::string result = input;

    // .length -> # prefix operator
    {
        std::regex lengthProp(R"((\w+)\.length\b)");
        result = std::regex_replace(result, lengthProp, "#$1");
    }

    // .indexOf(x) -> _indexOf(arr, x)
    {
        std::regex indexOfMethod(R"((\w+)\.indexOf\(([^)]*)\))");
        result = std::regex_replace(result, indexOfMethod, "_indexOf($1, $2)");
    }

    // .concat(...) -> _concat(arr, ...)
    {
        std::regex concatMethod(R"((\w+|\{\})\.concat\()");
        result = std::regex_replace(result, concatMethod, "_concat($1, ");
    }

    // .push(x) -> table.insert(arr, x)
    {
        std::regex pushMethod(R"((\w+)\.push\(([^)]*)\))");
        result = std::regex_replace(result, pushMethod, "table.insert($1, $2)");
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
    // First pass: replace function declarations and track brace→end mapping
    std::string output;
    output.reserve(result.size() * 2);

    size_t i = 0;
    // Stack of brace depths where functions start
    std::vector<int> funcStartDepths;
    int braceDepth = 0;

    while (i < result.size()) {
        // Detect function keyword
        if (i + 8 <= result.size() && result.substr(i, 8) == "function" &&
            (i == 0 || !std::isalnum(result[i - 1]))) {
            // Find opening (
            size_t funcStart = i;
            size_t j = i + 8;
            // Skip whitespace and optional function name
            while (j < result.size() && (std::isspace(result[j]) || std::isalnum(result[j]) || result[j] == '_'))
                ++j;

            if (j < result.size() && result[j] == '(') {
                // Find matching )
                int parenDepth = 1;
                ++j;
                while (j < result.size() && parenDepth > 0) {
                    if (result[j] == '(') ++parenDepth;
                    else if (result[j] == ')') --parenDepth;
                    ++j;
                }

                // Skip whitespace before {
                while (j < result.size() && std::isspace(result[j])) ++j;

                if (j < result.size() && result[j] == '{') {
                    // Output: function name(params)  (without {)
                    // Extract function header up to {
                    output += result.substr(funcStart, j - funcStart);
                    output += ' '; // space instead of {
                    funcStartDepths.push_back(braceDepth);
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
            if (!funcStartDepths.empty() && braceDepth == funcStartDepths.back()) {
                output += " end";
                funcStartDepths.pop_back();
            } else {
                output += result[i];
            }
        } else {
            output += result[i];
        }
        ++i;
    }

    result = std::move(output);

    // Transform this.prop -> self.prop (for constructors)
    {
        std::regex thisDot(R"(\bthis\.)");
        result = std::regex_replace(result, thisDot, "self.");
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformVarDeclarations(const std::string &input) const {
    std::string result = input;

    // var/let/const -> local
    {
        std::regex varDecl(R"(\bvar\s+)");
        result = std::regex_replace(result, varDecl, "local ");
    }
    {
        std::regex letDecl(R"(\blet\s+)");
        result = std::regex_replace(result, letDecl, "local ");
    }
    {
        std::regex constDecl(R"(\bconst\s+)");
        result = std::regex_replace(result, constDecl, "local ");
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformNewExpression(const std::string &input) const {
    std::regex newExpr(R"(\bnew\s+(\w+)\s*\()");
    return std::regex_replace(input, newExpr, "$1(");
}

std::string EcmaScriptToLuaTransformer::transformTernaryOperator(const std::string &input) const {
    // cond ? a : b -> (cond and a or b)
    std::regex ternary(R"(([^?]+)\?([^:]+):(.+))");
    std::smatch match;
    if (std::regex_match(input, match, ternary)) {
        auto trim = [](const std::string &s) {
            size_t start = s.find_first_not_of(" \t");
            size_t end = s.find_last_not_of(" \t");
            return (start == std::string::npos) ? "" : s.substr(start, end - start + 1);
        };
        return "(" + trim(match[1].str()) + " and " + trim(match[2].str()) +
               " or " + trim(match[3].str()) + ")";
    }
    return input;
}

std::string EcmaScriptToLuaTransformer::transformDOMMethods(const std::string &input) const {
    std::string result = input;

    // DOM method calls use . in JS but : in Lua (to pass self)
    // .getElementsByTagName( -> :getElementsByTagName(
    {
        size_t pos = 0;
        while ((pos = result.find(".getElementsByTagName(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 21;
        }
    }

    // .getAttribute( -> :getAttribute(
    {
        size_t pos = 0;
        while ((pos = result.find(".getAttribute(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 14;
        }
    }

    // .getTagName( -> :getTagName(
    {
        size_t pos = 0;
        while ((pos = result.find(".getTagName(", pos)) != std::string::npos) {
            result.replace(pos, 1, ":");
            pos += 12;
        }
    }

    return result;
}

std::string EcmaScriptToLuaTransformer::transformSemicolons(const std::string &input) const {
    std::string result = input;

    // Remove trailing semicolons
    while (!result.empty() && result.back() == ';') {
        result.pop_back();
    }

    // Replace ; with newline in multi-statement contexts
    {
        std::string temp;
        temp.reserve(result.size());
        for (size_t i = 0; i < result.size(); ++i) {
            if (result[i] == ';') {
                // Replace with newline for statement separation
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
    // Already boolean-producing expressions don't need wrapping
    if (expr.find("==") != std::string::npos ||
        expr.find("~=") != std::string::npos ||
        expr.find(">=") != std::string::npos ||
        expr.find("<=") != std::string::npos ||
        expr.find("_scxml_truthy") != std::string::npos ||
        expr.find("_isArray") != std::string::npos) {
        return false;
    }

    // Check for standalone < or > (not inside <= or >=)
    for (size_t i = 0; i < expr.size(); ++i) {
        if ((expr[i] == '<' || expr[i] == '>') &&
            (i + 1 >= expr.size() || expr[i + 1] != '=')) {
            return false;
        }
    }

    // Boolean literals
    std::string trimmed = expr;
    auto start = trimmed.find_first_not_of(" \t\n");
    auto end = trimmed.find_last_not_of(" \t\n");
    if (start != std::string::npos) {
        trimmed = trimmed.substr(start, end - start + 1);
    }
    if (trimmed == "true" || trimmed == "false") {
        return false;
    }

    // Pure In() predicate calls are already boolean
    if (trimmed.find("In(") != std::string::npos) {
        std::regex pureIn(R"(^[\s()]*(?:In\([^)]+\)[\s()]*(?:\s+and\s+|\s+or\s+)?)+$)");
        if (std::regex_match(trimmed, pureIn)) {
            return false;
        }
    }

    // 'not' prefix produces boolean
    if (trimmed.substr(0, 4) == "not ") {
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
