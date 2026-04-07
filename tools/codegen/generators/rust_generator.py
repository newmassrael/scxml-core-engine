"""
Rust Code Generator for W3C SCXML State Machines

Generates idiomatic Rust code (.rs) implementing `sce_rust_runtime::StatePolicy`
for each SCXML source. Extends BaseCodeGenerator with Rust-specific template
rendering.

Output (Phase 2+):
  - {name}_sm.rs : Single Rust file with State/Event enums + Policy struct

Phase 1 scope: skeleton only. Emits a placeholder file containing a stub
`impl StatePolicy` that compiles against the runtime crate but has no real
transition logic. Phases 2-4 fill in the Jinja2 templates to reach 202/202.

Source of truth: ports the C++ Jinja2 templates at `tools/codegen/templates/*.jinja2`
1:1. Uses C++ `StaticExecutionEngine` + `StatePolicy` as the runtime contract
reference, NOT Kotlin's `StateMachineEngine.kt`.
"""

import re
from pathlib import Path
from typing import Optional

from jinja2 import Environment

from generators.base import BaseCodeGenerator
from scxml_parser import SCXMLModel
from license_config import LICENSE_CONFIG
from ecmascript_to_lua import (
    EcmaScriptToLuaTransformer,
    ExpressionContext,
)


# Rust reserved keywords (2021 edition) — must be escaped with `r#` prefix
# when used as identifiers in generated code.
RUST_KEYWORDS = frozenset({
    'as', 'break', 'const', 'continue', 'crate', 'else', 'enum', 'extern',
    'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod',
    'move', 'mut', 'pub', 'ref', 'return', 'self', 'Self', 'static', 'struct',
    'super', 'trait', 'true', 'type', 'unsafe', 'use', 'where', 'while',
    'async', 'await', 'dyn',
    # Reserved for future use
    'abstract', 'become', 'box', 'do', 'final', 'macro', 'override', 'priv',
    'typeof', 'unsized', 'virtual', 'yield', 'try', 'union',
})


