// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * SCXML State Diagram Visualizer using ELK.js + D3.js
 *
 * Uses ELK (Eclipse Layout Kernel) for hierarchical layout computation
 * and D3.js for SVG rendering and interaction.
 */

// Debug mode
const DEBUG_MODE = new URLSearchParams(window.location.search).has('debug');

// PANEL_HIGHLIGHT_DURATION is defined in execution-controller.js (loaded before this file)

// Layout constants for node sizing
const LAYOUT_CONSTANTS = {
    STATE_BASE_HEIGHT: 70,        // Base height for state ID + separator + padding
    ACTION_HEIGHT: 34,             // Height per action (box + spacing)
    STATE_MIN_WIDTH: 140,          // Minimum width for state box
    STATE_MAX_WIDTH: 800,          // Maximum width to prevent excessive expansion
    STATE_MIN_HEIGHT: 50,          // Minimum height for states without actions
    TEXT_LEFT_MARGIN_PERCENT: 0.10, // Left margin as percentage of state width (10%)
    TEXT_PADDING: 8,               // Additional padding for text positioning
    VIEWPORT_PADDING: 40           // Padding for centerDiagram viewport fit
};

// ELK (Eclipse Layout Kernel) configuration constants
const ELK_LAYOUT_CONFIG = {
    NODE_SPACING: '150',           // Horizontal spacing between nodes on same layer
    LAYER_SPACING: '180',          // Vertical spacing between layers
    COMPOUND_PADDING_TOP: '60',    // Top padding for compound/parallel containers
    COMPOUND_PADDING_SIDE: '35',   // Left/right/bottom padding for compounds
    PARALLEL_CHILD_SPACING: '40'   // Spacing between parallel children (horizontal layout)
};

// Path routing constants
const PATH_CONSTANTS = {
    MIN_SEGMENT_LENGTH: 30,        // Minimum orthogonal path segment length (prevents tight corners)
    COMPOUND_BOUNDS_PADDING: 25,   // Padding when calculating compound bounding boxes from children
    INITIAL_NODE_HALF_WIDTH: 30    // Half-width of initial pseudo-node for boundary calculations
};

class SCXMLVisualizer {
    // Layout constants
    static COMPOUND_PADDING = 25;
    static COMPOUND_TOP_PADDING = 50;
    
    // Helper: Check if node is compound or parallel type
    static isCompoundOrParallel(node) {
        return node && (node.type === 'compound' || node.type === 'parallel');
    }
    
    /**
     * Find the outermost collapsed compound/parallel ancestor of a node
     * Used for snap point calculation and visual redirect when states are collapsed
     * @param {string} nodeId - Node ID to find ancestor for
     * @param {Array} nodes - All nodes in the graph
     * @returns {Object|null} Collapsed ancestor node or null
     */
    static findCollapsedAncestor(nodeId, nodes) {
        let outermostCollapsed = null;
        
        // Find direct parent first
        const parent = nodes.find(n => 
            SCXMLVisualizer.isCompoundOrParallel(n) &&  // Only compound/parallel can have children
            n.children &&
            n.children.includes(nodeId)
        );
        
        if (!parent) {
            return null;
        }
        
        // If parent is collapsed, remember it
        if (parent.collapsed) {
            outermostCollapsed = parent;
        }
        
        // Recursively check if parent has collapsed ancestors
        const grandparent = SCXMLVisualizer.findCollapsedAncestor(parent.id, nodes);
        if (grandparent) {
            outermostCollapsed = grandparent;  // Prefer outermost
        }
        
        return outermostCollapsed;
    }
    
