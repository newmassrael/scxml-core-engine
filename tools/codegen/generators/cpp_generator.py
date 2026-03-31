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
            inl_filename=inl_filename
        )

        # Render implementation (.inl)
        inl_output = inl_template.render(
            model=model, base_path=base_path, license_config=LICENSE_CONFIG
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
        if model.static_invokes:
            children_file = Path(output_dir) / f"{input_stem}_children.txt"
            with open(children_file, 'w') as f:
                for invoke_info in model.static_invokes:
                    child_name = invoke_info.get('child_name', '')
                    if child_name:
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

    def _generate_fallback(self, model: SCXMLModel, scxml_path: str,
                           output_dir: str) -> bool:
        """
        Generate C++ Interpreter wrapper for dynamic features.

        Creates a wrapper that uses the runtime Interpreter engine
        for SCXML files with features not supported by static codegen.
        """
        wrapper_template = """#pragma once
#include <memory>
#include "runtime/StateMachine.h"
#include "model/SCXMLModel.h"

namespace SCE::Generated::{{ model.name }} {

// Interpreter wrapper for {{ model.name }}
// Reason: Static codegen does not support this SCXML file's features
// Uses runtime/StateMachine for dynamic execution
class {{ model.name }} {
public:
    {{ model.name }}() {
        // TODO: Load SCXML file and create StateMachine instance
    }

    void run() {
        // TODO: Implement using StateMachine
    }
};

} // namespace SCE::Generated::{{ model.name }}
"""
        template = self.env.from_string(wrapper_template)
        output = template.render(model=model)

        input_stem = Path(scxml_path).stem
        output_path = Path(output_dir) / f"{input_stem}_sm.h"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            f.write(output)

        print(f"  Generated wrapper: {output_path}")
        return True