class RustCodeGenerator(BaseCodeGenerator):
    """
    Rust code generator using Jinja2 templates.

    Generates single-file `{name}_sm.rs` outputs implementing the
    `sce_rust_runtime::StatePolicy` trait. Mirrors the structural shape of
    `kotlin_generator.py` but emits Rust idioms (enums with `#[derive(...)]`,
    `match` expressions, free functions generic over policies).
    """

    LANGUAGE = 'rust'

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._lua_transformer = EcmaScriptToLuaTransformer()

    def _default_template_dir(self) -> Path:
        """Rust templates live in `templates/rust/`."""
        return Path(__file__).parent.parent / 'templates' / 'rust'

    def _register_filters(self, env: Environment):
        """Register Rust-specific Jinja2 filters."""
        env.filters['to_snake_case'] = self._to_snake_case
        env.filters['to_pascal_case'] = self._to_pascal_case
        env.filters['to_rust_type'] = self._to_rust_type
        env.filters['escape_rust'] = self._escape_rust_string
        env.filters['to_rust_string_expr'] = self._to_rust_string_expr
        env.filters['to_event_variant'] = self._to_event_variant
        env.filters['to_state_variant'] = self._to_state_variant
        env.filters['to_rust_literal'] = self._to_rust_literal
        env.filters['escape_keyword'] = self._escape_rust_keyword
        env.filters['to_in_predicate_rust'] = self._to_in_predicate_rust
        env.filters['normalize_ws'] = self._normalize_whitespace
        # W3C SCXML 6.4: Invoke child name -> Rust type name (same as PascalCase)
        env.filters['to_machine_name'] = self._to_pascal_case
        # ECMAScript→Lua codegen-time transformation filters
        env.filters['to_lua_expr'] = self._to_lua_expr
        env.filters['to_lua_guard'] = self._to_lua_guard
        env.filters['to_lua_script'] = self._to_lua_script

    # ──────────────────────────────────────────────
    # Jinja2 Filters
    # ──────────────────────────────────────────────

    @staticmethod
    def _to_snake_case(name: str) -> str:
        """
        Convert identifier to snake_case for Rust function/variable/module names.

        Examples:
            "trackIndex"  -> "track_index"
            "MyState"     -> "my_state"
            "s0final"     -> "s0final"
            "error.execution" -> "error_execution"
        """
        if not name:
            return "empty"
        # Normalize separators
        name = re.sub(r'[.\-]', '_', name)
        # Insert underscore before uppercase letters (but not at start)
        result = re.sub(r'(?<!^)(?=[A-Z])', '_', name)
        return result.lower()

    @staticmethod
    def _to_pascal_case(name: str) -> str:
        """
        Convert identifier to PascalCase for Rust struct/enum/variant names.

        Examples:
            "stopped"       -> "Stopped"
            "playing"       -> "Playing"
            "my_state"      -> "MyState"
            "error"         -> "Error"
            "pass"          -> "Pass"
            "error.execution" -> "ErrorExecution"
        """
        if not name:
            return "Empty"
        parts = re.split(r'[._\-]', name)
        return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)

    @staticmethod
    def _to_rust_type(var_type: str) -> str:
        """
        Map SCXML variable type to Rust type.

        W3C SCXML 5.3: Datamodel variable type classification.
        """
        type_map = {
            'int': 'i64',
            'string': 'String',
            'bool': 'bool',
            'runtime': 'sce_rust_runtime::ScriptValue',
        }
        return type_map.get(var_type, 'sce_rust_runtime::ScriptValue')

    @staticmethod
    def _normalize_whitespace(text: str) -> str:
        """W3C SCXML B.2 test 558: Normalize whitespace (trim + collapse runs to single space)."""
        import re
        return re.sub(r'\s+', ' ', text.strip())

    @staticmethod
    def _escape_rust_string(text: str) -> str:
        """Escape characters for Rust string literals."""
        if not text:
            return ""
        text = text.replace('\\', '\\\\')
        text = text.replace('"', '\\"')
        text = text.replace('\n', '\\n')
        text = text.replace('\r', '\\r')
        text = text.replace('\t', '\\t')
        return text

    @staticmethod
    def _to_rust_string_expr(expr: str) -> str:
        """
        Convert an SCXML string expression to a Rust string expression.

        - Single-quoted SCXML string literals → double-quoted Rust with escaping.
        - Non-string expressions pass through (Phase 3 will transform ECMAScript).

        Examples:
            "'hello'"       -> '"hello"'
            "'hi \"x\"'"    -> '"hi \\"x\\""'
            "varName"       -> "varName"
        """
        if not expr:
            return '""'
        stripped = expr.strip()
        if len(stripped) >= 2 and stripped.startswith("'") and stripped.endswith("'"):
            inner = stripped[1:-1]
            inner = inner.replace('\\', '\\\\')
            inner = inner.replace('"', '\\"')
            return f'"{inner}"'
        return expr

    @staticmethod
    def _to_event_variant(event_name: str) -> str:
        """
        Convert dot-separated SCXML event name to Rust enum variant PascalCase.

        Rust enums are flat (no nested namespaces), so dotted names collapse into
        a single identifier with PascalCase word boundaries.

        Examples:
            "play"              -> "Play"
            "switch_on"         -> "SwitchOn"
            "error.execution"   -> "ErrorExecution"
            "done.state.s1"     -> "DoneStateS1"
            "done.invoke.inv1"  -> "DoneInvokeInv1"
        """
        if not event_name:
            return "Empty"
        parts = re.split(r'[._\-]', event_name)
        return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)

    @staticmethod
    def _to_state_variant(state_id: str) -> str:
        """
        Convert SCXML state ID to Rust enum variant PascalCase.

        Examples:
            "stopped"    -> "Stopped"
            "s1"         -> "S1"
            "pass"       -> "Pass"
            "my_state"   -> "MyState"
            "s0final"    -> "S0final"
        """
        if not state_id:
            return "Empty"
        parts = re.split(r'[_\-]', state_id)
        return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)

    @staticmethod
    def _escape_rust_keyword(name: str) -> str:
        """Prefix Rust keywords with `r#` to make them valid identifiers."""
        if name in RUST_KEYWORDS:
            return f'r#{name}'
        return name

    @staticmethod
    def _to_rust_literal(value) -> str:
        """
        Render a Python value as a Rust literal expression.

        Used for default values of datamodel variables in generated code.
        """
        if value is None:
            return 'None'
        if isinstance(value, bool):
            return 'true' if value else 'false'
        if isinstance(value, int):
            return str(value)
        if isinstance(value, float):
            return f'{value}_f64'
        if isinstance(value, str):
            escaped = RustCodeGenerator._escape_rust_string(value)
            return f'"{escaped}".to_string()'
        return 'Default::default()'

    @staticmethod
    def _to_in_predicate_rust(cond_cpp: str) -> str:
        """
        Transform C++ In() predicate code to Rust.

        W3C SCXML 5.9.2: The parser stores In() predicates as C++ code in
        the ``cond_cpp`` field. This filter converts them to Rust method
        calls on ``self``.

        Transformations:
            ``this->isStateActive("s1")``  -> ``self.is_state_active("s1")``
            ``isStateActive("s1")``        -> ``self.is_state_active("s1")``

        Boolean operators (``&&``, ``||``, ``!``) pass through unchanged
        since they are identical in C++ and Rust.

        Examples:
            >>> RustCodeGenerator._to_in_predicate_rust('isStateActive("s1")')
            'self.is_state_active("s1")'
            >>> RustCodeGenerator._to_in_predicate_rust('this->isStateActive("s1") && isStateActive("s2")')
            'self.is_state_active("s1") && self.is_state_active("s2")'
            >>> RustCodeGenerator._to_in_predicate_rust('!isStateActive("s1")')
            '!self.is_state_active("s1")'
        """
        if not cond_cpp:
            return ""
        # Strip the C++ this-> prefix first, then replace the method name
        result = cond_cpp.replace('this->', '')
        result = re.sub(
            r'isStateActive\("([^"]+)"\)',
            r'self.is_state_active("\1")',
            result,
        )
        return result

    # ──────────────────────────────────────────────
    # ECMAScript→Lua Transformation Filters
    # ──────────────────────────────────────────────

    def _to_lua_expr(self, expr: str) -> str:
        """Transform ECMAScript expression to Lua at codegen time (General context)."""
        if not expr:
            return ''
        return self._lua_transformer.transform(expr.rstrip(';'), ExpressionContext.General)

    def _to_lua_guard(self, expr: str) -> str:
        """Transform ECMAScript guard to Lua with truthiness wrapping."""
        if not expr:
            return 'true'
        return self._lua_transformer.transform(expr.rstrip(';'), ExpressionContext.Guard)

    def _to_lua_script(self, script: str) -> str:
        """Transform ECMAScript script block to Lua."""
        if not script:
            return ''
        return self._lua_transformer.transform_script(script)

    # ──────────────────────────────────────────────
    # Code Generation
    # ──────────────────────────────────────────────

    def _generate_output(self, model: SCXMLModel, scxml_path: str,
                         output_dir: str, depfile_path: Optional[str]) -> bool:
        """
        Generate a single Rust file (.rs) from the analyzed SCXML model.

        Rust templates access model fields directly (no pre-computed context needed).
        """
        machine_name = self._to_pascal_case(model.name)
        input_stem = Path(scxml_path).stem

        # Validate that model has an initial state (required for Rust codegen)
        if not model.initial:
            print(f"  Rust codegen: No initial state declared for '{model.name}'")
            return False

        # Load Rust state machine template
        template = self.env.get_template('state_machine.rs.jinja2')

        output = template.render(
            model=model,
            machine_name=machine_name,
            license_config=LICENSE_CONFIG,
        )

        # Write output
        output_path = Path(output_dir) / f"{input_stem}_sm.rs"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            f.write(output)

        print(f"  Generated: {output_path}")

        # CMake DEPFILE for incremental builds (parallel with kotlin/cpp generators)
        if depfile_path:
            self._write_depfile(depfile_path, [output_path])

        return True

    def _get_python_deps(self) -> list:
        """Track rust_generator.py as a CMake/Cargo dependency."""
        deps = super()._get_python_deps()
        deps.append(str(Path(__file__).resolve()))
        return deps

