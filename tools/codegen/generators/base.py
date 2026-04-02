"""
Base Code Generator for W3C SCXML State Machines

Provides language-agnostic SCXML analysis, feature detection, and
template rendering infrastructure. Language-specific generators
(C++, Kotlin, etc.) extend this base class.

Architecture:
  SCXML File -> SCXMLParser -> SCXMLModel -> BaseCodeGenerator (shared analysis)
                                                  |
                                          +-----------------+
                                          |                 |
                                   CppCodeGenerator   KotlinCodeGenerator
                                   (.h + .inl)        (.kt)
"""

import sys
from abc import ABC, abstractmethod
from pathlib import Path
from typing import Optional

from jinja2 import Environment, FileSystemLoader, select_autoescape
from scxml_parser import SCXMLParser, SCXMLModel


class DependencyTrackingLoader(FileSystemLoader):
    """
    Jinja2 loader that tracks which templates are actually loaded during rendering.

    Used for CMake DEPFILE generation - only templates that are actually used
    will be listed as dependencies, enabling fine-grained incremental builds.
    """
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.loaded_templates = set()
        self.template_dir = args[0] if args else kwargs.get('searchpath')

    def get_source(self, environment, template):
        """Track template load and delegate to parent"""
        self.loaded_templates.add(template)
        return super().get_source(environment, template)

    def get_dependency_paths(self):
        """Get absolute paths of loaded templates for CMake DEPFILE"""
        paths = []
        for template_name in self.loaded_templates:
            template_path = Path(self.template_dir) / template_name
            if template_path.exists():
                paths.append(str(template_path.resolve()))
        return paths