    constructor(containerId, scxmlStructure) {
        this.container = d3.select(`#${containerId}`);
        this.states = scxmlStructure.states || [];
        this.transitions = scxmlStructure.transitions || [];
        this.initialState = scxmlStructure.initial || '';
        this.activeStates = new Set();

        // Debug mode from URL parameter (?debug)
        this.debugMode = DEBUG_MODE;

        // Show snap points from URL parameter (?debug)
        this.showSnapPoints = new URLSearchParams(window.location.search).has('debug');

        // Adaptive algorithm selection for drag optimization
        this.dragOptimizationTimer = null;
        this.isDraggingAny = false;

        // Progressive optimization: background CSP cancellation
        this.backgroundOptimization = null;

        // Timeout tracking for consistent cancellation (like State Actions DOM re-render)
        this.transitionHighlightTimeout = null;
        this.transitionPanelHighlightTimeout = null;

        // Custom transition label positions
        // Map: transitionKey -> { x, y, routingHash }
        // routingHash tracks when transition coordinates change (for auto-reset)
        this.customLabelPositions = new Map();

        // Container dimensions
        const containerNode = this.container.node();
        const clientWidth = containerNode ? containerNode.clientWidth : 0;
        const clientHeight = containerNode ? containerNode.clientHeight : 0;

        this.width = clientWidth > 0 ? clientWidth : 800;
        this.height = clientHeight > 0 ? clientHeight : 500;

        logger.debug(`Initializing ELK-based visualizer: ${this.width}x${this.height}`);
        logger.debug(`Initial state: ${this.initialState}`);
        logger.debug(`States: ${this.states.length}, Transitions: ${this.transitions.length}`);

        if (this.debugMode) {
            logger.debug('[DEBUG] Transition details:', this.transitions);
        }

        // Initialize ELK
        this.elk = new ELK();

        // Initialize helper modules (must be before initGraph)
        this.nodeBuilder = new NodeBuilder(this);
        this.linkBuilder = new LinkBuilder(this);
        this.layoutManager = new LayoutManager(this);
        this.renderer = new Renderer(this);
        this.pathCalculator = new PathCalculator(this);
        this.focusManager = new TransitionFocusManager(this);
        this.interactionHandler = new InteractionHandler(this);
        this.collisionDetector = new CollisionDetector(this);

        // Store initialization promise for external waiting
        this.initPromise = this.initGraph();

        setTimeout(() => this.resize(), 100);
    }

    /**
     * Initialize graph
     */
    async initGraph() {
        // Clear previous content
        this.container.selectAll('*').remove();

        // Create SVG
        this.svg = this.container
            .append('svg')
            .attr('viewBox', `0 0 ${this.width} ${this.height}`)
            .attr('preserveAspectRatio', 'xMidYMid meet');

        // Zoom container
        this.zoomContainer = this.svg.append('g').attr('class', 'zoom-container');

        // Zoom behavior
        this.zoom = d3.zoom()
            .scaleExtent([0.1, 4])
            .on('zoom', (event) => {
                this.zoomContainer.attr('transform', event.transform);
            });

        this.svg.call(this.zoom);
        this.initialTransform = d3.zoomIdentity;

        // Arrowhead markers (default + color variants)
        const defs = this.svg.append('defs');

        // Default arrowhead
        defs.append('marker')
            .attr('id', 'arrowhead')
            .attr('viewBox', '0 -5 10 10')
            .attr('refX', 7)
            .attr('refY', 0)
            .attr('markerWidth', 6)
            .attr('markerHeight', 6)
            .attr('orient', 'auto')
            .append('path')
            .attr('d', 'M0,-5L10,0L0,5')
            .attr('fill', '#57606a');

        // Color variant arrowheads (matching CSS transition-color-N)
        const arrowColors = [
            '#0969da',  // Blue (color-0)
            '#1a7f37',  // Green (color-1)
            '#d18616',  // Orange (color-2)
            '#8250df',  // Purple (color-3)
            '#0a7ea4',  // Teal (color-4)
            '#cf222e'   // Pink/Red (color-5)
        ];

        arrowColors.forEach((color, index) => {
            defs.append('marker')
                .attr('id', `arrowhead-${index}`)
                .attr('viewBox', '0 -5 10 10')
                .attr('refX', 7)
                .attr('refY', 0)
                .attr('markerWidth', 6)
                .attr('markerHeight', 6)
                .attr('orient', 'auto')
                .append('path')
                .attr('d', 'M0,-5L10,0L0,5')
                .attr('fill', color);
        });

        // Build data
        this.nodes = this.buildNodes();
        
        // W3C SCXML 3.6: Auto-expand ancestors of initial targets
        this.nodeBuilder.expandInitialPaths(this.nodes);
        
        this.allLinks = this.buildLinks();

        // Initialize layout optimizer
        this.layoutOptimizer = new TransitionLayoutOptimizer(this.nodes, this.allLinks, this);

        // Compute layout
        await this.computeLayout();

        // Render
        this.render();
        
        // Note: centerDiagram is called in main.js after container is visible
        // to ensure accurate dimensions (not 0x0)
    }

    async computeLayout() {
        if (this.debugMode) {
            logger.debug('Computing ELK layout...');
        }

        const elkGraph = this.buildELKGraph();
        const layouted = await this.elk.layout(elkGraph);

        this.applyELKLayout(layouted);

        if (this.debugMode) {
            logger.debug('ELK layout computed');
        }
    }

