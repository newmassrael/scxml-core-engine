"""
C++ Code Generator for W3C SCXML State Machines

Generates C++ header-only code (.h + .inl) using CRTP policy pattern.
Extends BaseCodeGenerator with C++-specific template rendering.

Output:
  - {name}_sm.h   : Header with policy struct, State/Event enums, includes
  - {name}_sm.inl : Inline implementation (transitions, actions, entry/exit)
  - {name}_children.txt : Child SCXML list for CMake (if static invokes)
"""

from pathlib import Path
from typing import Optional

from jinja2 import Environment

from generators.base import BaseCodeGenerator
from scxml_parser import SCXMLModel
from license_config import LICENSE_CONFIG


class CppCodeGenerator(BaseCodeGenerator):
    """
    C++ code generator using Jinja2 templates.

    Generates CRTP-based policy structs consumed by StaticExecutionEngine<Policy>.
    """

    LANGUAGE = 'cpp'

    def _default_template_dir(self) -> Path:
        """C++ templates are in the main templates/ directory."""
        return Path(__file__).parent.parent / 'templates'

    def _register_filters(self, env: Environment):
        """Register C++-specific Jinja2 filters."""
        env.filters['capitalize'] = self._capitalize_state
        env.filters['escape_cpp'] = self._escape_cpp_string

    def _capitalize_state(self, state_id: str) -> str:
        """Capitalize state/event names for C++ enums."""
        if not state_id:
            # W3C SCXML C.2: Empty event for BasicHTTP content-only send (test 520)
            return "Empty"
        if state_id.lower() == 'pass':
            return 'Pass'
        if state_id.lower() == 'fail':
            return 'Fail'
        return state_id[0].upper() + state_id[1:]

    def _escape_cpp_string(self, text: str) -> str:
        """Escape C++ string literals."""
        if not text:
            return ""
        text = text.replace('\\', '\\\\')
        text = text.replace('"', '\\"')
        text = text.replace('\n', '\\n')
        text = text.replace('\r', '\\r')
        text = text.replace('\t', '\\t')
        return text

    def _generate_output(self, model: SCXMLModel, scxml_path: str,
                         output_dir: str, depfile_path: Optional[str]) -> bool:
        """
        Generate C++ header (.h) and inline implementation (.inl) files.

        Uses Jinja2 templates: state_machine.jinja2, state_machine_inl.jinja2
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
        invoke_entries = self._compute_invoke_entries(model)
        for entries in invoke_entries.values():
            for entry in entries:
                child_name = entry.get('child_name', '')
                entry['child_class'] = self._capitalize_state(child_name) if child_name else ''

        # Pre-computed context shared by header and inl templates
        shared_context = dict(
            initial_entry_root=initial_entry_root,
            ancestor_chains=ancestor_chains,
            effective_transitions=effective_transitions,
            parent_map=parent_map,
            leaf_map=leaf_map,
            parallel_descendants=parallel_descendants,
            deep_initial_entries=deep_initial_entries,
            invoke_entries=invoke_entries,
        )

        # Load C++ templates
        header_template = self.env.get_template('state_machine.jinja2')
        inl_template = self.env.get_template('state_machine_inl.jinja2')

        # Calculate base_path for DataModelInitHelper file loading
        # ARCHITECTURE.md: basePath is resolved from executable location at runtime
        base_path = Path(output_dir).name

        # Use input filename (without extension) for unique output naming
        # W3C SCXML 6.4: Multiple tests may use same SCXML name attribute
        input_stem = Path(scxml_path).stem
        inl_filename = f"{input_stem}_sm.inl"

        # Render header (.h) with centralized license configuration
        header_output = header_template.render(
            model=model, base_path=base_path, license_config=LICENSE_CONFIG,
            inl_filename=inl_filename, **shared_context
        )

        # Render implementation (.inl)
        inl_output = inl_template.render(
            model=model, base_path=base_path, license_config=LICENSE_CONFIG,
            **shared_context
        )

        # Write output files
        output_path = Path(output_dir) / f"{input_stem}_sm.h"
        inl_path = Path(output_dir) / inl_filename
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            f.write(header_output)

        with open(inl_path, 'w') as f:
            f.write(inl_output)

        print(f"  Generated: {output_path}")
        print(f"  Generated: {inl_path}")

        # W3C SCXML 6.4: Write child state machines metadata for CMake
        # Includes both static invoke children and hybrid invoke children (pure AOT)
        all_children = []
        if model.static_invokes:
            for invoke_info in model.static_invokes:
                child_name = invoke_info.get('child_name', '')
                if child_name:
                    all_children.append(child_name)

        # W3C SCXML 6.4: Pre-generate hybrid invoke children as pure AOT
        # Matches Rust strategy: srcexpr/contentexpr children resolved at codegen time
        if model.hybrid_invokes:
            parent_dir = Path(scxml_path).parent
            for invoke_info in model.hybrid_invokes:
                child_name = invoke_info.get('child_name', '')
                if not child_name:
                    continue
                srcexpr = invoke_info.get('srcexpr', '')
                contentexpr = invoke_info.get('contentexpr', '')
                child_scxml = None

                if srcexpr:
                    # Scan resource directory for child SCXML (non-parent file)
                    parent_name = Path(scxml_path).name
                    for f in sorted(parent_dir.glob('*.scxml')):
                        if f.name != parent_name:
                            child_scxml = f
                            break

                if contentexpr or (srcexpr and child_scxml is None):
                    # Generate trivial child that immediately reaches final state
                    child_scxml = Path(output_dir) / f'{child_name}.scxml'
                    child_scxml.write_text(
                        '<?xml version="1.0"?>\n'
                        '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
                        f'name="{child_name}" initial="final" version="1.0">\n'
                        '  <final id="final"/>\n'
                        '</scxml>\n'
                    )

                if child_scxml and child_scxml.exists():
                    # Copy child SCXML to output dir with expected name
                    dest = Path(output_dir) / f'{child_name}.scxml'
                    if child_scxml != dest:
                        import shutil
                        shutil.copy2(child_scxml, dest)
                    all_children.append(child_name)

        if all_children:
            children_file = Path(output_dir) / f"{input_stem}_children.txt"
            with open(children_file, 'w') as f:
                for child_name in all_children:
                    f.write(f"{child_name}\n")
            print(f"  Child metadata: {children_file}")

        # CMake DEPFILE for incremental builds
        if depfile_path:
            self._write_depfile(depfile_path, [output_path, inl_path])

        return True

    def _get_python_deps(self) -> list:
        """Add cpp_generator.py to CMake dependency tracking."""
        deps = super()._get_python_deps()
        deps.append(str(Path(__file__).resolve()))
        return deps

