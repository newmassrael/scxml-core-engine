"""
Kotlin Code Generator for W3C SCXML State Machines

Generates idiomatic Kotlin code (.kt) using sealed interfaces and coroutines.
Extends BaseCodeGenerator with Kotlin-specific template rendering.

Output:
  - {name}Sm.kt : Single Kotlin file with State/Event sealed interfaces + StateMachine class

Design: See KOTLIN_CODEGEN_DESIGN.md for architecture decisions.
"""

import re
from pathlib import Path
from typing import Any, Dict, List, Optional

from jinja2 import Environment

from generators.base import BaseCodeGenerator
from scxml_parser import SCXMLModel
from license_config import LICENSE_CONFIG


class KotlinCodeGenerator(BaseCodeGenerator):
    """
    Kotlin code generator using Jinja2 templates.

    Generates sealed interface hierarchies and StateMachineEngine subclasses
    for Kotlin/Android (Compose-ready, Coroutines-based).

    Phase 1: Pure static only (no script engine, no ECMAScript expressions).
    """

    LANGUAGE = 'kotlin'

    def _default_template_dir(self) -> Path:
        """Kotlin templates are in templates/kotlin/ directory."""
        return Path(__file__).parent.parent / 'templates' / 'kotlin'

    def _register_filters(self, env: Environment):
        """Register Kotlin-specific Jinja2 filters."""
        env.filters['to_pascal_case'] = self._to_pascal_case
        env.filters['to_camel_case'] = self._to_camel_case
        env.filters['to_kotlin_type'] = self._to_kotlin_type
        env.filters['escape_kotlin'] = self._escape_kotlin_string
        env.filters['to_kotlin_string_expr'] = self._to_kotlin_string_expr
        env.filters['to_event_class_name'] = self._to_event_class_name
        env.filters['to_state_class_name'] = self._to_state_class_name

    # ──────────────────────────────────────────────
    # Jinja2 Filters
    # ──────────────────────────────────────────────

    @staticmethod
    def _to_pascal_case(name: str) -> str:
        """
        Convert identifier to PascalCase for Kotlin class/object names.

        Examples:
            "stopped"       -> "Stopped"
            "playing"       -> "Playing"
            "my_state"      -> "MyState"
            "error"         -> "Error"
            "pass"          -> "Pass"
        """
        if not name:
            return "Empty"
        # Split on dots, underscores, hyphens
        parts = re.split(r'[._\-]', name)
        return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)

    @staticmethod
    def _to_camel_case(name: str) -> str:
        """
        Convert identifier to camelCase for Kotlin property names.

        Examples:
            "track_index"   -> "trackIndex"
            "myVar"         -> "myVar"
        """
        if not name:
            return ""
        parts = re.split(r'[._\-]', name)
        first = parts[0]
        rest = [p[0].upper() + p[1:] if p else '' for p in parts[1:]]
        return first + ''.join(rest)

    @staticmethod
    def _to_kotlin_type(var_type: str) -> str:
        """
        Map SCXML variable type to Kotlin type.

        W3C SCXML 5.3: Datamodel variable type classification.
        """
        type_map = {
            'int': 'Int',
            'string': 'String',
            'bool': 'Boolean',
            'runtime': 'Any',  # Phase 2: Script engine evaluated
        }
        return type_map.get(var_type, 'Any')

    @staticmethod
    def _to_kotlin_default(var: dict) -> str:
        """Return Kotlin default value for a datamodel variable."""
        var_type = var.get('type', 'runtime')
        expr = var.get('expr') or ''

        if var_type == 'int':
            return expr if expr else '0'
        elif var_type == 'string':
            # Strip surrounding quotes from SCXML expression
            if expr.startswith('"') and expr.endswith('"'):
                return expr
            return '""'
        elif var_type == 'bool':
            return expr if expr in ('true', 'false') else 'false'
        return 'null'

    @staticmethod
    def _escape_kotlin_string(text: str) -> str:
        """Escape Kotlin string literals."""
        if not text:
            return ""
        text = text.replace('\\', '\\\\')
        text = text.replace('"', '\\"')
        text = text.replace('\n', '\\n')
        text = text.replace('\r', '\\r')
        text = text.replace('\t', '\\t')
        text = text.replace('$', '\\$')
        return text

    @staticmethod
    def _to_kotlin_string_expr(expr: str) -> str:
        """
        Convert SCXML string expression to Kotlin string expression.

        Handles single-quoted string literals → double-quoted with proper escaping.
        Non-string expressions (variables, arithmetic) pass through unchanged.

        Examples:
            "'hello'"           -> '"hello"'
            "'hello \"world\"'" -> '"hello \\"world\\""'
            "varName"           -> "varName"
        """
        if not expr:
            return '""'
        stripped = expr.strip()
        if len(stripped) >= 2 and stripped.startswith("'") and stripped.endswith("'"):
            inner = stripped[1:-1]
            inner = inner.replace('\\', '\\\\')
            inner = inner.replace('"', '\\"')
            inner = inner.replace('$', '\\$')
            return f'"{inner}"'
        return expr

    @staticmethod
    def _to_event_class_name(event_name: str) -> str:
        """
        Convert dot-separated SCXML event name to Kotlin nested class reference.

        Dot separators become nested sealed interface access (.).
        Underscores/hyphens within segments become PascalCase boundaries.

        Examples:
            "play"              -> "Play"
            "switch_on"         -> "SwitchOn"
            "error.execution"   -> "Error.Execution"
            "done.state.s1"     -> "Done.State.S1"
        """
        if not event_name:
            return "Empty"
        dot_parts = event_name.split('.')
        result = []
        for dot_part in dot_parts:
            # Each dot segment is PascalCase from underscore/hyphen parts
            sub_parts = re.split(r'[_\-]', dot_part)
            pascal = ''.join(p[0].upper() + p[1:] if p else '' for p in sub_parts)
            result.append(pascal)
        return '.'.join(result)

    @staticmethod
    def _to_state_class_name(state_id: str) -> str:
        """
        Convert SCXML state ID to Kotlin PascalCase class name.

        Examples:
            "stopped"    -> "Stopped"
            "s1"         -> "S1"
            "pass"       -> "Pass"
            "my_state"   -> "MyState"
            "s0final"    -> "S0final"
        """
        if not state_id:
            return "Empty"
        # Split on underscores/hyphens for PascalCase
        parts = re.split(r'[_\-]', state_id)
        return ''.join(p[0].upper() + p[1:] if p else '' for p in parts)

    # ──────────────────────────────────────────────
    # Event Tree Builder
    # ──────────────────────────────────────────────

    @staticmethod
    def _build_event_tree(events: set) -> Dict[str, Any]:
        """
        Build hierarchical event tree from flat dot-separated event names.

        W3C SCXML 3.12.1: Event prefix matching via Kotlin sealed interface hierarchy.

        Input:  {"play", "pause", "stop", "error.execution", "error.communication"}
        Output: {
            "play":  {"_leaf": True},
            "pause": {"_leaf": True},
            "stop":  {"_leaf": True},
            "error": {
                "_leaf": False,
                "execution":    {"_leaf": True},
                "communication": {"_leaf": True}
            }
        }

        A node with "_leaf": True AND children means it's both a concrete event
        and a parent for prefix matching (e.g., "error" and "error.execution" both exist).
        """
        tree: Dict[str, Any] = {}

        for event_name in sorted(events):
            parts = event_name.split('.')
            node = tree
            for i, part in enumerate(parts):
                if part not in node:
                    node[part] = {'_leaf': False}
                if i == len(parts) - 1:
                    node[part]['_leaf'] = True
                node = node[part]

        return tree

    @staticmethod
    def _collect_leaf_events(tree: Dict[str, Any], prefix: str = '') -> List[str]:
        """Collect all leaf event names from event tree (fully qualified dot paths)."""
        leaves = []
        for key, value in tree.items():
            if key == '_leaf':
                continue
            full_name = f"{prefix}.{key}" if prefix else key
            if value.get('_leaf', False):
                leaves.append(full_name)
            leaves.extend(KotlinCodeGenerator._collect_leaf_events(value, full_name))
        return leaves

    @staticmethod
    def _render_event_tree(tree: Dict[str, Any], parent_type: str, indent: str = "    ") -> str:
        """
        Render event tree as Kotlin sealed interface hierarchy.

        Recursive Python method avoids Jinja2 macro recursion issues.
        """
        lines = []
        for key in sorted(tree.keys()):
            if key == '_leaf':
                continue
            node = tree[key]
            # PascalCase: split on underscore/hyphen within each segment
            sub_parts = re.split(r'[_\-]', key)
            class_name = ''.join(p[0].upper() + p[1:] if p else '' for p in sub_parts) if key else 'Empty'

            # Collect children (non-_leaf keys)
            children = [k for k in node.keys() if k != '_leaf']

            if children:
                # Branch node: sealed interface with nested children
                lines.append(f"{indent}sealed interface {class_name} : {parent_type} {{")
                if node.get('_leaf', False):
                    # Both a concrete event and a parent for prefix matching
                    lines.append(f"{indent}    data object Self : {class_name}")
                # Recurse into children
                child_lines = KotlinCodeGenerator._render_event_tree(
                    node, class_name, indent + "    "
                )
                if child_lines:
                    lines.append(child_lines)
                lines.append(f"{indent}}}")
            else:
                # Leaf node: data object
                lines.append(f"{indent}data object {class_name} : {parent_type}")

        return '\n'.join(lines)

    # ──────────────────────────────────────────────
    # Event Reference Helpers
    # ──────────────────────────────────────────────

    @staticmethod
    def _collect_branch_events(tree: Dict[str, Any], prefix: str = '') -> set:
        """
        Collect event names that are both leaf and branch (have Self data object).

        W3C SCXML 3.12.1: Events like "foo" that also have children like "foo.zoo"
        require `.Self` suffix when used as concrete event references (raise, send),
        because the event class name alone refers to the sealed interface.

        Returns:
            Set of dot-separated event names that need `.Self` suffix
        """
        branch_events = set()
        for key, node in tree.items():
            if key == '_leaf':
                continue
            full_name = f"{prefix}.{key}" if prefix else key
            children = [k for k in node.keys() if k != '_leaf']
            if node.get('_leaf', False) and children:
                branch_events.add(full_name)
            branch_events.update(
                KotlinCodeGenerator._collect_branch_events(node, full_name)
            )
        return branch_events

    def _make_to_event_ref(self, branch_events: set):
        """
        Create a Jinja2 filter for concrete event references (raise, send).

        Events that are both concrete and branch nodes (e.g., "foo" with "foo.zoo")
        need `.Self` suffix to reference the data object, not the sealed interface.
        """
        to_event_class_name = self._to_event_class_name

        def to_event_ref(event_name: str) -> str:
            class_name = to_event_class_name(event_name)
            if event_name in branch_events:
                return class_name + '.Self'
            return class_name

        return to_event_ref

    # ──────────────────────────────────────────────
    # Kotlin-Specific Helpers
    # ──────────────────────────────────────────────

    def _make_parallel_complete_check(self, model: SCXMLModel, machine_name: str):
        """
        Create a Jinja2 filter that generates a Kotlin expression checking
        if all regions of a parallel state have reached a final child.

        W3C SCXML 3.7.1: All child regions must be in a final state.
        """
        def to_parallel_complete_check(parallel_id: str) -> str:
            regions = model.parallel_regions.get(parallel_id, [])
            if not regions:
                return "false"
            checks = []
            for region_id in regions:
                # Find final states within this region
                finals = []
                for state_id, state in model.states.items():
                    if state.parent == region_id and state.is_final:
                        finals.append(state_id)
                if finals:
                    # Check if any final state in this region is active
                    cond = ' || '.join(
                        f'activeStateIds.contains("{f}")'
                        for f in finals
                    )
                    checks.append(f'({cond})')
                else:
                    # Region has no final state — parallel can never complete
                    return "false"
            return ' && '.join(checks)
        return to_parallel_complete_check

    # ──────────────────────────────────────────────
    # Code Generation
    # ──────────────────────────────────────────────

    def _generate_output(self, model: SCXMLModel, scxml_path: str,
                         output_dir: str, depfile_path: Optional[str]) -> bool:
        """
        Generate single Kotlin file (.kt) from analyzed SCXML model.

        Uses Jinja2 templates from templates/kotlin/:
          - state_machine.kt.jinja2 (main orchestrator)
        """
        # Shared model analysis (language-agnostic, from BaseCodeGenerator)
        self._resolve_internal_transitions(model)
        model.scxml_base_path = self._compute_scxml_base_path(scxml_path)
        initial_entry_root = self._compute_initial_entry_root(model)
        ancestor_chains = self._compute_ancestor_chains(model)
        effective_transitions = self._compute_effective_transitions(model, ancestor_chains)
        parent_map = self._compute_parent_map(model)
        leaf_map = self._compute_leaf_map(model)
        parallel_descendants = self._compute_parallel_descendants(model)
        deep_initial_entries = self._compute_deep_initial_entries(model)

        # Invoke entries: base computes language-agnostic data, add Kotlin class names
        invoke_entries = self._compute_invoke_entries(model)
        for entries in invoke_entries.values():
            for entry in entries:
                child_name = entry.get('child_name', '')
                entry['child_class'] = self._to_pascal_case(child_name) if child_name else ''

        # W3C SCXML 3.13: Pre-compute ancestor transition maps for processEvent routing
        # Eliminates inline ancestor scanning in process_event.kt.jinja2
        ancestors_with_event_transitions = {}
        ancestors_with_null_transitions = {}
        for state_id in model.states:
            event_ancs = []
            null_ancs = []
            for anc_id in ancestor_chains.get(state_id, []):
                anc_state = model.states[anc_id]
                has_event = any(t.event for t in anc_state.transitions)
                has_null = any(not t.event and t.target for t in anc_state.transitions)
                if has_event:
                    event_ancs.append(anc_id)
                if has_null:
                    null_ancs.append(anc_id)
            ancestors_with_event_transitions[state_id] = event_ancs
            ancestors_with_null_transitions[state_id] = null_ancs

        # Kotlin-specific: Build event tree for sealed interface hierarchy
        kotlin_events = {e for e in model.events if e != 'Wildcard'}
        event_tree = self._build_event_tree(kotlin_events)
        leaf_events = self._collect_leaf_events(event_tree)

        # W3C SCXML 3.12.1: Identify events needing .Self suffix for concrete references
        branch_events = self._collect_branch_events(event_tree)
        self.env.filters['to_event_ref'] = self._make_to_event_ref(branch_events)

        # Pre-render event hierarchy (recursive Python, not Jinja2 macro)
        machine_name = self._to_pascal_case(model.name)
        event_members = self._render_event_tree(
            event_tree, f"{machine_name}Event"
        )

        # W3C SCXML 3.7.1: Register parallel completion check filter
        self.env.filters['to_parallel_complete_check'] = \
            self._make_parallel_complete_check(model, machine_name)

        input_stem = Path(scxml_path).stem

        # Load main Kotlin template
        template = self.env.get_template('state_machine.kt.jinja2')

        # Render with model context
        output = template.render(
            model=model,
            machine_name=machine_name,
            event_tree=event_tree,
            event_members=event_members,
            leaf_events=leaf_events,
            license_config=LICENSE_CONFIG,
            kotlin_default=self._to_kotlin_default,
            initial_entry_root=initial_entry_root,
            ancestor_chains=ancestor_chains,
            effective_transitions=effective_transitions,
            parent_map=parent_map,
            leaf_map=leaf_map,
            parallel_descendants=parallel_descendants,
            deep_initial_entries=deep_initial_entries,
            invoke_entries=invoke_entries,
            ancestors_with_event_transitions=ancestors_with_event_transitions,
            ancestors_with_null_transitions=ancestors_with_null_transitions,
        )

        # Post-process: close the class and normalize whitespace
        # (trim_blocks=True eats newlines between {% include %} and closing brace)
        output = output.rstrip() + '\n}\n'

        # Write single .kt output file
        output_path = Path(output_dir) / f"{input_stem}Sm.kt"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            f.write(output)

        print(f"  Generated: {output_path}")

        # CMake DEPFILE for incremental builds
        if depfile_path:
            self._write_depfile(depfile_path, [output_path])

        return True

    def _get_python_deps(self) -> list:
        """Add kotlin_generator.py to CMake dependency tracking."""
        deps = super()._get_python_deps()
        deps.append(str(Path(__file__).resolve()))
        return deps