    centerDiagram(targetStates = null) {
        // Delegated to FocusManager for centralized focus management
        return this.focusManager.centerDiagram(targetStates);
    }

    // Delegate methods to helper modules
    buildNodes() { return this.nodeBuilder.buildNodes(); }
    getNodeWidth(node) { return this.nodeBuilder.getNodeWidth(node); }
    getNodeHeight(node) { return this.nodeBuilder.getNodeHeight(node); }
    getVisibleNodes() { 
        // Always recalculate - visibility is cheap and critical to be correct
        return this.nodeBuilder.getVisibleNodes();
    }

    buildLinks() { return this.linkBuilder.buildLinks(); }
    getVisibleLinks(allLinks, nodes) {
        // Always recalculate - link visibility depends on node states
        return this.linkBuilder.getVisibleLinks(allLinks, nodes);
    }
    findCollapsedAncestor(nodeId, nodes) { return SCXMLVisualizer.findCollapsedAncestor(nodeId, nodes); }

    buildELKGraph() { return this.layoutManager.buildELKGraph(); }
    applyELKLayout(layouted) { return this.layoutManager.applyELKLayout(layouted); }
    updateCompoundBounds(compoundNode) { return this.layoutManager.updateCompoundBounds(compoundNode); }
    findCompoundParent(nodeId) { return this.layoutManager.findCompoundParent(nodeId); }
    findTopmostCompoundParent(nodeId) { return this.layoutManager.findTopmostCompoundParent(nodeId); }
    getAllDescendantIds(parentId) { return this.layoutManager.getAllDescendantIds(parentId); }

    /**
     * Custom transition label position management
     *
     * Allows users to drag transition labels to custom positions.
     * Positions are automatically reset when transition coordinates change.
     * Positions persist across state navigation (collapsed/expanded states).
     */

    /**
     * Get unique key for transition label position storage
     * Uses visualSource/visualTarget to support collapsed states
     * @param {Object} transition - Transition data
     * @returns {string} Unique key (e.g., "s1→s2")
     */
    getTransitionKey(transition) {
        const visualSource = transition.visualSource || transition.source;
        const visualTarget = transition.visualTarget || transition.target;
        return `${visualSource}→${visualTarget}`;
    }

    /**
     * Create hash from transition routing to detect coordinate changes
     * Includes source/target points and edges for complete change detection
     * @param {Object} transition - Transition data with routing information
     * @returns {string|null} Routing hash or null if no routing
     */
    getRoutingHash(transition) {
        if (!transition.routing || !transition.routing.sourcePoint || !transition.routing.targetPoint) {
            return null;
        }
        const sp = transition.routing.sourcePoint;
        const tp = transition.routing.targetPoint;
        const se = transition.routing.sourceEdge;
        const te = transition.routing.targetEdge;
        return `${sp.x.toFixed(1)},${sp.y.toFixed(1)}[${se}]-${tp.x.toFixed(1)},${tp.y.toFixed(1)}[${te}]`;
    }

    /**
     * Save custom position for transition label
     * Stores position with routing hash for auto-reset on coordinate changes
     * @param {Object} transition - Transition data
     * @param {number} x - Center X coordinate
     * @param {number} y - Center Y coordinate
     */
    setCustomLabelPosition(transition, x, y) {
        const key = this.getTransitionKey(transition);
        const routingHash = this.getRoutingHash(transition);
        this.customLabelPositions.set(key, { x, y, routingHash });
        if (this.debugMode) {
            logger.debug(`[CUSTOM LABEL] Set position for ${key}: (${x.toFixed(1)}, ${y.toFixed(1)})`);
        }
    }

    /**
     * Get custom position for transition label if available
     * Returns null if no custom position or routing has changed
     * @param {Object} transition - Transition data
     * @returns {Object|null} Position {x, y} or null
     */
    getCustomLabelPosition(transition) {
        const key = this.getTransitionKey(transition);
        const stored = this.customLabelPositions.get(key);

        if (!stored) {
            return null; // No custom position
        }

        // Check if routing coordinates have changed (auto-reset)
        const currentRoutingHash = this.getRoutingHash(transition);
        if (currentRoutingHash && stored.routingHash !== currentRoutingHash) {
            if (this.debugMode) {
                logger.debug(`[CUSTOM LABEL] Routing changed for ${key}, resetting to default`);
            }
            this.customLabelPositions.delete(key);
            return null;
        }

        return { x: stored.x, y: stored.y };
    }

