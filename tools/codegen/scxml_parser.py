#!/usr/bin/env python3
"""
SCXML Parser for Static Code Generation

Parses W3C SCXML files and extracts state machine model for code generation.
Replaces C++ SCXMLParser for Python-based code generation.
"""

from lxml import etree
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Set
from pathlib import Path

# W3C SCXML namespace
SCXML_NS = {'sc': 'http://www.w3.org/2005/07/scxml'}
def ns_find(elem, tag):
    """Find element with namespace"""
    return elem.find(f'sc:{tag}', SCXML_NS)

def ns_findall(elem, tag):
    """Find all elements with namespace"""
    return elem.findall(f'sc:{tag}', SCXML_NS)


@dataclass
class Transition:
    """W3C SCXML 3.3: Transition element"""
    event: str = ""
    target: str = ""
    cond: str = ""  # guard condition (original SCXML expression)
    cond_cpp: str = ""  # C++ code for pure In() predicates (empty if needs JSEngine)
    is_pure_in_predicate: bool = False  # True if cond is ONLY In() predicates
    type: str = "external"  # external or internal
    actions: List[Dict] = field(default_factory=list)  # executable content


@dataclass
class State:
    """W3C SCXML 3.3: State element"""
    id: str
    initial: str = ""
    initial_children: List[str] = field(default_factory=list)  # W3C SCXML 3.6: Parsed list from space-separated initial attribute
    is_final: bool = False
    is_parallel: bool = False
    parent: Optional[str] = None
    transitions: List[Transition] = field(default_factory=list)
    on_entry: List[Dict] = field(default_factory=list)
    on_exit: List[Dict] = field(default_factory=list)
    datamodel: List[Dict] = field(default_factory=list)
    invokes: List[Dict] = field(default_factory=list)
    static_invokes: List[Dict] = field(default_factory=list)  # Static invoke info for member generation
    hybrid_invokes: List[Dict] = field(default_factory=list)  # Hybrid invoke (contentexpr): AOT parent + Interpreter child
    donedata: Optional[Dict] = None  # W3C SCXML 5.5: Donedata for final states
    document_order: int = 0  # W3C SCXML 3.13: Document order for exit order tie-breaking
    initial_transition_actions: List[Dict] = field(default_factory=list)  # W3C SCXML 3.3.2: <initial> transition executable content
    initial_history_id: str = ""  # W3C SCXML 3.11: History state ID if initial targets history
    initial_history_default_target: str = ""  # W3C SCXML 3.11: Default target of history state (fallback)
    initial_history_default_actions: List[Dict] = field(default_factory=list)  # W3C SCXML 3.11: Executable content of history default transition


@dataclass
class SCXMLModel:
    """Simplified SCXML model for code generation"""
    name: str
    initial: str = ""
    initial_leaf: str = ""  # W3C SCXML 3.3: Resolved leaf state of initial (for compound states)
    binding: str = "early"  # W3C SCXML 5.3: early or late
    datamodel_type: str = "ecmascript"  # W3C SCXML B.1

    states: Dict[str, State] = field(default_factory=dict)
    events: Set[str] = field(default_factory=set)
    history_default_targets: Dict[str, str] = field(default_factory=dict)  # W3C SCXML 3.11: history_id -> default_target
    history_states: Dict[str, Dict] = field(default_factory=dict)  # W3C SCXML 3.11: history_id -> {parent, type, default_target}

    # Feature flags (for determining static vs interpreter wrapper)
    has_dynamic_expressions: bool = False
    has_parallel_states: bool = False
    has_history_states: bool = False
    has_invoke: bool = False
    has_dynamic_invoke: bool = False  # Deprecated: use has_dynamic_file_invoke or has_hybrid_invoke
    has_hybrid_invoke: bool = False  # True if any invoke uses contentexpr (AOT parent + Interpreter child)
    has_dynamic_file_invoke: bool = False  # True if any invoke uses srcexpr (Full Interpreter wrapper)
    has_event_metadata: bool = False
    has_parent_communication: bool = False  # True if <send target="#_parent"> detected
    has_child_communication: bool = False  # True if <send target="#_child"> detected
    needs_jsengine: bool = False
    uses_in_predicate: bool = False  # W3C SCXML 3.12.1: True if In() predicate used (requires activeStates_ management)
    has_transition_actions: bool = False  # W3C SCXML 3.13: True if any transition has executable content

    # W3C SCXML 5.10: Event metadata field flags
    needs_event_name: bool = False
    needs_event_data: bool = False
    needs_event_type: bool = False
    needs_event_sendid: bool = False
    needs_event_origin: bool = False
    needs_event_origintype: bool = False
    needs_event_invokeid: bool = False

    # Data model variables
    variables: List[Dict] = field(default_factory=list)

    # W3C SCXML 5.8: Global (top-level) scripts executed at document load time
    global_scripts: List[Dict] = field(default_factory=list)

    # Static invoke information (for code generation)
    static_invokes: List[Dict] = field(default_factory=list)
    # Hybrid invoke information (contentexpr): AOT parent + Interpreter child
    hybrid_invokes: List[Dict] = field(default_factory=list)
    
    # W3C SCXML 3.4: Parallel state regions (parallel_id -> [child_region_ids])
    parallel_regions: Dict[str, List[str]] = field(default_factory=dict)


