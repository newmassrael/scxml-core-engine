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
from typing import Any, Dict, List, Optional

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

        # Register cross-engine compatibility tests (match minijinja built-ins)
        self.env.tests['startingwith'] = lambda s, prefix: s.startswith(prefix) if isinstance(s, str) else False
        self.env.tests['endingwith'] = lambda s, suffix: s.endswith(suffix) if isinstance(s, str) else False
        # Register cross-engine compatibility filters (replace Python-specific string methods)
        self.env.filters['split'] = lambda s: s.split() if isinstance(s, str) else []
        self.env.filters['slice_from'] = lambda s, n: s[n:] if isinstance(s, str) and n < len(s) else ''

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

    def _generate_fallback(self, model: SCXMLModel, scxml_path: str,
                           output_dir: str) -> bool:
        """
        Default fallback: static generation not possible.

        Per CLAUDE.md: AOT failures must be fixed in codegen, not bypassed.
        Subclasses may override if language-specific fallback is needed.

        Args:
            model: SCXMLModel that cannot be statically generated
            scxml_path: Original SCXML file path
            output_dir: Target directory

        Returns:
            True if fallback generation succeeded, False if not supported
        """
        lang = self.LANGUAGE or 'Unknown'
        print(f"  {lang} AOT: Cannot generate code for '{model.name}' "
              f"(dynamic features not supported — fix codegen, not fallback)")
        return False

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

            for block in state.on_entry_blocks + state.on_exit_blocks:
                for action in block:
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

        # W3C SCXML 6.4: If any invoked child needs script engine, parent must
        # carry one too — needed for param evaluation and finalize execution
        if not model.needs_script_engine:
            for state in model.states.values():
                for si in state.static_invokes:
                    if si.get('child_needs_script_engine', False):
                        model.needs_script_engine = True
                        break
                if model.needs_script_engine:
                    break

        # Reapply script engine implications after potential child-driven activation
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
            model.needs_assign_helper = True
            model.needs_foreach = True
            model.needs_guard_helper = True

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
            # W3C SCXML C.2: BasicHTTPEventProcessor with HTTP target
            send_type = action.get('send_type', '')
            target = action.get('target', '')
            if (send_type == 'http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor'
                    and (target.startswith('http://') or target.startswith('https://'))):
                model.needs_http_send = True

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

        Also sets per-transition flags for codegen optimization:
        - needs_string_matching: True if runtime EventMatchingHelper is needed
        - matching_enum_values: Precomputed enum values for direct comparison
        - model.needs_event_matching_helper: True if any transition needs runtime matching
        """
        # model.events already contains invoke lifecycle events
        # (added by _add_system_events), no need to extend again
        all_events = list(model.events)
        model.needs_event_matching_helper = False

        for state in model.states.values():
            for transition in state.transitions:
                if not transition.event:
                    continue

                # W3C SCXML 5.9.3: Wildcards and glob patterns need runtime matching (C++)
                # but still compute prefix_matching_events for Kotlin codegen
                if transition.event in ['*', '.*', '_*'] or '.*' in transition.event or ' ' in transition.event:
                    transition.needs_string_matching = True
                    model.needs_event_matching_helper = True

                    # Compute prefix_matching_events for Kotlin templates
                    matching_events = set()
                    descriptors = transition.event.split()
                    for descriptor in descriptors:
                        if descriptor in ['*', '.*', '_*']:
                            matching_events.update(all_events)
                        elif descriptor.endswith('.*'):
                            base = descriptor[:-2]
                            for event_name in all_events:
                                if event_name.startswith(base + '.'):
                                    matching_events.add(event_name)
                        else:
                            for event_name in all_events:
                                if event_name == descriptor or event_name.startswith(descriptor + '.'):
                                    matching_events.add(event_name)
                    transition.prefix_matching_events = sorted(matching_events)
                    continue

                matching_events = set()

                # Standard prefix matching: "foo" matches "foo" and "foo.bar"
                for event_name in all_events:
                    if event_name == transition.event:
                        matching_events.add(event_name)
                    elif event_name.startswith(transition.event + '.'):
                        matching_events.add(event_name)

                sorted_matching = sorted(matching_events)
                transition.prefix_matching_events = sorted_matching
                transition.matching_enum_values = sorted_matching
                transition.needs_string_matching = False

    # ──────────────────────────────────────────────
    # Model analysis helpers (language-agnostic)
    #
    # These compute derived data structures from the parsed SCXML model
    # that any language generator may need for code generation.
    # ──────────────────────────────────────────────

    @staticmethod
    def _resolve_internal_transitions(model: SCXMLModel):
        """
        W3C SCXML 3.13: Resolve internal transition types.

        An internal transition falls back to external if the target is not
        a proper descendant of the source state, or the source is not compound.
        Mutates transition objects in-place.
        """
        for state_id, state in model.states.items():
            is_compound = (not state.is_parallel and not state.is_final
                           and any(s.parent == state_id for s in model.states.values()))
            for trans in state.transitions:
                if trans.type == 'internal' and trans.target:
                    is_descendant = False
                    current = trans.target
                    while current and current in model.states:
                        parent = model.states[current].parent
                        if parent == state_id:
                            is_descendant = True
                            break
                        current = parent
                    if not is_descendant or not is_compound:
                        trans.type = 'external'
                    else:
                        trans.is_true_internal = True
                        trans.internal_source = state_id

    @staticmethod
    def _compute_initial_entry_root(model: SCXMLModel) -> str:
        """
        W3C SCXML 3.2/3.4: Compute initial entry root.

        Walks up from the resolved leaf initial state to find the top-level
        ancestor. Used by enterInitialConfiguration() in generated code.
        """
        initial_entry_root = model.initial
        current = model.initial
        while current in model.states:
            parent = model.states[current].parent
            if parent and parent in model.states:
                initial_entry_root = parent
                current = parent
            else:
                break
        return initial_entry_root

    @staticmethod
    def _compute_ancestor_chains(model: SCXMLModel) -> Dict[str, List[str]]:
        """
        W3C SCXML 3.13: Compute ancestor chains for transition routing.

        Each state maps to an ordered list of ancestor state IDs (parent first).
        Used by processEvent to check ancestor transitions when self returns Ignored.
        """
        ancestor_chains: Dict[str, List[str]] = {}
        for state_id, state in model.states.items():
            chain: List[str] = []
            current_id = state.parent
            while current_id and current_id in model.states:
                chain.append(current_id)
                current_id = model.states[current_id].parent
            ancestor_chains[state_id] = chain
        return ancestor_chains

    @staticmethod
    def _compute_effective_transitions(
        model: SCXMLModel,
        ancestor_chains: Dict[str, List[str]]
    ) -> Dict[str, list]:
        """
        W3C SCXML 3.13: Compute effective transitions (self + ancestors).

        Used by executeTransitionActions to execute ancestor transition actions.
        """
        effective_transitions: Dict[str, list] = {}
        for state_id, state in model.states.items():
            transitions = list(state.transitions)
            for anc_id in ancestor_chains[state_id]:
                transitions.extend(model.states[anc_id].transitions)
            effective_transitions[state_id] = transitions
        return effective_transitions

    @staticmethod
    def _compute_parent_map(model: SCXMLModel) -> Dict[str, str]:
        """
        W3C SCXML 3.3: Build parent map for state hierarchy.

        Maps state_id -> parent_id for states with a parent.
        """
        parent_map: Dict[str, str] = {}
        for state_id, state in model.states.items():
            if state.parent and state.parent in model.states:
                parent_map[state_id] = state.parent
        return parent_map

    @staticmethod
    def _compute_leaf_map(model: SCXMLModel) -> Dict[str, str]:
        """
        W3C SCXML 3.3/3.4: Build leaf map for compound/parallel state resolution.

        Maps non-leaf states to their initial leaf states.
        """
        leaf_map: Dict[str, str] = {}
        for state_id, state in model.states.items():
            leaf = model.resolve_to_leaf(state_id)
            if leaf != state_id:
                leaf_map[state_id] = leaf
        return leaf_map

    @staticmethod
    def _collect_descendants(model: SCXMLModel, parent_id: str, result: List[str]):
        """Collect all descendant state IDs of a parent state (recursive)."""
        for state_id, state in model.states.items():
            if state.parent == parent_id:
                result.append(state_id)
                BaseCodeGenerator._collect_descendants(model, state_id, result)

    @staticmethod
    def _compute_parallel_descendants(model: SCXMLModel) -> Dict[str, List[str]]:
        """
        W3C SCXML 3.4: Compute descendants for each parallel state.

        Used by onExit to collect and exit active descendants.
        """
        parallel_descendants: Dict[str, List[str]] = {}
        for parallel_id in model.parallel_regions:
            descendants: List[str] = []
            BaseCodeGenerator._collect_descendants(model, parallel_id, descendants)
            parallel_descendants[parallel_id] = descendants
        return parallel_descendants

    @staticmethod
    def _compute_deep_initial_entries(
        model: SCXMLModel,
    ) -> Dict[str, List[Dict[str, Any]]]:
        """
        W3C SCXML 3.6: Compute deep initial entry order for space-separated initials.

        When a compound state has initial="target1 target2" (deep descendant targets),
        the codegen must enter ancestors along the path without triggering their
        default initial child entry, then enter the actual targets with full onEntry.
        """
        deep_initial_entries: Dict[str, List[Dict[str, Any]]] = {}
        for state_id, state in model.states.items():
            if len(state.initial_children) > 1:
                target_set = set(state.initial_children)
                all_path_states: set = set()
                for target_id in state.initial_children:
                    current = target_id
                    visited: set = set()
                    while current != state_id and current in model.states:
                        if current in visited:
                            break
                        visited.add(current)
                        all_path_states.add(current)
                        current = model.states[current].parent
                entry_order = []
                for sid in sorted(all_path_states,
                                  key=lambda s: model.states[s].document_order):
                    entry_order.append({
                        'id': sid,
                        'is_target': sid in target_set,
                    })
                deep_initial_entries[state_id] = entry_order
        return deep_initial_entries

    @staticmethod
    def _compute_invoke_entries(model: SCXMLModel) -> Dict[str, list]:
        """
        W3C SCXML 6.4: Compute invoke entries for each state.

        Returns language-agnostic invoke info. Each entry contains child_name
        (raw SCXML name); language generators should post-process to add
        language-specific class/type names.
        """
        invoke_entries: Dict[str, list] = {}
        for state_id, state in model.states.items():
            if state.static_invokes:
                entries = []
                for si in state.static_invokes:
                    invoke_id = si.get('invoke_id', '')
                    specific_done = f"done.invoke.{invoke_id}" if invoke_id else ""
                    if specific_done and specific_done in model.events:
                        done_event = specific_done
                    else:
                        done_event = 'done.invoke'
                    entries.append({
                        'invoke_id': invoke_id,
                        'child_name': si.get('child_name', ''),
                        'autoforward': si.get('autoforward', False),
                        'done_event': done_event,
                        'has_done_event': done_event in model.events or 'done.invoke' in model.events,
                        'params': si.get('params', []),
                        'namelist': si.get('namelist', ''),
                        'idlocation': si.get('idlocation', ''),
                        'finalize_content': si.get('finalize_content', ''),
                        'state_id': state_id,
                        'child_needs_script_engine': si.get('child_needs_script_engine', False),
                    })
                invoke_entries[state_id] = entries

        # W3C SCXML 6.4: Hybrid invoke support (srcexpr/contentexpr)
        for state_id, state in model.states.items():
            if state.hybrid_invokes:
                entries = invoke_entries.get(state_id, [])
                for hi in state.hybrid_invokes:
                    invoke_id = hi.get('invoke_id', '')
                    specific_done = f"done.invoke.{invoke_id}" if invoke_id else ""
                    if specific_done and specific_done in model.events:
                        done_event = specific_done
                    else:
                        done_event = 'done.invoke'
                    entries.append({
                        'invoke_id': invoke_id,
                        'child_name': '',
                        'autoforward': hi.get('autoforward', False),
                        'done_event': done_event,
                        'has_done_event': done_event in model.events or 'done.invoke' in model.events,
                        'params': hi.get('params', []),
                        'namelist': '',
                        'idlocation': hi.get('idlocation', ''),
                        'finalize_content': '',
                        'state_id': state_id,
                        'child_needs_script_engine': False,
                        'is_hybrid': True,
                        'srcexpr': hi.get('srcexpr', ''),
                        'contentexpr': hi.get('contentexpr', ''),
                    })
                invoke_entries[state_id] = entries

        return invoke_entries

    @staticmethod
    def _compute_scxml_base_path(scxml_path: str) -> str:
        """Compute relative base path for runtime SCXML resource resolution."""
        try:
            return str(Path(scxml_path).parent.relative_to(Path.cwd()))
        except ValueError:
            return str(Path(scxml_path).parent)

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