    render() { return this.renderer.render(); }
    generateSnapPointsData(visibleNodes, visibleLinks = null) { return this.renderer.generateSnapPointsData(visibleNodes, visibleLinks); }
    renderSnapPoints(visibleNodes, visibleLinks = null) { return this.renderer.renderSnapPoints(visibleNodes, visibleLinks); }
    updateSnapPointPositions() { return this.renderer.updateSnapPointPositions(); }
    getDirectionForEdge(edge) { return this.renderer.getDirectionForEdge(edge); }
    getActionTextLeftMargin(node) { return this.renderer.getActionTextLeftMargin(node); }
    renderActionTexts(stateGroup, node) { return this.renderer.renderActionTexts(stateGroup, node); }
    formatActionText(action) { return this.renderer.formatActionText(action); }

    getTransitionLabelText(link) { return this.pathCalculator.getTransitionLabelText(link); }
    getTransitionLabelPosition(link, path) { return this.pathCalculator.getTransitionLabelPosition(link, path); }
    getNodeSide(node, point) { return this.pathCalculator.getNodeSide(node, point); }
    getNodeBoundaryPoint(node, angle) { return this.pathCalculator.getNodeBoundaryPoint(node, angle); }
    getOrthogonalIncomingDirection(sourceNode, targetNode, snapPoint) { return this.pathCalculator.getOrthogonalIncomingDirection(sourceNode, targetNode, snapPoint); }
    getOrthogonalOutgoingDirection(sourceNode, targetNode, snapPoint) { return this.pathCalculator.getOrthogonalOutgoingDirection(sourceNode, targetNode, snapPoint); }
    getOrthogonalBoundaryPoint(node, direction) { return this.pathCalculator.getOrthogonalBoundaryPoint(node, direction); }
    getNodeBounds(node, padding = 0) { return this.pathCalculator.getNodeBounds(node, padding); }
    horizontalLineIntersectsNode(node, y, x1, x2) { return this.pathCalculator.horizontalLineIntersectsNode(node, y, x1, x2); }
    verticalLineIntersectsNode(node, x, y1, y2) { return this.pathCalculator.verticalLineIntersectsNode(node, x, y1, y2); }
    getObstacleNodes(sourceId, targetId) { return this.pathCalculator.getObstacleNodes(sourceId, targetId); }
    findCollisionFreeY(x, y, direction, obstacles, margin = 20) { return this.pathCalculator.findCollisionFreeY(x, y, direction, obstacles, margin); }
    calculateLinkDirections(sourceNode, targetNode, link) { return this.pathCalculator.calculateLinkDirections(sourceNode, targetNode, link); }
    createOrthogonalPath(sourceNode, targetNode, link, snapConfig) { return this.pathCalculator.createOrthogonalPath(sourceNode, targetNode, link, snapConfig); }
    getLinkPath(link) { return this.pathCalculator.getLinkPath(link); }

    updateLinks(fastMode = false) { return this.interactionHandler.updateLinks(fastMode); }
    updateLinksFast() { return this.interactionHandler.updateLinksFast(); }
    updateLinksOptimal() { return this.interactionHandler.updateLinksOptimal(); }
    highlightActiveStates(activeStates) { return this.interactionHandler.highlightActiveStates(activeStates); }
    highlightActiveStatesVisual() { return this.interactionHandler.highlightActiveStatesVisual(); }
    animateTransition(transitionId) { return this.interactionHandler.animateTransition(transitionId); }
    renderTransitionList(allTransitions) { return this.interactionHandler.renderTransitionList(allTransitions); }
    setActiveTransition(transitionId) { return this.interactionHandler.setActiveTransition(transitionId); }
    clearTransitionHighlights() { return this.interactionHandler.clearTransitionHighlights(); }
    clearActiveTransition() { return this.interactionHandler.clearActiveTransition(); }
    highlightTransition(transitionId, duration) { return this.interactionHandler.highlightTransition(transitionId, duration); }
    cancelPendingHighlights() { return this.focusManager.cancelPendingHighlights(); }
    highlightLink(linkId) { return this.focusManager.highlightLink(linkId); }
    highlightLabel(linkId) { return this.focusManager.highlightLabel(linkId); }
    scheduleHighlightRemoval(linkId, duration) { return this.focusManager.scheduleHighlightRemoval(linkId, duration); }
    focusOnTransition(transitionId) { return this.interactionHandler.focusOnTransition(transitionId); }
    resize() { return this.interactionHandler.resize(); }
    resetView() { return this.interactionHandler.resetView(); }
    toggleCompoundState(stateId) { return this.interactionHandler.toggleCompoundState(stateId); }
}
