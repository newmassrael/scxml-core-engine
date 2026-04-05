"""
ECMAScript-to-Lua Expression Transformer

1:1 port of C++ EcmaScriptToLuaTransformer.cpp (1772 lines).
Transforms the ECMAScript subset used in W3C SCXML tests into equivalent Lua 5.4.

Used at **codegen time** by the Rust code generator so that generated Rust
embeds Lua-ready expression strings as literals. The Rust LuaEngine therefore
just compiles and executes Lua — it does not know about JavaScript syntax.

Transformation categories:
  - Operators: &&->and, ||->or, !->not, ===->==, !==->~=, !=->~=
  - Type checks: typeof x === 'undefined' -> x == nil
  - Null/undefined: null/undefined -> nil
  - Array literals: [1,2,3] -> {1,2,3}
  - String concat: + with strings -> .. (left to Lua runtime __add)
  - Array methods: .length->#, .indexOf()->_indexOf(), .concat()->_concat()
  - instanceof: x instanceof Array -> _isArray(x)
  - Function syntax: function(x){return y;} -> function(x) return y end
  - DOM methods: . -> : for getElementsByTagName/getAttribute/getTagName
  - Math builtins: Math.sqrt -> math.sqrt, Math.pow(a,b) -> (a)^(b)

W3C SCXML compliance: Covers all 202 W3C test expression patterns.
"""

from __future__ import annotations

from enum import Enum, auto
from typing import List, Tuple


class ExpressionContext(Enum):
    """How the expression is used — affects truthiness handling."""
    General = auto()   # Assignment, log, data init — no truthiness wrapping
    Guard = auto()     # Transition cond, if/elseif — may need truthiness wrapping


def _is_word_char(c: str) -> bool:
    """Equivalent to regex \\w (alphanumeric + underscore)."""
    return c.isalnum() or c == '_'


def _find_word(s: str, word: str, start: int = 0) -> int:
    """Find keyword with word boundaries (\\bword\\b). Returns -1 if not found."""
    wlen = len(word)
    pos = start
    while True:
        pos = s.find(word, pos)
        if pos == -1:
            return -1
        left_ok = (pos == 0 or not _is_word_char(s[pos - 1]))
        right_ok = (pos + wlen >= len(s) or not _is_word_char(s[pos + wlen]))
        if left_ok and right_ok:
            return pos
        pos += 1


def _replace_word(s: str, word: str, replacement: str) -> str:
    """Replace all word-bounded occurrences: \\bword\\b -> replacement."""
    wlen = len(word)
    result = []
    last = 0
    pos = 0
    while True:
        pos = _find_word(s, word, pos)
        if pos == -1:
            break
        result.append(s[last:pos])
        result.append(replacement)
        last = pos + wlen
        pos = last
    result.append(s[last:])
    return ''.join(result)


def _replace_word_at_top_level(s: str, word: str, replacement: str) -> str:
    """Replace word-bounded occurrences only at bracket depth 0."""
    wlen = len(word)
    result = []
    depth = 0
    i = 0
    while i < len(s):
        c = s[i]
        if c in '{([':
            depth += 1
            result.append(c)
            i += 1
        elif c in '})]':
            if depth > 0:
                depth -= 1
            result.append(c)
            i += 1
        elif (depth == 0 and
              i + wlen <= len(s) and
              s[i:i + wlen] == word and
              (i == 0 or not _is_word_char(s[i - 1])) and
              (i + wlen >= len(s) or not _is_word_char(s[i + wlen]))):
            result.append(replacement)
            i += wlen
        else:
            result.append(c)
            i += 1
    return ''.join(result)


def _replace_keyword_prefix(s: str, keyword: str, replacement: str) -> str:
    """Replace \\bkeyword\\s+ -> replacement (for var/let/const -> local/strip)."""
    kwlen = len(keyword)
    result = []
    last = 0
    pos = 0
    while True:
        pos = _find_word(s, keyword, pos)
        if pos == -1:
            break
        after_kw = pos + kwlen
        if after_kw < len(s) and s[after_kw].isspace():
            result.append(s[last:pos])
            result.append(replacement)
            # Consume all whitespace after keyword
            space_end = after_kw
            while space_end < len(s) and s[space_end].isspace():
                space_end += 1
            last = space_end
            pos = last
        else:
            pos += 1
    result.append(s[last:])
    return ''.join(result)


def _skip_spaces(s: str, pos: int) -> int:
    while pos < len(s) and s[pos].isspace():
        pos += 1
    return pos


def _read_word(s: str, pos: int) -> int:
    while pos < len(s) and _is_word_char(s[pos]):
        pos += 1
    return pos


def _trim(s: str) -> str:
    return s.strip(' \t')


def _find_matching_close(s: str, open_pos: int, open_ch: str, close_ch: str) -> int:
    depth = 1
    pos = open_pos + 1
    while pos < len(s) and depth > 0:
        if s[pos] == open_ch:
            depth += 1
        elif s[pos] == close_ch:
            depth -= 1
        if depth > 0:
            pos += 1
    return pos if depth == 0 else (len(s) - 1 if s else 0)


def _find_preceding_identifier(s: str, dot_pos: int) -> int:
    """Find start of identifier preceding dot_pos. Returns -1 if none."""
    if dot_pos == 0:
        return -1
    end = dot_pos
    start = dot_pos
    while start > 0 and _is_word_char(s[start - 1]):
        start -= 1
    return start if start < end else -1