class BaseCodeGenerator(ABC):
    """
    Abstract base class for SCXML code generators.

    Implements the Template Method pattern:
      generate() orchestrates the pipeline (parse -> analyze -> render)
      _generate_output() is overridden by language-specific subclasses

    Shared responsibilities (language-agnostic):
      - SCXML parsing via SCXMLParser
      - Feature detection (_analyze_model_features)
      - Variable type classification (_classify_variables)
      - Event prefix matching (_build_prefix_matching)
      - Static generation capability check (_can_generate_static)

    Subclass responsibilities (language-specific):
      - Jinja2 filter registration (_register_filters)
      - Template rendering and file output (_generate_output)
      - Interpreter fallback generation (_generate_fallback)
    """

    # Target language identifier (override in subclasses)
    LANGUAGE = None

    def __init__(self, template_dir: Optional[str] = None):
        """
        Initialize code generator with Jinja2 template environment.

        Args:
            template_dir: Path to language-specific templates directory.
                          If None, uses default templates/ relative to codegen.py.
        """
        if template_dir is None:
            template_dir = self._default_template_dir()

        self.loader = DependencyTrackingLoader(str(template_dir))
        self.env = Environment(
            loader=self.loader,
            autoescape=select_autoescape(['html', 'xml']),
            trim_blocks=True,
            lstrip_blocks=True
        )

        # Let subclasses register language-specific filters
        self._register_filters(self.env)

    @abstractmethod
    def _default_template_dir(self) -> Path:
        """Return the default template directory for this language."""

    @abstractmethod
    def _register_filters(self, env: Environment):
        """Register language-specific Jinja2 filters on the environment."""

    @abstractmethod
    def _generate_output(self, model: SCXMLModel, scxml_path: str,
                         output_dir: str, depfile_path: Optional[str]) -> bool:
        """
        Generate language-specific output files from analyzed model.

        Called after shared analysis is complete. The model has all
        feature flags, variable types, and prefix matching populated.

        Args:
            model: Fully analyzed SCXMLModel
            scxml_path: Original SCXML file path (for output naming)
            output_dir: Target directory for generated files
            depfile_path: Optional CMake DEPFILE path

        Returns:
            True if generation succeeded
        """

    @abstractmethod
    def _generate_fallback(self, model: SCXMLModel, scxml_path: str,
                           output_dir: str) -> bool:
        """
        Generate fallback output when static generation is not possible.

        For C++: generates Interpreter wrapper.
        For other languages: may return False (unsupported).

        Args:
            model: SCXMLModel that cannot be statically generated
            scxml_path: Original SCXML file path
            output_dir: Target directory

        Returns:
            True if fallback generation succeeded, False if not supported
        """

    def generate(self, scxml_path: str, output_dir: str,
                 as_child: bool = False, depfile_path: str = None) -> bool:
        """
        Generate code from SCXML file.

        Template Method: orchestrates the shared analysis pipeline,
        then delegates to language-specific _generate_output().

        Args:
            scxml_path: Path to SCXML input file
            output_dir: Directory for generated output files
            as_child: If True, generate as invoked child state machine
            depfile_path: Optional CMake DEPFILE path for incremental builds

        Returns:
            True if generation succeeded, False otherwise
        """
        try:
            # Parse SCXML into language-agnostic model
            model = self._parse_scxml(scxml_path, as_child)

            # Shared analysis pipeline
            self._classify_variables(model)
            self._analyze_model_features(model)
            self._add_system_events(model)
            self._build_prefix_matching(model)

            print(f"Generating code for: {model.name}")
            print(f"  States: {len(model.states)}")
            print(f"  Events: {len(model.events)}")
            print(f"  Needs ScriptEngine: {model.needs_script_engine}")

            # Check if static generation is possible
            if not self._can_generate_static(model):
                print(f"  -> Generating fallback (dynamic features detected)")
                return self._generate_fallback(model, scxml_path, output_dir)

            # Delegate to language-specific output generation
            return self._generate_output(model, scxml_path, output_dir, depfile_path)

        except Exception as e:
            print(f"Error generating code: {e}", file=sys.stderr)
            import traceback
            traceback.print_exc()
            return False

    # ──────────────────────────────────────────────
    # Shared pipeline methods (language-agnostic)
    # ──────────────────────────────────────────────

    def _parse_scxml(self, scxml_path: str, as_child: bool) -> SCXMLModel:
        """
        Parse SCXML file into model with project-relative path resolution.

        W3C SCXML 6.4: Resolves source path for child invoke resolution.
        Uses project-relative path for platform portability (Native + WASM).
        """
        parser = SCXMLParser()
        model = parser.parse_file(scxml_path)

        # Resolve project-relative path for portability
        scxml_abs_path = Path(scxml_path).resolve()
        # tools/codegen/generators/base.py -> tools/codegen/ -> tools/ -> project root
        project_root = Path(__file__).parent.parent.parent.parent.resolve()
        try:
            model.scxml_source_path = str(scxml_abs_path.relative_to(project_root))
        except ValueError:
            model.scxml_source_path = str(scxml_abs_path)

        # W3C SCXML 6.4: Force template generation for invoked children
        if as_child:
            model.has_parent_communication = True

        return model

    def _classify_variables(self, model: SCXMLModel):
        """
        Classify datamodel variables by type.

        Determines whether variables are static (int, string, bool) or
        require runtime evaluation (script engine).
        """
        for var in model.variables:
            expr = var.get('expr', '')
            content = var.get('content', '')

            if not expr and not content:
                var['type'] = 'runtime'
            elif expr == '0' or (expr and expr.isdigit()):
                var['type'] = 'int'
            elif expr.startswith('"') and expr.endswith('"'):
                var['type'] = 'string'
            elif expr in ['true', 'false']:
                var['type'] = 'bool'
            else:
                var['type'] = 'runtime'
                model.needs_script_engine = True

    def _analyze_model_features(self, model: SCXMLModel):
        """
        Analyze model and set feature flags.

        Determines which helpers are needed based on
        SCXML features used in the model. Feature flags are
        language-agnostic; templates interpret them for each language.
        """
        # W3C SCXML B.1: ECMAScript datamodel requires script engine only if used
        # W3C SCXML 3.12: TransitionHelper always needed for event matching
        model.needs_transition_helper = True
        model.needs_event_type_helper = False
        model.needs_assign_helper = False
        model.needs_foreach = False
        model.needs_guard_helper = False
        model.needs_send_helper = False
        model.needs_event_data_helper = False
        model.needs_donedata_helper = False

        # Event metadata fields
        model.needs_event_name = False
        model.needs_event_data = False
        model.needs_event_type = False
        model.needs_event_sendid = False
        model.needs_event_origin = False
        model.needs_event_origintype = False
        model.needs_event_invokeid = False
        model.needs_external_flag = False

        # Scan all actions
        for state in model.states.values():
            for transition in state.transitions:
                if transition.cond:
                    model.needs_guard_helper = True
                for action in transition.actions:
                    self._analyze_action(action, model)

            for action in state.on_entry + state.on_exit:
                self._analyze_action(action, model)

        # Script engine implies full event metadata
        if model.needs_script_engine:
            model.needs_event_name = True
            model.needs_event_data = True
            model.needs_event_type = True
            model.needs_event_sendid = True
            model.needs_event_origin = True
            model.needs_event_origintype = True
            model.needs_event_invokeid = True
            model.needs_external_flag = True
            model.events.add('error.execution')
            model.needs_event_type_helper = True
            # W3C SCXML B.1: Script engine always needs these helpers
            model.needs_assign_helper = True
            model.needs_foreach = True
            model.needs_guard_helper = True

        # W3C SCXML 5.5: Detect donedata in final states
        for state in model.states.values():
            if state.is_final and state.donedata is not None:
                model.needs_donedata_helper = True
                model.events.add('error.execution')
                if state.donedata.get('params') or state.donedata.get('contentexpr'):
                    model.needs_script_engine = True
                break

    def _analyze_action(self, action, model: SCXMLModel):
        """Analyze single action for feature detection."""
        action_type = action.get('type', '')

        if action_type == 'send':
            model.needs_send_helper = True
            model.events.add('error.execution')
            if action.get('params'):
                model.needs_event_data_helper = True
            if action.get('delay') or action.get('delayexpr'):
                model.needs_event_scheduler = True
            if action.get('type') == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor':
                model.needs_external_flag = True
            if '_event.sendid' in str(action):
                model.needs_event_sendid = True
            if '_event.origin' in str(action):
                model.needs_event_origin = True
            if '_event.invokeid' in str(action):
                model.needs_event_invokeid = True

        elif action_type == 'cancel':
            model.needs_event_scheduler = True

        elif action_type == 'assign':
            model.needs_assign_helper = True

        elif action_type == 'foreach':
            model.needs_foreach = True

        elif action_type == 'if' and action.get('cond'):
            model.needs_guard_helper = True
            cond = action.get('cond', '')
            if '_event.type' in cond:
                model.needs_event_type = True
            if '_event.data' in cond:
                model.needs_event_data = True
            if '_event.name' in cond:
                model.needs_event_name = True

            # Recursively analyze nested actions in if/elseif/else branches
            for nested_action in action.get('then_actions', []):
                self._analyze_action(nested_action, model)
            for elseif_branch in action.get('elseif_branches', []):
                for nested_action in elseif_branch.get('actions', []):
                    self._analyze_action(nested_action, model)
            for nested_action in action.get('else_actions', []):
                self._analyze_action(nested_action, model)

    def _add_system_events(self, model: SCXMLModel):
        """Add system-level events (wildcards, invoke events) to model."""
        # W3C SCXML 3.12.1: Wildcard patterns include '*' and '.*'
        has_wildcard = any(
            any(t.event == '*' or t.event == '.*' for t in state.transitions)
            for state in model.states.values()
        )
        if has_wildcard:
            model.events.add('Wildcard')

        # W3C SCXML 6.4: Invoke lifecycle events
        if model.static_invokes:
            model.events.add('done.invoke')
            model.events.add('cancel.invoke')
            model.events.add('error.execution')

    def _build_prefix_matching(self, model: SCXMLModel):
        """
        Build prefix matching for event transitions.

        W3C SCXML 3.12.1: event="error" matches "error", "error.execution", etc.
        Handles space-separated multiple event descriptors (e.g., "foo bar").
        Handles explicit wildcard descriptors (e.g., "foo.*").
        """
        # model.events already contains invoke lifecycle events
        # (added by _add_system_events), no need to extend again
        all_events = list(model.events)

        for state in model.states.values():
            for transition in state.transitions:
                if not transition.event or transition.event in ['*', '.*', '_*']:
                    continue

                matching_events = set()

                # W3C SCXML 3.12.1: Split space-separated event descriptors
                descriptors = transition.event.split()

                for descriptor in descriptors:
                    if descriptor.endswith('.*'):
                        # W3C SCXML 3.12.1: "foo.*" matches descendants only (not "foo" itself)
                        base = descriptor[:-2]
                        for event_name in all_events:
                            if event_name.startswith(base + '.'):
                                matching_events.add(event_name)
                    else:
                        # Standard prefix matching: "foo" matches "foo" and "foo.bar"
                        for event_name in all_events:
                            if event_name == descriptor:
                                matching_events.add(event_name)
                            elif event_name.startswith(descriptor + '.'):
                                matching_events.add(event_name)

                transition.prefix_matching_events = sorted(matching_events)

    def _can_generate_static(self, model: SCXMLModel) -> bool:
        """
        Determine if model can be statically code-generated.

        Returns True if all features can be handled by static generation.
        Returns False if dynamic features require fallback (e.g., Interpreter).
        """
        if not model.initial:
            print(f"    Reason: No initial state")
            return False

        # W3C SCXML 3.13: Validate initial states exist
        initial_states = model.initial.split()
        if len(initial_states) > 1:
            missing_states = [s for s in initial_states if s not in model.states]
            if missing_states:
                print(f"    Reason: Initial states '{', '.join(missing_states)}' not found in model")
                return False
        else:
            if model.initial not in model.states:
                print(f"    Reason: Initial state '{model.initial}' not found in model")
                return False

        return True

    def _get_python_deps(self) -> list:
        """
        Return Python source files that should be tracked as CMake dependencies.

        Subclasses should override to add their own module files.
        Changes to any listed file will trigger CMake regeneration.
        """
        codegen_dir = Path(__file__).parent.parent
        deps = [
            str(Path(__file__).resolve()),                          # base.py
            str((codegen_dir / 'scxml_parser.py').resolve()),       # parser
            str((codegen_dir / 'license_config.py').resolve()),     # license headers
            str((codegen_dir / 'generators' / '__init__.py').resolve()),  # package
        ]
        # Dispatcher script may be renamed on install (codegen.py -> scxml-codegen),
        # so track whichever entry point actually exists
        for candidate in ['codegen.py', 'scxml-codegen']:
            script_path = codegen_dir / candidate
            if script_path.exists():
                deps.append(str(script_path.resolve()))
                break
        return deps

    def _write_depfile(self, depfile_path: str, output_paths: list):
        """
        Write CMake DEPFILE for incremental builds.

        Args:
            depfile_path: Path to write the Makefile-format dependency file
            output_paths: List of generated output file paths
        """
        template_deps = self.loader.get_dependency_paths()
        python_deps = self._get_python_deps()
        all_deps = template_deps + python_deps

        with open(depfile_path, 'w') as f:
            deps_escaped = ' '.join(d.replace(' ', '\\ ') for d in all_deps)
            for output_path in output_paths:
                output_escaped = str(output_path).replace(' ', '\\ ')
                f.write(f"{output_escaped}: {deps_escaped}\n")
