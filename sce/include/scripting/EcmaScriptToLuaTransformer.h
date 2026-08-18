// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <string>
#include <unordered_map>
#include <vector>

namespace SCE {

/**
 * @brief Transforms ECMAScript expressions to Lua equivalents
 *
 * Pattern-based transformer for the subset of ECMAScript used in W3C SCXML tests.
 * Designed for use inside LuaEngine::evaluateExpression() — receives raw ECMAScript
 * expression strings and returns Lua-compatible equivalents.
 *
 * Transformation categories:
 *   - Operators: &&->and, ||->or, !->not, ===->==, !==->~=, !=->~=
 *   - Type checks: typeof x === 'undefined' -> x == nil
 *   - Null/undefined: null/undefined -> nil
 *   - Array literals: [1,2,3] -> {1,2,3}
 *   - String concat: + with strings -> ..
 *   - Array methods: .length->#, .indexOf()->_indexOf(), .concat()->_concat()
 *   - instanceof: x instanceof Array -> _isArray(x)
 *   - Function syntax: function(x){return y;} -> function(x) return y end
 *   - DOM methods: preserved (handled via Lua userdata metatables)
 *
 * W3C SCXML compliance: Covers all 202 W3C test expression patterns.
 */
class EcmaScriptToLuaTransformer {
public:
    /**
     * @brief Context for expression transformation
     *
     * Provides hints about how the expression is used,
     * enabling context-sensitive transformations (e.g., truthiness wrapping for guards).
     */
    enum class ExpressionContext {
        General,  // Assignment, log, data init — no truthiness wrapping
        Guard     // Transition cond, if/elseif — may need truthiness wrapping
    };

    EcmaScriptToLuaTransformer() = default;

    /**
     * @brief Transform an ECMAScript expression to Lua
     * @param ecmaScript Raw ECMAScript expression string
     * @param context How the expression is used (affects truthiness handling)
     * @return Lua-compatible expression string
     */
    std::string transform(const std::string &ecmaScript, ExpressionContext context = ExpressionContext::General) const;

    /**
     * @brief Transform a multi-line script block (may contain function definitions, statements)
     * @param script ECMAScript script block
     * @return Lua-compatible script block
     */
    std::string transformScript(const std::string &script) const;

    /**
     * @brief Clear the expression transformation cache
     *
     * Should be called on engine reset to free memory.
     * The cache is naturally bounded by the number of unique expressions
     * in an SCXML document.
     */
    void clearCache();

private:
    // === Transformation pipeline stages ===

    // Stage 1: Protect string literals from transformation
    //
    // The placeholder that stands in for a literal is identifier-shaped
    // (`_SCESTR0_`), not a non-word marker. Every later pass finds the
    // receiver of a member access by scanning word characters backwards, so a
    // placeholder those scans stop at is a receiver no pass can see:
    // `'abc'.length` reached Lua unrewritten and `'a,b'.split(',')` reached it
    // as `'a,b'.#split(',')`, neither of which is Lua at all.
    //
    // `prefix` is chosen against the input rather than fixed, so an author
    // identifier that happens to spell one cannot be mistaken for a literal.
    struct ProtectedString {
        std::string processed;
        std::vector<std::string> literals;
        std::string prefix;
    };

    ProtectedString protectStringLiterals(const std::string &input) const;
    std::string restoreStringLiterals(const std::string &processed, const ProtectedString &protectedInput) const;

    // Stage 2: Structural transforms (script-only, must see original JS syntax)
    std::string transformForInLoops(const std::string &input) const;
    std::string transformForLoops(const std::string &input) const;
    std::string transformConditionalBlocks(const std::string &input) const;
    std::string transformBareExpressions(const std::string &input) const;

    // Stage 3: Pattern-based transformations (applied to protected string)
    std::string transformCompoundAssignment(const std::string &input) const;
    std::string transformIncrementDecrement(const std::string &input) const;
    std::string transformTypeofPatterns(const std::string &input) const;
    std::string transformInstanceofPatterns(const std::string &input) const;
    std::string transformNullUndefined(const std::string &input) const;
    std::string transformOperators(const std::string &input) const;
    std::string transformArrayLiterals(const std::string &input) const;
    std::string transformArrayIndexing(const std::string &input) const;
    std::string transformArrayMethods(const std::string &input) const;
    std::string transformStringConcat(const std::string &input) const;
    std::string transformFunctionSyntax(const std::string &input) const;
    std::string transformVarDeclarations(const std::string &input) const;
    std::string transformNewExpression(const std::string &input) const;
    std::string transformTernaryOperator(const std::string &input) const;
    // Takes the placeholder prefix because a string key and a name key are
    // both identifier-shaped once literals are protected, and only the
    // prefix tells them apart.
    std::string transformObjectLiterals(const std::string &input, const std::string &placeholderPrefix) const;
    std::string transformMathBuiltins(const std::string &input) const;
    std::string transformDOMMethods(const std::string &input) const;
    std::string transformSemicolons(const std::string &input) const;
    std::string parenthesizeBitwiseOperands(const std::string &input) const;

    // Stage 3: Guard-specific truthiness wrapping
    std::string wrapTruthiness(const std::string &input) const;
    bool needsTruthinessWrapping(const std::string &expr) const;

    // Expression transformation cache: same input + context always produces same output.
    // General and Guard contexts are cached separately (Guard adds truthiness wrapping).
    mutable std::unordered_map<std::string, std::string> generalCache_;
    mutable std::unordered_map<std::string, std::string> guardCache_;
    mutable std::unordered_map<std::string, std::string> scriptCache_;
};

}  // namespace SCE