class SCXMLParser:
    """
    W3C SCXML parser for static code generation

    Parses SCXML XML files and extracts state machine structure.
    Detects features requiring JSEngine or Interpreter wrapper.
    """

    # W3C SCXML namespace
    SCXML_NS = {'scxml': 'http://www.w3.org/2005/07/scxml'}

    # W3C SCXML 5.10.1: Event metadata field names
    EVENT_METADATA_FIELDS = [
        '_event.origin',
        '_event.origintype',
        '_event.sendid',
        '_event.invokeid',
        '_event.type'
    ]

    def __init__(self):
        self.model = None
        self.document_order_counter = 0  # W3C SCXML 3.13: Track document order
        self.invoke_counter = 0  # W3C SCXML 6.4.1: Generate invoke IDs when missing

    def parse_file(self, scxml_path: str) -> SCXMLModel:
        """
        Parse SCXML file and return model

        Args:
            scxml_path: Path to SCXML file

        Returns:
            SCXMLModel with extracted state machine structure
        """
        # Store scxml_path for child resolution in _process_static_invokes
        self.scxml_path = scxml_path

        tree = etree.parse(scxml_path)
        root = tree.getroot()

        # Note: Using lxml with QName().localname, no need to strip namespaces

        # Use filename for model name to ensure uniqueness
        # W3C SCXML 6.4: Multiple tests may use same SCXML name attribute (e.g., test338 and test347 both use "machineName")
        # Using filename (not name attribute) ensures unique namespaces (test338_machineName vs test347_machineName)
        name = Path(scxml_path).stem

        # Get initial attribute (W3C SCXML 3.6)
        initial = root.get('initial', '')

        # W3C SCXML 3.6: If no initial attribute, default to first child state in document order
        if not initial:
            # Find first child state element
            first_state = ns_find(root, 'state')
            if first_state is None:
                first_state = ns_find(root, 'parallel')
            if first_state is None:
                first_state = ns_find(root, 'final')

            if first_state is not None:
                initial = first_state.get('id', '')

        # Create model
        self.model = SCXMLModel(
            name=name,
            initial=initial,
            binding=root.get('binding', 'early'),
            datamodel_type=root.get('datamodel', 'ecmascript')
        )

        # Parse datamodel
        self._parse_datamodel(root)

        # W3C SCXML 5.8: Parse global (top-level) scripts
        self._parse_global_scripts(root)

        # Parse states recursively
        self._parse_states(root, None)

        # Detect features
        self._detect_features()
        
        # Process static invoke info - extract child names from src paths
        self._process_static_invokes()

        # Resolve deep initial state (matches C++ generator behavior)
        # W3C SCXML 3.6: Follow initial attributes recursively to find leaf state
        self._resolve_deep_initial()
        
        # W3C SCXML 3.13: Apply parallel initial state overrides
        # If scxml initial contains space-separated states, override each region's initial
        self._apply_parallel_initial_overrides()

        # Resolve history state transitions (W3C SCXML 3.11)
        # Replace history state targets with their default transition targets
        self._resolve_history_targets()
        
        # W3C SCXML 3.3: After history resolution, compute the actual leaf state for initial
        # This handles cases where scxml initial points to a compound state whose initial was resolved
        # Example: <scxml initial="s0"> where s0's initial was resolved from history to leaf
        if self.model.initial:
            self.model.initial_leaf = self._resolve_to_leaf_state(self.model.initial)
        
        # W3C SCXML 3.4: Compute parallel regions (child states of parallel states)
        self._compute_parallel_regions()

        # W3C SCXML 3.13: Check if any transitions have actions (for static optimization)
        self._detect_transition_actions()

        # W3C SCXML 3.7: Add done.state events for states with final children
        self._add_done_state_events()

        # W3C SCXML 6.4: Set invoke event flags (specific vs generic done.invoke)
        self._set_invoke_event_flags()

        # W3C SCXML 6.2: Collect events from child state machines that send to parent
        # ARCHITECTURE.md AOT-First Migration: Auto-add child→parent events to parent Event enum
        # Prevents compilation errors when child sends events only caught by wildcard transitions (test 243)
        self._collect_child_to_parent_events()

        # W3C SCXML 3.6: Parse space-separated initial attributes into lists
        self._parse_initial_children()

        return self.model

    def _parse_datamodel(self, root):
        """
        Parse <datamodel> elements (W3C SCXML 5.2)

        ARCHITECTURE.md Zero Duplication: Match Interpreter behavior
        - Interpreter: ParsingCommon::findFirstChildElement() searches direct children only
        - AOT: Use './sc:datamodel' (not './/sc:datamodel') to search direct children only
        - Prevents false "scoped datamodel" detection for parent/child state machines
        """
        for datamodel in root.findall('./sc:datamodel', SCXML_NS):
            for data in ns_findall(datamodel, 'data'):
                var_id = data.get('id')
                expr = data.get('expr', '')
                src = data.get('src', '')

                # W3C SCXML 5.2.2 & B.2: Get inline XML/text content only
                # ARCHITECTURE.md Zero Duplication: For src files, DataModelInitHelper loads at runtime
                # Parser only handles inline content, runtime handles src files (matches Interpreter)
                content = ''
                
                # W3C SCXML 5.2.2: src attribute handled by DataModelInitHelper at runtime
                # AOT code generation: Pass empty content for src files, Helper loads file dynamically
                if not src:
                    # No src attribute - check for inline XML child elements (W3C SCXML B.2)
                    if len(data) > 0:
                        # Has child elements - serialize all children as XML string
                        # This matches Interpreter behavior: DataModelParser::parseDataModelItem()
                        # Example: <data id="var1"><books><book title="x"/></books></data>
                        child_xml_parts = []
                        for child in data:
                            child_str = etree.tostring(child, encoding='unicode', method='xml')
                            child_xml_parts.append(child_str)
                        content = ''.join(child_xml_parts)
                    else:
                        # No child elements - use text content
                        content = data.text or ''

                self.model.variables.append({
                    'id': var_id,
                    'expr': expr,
                    'src': src,
                    'content': content.strip()
                })

                # Detect if expression requires JSEngine
                # W3C SCXML 5.2: All datamodel variables are runtime-evaluated (handled by JSEngine)
                self.model.needs_jsengine = True

    def _parse_global_scripts(self, root):
        """
        Parse top-level &lt;script&gt; elements (W3C SCXML 5.8)

        Global scripts are children of &lt;scxml&gt; root and are executed
        at document load time, after datamodel initialization but before
        the state machine starts.

        ARCHITECTURE.md Zero Duplication: Follows FileLoadingHelper pattern.
        Algorithm matches C++ FileLoadingHelper::loadExternalScript().
        See rsm/include/common/FileLoadingHelper.h for C++ implementation.

        W3C SCXML 5.8: "If the script can not be downloaded within a platform-specific
        timeout interval, the document is considered non-conformant, and the platform
        MUST reject it."
        """
        import logging

        # Find direct children &lt;script&gt; elements of &lt;scxml&gt; root
        for script_elem in root.findall('./sc:script', SCXML_NS):
            src = script_elem.get('src', '')
            content = script_elem.text or ''

            # W3C SCXML 5.8: External script loading (src attribute)
            if src:
                try:
                    # ARCHITECTURE.md Zero Duplication: FileLoadingHelper::loadExternalScript() equivalent
                    # Step 1: Normalize path (remove "file:" prefix) - FileLoadingHelper::normalizePath()
                    normalized_src = src
                    if normalized_src.startswith('file://'):
                        normalized_src = normalized_src[7:]  # Remove "file://"
                    elif normalized_src.startswith('file:'):
                        normalized_src = normalized_src[5:]  # Remove "file:"

                    # Step 2: Resolve path relative to SCXML file location
                    scxml_dir = Path(self.scxml_path).parent
                    script_path = scxml_dir / normalized_src
                    script_path = script_path.resolve()

                    # Step 3: Security validation - prevent path traversal attacks
                    # FileLoadingHelper::loadExternalScript() security check equivalent
                    scxml_dir_resolved = scxml_dir.resolve()

                    # Check if script_path is within allowed directory tree
                    # Use relative_to() to verify path is inside scxml_dir
                    try:
                        relative_path = script_path.relative_to(scxml_dir_resolved)
                        # If relative_to() succeeds, check for ".." in path
                        if '..' in str(relative_path):
                            raise ValueError("Path contains '..' after resolution")
                    except ValueError:
                        # script_path is not relative to scxml_dir (path traversal attempt)
                        raise ValueError(
                            f"Security violation: Script path '{src}' resolves outside SCXML directory. "
                            f"Resolved to: {script_path}, SCXML dir: {scxml_dir_resolved}"
                        )

                    # Step 4: Load file content - FileLoadingHelper::loadFileContent() equivalent
                    logging.info(f"FileLoadingHelper pattern: W3C SCXML 5.8 - Loading external script: {src} (resolved to {script_path})")
                    content = script_path.read_text(encoding='utf-8')

                    # W3C SCXML 5.8: Content loaded successfully

                except FileNotFoundError:
                    # W3C SCXML 5.8: Document MUST be rejected if script cannot be loaded
                    # FileLoadingHelper::loadExternalScript() error message equivalent
                    raise ValueError(
                        f"W3C SCXML 5.8: External script file not found: '{src}' "
                        f"(resolved to {script_path}). Document is non-conformant and MUST be rejected."
                    )
                except PermissionError as e:
                    # W3C SCXML 5.8: Document MUST be rejected if script cannot be loaded
                    raise ValueError(
                        f"W3C SCXML 5.8: Cannot read external script file: '{src}' "
                        f"(resolved to {script_path}). Permission denied: {e}"
                    )
                except ValueError as e:
                    # Security violation or other value error - propagate
                    raise
                except Exception as e:
                    # Any other error loading script
                    raise ValueError(
                        f"W3C SCXML 5.8: Failed to load external script: '{src}'. Error: {e}"
                    )

            self.model.global_scripts.append({
                'type': 'script',
                'src': src,
                'content': content.strip()
            })

            # W3C SCXML 5.8: Global scripts require JSEngine
            self.model.needs_jsengine = True

    def _parse_states(self, parent_elem, parent_id: Optional[str]):
        """
        Recursively parse states (W3C SCXML 3.3)

        Args:
            parent_elem: Parent XML element
            parent_id: Parent state ID (None for root)
        """
        # Parse <state> elements
        for state_elem in ns_findall(parent_elem, 'state'):
            state_id = state_elem.get('id')
            if not state_id:
                continue

            state = State(
                id=state_id,
                initial=state_elem.get('initial', ''),
                parent=parent_id,
                document_order=self.document_order_counter
            )
            self.document_order_counter += 1

            # Parse transitions
            for trans_elem in ns_findall(state_elem, 'transition'):
                transition = self._parse_transition(trans_elem)
                state.transitions.append(transition)

                # Collect event names
                if transition.event:
                    # W3C SCXML 5.9.3: Add transition events to enum, but skip wildcard patterns
                    # Wildcards (*) and patterns (foo.*) are handled by EventMatchingHelper at runtime
                    if transition.event not in ['*', '.*', '_*']:
                        # Handle multiple events (e.g., "event1 event2")
                        for event in transition.event.split():
                            # Skip wildcards and wildcard patterns
                            if event not in ['*', '.*', '_*'] and not event.endswith('.*'):
                                # Regular event - add to enum
                                self.model.events.add(event)

            # Parse onentry
            for entry_elem in ns_findall(state_elem, 'onentry'):
                state.on_entry.extend(self._parse_executable_content(entry_elem))

            # Parse onexit
            for exit_elem in ns_findall(state_elem, 'onexit'):
                state.on_exit.extend(self._parse_executable_content(exit_elem))

            # W3C SCXML 3.3.2: Parse <initial> transition executable content
            # This content executes AFTER parent onentry and BEFORE child state entry
            initial_elem = ns_find(state_elem, 'initial')
            if initial_elem is not None:
                # <initial> contains a <transition> element with executable content
                initial_trans_elem = ns_find(initial_elem, 'transition')
                if initial_trans_elem is not None:
                    state.initial_transition_actions = self._parse_executable_content(initial_trans_elem)
                    # W3C SCXML 3.3: Extract target from <initial> transition (if state.initial empty)
                    # History state targets will be resolved later by _resolve_history_targets()
                    if not state.initial:
                        initial_target = initial_trans_elem.get('target', '')
                        if initial_target:
                            state.initial = initial_target

            # Parse datamodel
            for datamodel in ns_findall(state_elem, 'datamodel'):
                for data in ns_findall(datamodel, 'data'):
                    # W3C SCXML 5.2: Extract content from text node or content attribute
                    content = (data.text or '').strip() if data.text else ''
                    state.datamodel.append({
                        'id': data.get('id'),
                        'expr': data.get('expr', ''),
                        'src': data.get('src', ''),
                        'content': content
                    })

            # Parse invoke
            for invoke_elem in ns_findall(state_elem, 'invoke'):
                invoke = self._parse_invoke(invoke_elem)
                state.invokes.append(invoke)
                self.model.has_invoke = True
                
                # Track invoke types (static, hybrid, dynamic file)
                if invoke.get('is_hybrid', False):
                    # Hybrid invoke: AOT parent + Interpreter child (srcexpr or contentexpr)
                    hybrid_invoke = {
                        'invoke_id': invoke.get('id', ''),
                        'state_name': state_id,
                        'srcexpr': invoke.get('srcexpr', ''),  # W3C SCXML 6.4: Runtime file path evaluation
                        'contentexpr': invoke.get('contentexpr', ''),  # W3C SCXML 6.4: Runtime content evaluation
                        'autoforward': invoke.get('autoforward', 'false') == 'true',
                        'params': invoke.get('params', []),
                        'idlocation': invoke.get('idlocation', '')
                    }
                    state.hybrid_invokes.append(hybrid_invoke)
                    self.model.hybrid_invokes.append(hybrid_invoke)
                elif invoke.get('is_dynamic_file', False):
                    # Handled by has_dynamic_file_invoke flag (set in parseInvokeElement)
                    pass
                elif invoke.get('is_static', False):
                    # Pure static invoke: compile-time known child SCXML (src or inline content)
                        # Pure static invoke: compile-time known child SCXML (src or inline content)
                        # Build static invoke info (matches C++ StaticInvokeInfo)
                        # W3C SCXML 6.5: Convert finalize actions to JavaScript code
                        finalize_actions = invoke.get('finalize', [])
                        finalize_script = self._actions_to_javascript(finalize_actions) if finalize_actions else ''
                        
                        static_invoke = {
                            'invoke_id': invoke.get('id', ''),
                            'child_name': '',  # Will be set after parsing child file name
                            'state_name': state_id,
                            'autoforward': invoke.get('autoforward', 'false') == 'true',
                            'finalize_content': finalize_script,
                            'src': invoke.get('src', ''),
                            'params': invoke.get('params', []),
                            'idlocation': invoke.get('idlocation', ''),  # W3C SCXML 6.4.1
                            'namelist': invoke.get('namelist', '')  # W3C SCXML 6.4.1: namelist validation
                        }
                        state.static_invokes.append(static_invoke)
                        self.model.static_invokes.append(static_invoke)

            self.model.states[state_id] = state

            # Recursively parse child states
            self._parse_states(state_elem, state_id)

            # W3C SCXML 3.6: If compound state has no explicit initial, use first child in document order
            if not state.initial:
                # Find first child state (lowest document_order)
                # Exclude history states (they are tracked separately in model.history_states)
                first_child = None
                for child_id, child_state in self.model.states.items():
                    if child_state.parent == state_id:
                        # Skip history states (not in model.states, tracked in model.history_states)
                        if child_id in self.model.history_states:
                            continue
                        if first_child is None or child_state.document_order < first_child.document_order:
                            first_child = child_state
                
                if first_child:
                    state.initial = first_child.id

        # Parse <final> elements
        for final_elem in ns_findall(parent_elem, 'final'):
            final_id = final_elem.get('id')
            if not final_id:
                continue

            state = State(
                id=final_id,
                is_final=True,
                parent=parent_id,
                document_order=self.document_order_counter
            )
            self.document_order_counter += 1

            # Parse onentry/onexit for final states
            for entry_elem in ns_findall(final_elem, 'onentry'):
                state.on_entry.extend(self._parse_executable_content(entry_elem))

            for exit_elem in ns_findall(final_elem, 'onexit'):
                state.on_exit.extend(self._parse_executable_content(exit_elem))

            # Parse <donedata> for final states (W3C SCXML 5.5)
            donedata_elem = ns_find(final_elem, 'donedata')
            if donedata_elem is not None:
                state.donedata = self._parse_donedata(donedata_elem)

            self.model.states[final_id] = state

        # Parse <parallel> elements
        for parallel_elem in ns_findall(parent_elem, 'parallel'):
            parallel_id = parallel_elem.get('id')
            if not parallel_id:
                continue

            state = State(
                id=parallel_id,
                is_parallel=True,
                parent=parent_id,
                document_order=self.document_order_counter
            )
            self.document_order_counter += 1

            # W3C SCXML 3.4: Parallel states can have transitions and onexit/onentry
            # Parse transitions
            for trans_elem in ns_findall(parallel_elem, 'transition'):
                transition = self._parse_transition(trans_elem)
                state.transitions.append(transition)

                # Collect event names
                if transition.event:
                    if transition.event not in ['*', '.*', '_*']:
                        for event in transition.event.split():
                            if event not in ['*', '.*', '_*'] and not event.endswith('.*'):
                                self.model.events.add(event)

            # Parse onentry
            for entry_elem in ns_findall(parallel_elem, 'onentry'):
                state.on_entry.extend(self._parse_executable_content(entry_elem))

            # Parse onexit
            for exit_elem in ns_findall(parallel_elem, 'onexit'):
                state.on_exit.extend(self._parse_executable_content(exit_elem))

            self.model.states[parallel_id] = state
            self.model.has_parallel_states = True

            # Recursively parse child states (parallel regions)
            self._parse_states(parallel_elem, parallel_id)

        # Parse <history> elements (W3C SCXML 3.11)
        for history_elem in ns_findall(parent_elem, 'history'):
            history_id = history_elem.get('id')
            if not history_id:
                continue

            history_type = history_elem.get('type', 'shallow')  # W3C SCXML 3.11: shallow or deep

            # W3C SCXML 3.11: History states have default transitions
            # Extract the default target and executable content from the history's transition element
            default_target = None
            default_actions = []
            for trans_elem in ns_findall(history_elem, 'transition'):
                target = trans_elem.get('target')
                if target:
                    default_target = target
                    # W3C SCXML 3.11: Parse executable content of history default transition
                    default_actions = self._parse_executable_content(trans_elem)
                    break  # Use first transition as default

            if default_target:
                # Map history state ID to its default transition target
                self.model.history_default_targets[history_id] = default_target

                # Store comprehensive history state information
                # Leaf target will be resolved later in _resolve_history_targets()
                self.model.history_states[history_id] = {
                    'parent': parent_id,  # Parent compound state
                    'type': history_type,  # shallow or deep
                    'default_target': default_target,  # Default transition target
                    'default_actions': default_actions  # W3C SCXML 3.11: Executable content of default transition
                }

                self.model.has_history_states = True

    def _parse_transition(self, trans_elem) -> Transition:
        """Parse <transition> element (W3C SCXML 3.3)"""
        # W3C SCXML 3.13: Guard condition can be specified via 'cond' or 'expr' attribute
        cond = trans_elem.get('cond', '') or trans_elem.get('expr', '')
        
        # W3C SCXML 5.9.2: Check if condition is pure In() predicate
        is_pure_in = False
        cond_cpp = ''
        if cond and self._is_pure_in_predicate(cond):
            is_pure_in = True
            cond_cpp = self._convert_in_to_cpp(cond)
        
        transition = Transition(
            event=trans_elem.get('event', ''),
            target=trans_elem.get('target', ''),
            cond=cond,
            cond_cpp=cond_cpp,
            is_pure_in_predicate=is_pure_in,
            type=trans_elem.get('type', 'external')
        )

        # Parse executable content
        transition.actions = self._parse_executable_content(trans_elem)

        # Detect guard conditions requiring JSEngine
        if transition.cond:
            if self._requires_jsengine(transition.cond):
                self.model.needs_jsengine = True

        return transition

    def _parse_executable_content(self, parent_elem) -> List[Dict]:
        """
        Parse executable content (actions) from SCXML element

        Handles: raise, send, assign, if, foreach, log, script, cancel
        """
        actions = []

        for child in parent_elem:
            # Skip text nodes and comments
            if not isinstance(child.tag, str):
                continue
            # Use localname to strip namespace
            tag = etree.QName(child).localname
            action = {'type': tag}

            if tag == 'raise':
                # W3C SCXML 3.8.1: <raise>
                action['event'] = child.get('event', '')
                self.model.events.add(action['event'])

            elif tag == 'send':
                # W3C SCXML 6.2: <send>
                action['event'] = child.get('event', '')
                action['eventexpr'] = child.get('eventexpr', '')
                action['target'] = child.get('target', '')
                action['targetexpr'] = child.get('targetexpr', '')
                
                # W3C SCXML 6.2: Detect parent communication (<send target="#_parent">)
                if action['target'] == '#_parent':
                    self.model.has_parent_communication = True
                # W3C SCXML 6.4.1: Detect child communication (<send target="#_child">)
                elif action['target'] == '#_child':
                    self.model.has_child_communication = True
                action['send_type'] = child.get('type', '')  # Renamed from 'type' to avoid conflict
                action['delay'] = child.get('delay', '')
                action['delayexpr'] = child.get('delayexpr', '')
                action['id'] = child.get('id', '')
                action['idlocation'] = child.get('idlocation', '')
                action['namelist'] = child.get('namelist', '')  # W3C SCXML C.1: namelist for event data
                
                # W3C SCXML 6.2.4 & 5.11: Namelist requires JSEngine for variable existence checking
                # W3C SCXML C.1: NamelistHelper::evaluateNamelist() uses JSEngine.getVariable()
                # Even if namelist contains static variable names, we need JSEngine to check if they exist at runtime
                if action['namelist']:
                    self.model.needs_jsengine = True

                # Parse <param> children
                action['params'] = []
                for param in ns_findall(child, 'param'):
                    param_expr = param.get('expr', '')
                    
                    # W3C SCXML Appendix B.2: Detect static string literals for Pure Static optimization (test 531)
                    is_static_literal = False
                    static_value = ''
                    if param_expr and self._is_static_string_literal(param_expr):
                        is_static_literal = True
                        static_value = self._extract_static_string_literal(param_expr)
                    
                    action['params'].append({
                        'name': param.get('name'),
                        'expr': param_expr,
                        'location': param.get('location', ''),
                        'is_static_literal': is_static_literal,
                        'static_value': static_value
                    })

                    # W3C SCXML 5.10/6.2: Param expr requires JSEngine evaluation (test 233)
                    # Per W3C spec, all expr attributes are ECMAScript expressions
                    # Static literals (e.g., 'test') can be optimized at parse time (Pure Static)
                    # Dynamic expressions (including number literals like '2') require JSEngine
                    if param_expr and not is_static_literal:
                        self.model.needs_jsengine = True

                # Parse <content>
                content_elems = ns_findall(child, 'content')
                if content_elems:
                    content_elem = content_elems[0]
                    action['contentexpr'] = content_elem.get('expr', '')

                    # W3C SCXML 6.2 & 5.9.2: Serialize XML child elements (matches <data> parsing behavior)
                    # This enables ECMAScript DOM object creation for _event.data (test 561)
                    content = ''
                    if len(content_elem) > 0:
                        # Has child elements - serialize all children as XML string
                        # Matches DataModelParser behavior for inline XML (test 557)
                        child_xml_parts = []
                        for child_node in content_elem:
                            child_str = etree.tostring(child_node, encoding='unicode', method='xml')
                            child_xml_parts.append(child_str)
                        content = ''.join(child_xml_parts)
                    else:
                        # No child elements - use text content
                        content = content_elem.text or ''

                    action['content'] = content.strip()
                else:
                    action['content'] = ''
                    action['contentexpr'] = ''

                # Detect dynamic expressions
                if action['eventexpr'] or action['targetexpr'] or action['delayexpr']:
                    self.model.has_dynamic_expressions = True
                    self.model.needs_jsengine = True

                # W3C SCXML C.1 (test 496): targetexpr may result in unreachable target → error.communication
                if action['targetexpr']:
                    self.model.events.add('error.communication')

                if action['event']:
                    self.model.events.add(action['event'])
                elif action['content'] and not action['eventexpr']:
                    # W3C SCXML C.2: content-only send (test 520) - empty event name
                    self.model.events.add('')

            elif tag == 'assign':
                # W3C SCXML 5.4: <assign>
                action['location'] = child.get('location', '')
                action['expr'] = child.get('expr', '')
                
                # W3C SCXML 5.4: Handle XML child content (e.g., <assign location="Var1"><scxml>...</scxml></assign>)
                # Serialize child elements to string for expression
                if len(child) > 0:  # Has child elements
                    # Serialize all child elements to string
                    children_xml = ''
                    for child_elem in child:
                        if isinstance(child_elem.tag, str):  # Skip comments/text nodes
                            # Serialize with c14n (canonical XML, no namespace prefixes)
                            elem_bytes = etree.tostring(child_elem, method='c14n')
                            elem_str = elem_bytes.decode('utf-8')
                            # Strip whitespace and newlines for single-line expression
                            elem_str = ' '.join(elem_str.split())
                            children_xml += elem_str
                    if children_xml:
                        action['content'] = children_xml
                else:
                    action['content'] = ''

                # W3C SCXML 5.3: All assignments in ECMAScript datamodel require JSEngine
                # This matches C++ generator behavior
                self.model.needs_jsengine = True

            elif tag == 'if':
                # W3C SCXML 3.12.1: <if><elseif><else> with complex sibling structure
                # Actions after <if> but before <elseif>/<else> are the "then" branch
                # Actions after <elseif> but before next <elseif>/<else> are elseif branches
                # Actions after <else> until </if> are else branch

                cond = child.get('cond', '')
                # W3C SCXML 5.9.2: Check if condition is pure In() predicate
                is_pure_in = False
                cond_cpp = ''
                if cond and self._is_pure_in_predicate(cond):
                    is_pure_in = True
                    cond_cpp = self._convert_in_to_cpp(cond)

                action['cond'] = cond
                action['cond_cpp'] = cond_cpp
                action['is_pure_in_predicate'] = is_pure_in
                action['elseif_branches'] = []
                action['else_actions'] = []
                
                # Collect all children (if block content + elseif/else markers)
                children = list(child)
                action['then_actions'] = []
                current_branch = action['then_actions']
                
                i = 0
                while i < len(children):
                    elem = children[i]
                    # Skip text nodes and comments
                    if not isinstance(elem.tag, str):
                        i += 1
                        continue
                    elem_tag = etree.QName(elem).localname
                    
                    if elem_tag == 'elseif':
                        # Start new elseif branch
                        elseif_cond = elem.get('cond', '')
                        # W3C SCXML 5.9.2: Check if condition is pure In() predicate
                        elseif_is_pure_in = False
                        elseif_cond_cpp = ''
                        if elseif_cond and self._is_pure_in_predicate(elseif_cond):
                            elseif_is_pure_in = True
                            elseif_cond_cpp = self._convert_in_to_cpp(elseif_cond)

                        branch = {
                            'cond': elseif_cond,
                            'cond_cpp': elseif_cond_cpp,
                            'is_pure_in_predicate': elseif_is_pure_in,
                            'actions': []
                        }
                        action['elseif_branches'].append(branch)
                        current_branch = branch['actions']
                    elif elem_tag == 'else':
                        # Start else branch
                        current_branch = action['else_actions']
                    else:
                        # Regular action - add to current branch
                        branch_action = {'type': elem_tag}
                        
                        # Parse based on action type
                        if elem_tag == 'raise':
                            branch_action['event'] = elem.get('event', '')
                            if branch_action['event']:
                                self.model.events.add(branch_action['event'])
                        elif elem_tag == 'send':
                            branch_action.update({
                                'event': elem.get('event', ''),
                                'target': elem.get('target', ''),
                                'targetexpr': elem.get('targetexpr', ''),
                                'type': elem.get('type', ''),
                                'id': elem.get('id', ''),
                                'idlocation': elem.get('idlocation', ''),
                                'delay': elem.get('delay', ''),
                                'delayexpr': elem.get('delayexpr', ''),
                                'namelist': elem.get('namelist', ''),
                                'params': [],
                                'content': ''
                            })
                            # Parse <param> children (W3C SCXML 6.2)
                            for param in ns_findall(elem, 'param'):
                                branch_action['params'].append({
                                    'name': param.get('name'),
                                    'expr': param.get('expr', ''),
                                    'location': param.get('location', '')
                                })
                            # Parse <content> (W3C SCXML 6.2)
                            content_elems = ns_findall(elem, 'content')
                            if content_elems:
                                content_elem = content_elems[0]
                                branch_action['content'] = content_elem.text or ''
                        elif elem_tag == 'assign':
                            branch_action.update({
                                'location': elem.get('location', ''),
                                'expr': elem.get('expr', '')
                            })
                            # W3C SCXML 5.3: All assignments require JSEngine
                            self.model.needs_jsengine = True
                        elif elem_tag == 'log':
                            branch_action.update({
                                'label': elem.get('label', ''),
                                'expr': elem.get('expr', '')
                            })
                        elif elem_tag == 'script':
                            branch_action.update({
                                'src': elem.get('src', ''),
                                'content': elem.text or ''
                            })
                            self.model.needs_jsengine = True
                        
                        current_branch.append(branch_action)
                    
                    i += 1
                
                if self._requires_jsengine(action['cond']):
                    self.model.needs_jsengine = True

            elif tag == 'foreach':
                # W3C SCXML 4.6: <foreach>
                action['array'] = child.get('array', '')
                action['item'] = child.get('item', '')
                action['index'] = child.get('index', '')
                action['actions'] = self._parse_executable_content(child)
                self.model.needs_jsengine = True

            elif tag == 'log':
                # W3C SCXML 3.8.8: <log>
                action['label'] = child.get('label', '')
                action['expr'] = child.get('expr', '')
                
                # W3C SCXML 5.10: Any expr attribute requires JSEngine for variable/expression evaluation
                # This includes: _event, Var1, or any ECMAScript expression
                if action['expr']:
                    self.model.needs_jsengine = True

            elif tag == 'script':
                # W3C SCXML 3.8.6: <script>
                action['src'] = child.get('src', '')
                action['content'] = child.text or ''
                self.model.needs_jsengine = True

            elif tag == 'cancel':
                # W3C SCXML 6.2: <cancel>
                action['sendid'] = child.get('sendid', '')
                action['sendidexpr'] = child.get('sendidexpr', '')

            actions.append(action)

        return actions

    def _parse_donedata(self, donedata_elem) -> Dict:
        """Parse <donedata> element (W3C SCXML 5.5)"""
        donedata = {
            'params': [],
            'content': '',
            'contentexpr': ''
        }

        # Parse <param> elements
        for param_elem in ns_findall(donedata_elem, 'param'):
            param = {
                'name': param_elem.get('name', ''),
                'expr': param_elem.get('expr', ''),
                'location': param_elem.get('location', '')
            }
            donedata['params'].append(param)

        # Parse <content> element
        content_elem = ns_find(donedata_elem, 'content')
        if content_elem is not None:
            donedata['contentexpr'] = content_elem.get('expr', '')
            if content_elem.text:
                donedata['content'] = content_elem.text.strip()

        return donedata

    def _parse_invoke(self, invoke_elem) -> Dict:
        """Parse <invoke> element (W3C SCXML 6.4)"""
        # W3C SCXML 6.4.1: Generate invoke ID if not provided
        invoke_id = invoke_elem.get('id', '')
        if not invoke_id:
            invoke_id = f"_invoke_{self.invoke_counter}"
            self.invoke_counter += 1
        
        invoke = {
            'type': invoke_elem.get('type', ''),
            'src': invoke_elem.get('src', ''),
            'srcexpr': invoke_elem.get('srcexpr', ''),
            'id': invoke_id,
            'idlocation': invoke_elem.get('idlocation', ''),
            'autoforward': invoke_elem.get('autoforward', 'false'),
            'namelist': invoke_elem.get('namelist', ''),
            'params': [],
            'finalize': [],
            'content': '',
            'contentexpr': ''
        }

        # Parse inline content
        content_elem = invoke_elem.find('{http://www.w3.org/2005/07/scxml}content')
        if content_elem is not None:
            # Check for expr attribute (dynamic content expression)
            contentexpr = content_elem.get('expr', '')
            invoke['contentexpr'] = contentexpr

            # Check for inline SCXML child element (static content)
            child_scxml = content_elem.find('{http://www.w3.org/2005/07/scxml}scxml')
            if child_scxml is not None:
                # Store inline SCXML element for later extraction
                invoke['content_scxml'] = child_scxml
                invoke['has_inline_scxml'] = True
            else:
                invoke['content'] = content_elem.get('expr', '')
                invoke['has_inline_scxml'] = False

        # Parse <param> children
        for param in ns_findall(invoke_elem, 'param'):
            invoke['params'].append({
                'name': param.get('name'),
                'expr': param.get('expr', ''),
                'location': param.get('location', '')
            })

        # Parse <finalize>
        finalize_elem = invoke_elem.find('{http://www.w3.org/2005/07/scxml}finalize')
        if finalize_elem is not None:
            invoke['finalize'] = self._parse_executable_content(finalize_elem)

        # ARCHITECTURE.md All-or-Nothing Strategy: Classify invoke as static or dynamic
        # W3C SCXML 6.4: Invoke types and their code generation strategies
        # 
        # Static invoke (Pure AOT):
        #   - type="scxml" (or empty/default)
        #   - src="file.scxml" (compile-time known child SCXML file) OR
        #   - <content><scxml>...</scxml></content> (inline child, extracted to file)
        #   - No dynamic features (srcexpr, contentexpr)
        # 
        # Hybrid invoke (AOT parent + Interpreter child):
        #   - srcexpr="expr" (runtime file path evaluation via JSEngine)
        #   - contentexpr="expr" OR <content expr="var"/> (runtime SCXML content evaluation)
        #   - Parent: AOT code with JSEngine for expression evaluation
        #   - Child: Runtime Interpreter instance via InvokeHelper
        # 
        # Note: srcexpr is now Static Hybrid (was Full Interpreter wrapper)
        #   - Rationale: srcexpr only requires JSEngine for path evaluation, not full wrapper
        #   - Implementation: JSEngine evaluates srcexpr → InvokeHelper loads child SCXML

        has_static_child = (
            invoke['src'] != '' or  # External child SCXML file
            invoke.get('has_inline_scxml', False)  # Inline <content><scxml>...</scxml></content>
        )

        # W3C SCXML 6.4: Classify invoke type
        # 
        # Pure Static Strategy (ARCHITECTURE.md lines 285-315):
        # - All invoke parameters known at compile-time (src, id, namelist, params)
        # - Child SCXML file or inline content available at build-time
        # - No runtime expression evaluation required
        # - Fully compile-time type-safe parent-child communication
        # 
        # W3C SCXML 6.4: <invoke src="child.scxml"/> or <invoke><content>...</content></invoke>
        is_static_invoke = (
            (invoke['type'] == '' or invoke['type'] == 'scxml' or
             invoke['type'] == 'http://www.w3.org/TR/scxml/') and
            invoke['srcexpr'] == '' and  # No dynamic file path (W3C SCXML 6.4.3)
            invoke['contentexpr'] == '' and  # No runtime content expression (W3C SCXML 6.4.4)
            has_static_child  # Must have compile-time known child
        )
        
        # W3C SCXML 6.4: Classify invoke as Static Hybrid (AOT parent + Interpreter child)
        # 
        # Static Hybrid Strategy (ARCHITECTURE.md lines 316-375):
        # - srcexpr/contentexpr require runtime expression evaluation via JSEngine
        # - SCXML content loaded/evaluated at runtime (not compile-time)
        # - AOT parent maintains static state machine structure (enums, switch statements)
        # - Interpreter child handles dynamic SCXML from runtime-evaluated expression
        # 
        # W3C SCXML 6.4.3: srcexpr attribute evaluates to URI identifying SCXML file
        # W3C SCXML 6.4.4: contentexpr attribute evaluates to SCXML content string
        # 
        # Example (srcexpr): <invoke srcexpr="pathVar"/> → JSEngine evaluates pathVar at runtime
        # Example (contentexpr): <invoke contentexpr="scxmlVar"/> → JSEngine evaluates scxmlVar at runtime
        is_hybrid_invoke = (
            (invoke['type'] == '' or invoke['type'] == 'scxml' or
             invoke['type'] == 'http://www.w3.org/TR/scxml' or   # W3C SCXML type (no trailing slash)
             invoke['type'] == 'http://www.w3.org/TR/scxml/') and # W3C SCXML type (with trailing slash - legacy)
            (invoke['srcexpr'] != '' or invoke['contentexpr'] != '')  # Runtime expression (srcexpr or contentexpr)
        )
        
        # Note: srcexpr is no longer considered dynamic file invoke (now Static Hybrid)
        is_dynamic_file_invoke = False  # Deprecated: srcexpr now handled as hybrid invoke

        invoke['is_static'] = is_static_invoke
        invoke['is_hybrid'] = is_hybrid_invoke
        invoke['is_dynamic_file'] = is_dynamic_file_invoke

        # Set model flags
        # Hybrid Strategy: contentexpr → AOT parent + Interpreter child
        # Dynamic file invoke (srcexpr) → Full Interpreter wrapper

        if is_hybrid_invoke:
            self.model.has_hybrid_invoke = True
            self.model.needs_jsengine = True  # JSEngine for srcexpr/contentexpr evaluation

        # W3C SCXML 6.4.1: Namelist validation requires JSEngine (even for static invokes)
        if is_static_invoke and invoke['namelist']:
            self.model.needs_jsengine = True  # JSEngine for namelist variable validation
        
        if is_dynamic_file_invoke:
            # Deprecated: srcexpr now handled as hybrid invoke
            self.model.has_dynamic_file_invoke = True
            self.model.has_dynamic_expressions = True

        return invoke

    def _is_pure_in_predicate(self, expr: str) -> bool:
        """
        Check if expression contains ONLY In() predicates with && and || operators
        
        W3C SCXML 5.9.2: In(stateId) is a built-in function, not an ECMAScript expression.
        Pure In() predicates can be implemented with direct C++ calls without JSEngine.
        
        Examples of pure In() predicates:
        - "In('s1')" → True
        - "In('s1') && In('s2')" → True
        - "In('s1') || In('s2')" → True
        - "In('s1') && typeof x !== 'undefined'" → False (has ECMAScript)
        - "In(stateName)" → False (variable, not static string)
        
        Args:
            expr: Expression string to check
            
        Returns:
            True if expression is ONLY In() predicates, False otherwise
        """
        if not expr or 'In(' not in expr:
            return False
        
        import re
        
        # W3C SCXML B.1: XML entity escaping - convert back for parsing
        # &amp;&amp; → &&, &amp;| → ||
        clean_expr = expr.replace('&amp;&amp;', '&&').replace('&amp;|', '||').strip()
        
        # Pattern: Only In('...') with optional &&, ||, (, ), whitespace
        # In('state') with single quotes and static string literals only
        # Reject: In(var), In("state"), In(`state`)
        pattern = r"^[\s()&|]*(?:In\('[^']+'\)[\s()&|]*)+$"
        
        if not re.match(pattern, clean_expr):
            return False
        
        # Additional validation: No ECMAScript keywords
        ecma_keywords = ['typeof', '_event', 'function', 'var', 'let', 'const', 'return']
        for keyword in ecma_keywords:
            if keyword in clean_expr:
                return False
        
        return True
    
    def _convert_in_to_cpp(self, expr: str) -> str:
        """
        Convert In() predicates to direct C++ isStateActive() calls
        
        W3C SCXML 5.9.2: In(stateId) → this->isStateActive("stateId")
        ARCHITECTURE.md Zero Duplication: Uses InPredicateHelper internally
        
        Examples:
        - "In('s1')" → "this->isStateActive(\"s1\")"
        - "In('s1') && In('s2')" → "this->isStateActive(\"s1\") && this->isStateActive(\"s2\")"
        - "In('s1') || In('s2')" → "this->isStateActive(\"s1\") || this->isStateActive(\"s2\")"
        
        Args:
            expr: Pure In() predicate expression
            
        Returns:
            C++ code with direct isStateActive() calls
        """
        import re
        
        # W3C SCXML B.1: XML entity escaping - convert for C++ code
        cpp_expr = expr.replace('&amp;&amp;', '&&').replace('&amp;|', '||')
        
        # Transform: In('state') → this->isStateActive("state")
        # Use double quotes in C++ for consistency with generated code
        cpp_expr = re.sub(r"In\('([^']+)'\)", r'this->isStateActive("\1")', cpp_expr)
        
        return cpp_expr

    def _actions_to_javascript(self, actions: List[Dict]) -> str:
        """
        Convert finalize actions to JavaScript code (W3C SCXML 6.5)
        
        Supports: assign, script, log, if/elseif/else
        Returns: JavaScript code string (semicolon-separated statements)
        """
        if not actions:
            return ''
        
        js_lines = []
        
        for action in actions:
            action_type = action.get('type', '')
            
            if action_type == 'assign':
                # W3C SCXML 5.4: <assign location="Var1" expr="_event.data.aParam"/>
                location = action.get('location', '')
                expr = action.get('expr', '')
                if location and expr:
                    js_lines.append(f"{location} = {expr};")
            
            elif action_type == 'script':
                # W3C SCXML 5.8: <script>code</script>
                content = action.get('content', '')
                if content:
                    js_lines.append(content)
            
            elif action_type == 'log':
                # W3C SCXML 5.10: <log expr="expr" label="label"/>
                expr = action.get('expr', '')
                label = action.get('label', '')
                if expr:
                    log_msg = f'"{label}: " + {expr}' if label else expr
                    js_lines.append(f"console.log({log_msg});")
            
            elif action_type == 'if':
                # W3C SCXML 3.12.1: <if cond="...">...</if>
                cond = action.get('cond', '')
                then_actions = action.get('then', [])
                elseif_branches = action.get('elseif', [])
                else_actions = action.get('else', [])
                
                if cond:
                    js_lines.append(f"if ({cond}) {{")
                    if then_actions:
                        then_js = self._actions_to_javascript(then_actions)
                        if then_js:
                            js_lines.append(f"  {then_js}")
                    js_lines.append("}")
                    
                    for elseif in elseif_branches:
                        elseif_cond = elseif.get('cond', '')
                        elseif_actions = elseif.get('actions', [])
                        if elseif_cond:
                            js_lines.append(f"else if ({elseif_cond}) {{")
                            if elseif_actions:
                                elseif_js = self._actions_to_javascript(elseif_actions)
                                if elseif_js:
                                    js_lines.append(f"  {elseif_js}")
                            js_lines.append("}")
                    
                    if else_actions:
                        js_lines.append("else {")
                        else_js = self._actions_to_javascript(else_actions)
                        if else_js:
                            js_lines.append(f"  {else_js}")
                        js_lines.append("}")
        
        return ' '.join(js_lines)

    def _requires_jsengine(self, expr: str) -> bool:
        """
        Detect if expression requires JSEngine evaluation

        Checks for:
        - ECMAScript operators (typeof, In(), etc.)
        - _event metadata access
        - Complex expressions
        - C++ reserved keywords that would cause compilation errors
        - Invalid ECMAScript syntax (W3C SCXML 5.9: error.execution on evaluation failure)
        """
        if not expr:
            return False

        # W3C SCXML 5.9.2: Check if expression is ONLY In() predicates
        # Pure In() predicates can be implemented without JSEngine
        if 'In(' in expr:
            self.model.uses_in_predicate = True
            if self._is_pure_in_predicate(expr):
                # Pure In() predicate - no JSEngine needed
                return False
            # Mixed In() with ECMAScript - needs JSEngine
            return True

        # ECMAScript-specific features (excluding In() - handled above)
        js_features = ['typeof', '_event.', 'function', 'var ', 'let ', 'const ']

        for feature in js_features:
            if feature in expr:
                return True

        # W3C SCXML 5.9 & C.2: System-reserved identifiers starting with underscore
        # Examples: _event, _scxmleventname, _name (require JSEngine for runtime access)
        import re
        if re.search(r'\b_[a-zA-Z]\w*\b', expr):
            return True

        # W3C SCXML 5.9: ECMAScript comparison and logical operators require JSEngine
        # Examples: ==, !=, ===, !==, &&, ||, <, >, <=, >=
        # These operators indicate ECMAScript expressions that must be evaluated by JSEngine
        ecmascript_operators = ['==', '!=', '===', '!==', '&&', '||', '<=', '>=', '<', '>']
        for op in ecmascript_operators:
            if op in expr:
                return True

        # W3C SCXML B.2: ECMAScript string/number literals require JSEngine for proper boolean conversion
        # Examples: 'foo' (non-empty string → true), '' (empty string → false), 0 (→ false), 1 (→ true)
        # Must use JSEngine to ensure ECMAScript semantics (test 449)
        # Check for string literals (single or double quotes)
        if re.search(r"['\"]", expr):
            return True

        # Check for event metadata access
        for field in self.EVENT_METADATA_FIELDS:
            if field in expr:
                self.model.has_event_metadata = True
                return True

        # W3C SCXML 5.9: Expressions that cannot be evaluated as boolean or cause errors
        # Check for C++ reserved keywords that would be invalid if directly embedded in C++ code
        # These must be evaluated by JSEngine to properly raise error.execution
        cpp_reserved = ['return', 'break', 'continue', 'goto', 'switch', 'case', 'default',
                        'if', 'else', 'while', 'do', 'for', 'class', 'struct', 'typedef',
                        'using', 'namespace', 'template', 'typename', 'static', 'extern',
                        'inline', 'virtual', 'operator', 'new', 'delete', 'this', 'throw',
                        'try', 'catch', 'public', 'private', 'protected']

        # Check if expression is exactly a reserved word or starts with reserved word followed by non-identifier char
        expr_stripped = expr.strip()
        for keyword in cpp_reserved:
            # Exact match or keyword followed by non-alphanumeric (e.g., "return ", "return;")
            if expr_stripped == keyword or (expr_stripped.startswith(keyword) and
                                            len(expr_stripped) > len(keyword) and
                                            not expr_stripped[len(keyword)].isalnum() and
                                            expr_stripped[len(keyword)] != '_'):
                return True

        return False

    def _is_static_string_literal(self, expr: str) -> bool:
        """
        Detect if expression is a static string literal (e.g., 'test', "test")
        
        Returns True only for simple quoted strings without:
        - Variables or expressions
        - String concatenation
        - Escape sequences beyond basic ones
        
        W3C SCXML Appendix B.2: ECMAScript string literals like 'test' can be
        evaluated at parse time for Pure Static code generation (test 531 optimization)
        """
        if not expr:
            return False
        
        import re
        expr_stripped = expr.strip()
        
        # Match simple single-quoted or double-quoted string literals
        # Pattern: quote + any chars except that quote or backslash + quote
        # This excludes complex strings with variables, concatenation, or escape sequences
        single_quote_pattern = r"^'([^'\\]*)'$"
        double_quote_pattern = r'^"([^"\\]*)"$'
        
        if re.match(single_quote_pattern, expr_stripped) or re.match(double_quote_pattern, expr_stripped):
            return True
        
        return False
    
    def _extract_static_string_literal(self, expr: str) -> str:
        """
        Extract the value from a static string literal
        
        Assumes expr has been validated with _is_static_string_literal()
        Returns the string value without quotes
        """
        import re
        expr_stripped = expr.strip()
        
        # Extract content between quotes
        single_quote_match = re.match(r"^'([^'\\]*)'$", expr_stripped)
        if single_quote_match:
            return single_quote_match.group(1)

        double_quote_match = re.match(r'^"([^"\\]*)"$', expr_stripped)
        if double_quote_match:
            return double_quote_match.group(1)
        
        return expr_stripped  # Fallback (should not happen)

    def _process_static_invokes(self):
        """
        Process static invoke information - extract child names from src paths or inline content

        Matches C++ generator behavior:
        - Removes "file:" prefix from src
        - Extracts basename without extension
        - Generates unique invoke IDs if not specified
        - Detects if child needs JSEngine (for param passing)
        - NEW: Extracts inline <content><scxml>...</scxml></content> to separate files
        """
        from pathlib import Path
        from lxml import etree

        invoke_count = {}  # Track invoke count per state for auto-ID generation
        inline_child_count = 0  # Track inline children for unique naming

        for state in self.model.states.values():
            for invoke in state.invokes:
                if not invoke.get('is_static', False):
                    continue

                # Find corresponding static_invoke entry
                matching_static = None
                for si in state.static_invokes:
                    if si['src'] == invoke['src']:
                        matching_static = si
                        break

                if not matching_static:
                    continue

                # Handle inline <content><scxml> by extracting to separate file
                if invoke.get('has_inline_scxml', False):
                    child_scxml_elem = invoke['content_scxml']

                    # Generate unique child name with parent prefix to avoid conflicts
                    # W3C SCXML 6.4: Multiple tests may use same inline child name (e.g., test338 and test347 both use "machineName")
                    original_child_name = child_scxml_elem.get('name', '')
                    if original_child_name:
                        # Prefix with parent name for uniqueness: test347_machineName
                        child_name = f"{self.model.name}_{original_child_name}"
                    else:
                        # No name attribute: use parent_childN format
                        child_name = f"{self.model.name}_child{inline_child_count}"
                        inline_child_count += 1

                    # Extract to separate file in same directory as parent
                    parent_dir = Path(self.scxml_path).parent if hasattr(self, 'scxml_path') else Path('.')
                    child_scxml_path = parent_dir / f"{child_name}.scxml"

                    # Write inline SCXML to file
                    with open(child_scxml_path, 'w') as f:
                        f.write('<?xml version="1.0"?>\n\n')
                        f.write(etree.tostring(child_scxml_elem, encoding='unicode', pretty_print=True))

                    # Update static_invoke to reference extracted file
                    matching_static['src'] = f"{child_name}.scxml"
                    matching_static['child_name'] = child_name

                    # Parse extracted child to detect JSEngine needs and datamodel variables
                    try:
                        child_parser = SCXMLParser()
                        child_model = child_parser.parse_file(str(child_scxml_path))
                        matching_static['child_needs_jsengine'] = child_model.needs_jsengine
                        # W3C SCXML 6.3.2: Extract child datamodel variable names for namelist validation
                        matching_static['child_datamodel_vars'] = [var['id'] for var in child_model.variables]
                    except Exception:
                        matching_static['child_needs_jsengine'] = True
                        matching_static['child_datamodel_vars'] = []  # Empty list for safety

                # Handle external src (existing logic)
                elif invoke['src']:
                    src = invoke['src']

                    # Remove "file:" prefix if present
                    if src.startswith('file:'):
                        src = src[5:]

                    # Extract basename without extension
                    child_name = Path(src).stem
                    matching_static['child_name'] = child_name

                    # Parse child SCXML file to detect if it needs JSEngine
                    child_needs_jsengine = False
                    try:
                        # Construct child SCXML path relative to parent
                        parent_dir = Path(self.scxml_path).parent if hasattr(self, 'scxml_path') else Path('.')
                        child_scxml_path = parent_dir / f"{child_name}.scxml"

                        if child_scxml_path.exists():
                            # Parse child to check needs_jsengine and extract datamodel variables
                            child_parser = SCXMLParser()
                            child_model = child_parser.parse_file(str(child_scxml_path))
                            child_needs_jsengine = child_model.needs_jsengine
                            # W3C SCXML 6.3.2: Extract child datamodel variable names for namelist validation
                            child_datamodel_vars = [var['id'] for var in child_model.variables]
                    except Exception:
                        # If we can't parse child, assume it needs JSEngine for safety
                        child_needs_jsengine = True
                        child_datamodel_vars = []  # Empty list for safety

                    matching_static['child_needs_jsengine'] = child_needs_jsengine
                    matching_static['child_datamodel_vars'] = child_datamodel_vars

                # Generate invoke ID if not specified (matches C++ generator logic)
                if not matching_static['invoke_id']:
                    state_name = matching_static['state_name']
                    if state_name not in invoke_count:
                        invoke_count[state_name] = 0

                    matching_static['invoke_id'] = f"{state_name}_invoke_{invoke_count[state_name]}"
                    invoke_count[state_name] += 1

    def _resolve_deep_initial(self):
        """
        Resolve deep initial state (matches C++ generator behavior)

        W3C SCXML 3.6: If a compound state has an 'initial' attribute,
        follow it recursively until reaching a leaf (atomic) state.
        
        W3C SCXML 3.13: For parallel states, the initial attribute may contain
        space-separated state IDs representing multiple parallel regions' initial states.

        For example:
        - <scxml initial="s0"> where s0 has initial="s01"
        - This sets model.initial to "s01" (the leaf state)
        - <scxml initial="s2p112 s2p122"> (parallel initial states)
        - This keeps the space-separated format for parallel entry
        """
        if not self.model.initial:
            return

        # W3C SCXML 3.13: Check if initial contains space-separated state IDs (parallel initial states)
        initial_states = self.model.initial.split()
        
        if len(initial_states) > 1:
            # Multiple initial states (parallel entry)
            # Verify all states exist in model
            all_exist = all(state_id in self.model.states for state_id in initial_states)
            
            if all_exist:
                # Keep space-separated format for parallel initial states
                # This will be handled by code generator with parallel entry logic
                return
            else:
                # Some states don't exist - treat as single state (fallback to original behavior)
                initial_states = [self.model.initial]
        
        # Single initial state - resolve to leaf
        MAX_DEPTH = 20  # Safety limit to detect cycles
        current = initial_states[0]
        depth = 0

        while depth < MAX_DEPTH:
            # Check if current state exists
            if current not in self.model.states:
                # Initial state not found - leave as is
                return

            state = self.model.states[current]

            # Check if state has an initial child
            if state.initial and state.initial in self.model.states:
                # Follow to initial child
                current = state.initial
                depth += 1
            else:
                # Reached leaf state or final state
                break

        # Update model.initial to the resolved leaf state
        self.model.initial = current

    def _apply_parallel_initial_overrides(self):
        """
        Apply parallel initial state overrides (W3C SCXML 3.13)
        
        When scxml initial="s1 s2", these states override the initial attributes
        of their parent regions in parallel states.
        
        Example:
          <scxml initial="s2p112 s2p122">
            <parallel id="s2p1">
              <state id="s2p11" initial="s2p111">  <!-- Override to s2p112 -->
                <state id="s2p111"/>
                <state id="s2p112"/>
              </state>
              <state id="s2p12" initial="s2p121">  <!-- Override to s2p122 -->
                <state id="s2p121"/>
                <state id="s2p122"/>
              </state>
            </parallel>
        """
        if not self.model.initial:
            return
        
        # Check if initial contains space-separated states (parallel initial states)
        initial_states = self.model.initial.split()
        
        if len(initial_states) <= 1:
            # Single initial state - no overrides needed
            return
        
        # W3C SCXML 3.13: Override parent region initial attributes
        for state_id in initial_states:
            if state_id not in self.model.states:
                # State not found - skip
                continue
            
            # Find parent of this initial state
            state = self.model.states[state_id]
            parent_id = state.parent
            
            if not parent_id or parent_id not in self.model.states:
                # No parent or parent not found
                continue
            
            # Override parent's initial attribute
            parent_state = self.model.states[parent_id]
            parent_state.initial = state_id
        
        # After overrides, set model.initial to the first state
        # The parallel regions will use their overridden initial attributes
        self.model.initial = initial_states[0]

    def _resolve_history_targets(self):
        """
        Mark transitions targeting history states for runtime restoration (W3C SCXML 3.11)

        W3C SCXML 3.11: Instead of resolving history targets at parse time, we mark them
        for runtime restoration logic generation. This allows full W3C SCXML history
        semantics with recording and restoration.
        """
        if not self.model.history_default_targets:
            return  # No history states

        # Resolve default targets to leaf states
        for history_id, history_info in self.model.history_states.items():
            default_target = history_info['default_target']
            leaf_target = self._resolve_to_leaf_state(default_target)
            history_info['leaf_target'] = leaf_target

        # Mark transitions that target history states
        for state in self.model.states.values():
            for transition in state.transitions:
                if transition.target in self.model.history_default_targets:
                    # Add history restoration marker to transition
                    if not hasattr(transition, 'history_target'):
                        transition.history_target = transition.target
                    # Keep original target for template to generate restoration logic
        
        # W3C SCXML 3.11: Mark states whose initial transition targets history
        # This handles <initial><transition target="h1"/></initial> where h1 is a history state
        # Instead of replacing the target, we mark it for template to generate history restoration logic
        for state in self.model.states.values():
            if state.initial in self.model.history_default_targets:
                history_info = self.model.history_states[state.initial]
                leaf_target = history_info['leaf_target']
                default_actions = history_info.get('default_actions', [])

                # Mark this state as having initial->history transition
                state.initial_history_id = state.initial
                state.initial_history_default_target = leaf_target
                state.initial_history_default_actions = default_actions

                # Keep state.initial as the leaf target for fallback (when history is empty)
                # This ensures getInitialChild() returns a valid state for entry chain building
                state.initial = leaf_target

    def _resolve_to_leaf_state(self, state_id: str) -> str:
        """
        Resolve a state ID to its leaf state by following initial attributes

        Args:
            state_id: State ID to resolve

        Returns:
            Leaf state ID (atomic state with no initial attribute)
        """
        MAX_DEPTH = 20  # Safety limit
        current = state_id
        depth = 0

        while depth < MAX_DEPTH:
            if current not in self.model.states:
                # State not found - return as is
                return current

            state = self.model.states[current]

            # Check if state has an initial child
            if state.initial and state.initial in self.model.states:
                # Follow to initial child
                current = state.initial
                depth += 1
            else:
                # Reached leaf state (or final state, or state without initial)
                break

        return current

    def _compute_parallel_regions(self):
        """
        Compute child regions for each parallel state (W3C SCXML 3.4)
        
        Parallel state regions are direct child states (not nested descendants).
        These will be used for code generation to create per-region state variables.
        """
        for state_id, state in self.model.states.items():
            if state.is_parallel:
                # Find all direct child states
                child_regions = []
                for child_id, child_state in self.model.states.items():
                    if child_state.parent == state_id:
                        child_regions.append(child_id)
                
                # Sort by document order (state IDs in model.states maintain insertion order)
                self.model.parallel_regions[state_id] = child_regions

    def _detect_transition_actions(self):
        """
        Detect if any transitions have executable content (W3C SCXML 3.13)
        
        Sets has_transition_actions flag for conditional static optimization.
        If no transitions have actions, tryTransitionInState can remain static.
        """
        for state in self.model.states.values():
            for transition in state.transitions:
                if transition.actions:
                    self.model.has_transition_actions = True
                    return  # Early exit once we find any action
        
        # No transition actions found
        self.model.has_transition_actions = False

    def _add_done_state_events(self):
        """
        Add done.state events for states with final children (W3C SCXML 3.7)

        When a state has final child states, done.state.{state_id} events
        will be generated automatically at runtime. These need to be added
        to the event enum for compile-time type safety.
        """
        for state_id, state in self.model.states.items():
            # Skip parallel states (they use different done.state generation logic)
            if state.is_parallel:
                continue

            # Check if this state has any final children
            has_final_child = False
            for child_id, child_state in self.model.states.items():
                if child_state.parent == state_id and child_state.is_final:
                    has_final_child = True
                    break

            # If state has final children, add done.state.{state_id} event
            if has_final_child:
                done_event = f"done.state.{state_id}"
                self.model.events.add(done_event)

    def _set_invoke_event_flags(self):
        """
        W3C SCXML 6.4: Determine which invokes use specific done.invoke.id events
        
        After parsing all transitions, check if any transition waits for done.invoke.id event.
        If yes, set use_specific_event flag so template generates Event::Done_invoke_id.
        If no, use generic Event::Done_invoke (matches Interpreter behavior).
        
        Example:
        - test235: <transition event="done.invoke.foo"/> → use Event::Done_invoke_foo
        - test192: <transition event="done.invoke"/> → use Event::Done_invoke
        """
        # Build set of done.invoke.* events that are actually used in transitions
        used_done_invoke_events = set()
        for state in self.model.states.values():
            for transition in state.transitions:
                if transition.event and transition.event.startswith('done.invoke.'):
                    used_done_invoke_events.add(transition.event)
        
        # Update each invoke with flag indicating if specific event is used
        for state in self.model.states.values():
            for invoke_info in state.static_invokes:
                invoke_id = invoke_info['invoke_id']
                specific_event = f"done.invoke.{invoke_id}"
                invoke_info['use_specific_event'] = specific_event in used_done_invoke_events
            
            for invoke_info in state.hybrid_invokes:
                invoke_id = invoke_info['invoke_id']
                specific_event = f"done.invoke.{invoke_id}"
                invoke_info['use_specific_event'] = specific_event in used_done_invoke_events

    def _collect_child_to_parent_events(self):
        """
        Collect events from child state machines that send to parent (#_parent)
        
        W3C SCXML 6.2: Child state machines can send events to parent via &lt;send target="#_parent" event="xxx"/&gt;
        These events must be added to parent's Event enum for compile-time type safety.
        
        ARCHITECTURE.md Zero Duplication &amp; AOT-First Migration:
        - Matches Interpreter behavior where parent handles child events dynamically as strings
        - AOT approach: Auto-generate parent Event enum entries from child send actions
        - Prevents compilation errors when child sends events only caught by wildcard transitions
        
        Example (test 243):
        - Child: &lt;send target="#_parent" event="failure"/&gt;
        - Parent: &lt;transition event="*" target="fail"/&gt;
        - Without this method: ParentStateMachine::Event::Failure → compilation error
        - With this method: Failure automatically added to parent Event enum
        """
        if not self.model.static_invokes:
            return  # No static invokes, no children to scan
        
        # Track already-parsed children to avoid re-parsing
        parsed_children = {}
        
        # Scan all static invokes for child SCXML files
        for static_invoke in self.model.static_invokes:
            child_name = static_invoke.get('child_name', '')
            if not child_name:
                continue
            
            # Skip if already parsed
            if child_name in parsed_children:
                continue
            
            # Construct child SCXML path
            parent_dir = Path(self.scxml_path).parent if hasattr(self, 'scxml_path') else Path('.')
            child_scxml_path = parent_dir / f"{child_name}.scxml"
            
            if not child_scxml_path.exists():
                continue
            
            try:
                # Parse child SCXML to get its send actions
                child_parser = SCXMLParser()
                child_model = child_parser.parse_file(str(child_scxml_path))
                parsed_children[child_name] = child_model
                
                # Scan child for &lt;send target="#_parent"&gt; actions
                child_parent_events = set()
                
                # Scan all states in child
                for child_state in child_model.states.values():
                    # Check entry/exit actions
                    for action in child_state.on_entry + child_state.on_exit:
                        if action.get('type') == 'send' and action.get('target') == '#_parent':
                            event = action.get('event', '')
                            if event:
                                child_parent_events.add(event)
                    
                    # Check transition actions
                    for transition in child_state.transitions:
                        for action in transition.actions:
                            if action.get('type') == 'send' and action.get('target') == '#_parent':
                                event = action.get('event', '')
                                if event:
                                    child_parent_events.add(event)
                    
                    # W3C SCXML 3.3.2: Check initial transition actions
                    for action in child_state.initial_transition_actions:
                        if action.get('type') == 'send' and action.get('target') == '#_parent':
                            event = action.get('event', '')
                            if event:
                                child_parent_events.add(event)
                
                # Add collected events to parent's event set
                for event in child_parent_events:
                    self.model.events.add(event)
                    
            except Exception as e:
                # If child parsing fails, log warning but continue
                # This prevents parent parsing failure due to child SCXML errors
                import logging
                logging.warning(f"Failed to parse child SCXML {child_scxml_path} for parent event collection: {e}")
                continue

    def _parse_initial_children(self):
        """
        Parse space-separated initial attributes into lists with validation
        
        W3C SCXML 3.6: The initial attribute can contain space-separated descendant state IDs.
        Example: <state id="s1" initial="s11p112 s11p122">
        
        This method ONLY parses the attribute - ancestor path calculation is delegated
        to StateEntryHelper (ARCHITECTURE.md Zero Duplication Principle).
        
        Validation:
        - All initial targets must exist in the model
        - Invalid targets result in code generation error (reject invalid SCXML)
        """
        for state in self.model.states.values():
            if state.initial:
                # W3C SCXML 3.6: Split space-separated initial attribute
                state.initial_children = state.initial.split()
                
                # Validate all targets exist (reject invalid SCXML)
                for child_id in state.initial_children:
                    if child_id not in self.model.states:
                        raise ValueError(
                            f"W3C SCXML 3.6: Invalid initial target '{child_id}' in state '{state.id}'. "
                            f"Initial attribute references non-existent state."
                        )
            else:
                state.initial_children = []

    def _detect_features(self):
        """
        Detect features requiring Interpreter wrapper

        Sets flags:
        - has_dynamic_expressions
        - has_parallel_states
        - has_invoke
        - has_event_metadata
        - needs_jsengine
        """
        # Check for event metadata in guards
        for state in self.model.states.values():
            for transition in state.transitions:
                if transition.cond:
                    for field in self.EVENT_METADATA_FIELDS:
                        if field in transition.cond:
                            self.model.has_event_metadata = True
                            self.model.needs_jsengine = True

        # Summary
        return {
            'needs_jsengine': self.model.needs_jsengine,
            'uses_in_predicate': self.model.uses_in_predicate,
            'has_dynamic_expressions': self.model.has_dynamic_expressions,
            'has_parallel_states': self.model.has_parallel_states,
            'has_invoke': self.model.has_invoke,
            'has_event_metadata': self.model.has_event_metadata
        }


if __name__ == '__main__':
    import sys

    if len(sys.argv) < 2:
        print("Usage: python scxml_parser.py <scxml_file>")
        sys.exit(1)

    parser = SCXMLParser()
    model = parser.parse_file(sys.argv[1])

    print(f"Model: {model.name}")
    print(f"Initial: {model.initial}")
    print(f"States: {len(model.states)}")
    print(f"Events: {len(model.events)}")
    print(f"Needs JSEngine: {model.needs_jsengine}")
    print(f"Variables: {len(model.variables)}")