def _is_pure_in_predicate(expr: str) -> bool:
    """Check if expression is a pure In() predicate chain."""
    i = 0
    length = len(expr)
    found_in = False
    while i < length:
        c = expr[i]
        if c.isspace() or c == '(' or c == ')':
            i += 1
            continue
        if i + 3 <= length and expr[i:i + 3] == 'In(':
            found_in = True
            i += 3
            while i < length and expr[i] != ')':
                i += 1
            if i < length:
                i += 1
            continue
        if i + 3 <= length and expr[i:i + 3] == 'and' and (i + 3 >= length or not _is_word_char(expr[i + 3])):
            i += 3
            continue
        if i + 2 <= length and expr[i:i + 2] == 'or' and (i + 2 >= length or not _is_word_char(expr[i + 2])):
            i += 2
            continue
        return False
    return found_in


class EcmaScriptToLuaTransformer:
    """
    Transforms ECMAScript expressions to Lua equivalents.

    1:1 port of C++ SCE::EcmaScriptToLuaTransformer.
    Thread-safe: uses per-instance caches only.
    """

    def __init__(self):
        self._general_cache: dict[str, str] = {}
        self._guard_cache: dict[str, str] = {}
        self._script_cache: dict[str, str] = {}

    def clear_cache(self):
        self._general_cache.clear()
        self._guard_cache.clear()
        self._script_cache.clear()

    def transform(self, ecma_script: str, context: ExpressionContext = ExpressionContext.General) -> str:
        """Transform an ECMAScript expression to Lua."""
        if not ecma_script:
            return ecma_script

        cache = self._guard_cache if context == ExpressionContext.Guard else self._general_cache
        if ecma_script in cache:
            return cache[ecma_script]

        # Pre-pass: typeof patterns must be transformed BEFORE string protection
        pre_processed = self._transform_typeof_patterns(ecma_script)

        # Stage 1: Protect string literals
        processed, literals = self._protect_string_literals(pre_processed)

        # Stage 2: Apply transformation pipeline (order matters)
        processed = self._transform_math_builtins(processed)
        processed = self._transform_compound_assignment(processed)
        processed = self._transform_increment_decrement(processed)
        processed = self._transform_instanceof_patterns(processed)
        processed = self._transform_array_literals(processed)
        processed = self._transform_null_undefined(processed)
        processed = self._transform_new_expression(processed)
        processed = self._transform_array_methods(processed)
        processed = self._transform_array_indexing(processed)
        processed = self._transform_object_literals(processed)
        processed = self._transform_ternary_operator(processed)
        processed = self._transform_operators(processed)
        processed = self._transform_dom_methods(processed)
        processed = self._transform_var_declarations(processed)
        processed = self._transform_semicolons(processed)

        # Stage 3: Restore string literals
        result = self._restore_string_literals(processed, literals)

        # Stage 4: Guard-specific truthiness wrapping
        if context == ExpressionContext.Guard:
            result = self._wrap_truthiness(result)

        cache[ecma_script] = result
        return result

    def transform_script(self, script: str) -> str:
        """Transform a multi-line script block."""
        if not script:
            return script

        if script in self._script_cache:
            return self._script_cache[script]

        # Pre-pass: typeof before string protection
        pre_processed = self._transform_typeof_patterns(script)

        # Stage 1: Protect string literals
        processed, literals = self._protect_string_literals(pre_processed)

        # Stage 2: Structural transforms (must see original JS syntax with semicolons)
        processed = self._transform_for_in_loops(processed)
        processed = self._transform_for_loops(processed)

        # Stage 3: Apply transformations
        processed = self._transform_math_builtins(processed)
        processed = self._transform_compound_assignment(processed)
        processed = self._transform_increment_decrement(processed)
        processed = self._transform_function_syntax(processed)
        processed = self._transform_instanceof_patterns(processed)
        processed = self._transform_array_literals(processed)
        processed = self._transform_null_undefined(processed)
        processed = self._transform_new_expression(processed)
        processed = self._transform_array_methods(processed)
        processed = self._transform_array_indexing(processed)
        processed = self._transform_object_literals(processed)
        processed = self._transform_ternary_operator(processed)
        processed = self._transform_operators(processed)
        processed = self._transform_dom_methods(processed)
        processed = self._transform_var_declarations(processed)
        processed = self._transform_semicolons(processed)
        processed = self._transform_conditional_blocks(processed)
        processed = self._transform_bare_expressions(processed)

        # Stage 3: Restore string literals
        result = self._restore_string_literals(processed, literals)

        self._script_cache[script] = result
        return result

    # ── Stage 1: String Literal Protection ──

    def _protect_string_literals(self, input_str: str) -> Tuple[str, List[str]]:
        literals: List[str] = []
        output: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            c = input_str[i]

            # Strip JS block comments /* ... */
            if c == '/' and i + 1 < length and input_str[i + 1] == '*':
                i += 2
                while i + 1 < length and not (input_str[i] == '*' and input_str[i + 1] == '/'):
                    i += 1
                if i + 1 < length:
                    i += 1  # skip closing '/'
                i += 1
                continue

            # Strip JS line comments // ...
            if c == '/' and i + 1 < length and input_str[i + 1] == '/':
                i += 2
                while i < length and input_str[i] != '\n':
                    i += 1
                if i < length:
                    output.append('\n')
                i += 1
                continue

            if c in ("'", '"'):
                quote = c
                literal = [c]
                i += 1
                while i < length:
                    if input_str[i] == '\\' and i + 1 < length:
                        literal.append(input_str[i])
                        literal.append(input_str[i + 1])
                        i += 2
                        continue
                    if input_str[i] == quote:
                        literal.append(input_str[i])
                        break
                    literal.append(input_str[i])
                    i += 1

                idx = len(literals)
                literals.append(''.join(literal))
                output.append(f'\x01STR{idx}\x01')
            else:
                output.append(c)
            i += 1

        return ''.join(output), literals

    def _restore_string_literals(self, processed: str, literals: List[str]) -> str:
        result = processed
        for i, literal in enumerate(literals):
            placeholder = f'\x01STR{i}\x01'
            result = result.replace(placeholder, literal)
        return result

    # ── Pre-pass: typeof Transformation ──

    def _transform_typeof_patterns(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            typeof_pos = input_str.find('typeof', i)
            if typeof_pos == -1:
                result.append(input_str[i:])
                break

            # Check word boundary
            if typeof_pos > 0 and _is_word_char(input_str[typeof_pos - 1]):
                result.append(input_str[i:typeof_pos + 6])
                i = typeof_pos + 6
                continue

            result.append(input_str[i:typeof_pos])

            j = _skip_spaces(input_str, typeof_pos + 6)

            has_paren = j < length and input_str[j] == '('
            if has_paren:
                j = _skip_spaces(input_str, j + 1)

            var_start = j
            var_end = _read_word(input_str, j)

            if var_end == var_start:
                result.append('typeof')
                i = typeof_pos + 6
                continue

            var_name = input_str[var_start:var_end]

            after_typeof_expr = _skip_spaces(input_str, var_end)
            if has_paren and after_typeof_expr < length and input_str[after_typeof_expr] == ')':
                after_typeof_expr += 1

            # Check for comparison operator
            k = _skip_spaces(input_str, after_typeof_expr)
            op_len = 0
            is_neg = False

            if k + 2 < length and input_str[k:k + 3] == '!==':
                op_len = 3; is_neg = True
            elif k + 2 < length and input_str[k:k + 3] == '===':
                op_len = 3; is_neg = False
            elif k + 1 < length and input_str[k:k + 2] == '!=':
                op_len = 2; is_neg = True
            elif k + 1 < length and input_str[k:k + 2] == '==':
                op_len = 2; is_neg = False

            if op_len > 0:
                m = _skip_spaces(input_str, k + op_len)
                if m < length and input_str[m] in ("'", '"'):
                    quote = input_str[m]
                    m += 1
                    type_start = m
                    while m < length and input_str[m] != quote:
                        m += 1
                    type_str = input_str[type_start:m]
                    if m < length:
                        m += 1

                    lua_op = '~=' if is_neg else '=='

                    if type_str == 'undefined':
                        result.append(f'{var_name} {lua_op} nil')
                    else:
                        lua_type = 'table' if type_str == 'object' else type_str
                        result.append(f"type({var_name}) {lua_op} '{lua_type}'")
                    i = m
                    continue

            # Standalone typeof -> _typeof(varName)
            result.append(f'_typeof({var_name})')
            i = after_typeof_expr

        return ''.join(result)

    # ── Stage 2: Pattern Transformations ──

    def _transform_compound_assignment(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            eq_pos = input_str.find('=', i)
            if eq_pos == -1 or eq_pos == 0:
                result.append(input_str[i:])
                break

            prev = input_str[eq_pos - 1]
            if prev in ('=', '!', '<', '>'):
                result.append(input_str[i:eq_pos + 1])
                i = eq_pos + 1
                continue

            if prev not in ('+', '-', '*', '/', '%'):
                result.append(input_str[i:eq_pos + 1])
                i = eq_pos + 1
                continue

            op_pos = eq_pos - 1

            var_end = op_pos
            while var_end > i and input_str[var_end - 1].isspace():
                var_end -= 1
            var_start = var_end
            while var_start > i and _is_word_char(input_str[var_start - 1]):
                var_start -= 1

            if var_start == var_end:
                result.append(input_str[i:eq_pos + 1])
                i = eq_pos + 1
                continue

            var_name = input_str[var_start:var_end]
            op = prev

            rhs_start = eq_pos + 1
            while rhs_start < length and input_str[rhs_start].isspace():
                rhs_start += 1
            rhs_end = rhs_start
            while rhs_end < length and input_str[rhs_end] not in (';', '\n'):
                rhs_end += 1

            rhs = input_str[rhs_start:rhs_end]

            result.append(input_str[i:var_start])
            result.append(f'{var_name} = {var_name} {op} ({rhs})')
            i = rhs_end

        return ''.join(result)

    def _transform_increment_decrement(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            if input_str[i] == '+' and i + 1 < length and input_str[i + 1] == '+':
                # Postfix: var++
                var_end = i
                var_start = i
                while var_start > 0 and _is_word_char(input_str[var_start - 1]):
                    var_start -= 1

                if var_start < var_end:
                    # Remove already-appended var from result
                    result_str = ''.join(result)
                    result_str = result_str[:len(result_str) - (var_end - var_start)]
                    result.clear()
                    result.append(result_str)
                    var = input_str[var_start:var_end]
                    result.append(f'(function() local _t = {var} {var} = {var} + 1 return _t end)()')
                    i += 2
                    continue

                # Prefix: ++var
                after_op = i + 2
                word_end = _read_word(input_str, after_op)
                if word_end > after_op:
                    var = input_str[after_op:word_end]
                    result.append(f'(function() {var} = {var} + 1 return {var} end)()')
                    i = word_end
                    continue

            if input_str[i] == '-' and i + 1 < length and input_str[i + 1] == '-':
                var_end = i
                var_start = i
                while var_start > 0 and _is_word_char(input_str[var_start - 1]):
                    var_start -= 1

                if var_start < var_end:
                    result_str = ''.join(result)
                    result_str = result_str[:len(result_str) - (var_end - var_start)]
                    result.clear()
                    result.append(result_str)
                    var = input_str[var_start:var_end]
                    result.append(f'(function() local _t = {var} {var} = {var} - 1 return _t end)()')
                    i += 2
                    continue

                after_op = i + 2
                word_end = _read_word(input_str, after_op)
                if word_end > after_op:
                    var = input_str[after_op:word_end]
                    result.append(f'(function() {var} = {var} - 1 return {var} end)()')
                    i = word_end
                    continue

            result.append(input_str[i])
            i += 1

        return ''.join(result)

    def _transform_instanceof_patterns(self, input_str: str) -> str:
        result: List[str] = []
        i = 0

        while i < len(input_str):
            pos = _find_word(input_str, 'instanceof', i)
            if pos == -1:
                result.append(input_str[i:])
                break

            after_inst = _skip_spaces(input_str, pos + 10)
            array_end = _read_word(input_str, after_inst)
            if array_end > after_inst and input_str[after_inst:array_end] == 'Array':
                expr_end = pos
                while expr_end > i and input_str[expr_end - 1].isspace():
                    expr_end -= 1

                if expr_end > i and input_str[expr_end - 1] == ')':
                    depth = 1
                    k = expr_end - 2
                    while k > i and depth > 0:
                        if input_str[k] == ')':
                            depth += 1
                        elif input_str[k] == '(':
                            depth -= 1
                        if depth > 0:
                            k -= 1
                    expr_start = k
                else:
                    expr_start = expr_end
                    while expr_start > i and _is_word_char(input_str[expr_start - 1]):
                        expr_start -= 1

                expr = input_str[expr_start:expr_end]
                result.append(input_str[i:expr_start])
                result.append(f'_isArray({expr})')
                i = array_end
            else:
                result.append(input_str[i:pos + 10])
                i = pos + 10

        return ''.join(result)

    def _transform_null_undefined(self, input_str: str) -> str:
        result = _replace_word(input_str, 'undefined', 'nil')
        return _replace_word(result, 'null', 'nil')

    def _transform_operators(self, input_str: str) -> str:
        result = input_str

        # Order matters: !== before !=, === before ==  (1:1 with C++)

        # !== -> ~=
        pos = 0
        while True:
            pos = result.find('!==', pos)
            if pos == -1:
                break
            result = result[:pos] + '~=' + result[pos + 3:]
            pos += 2

        # === -> ==
        pos = 0
        while True:
            pos = result.find('===', pos)
            if pos == -1:
                break
            result = result[:pos] + '==' + result[pos + 3:]
            pos += 2

        # != -> ~= (must not re-match already-converted ~=)  (1:1 with C++)
        temp = []
        i = 0
        while i < len(result):
            if result[i] == '!' and i + 1 < len(result) and result[i + 1] == '=':
                temp.append('~=')
                i += 2  # skip both ! and =
            else:
                temp.append(result[i])
                i += 1
        result = ''.join(temp)

        # && -> and
        result = result.replace('&&', ' and ')
        # || -> or
        result = result.replace('||', ' or ')

        # Logical NOT: !expr -> not expr (must not match ~=)  (1:1 with C++)
        temp = []
        i = 0
        while i < len(result):
            if result[i] == '!':
                if i + 1 < len(result) and result[i + 1] == '=':
                    temp.append(result[i])
                else:
                    temp.append('not ')
            else:
                temp.append(result[i])
            i += 1
        result = ''.join(temp)

        # W3C SCXML B.2 (test 459): Parenthesize operands of bitwise OR/AND/XOR
        result = self._parenthesize_bitwise_operands(result)

        return result

    def _parenthesize_bitwise_operands(self, input_str: str) -> str:
        has_bitwise = any(c in '|&^' for c in input_str)
        if not has_bitwise:
            return input_str

        has_comparison = ('==' in input_str or '~=' in input_str or
                          '<=' in input_str or '>=' in input_str)
        if not has_comparison:
            for i, c in enumerate(input_str):
                if c in '<>' and (i + 1 >= len(input_str) or input_str[i + 1] != '='):
                    has_comparison = True
                    break
        if not has_comparison:
            return input_str

        parts = []
        start = 0
        for i, c in enumerate(input_str):
            if c in '|&^':
                operand = _trim(input_str[start:i])
                parts.append(f'({operand}) {c} ')
                start = i + 1
        last_op = _trim(input_str[start:])
        parts.append(f'({last_op})')
        return ''.join(parts)

    def _transform_array_literals(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            if input_str[i] == '[':
                is_property_access = False
                if i > 0:
                    prev = input_str[i - 1]
                    if prev.isalnum() or prev in ('_', ')', ']', '\x01'):
                        is_property_access = True

                if not is_property_access:
                    close_pos = _find_matching_close(input_str, i, '[', ']')
                    contents = input_str[i + 1:close_pos]
                    contents = self._transform_array_literals(contents)
                    # W3C SCXML 4.6: Replace null/undefined with sentinels at top level
                    contents = _replace_word_at_top_level(contents, 'null', '_NULL')
                    contents = _replace_word_at_top_level(contents, 'undefined', '_UNDEFINED')
                    result.append('{' + contents + '}')
                    i = close_pos + 1
                else:
                    result.append(input_str[i])
                    i += 1
            else:
                result.append(input_str[i])
                i += 1

        return ''.join(result)

    def _transform_array_indexing(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            if input_str[i] == '[':
                is_property_access = False
                if i > 0:
                    prev = input_str[i - 1]
                    if prev.isalnum() or prev in ('_', ')', ']', '\x01'):
                        is_property_access = True

                if is_property_access:
                    end = _find_matching_close(input_str, i, '[', ']')
                    index_expr = _trim(input_str[i + 1:end])

                    is_int_literal = (index_expr and len(index_expr) <= 9 and
                                      index_expr.isdigit())

                    if is_int_literal:
                        idx = int(index_expr)
                        result.append(f'[{idx + 1}]')
                        i = end + 1
                    else:
                        result.append(input_str[i])
                        i += 1
                else:
                    result.append(input_str[i])
                    i += 1
            else:
                result.append(input_str[i])
                i += 1

        return ''.join(result)

    def _transform_array_methods(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            if input_str[i] == '.':
                result_str = ''.join(result)

                # .length\b -> # prefix
                if (i + 7 <= length and input_str[i + 1:i + 7] == 'length' and
                        (i + 7 >= length or not _is_word_char(input_str[i + 7]))):
                    id_start = _find_preceding_identifier(result_str, len(result_str))
                    if id_start != -1:
                        var_name = result_str[id_start:]
                        result.clear()
                        result.append(result_str[:id_start])
                        result.append('#' + var_name)
                        i += 7
                        continue

                # .indexOf( -> _indexOf(var,
                if i + 9 <= length and input_str[i + 1:i + 9] == 'indexOf(':
                    id_start = _find_preceding_identifier(result_str, len(result_str))
                    if id_start != -1:
                        var_name = result_str[id_start:]
                        result.clear()
                        result.append(result_str[:id_start])
                        paren_open = i + 8
                        arg_end = _find_matching_close(input_str, paren_open, '(', ')')
                        args = input_str[paren_open + 1:arg_end]
                        result.append(f'_indexOf({var_name}, {args})')
                        i = arg_end + 1
                        continue

                # .concat( -> _concat(var,
                if i + 8 <= length and input_str[i + 1:i + 8] == 'concat(':
                    id_start = _find_preceding_identifier(result_str, len(result_str))
                    if id_start != -1:
                        var_name = result_str[id_start:]
                        result.clear()
                        result.append(result_str[:id_start])
                        result.append(f'_concat({var_name}, ')
                        i += 8
                        continue
                    # Handle {}.concat( or [].concat(
                    if len(result_str) >= 2:
                        last2 = result_str[-2:]
                        if last2 in ('{}', '[]'):
                            result.clear()
                            result.append(result_str[:-2])
                            result.append(f'_concat({last2}, ')
                            i += 8
                            continue

                # .push( -> table.insert(var,
                if i + 6 <= length and input_str[i + 1:i + 6] == 'push(':
                    id_start = _find_preceding_identifier(result_str, len(result_str))
                    if id_start != -1:
                        var_name = result_str[id_start:]
                        result.clear()
                        result.append(result_str[:id_start])
                        paren_open = i + 5
                        arg_end = _find_matching_close(input_str, paren_open, '(', ')')
                        args = input_str[paren_open + 1:arg_end]
                        result.append(f'table.insert({var_name}, {args})')
                        i = arg_end + 1
                        continue

                # .join( -> table.concat(var,
                if i + 6 <= length and input_str[i + 1:i + 6] == 'join(':
                    id_start = _find_preceding_identifier(result_str, len(result_str))
                    if id_start != -1:
                        var_name = result_str[id_start:]
                        result.clear()
                        result.append(result_str[:id_start])
                        paren_open = i + 5
                        arg_end = _find_matching_close(input_str, paren_open, '(', ')')
                        args = input_str[paren_open + 1:arg_end]
                        if not args:
                            args = "','"
                        result.append(f'table.concat({var_name}, {args})')
                        i = arg_end + 1
                        continue

            result.append(input_str[i])
            i += 1

        return ''.join(result)

    def _transform_function_syntax(self, input_str: str) -> str:
        """Transform JS function syntax to Lua."""

        class FuncInfo:
            def __init__(self, depth: int, is_constructor: bool):
                self.depth = depth
                self.is_constructor = is_constructor

        output: List[str] = []
        i = 0
        func_stack: List[FuncInfo] = []
        brace_depth = 0
        length = len(input_str)

        while i < length:
            # Detect function keyword
            if (i + 8 <= length and input_str[i:i + 8] == 'function' and
                    (i == 0 or not _is_word_char(input_str[i - 1]))):
                func_start = i
                j = i + 8
                while j < length and (input_str[j].isspace() or _is_word_char(input_str[j])):
                    j += 1

                if j < length and input_str[j] == '(':
                    paren_depth = 1
                    j += 1
                    while j < length and paren_depth > 0:
                        if input_str[j] == '(':
                            paren_depth += 1
                        elif input_str[j] == ')':
                            paren_depth -= 1
                        j += 1

                    while j < length and input_str[j].isspace():
                        j += 1

                    if j < length and input_str[j] == '{':
                        # Detect JS constructor pattern (this.)
                        is_constructor = False
                        check_depth = 1
                        for k in range(j + 1, length):
                            if input_str[k] == '{':
                                check_depth += 1
                            elif input_str[k] == '}':
                                check_depth -= 1
                            if check_depth <= 0:
                                break
                            if (check_depth == 1 and k + 5 <= length and
                                    input_str[k:k + 5] == 'this.'):
                                is_constructor = True
                                break

                        output.append(input_str[func_start:j])
                        if is_constructor:
                            output.append(' local self = {} ')
                        else:
                            output.append(' ')
                        func_stack.append(FuncInfo(brace_depth, is_constructor))
                        brace_depth += 1
                        i = j + 1
                        continue

            if input_str[i] == '{':
                brace_depth += 1
                output.append(input_str[i])
            elif input_str[i] == '}':
                brace_depth -= 1
                if func_stack and brace_depth == func_stack[-1].depth:
                    if func_stack[-1].is_constructor:
                        output.append(' return self end')
                    else:
                        output.append(' end')
                    func_stack.pop()
                else:
                    output.append(input_str[i])
            else:
                output.append(input_str[i])
            i += 1

        result = ''.join(output)

        # Transform this.prop -> self.prop
        pos = 0
        while True:
            pos = _find_word(result, 'this', pos)
            if pos == -1:
                break
            if pos + 4 < len(result) and result[pos + 4] == '.':
                result = result[:pos] + 'self' + result[pos + 4:]
            pos += 4

        return result

    def _transform_var_declarations(self, input_str: str) -> str:
        result = _replace_keyword_prefix(input_str, 'var', '')
        result = _replace_keyword_prefix(result, 'let', '')
        return _replace_keyword_prefix(result, 'const', '')

    def _transform_new_expression(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            pos = _find_word(input_str, 'new', i)
            if pos == -1:
                result.append(input_str[i:])
                break

            result.append(input_str[i:pos])

            j = _skip_spaces(input_str, pos + 3)
            name_end = _read_word(input_str, j)
            if name_end > j:
                k = _skip_spaces(input_str, name_end)
                if k < length and input_str[k] == '(':
                    result.append(input_str[j:name_end])
                    result.append('(')
                    i = k + 1
                    continue

            result.append('new')
            i = pos + 3

        return ''.join(result)

    def _transform_ternary_operator(self, input_str: str) -> str:
        q_pos = input_str.find('?')
        if q_pos == -1:
            return input_str
        c_pos = input_str.find(':', q_pos + 1)
        if c_pos == -1:
            return input_str

        cond = _trim(input_str[:q_pos])
        true_val = _trim(input_str[q_pos + 1:c_pos])
        false_val = _trim(input_str[c_pos + 1:])

        if not cond or not true_val or not false_val:
            return input_str

        return f'({cond} and {true_val} or {false_val})'

    def _transform_dom_methods(self, input_str: str) -> str:
        result = input_str
        for method in ('.getElementsByTagName(', '.getAttribute(', '.getTagName('):
            result = result.replace(method, ':' + method[1:])
        return result

    def _transform_object_literals(self, input_str: str) -> str:
        result: List[str] = []
        brace_depth = 0
        ternary_count = 0
        ternary_stack: List[int] = []

        for i, c in enumerate(input_str):
            if c == '{':
                brace_depth += 1
                ternary_stack.append(ternary_count)
                ternary_count = 0
                result.append(c)
            elif c == '}':
                brace_depth -= 1
                if ternary_stack:
                    ternary_count = ternary_stack.pop()
                result.append(c)
            elif c == '?' and brace_depth > 0:
                ternary_count += 1
                result.append(c)
            elif c == ':' and brace_depth > 0:
                if ternary_count > 0:
                    ternary_count -= 1
                    result.append(c)
                else:
                    # Check if preceded by identifier or string placeholder (object key)
                    key_end = i
                    while key_end > 0 and input_str[key_end - 1] in (' ', '\t'):
                        key_end -= 1
                    if key_end > 0 and (input_str[key_end - 1].isalnum() or input_str[key_end - 1] == '_'):
                        result.append(' =')
                    elif key_end > 0 and input_str[key_end - 1] == '\x01':
                        # JSON string key placeholder
                        placeholder_end = key_end
                        placeholder_start = key_end - 1
                        while placeholder_start > 0 and input_str[placeholder_start - 1] != '\x01':
                            placeholder_start -= 1
                        if placeholder_start > 0:
                            placeholder_start -= 1
                        placeholder = input_str[placeholder_start:placeholder_end]
                        result_str = ''.join(result)
                        ph_pos = result_str.rfind(placeholder)
                        if ph_pos != -1:
                            result.clear()
                            result.append(result_str[:ph_pos])
                            result.append('[' + placeholder + ']')
                            result.append(result_str[ph_pos + len(placeholder):])
                        result.append(' =')
                    else:
                        result.append(c)
            else:
                result.append(c)

        return ''.join(result)

    def _transform_semicolons(self, input_str: str) -> str:
        result = input_str.rstrip(';')
        result = result.replace(';', '\n')
        return result

    def _transform_math_builtins(self, input_str: str) -> str:
        result = input_str

        # Math.pow(a, b) -> (a)^(b)
        temp: List[str] = []
        i = 0
        while i < len(result):
            pos = result.find('Math.pow(', i)
            if pos == -1:
                temp.append(result[i:])
                break
            temp.append(result[i:pos])

            arg_start = pos + 9
            depth = 1
            comma_pos = -1
            j = arg_start
            while j < len(result) and depth > 0:
                if result[j] == '(':
                    depth += 1
                elif result[j] == ')':
                    depth -= 1
                    if depth == 0:
                        break
                elif result[j] == ',' and depth == 1 and comma_pos == -1:
                    comma_pos = j
                j += 1
            if depth == 0 and comma_pos != -1:
                a = _trim(result[arg_start:comma_pos])
                b = _trim(result[comma_pos + 1:j])
                temp.append(f'({a})^({b})')
                i = j + 1
            else:
                temp.append('Math.pow(')
                i = arg_start
        result = ''.join(temp)

        # Direct mappings
        replacements = [
            ('Math.sqrt', 'math.sqrt'), ('Math.abs', 'math.abs'),
            ('Math.floor', 'math.floor'), ('Math.ceil', 'math.ceil'),
            ('Math.max', 'math.max'), ('Math.min', 'math.min'),
            ('Math.random', 'math.random'), ('Math.log', 'math.log'),
            ('Math.exp', 'math.exp'), ('Math.sin', 'math.sin'),
            ('Math.cos', 'math.cos'), ('Math.tan', 'math.tan'),
            ('Math.PI', 'math.pi'), ('Math.E', 'math.exp(1)'),
        ]
        for from_str, to_str in replacements:
            result = result.replace(from_str, to_str)

        return result

    # ── Structural transforms (script-only) ──

    def _transform_for_in_loops(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            for_pos = _find_word(input_str, 'for', i)
            if for_pos == -1:
                result.append(input_str[i:])
                break
            result.append(input_str[i:for_pos])
            i = for_pos + 3

            paren_start = _skip_spaces(input_str, i)
            if paren_start >= length or input_str[paren_start] != '(':
                result.append('for')
                continue

            paren_end = _find_matching_close(input_str, paren_start, '(', ')')
            header = _trim(input_str[paren_start + 1:paren_end])

            # Strip var/let/const
            header_stripped = header
            for kw in ('var ', 'let ', 'const '):
                if header_stripped.startswith(kw):
                    header_stripped = header_stripped[len(kw):]
                    break
            header_stripped = _trim(header_stripped)

            # Check for " in " keyword
            in_pos = _find_word(header_stripped, 'in', 0)
            is_for_in = False
            loop_var = ''
            obj_expr = ''
            if in_pos != -1:
                before_in = _trim(header_stripped[:in_pos])
                after_in = _trim(header_stripped[in_pos + 2:])
                is_single_ident = bool(before_in) and all(_is_word_char(c) for c in before_in)
                if is_single_ident and after_in:
                    loop_var = before_in
                    obj_expr = after_in
                    is_for_in = True

            if not is_for_in:
                result.append('for')
                i = for_pos + 3
                continue

            i = paren_end + 1

            body_start = _skip_spaces(input_str, i)
            if body_start >= length or input_str[body_start] != '{':
                result.append(f'for {loop_var}, _ in pairs({obj_expr}) do end\n')
                continue
            body_end = _find_matching_close(input_str, body_start, '{', '}')
            body = _trim(input_str[body_start + 1:body_end])
            i = body_end + 1

            result.append(f'for {loop_var}, _ in pairs({obj_expr}) do\n{body}\nend\n')

        return ''.join(result)

    def _transform_for_loops(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            for_pos = _find_word(input_str, 'for', i)
            if for_pos == -1:
                result.append(input_str[i:])
                break
            result.append(input_str[i:for_pos])
            i = for_pos + 3

            paren_start = _skip_spaces(input_str, i)
            if paren_start >= length or input_str[paren_start] != '(':
                result.append('for')
                continue

            # Parse three semicolon-separated parts
            paren_depth = 1
            pos = paren_start + 1
            semi_positions: List[int] = []
            while pos < length and paren_depth > 0:
                if input_str[pos] == '(':
                    paren_depth += 1
                elif input_str[pos] == ')':
                    paren_depth -= 1
                elif input_str[pos] == ';' and paren_depth == 1:
                    semi_positions.append(pos)
                if paren_depth > 0:
                    pos += 1

            if len(semi_positions) != 2:
                result.append('for')
                i = for_pos + 3
                continue

            init = _trim(input_str[paren_start + 1:semi_positions[0]])
            cond = _trim(input_str[semi_positions[0] + 1:semi_positions[1]])
            incr = _trim(input_str[semi_positions[1] + 1:pos])
            i = pos + 1

            # Simplify i++/i-- -> i = i + 1 / i = i - 1
            if len(incr) >= 3:
                var_name = ''
                is_incr = is_decr = False
                if incr.endswith('++'):
                    var_name = _trim(incr[:-2])
                    is_incr = True
                elif incr.startswith('++'):
                    var_name = _trim(incr[2:])
                    is_incr = True
                elif incr.endswith('--'):
                    var_name = _trim(incr[:-2])
                    is_decr = True
                elif incr.startswith('--'):
                    var_name = _trim(incr[2:])
                    is_decr = True
                if is_incr:
                    incr = f'{var_name} = {var_name} + 1'
                elif is_decr:
                    incr = f'{var_name} = {var_name} - 1'

            # Find '{' for body
            body_start = _skip_spaces(input_str, i)
            if body_start >= length or input_str[body_start] != '{':
                result.append(input_str[for_pos:i])
                continue

            body_end = _find_matching_close(input_str, body_start, '{', '}')
            body = _trim(input_str[body_start + 1:body_end])
            i = body_end + 1

            # Detect common pattern: for (var i = 0; i < arr.length; i++)
            used_numeric_for = False
            init_trimmed = init
            for kw in ('let ', 'var ', 'const '):
                if init_trimmed.startswith(kw):
                    init_trimmed = init_trimmed[len(kw):]
                    break
            init_trimmed = _trim(init_trimmed)
            eq_pos = init_trimmed.find('=')
            if eq_pos != -1:
                loop_var = _trim(init_trimmed[:eq_pos])
                start_val = _trim(init_trimmed[eq_pos + 1:])
                if start_val == '0' and loop_var:
                    cond_pattern = f'{loop_var} < '
                    if cond.startswith(cond_pattern):
                        bound_expr = _trim(cond[len(cond_pattern):])
                        if len(bound_expr) > 7 and bound_expr.endswith('.length'):
                            arr_name = bound_expr[:-7]
                            result.append(f'for {loop_var} = 1, #{arr_name} do\n{body}\nend\n')
                            used_numeric_for = True

            if not used_numeric_for:
                result.append(f'{init}\nwhile {cond} do\n{body}\n{incr}\nend\n')

        return ''.join(result)

    def _transform_conditional_blocks(self, input_str: str) -> str:
        result: List[str] = []
        i = 0
        length = len(input_str)

        while i < length:
            if_pos = _find_word(input_str, 'if', i)
            if if_pos == -1:
                result.append(input_str[i:])
                break
            result.append(input_str[i:if_pos])
            i = if_pos + 2

            cond_start = _skip_spaces(input_str, i)
            if cond_start >= length or input_str[cond_start] != '(':
                result.append('if')
                continue

            cond_end = _find_matching_close(input_str, cond_start, '(', ')')
            condition = _trim(input_str[cond_start + 1:cond_end])
            i = cond_end + 1

            body_start = _skip_spaces(input_str, i)
            if body_start >= length or input_str[body_start] != '{':
                result.append(f'if ({condition})')
                i = body_start
                continue

            body_end = _find_matching_close(input_str, body_start, '{', '}')
            body = _trim(input_str[body_start + 1:body_end])
            i = body_end + 1

            result.append(f'if {condition} then\n{body}\n')

            # Check for else/else if
            while i < length:
                else_pos = _skip_spaces(input_str, i)
                if (else_pos + 4 <= length and input_str[else_pos:else_pos + 4] == 'else' and
                        (else_pos + 4 >= length or not _is_word_char(input_str[else_pos + 4]))):
                    i = else_pos + 4
                    after_else = _skip_spaces(input_str, i)

                    # "else if" -> "elseif"
                    if (after_else + 2 <= length and input_str[after_else:after_else + 2] == 'if' and
                            (after_else + 2 >= length or not _is_word_char(input_str[after_else + 2]))):
                        i = after_else + 2
                        ei_cond_start = _skip_spaces(input_str, i)
                        if ei_cond_start < length and input_str[ei_cond_start] == '(':
                            cond_end = _find_matching_close(input_str, ei_cond_start, '(', ')')
                            condition = _trim(input_str[ei_cond_start + 1:cond_end])
                            i = cond_end + 1

                            body_start = _skip_spaces(input_str, i)
                            if body_start < length and input_str[body_start] == '{':
                                body_end = _find_matching_close(input_str, body_start, '{', '}')
                                body = _trim(input_str[body_start + 1:body_end])
                                i = body_end + 1
                                result.append(f'elseif {condition} then\n{body}\n')
                                continue
                        i = else_pos + 4

                    # Plain "else { ... }"
                    body_start = _skip_spaces(input_str, i)
                    if body_start < length and input_str[body_start] == '{':
                        body_end = _find_matching_close(input_str, body_start, '{', '}')
                        body = _trim(input_str[body_start + 1:body_end])
                        i = body_end + 1
                        result.append(f'else\n{body}\n')
                break
            result.append('end\n')

        return ''.join(result)

    def _transform_bare_expressions(self, input_str: str) -> str:
        lines = input_str.split('\n')

        last_non_empty = -1
        for j in range(len(lines) - 1, -1, -1):
            if _trim(lines[j]):
                last_non_empty = j
                break

        result: List[str] = []
        for j, line in enumerate(lines):
            trimmed = _trim(line)
            if not trimmed or trimmed == 'end':
                result.append(line)
                continue

            is_statement = False

            # Check for assignment (but not ==, ~=, <=, >=)
            for k, c in enumerate(trimmed):
                if c == '=' and k > 0:
                    prev = trimmed[k - 1]
                    if prev not in ('~', '<', '>', '!'):
                        if k + 1 >= len(trimmed) or trimmed[k + 1] != '=':
                            is_statement = True
                            break

            if not is_statement:
                keywords = ('local', 'return', 'if', 'for', 'while', 'repeat',
                            'do', 'end', 'else', 'elseif', 'then', 'function', 'break')
                for kw in keywords:
                    kwlen = len(kw)
                    if (len(trimmed) >= kwlen and trimmed[:kwlen] == kw and
                            (len(trimmed) == kwlen or not _is_word_char(trimmed[kwlen]))):
                        is_statement = True
                        break

            if not is_statement:
                if '(' in trimmed:
                    is_statement = True

            if not is_statement:
                if j == last_non_empty:
                    lines[j] = 'return ' + trimmed
                else:
                    lines[j] = '_ = ' + trimmed

            result.append(lines[j])

        final = '\n'.join(result)
        return final.rstrip('\n')

    # ── Guard-specific truthiness wrapping ──

    def _needs_truthiness_wrapping(self, expr: str) -> bool:
        if any(op in expr for op in ('==', '~=', '>=', '<=')):
            return False
        if '_scxml_truthy' in expr or '_isArray' in expr:
            return False

        for i, c in enumerate(expr):
            if c in '<>' and (i + 1 >= len(expr) or expr[i + 1] != '='):
                return False

        trimmed = _trim(expr)
        if trimmed in ('true', 'false'):
            return False

        if 'In(' in trimmed and _is_pure_in_predicate(trimmed):
            return False

        if len(trimmed) >= 4 and trimmed[:4] == 'not ':
            return False

        return True

    def _wrap_truthiness(self, input_str: str) -> str:
        if not self._needs_truthiness_wrapping(input_str):
            return input_str
        return f'_scxml_truthy({input_str})'


# ── Module-level convenience singleton ──

_TRANSFORMER = EcmaScriptToLuaTransformer()


def transform_expression(expr: str, context: ExpressionContext = ExpressionContext.General) -> str:
    """Transform an ECMAScript expression to Lua using the module-level singleton."""
    return _TRANSFORMER.transform(expr, context)


def transform_guard(expr: str) -> str:
    """Transform a guard expression (with truthiness wrapping)."""
    return _TRANSFORMER.transform(expr, ExpressionContext.Guard)


def transform_script(script: str) -> str:
    """Transform a multi-line script block."""
    return _TRANSFORMER.transform_script(script)
