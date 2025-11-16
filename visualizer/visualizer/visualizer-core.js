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
    TEXT_PADDING: 8                // Additional padding for text positioning
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
    
    constructor(containerId, scxmlStructure) {
        this.container = d3.select(`#${containerId}`);
        this.states = scxmlStructure.states || [];
        this.transitions = scxmlStructure.transitions || [];
        this.initialState = scxmlStructure.initial || '';
        this.activeStates = new Set();

        // Debug mode from URL parameter (?debug)
        this.debugMode = DEBUG_MODE;

        // Show snap points from URL parameter (?show-snap)
        this.showSnapPoints = new URLSearchParams(window.location.search).has('show-snap');

        // Adaptive algorithm selection for drag optimization
        this.dragOptimizationTimer = null;
        this.isDraggingAny = false;

        // Progressive optimization: background CSP cancellation
        this.backgroundOptimization = null;

        // Timeout tracking for consistent cancellation (like State Actions DOM re-render)
        this.transitionHighlightTimeout = null;
        this.transitionPanelHighlightTimeout = null;

        // Container dimensions
        const containerNode = this.container.node();
        const clientWidth = containerNode ? containerNode.clientWidth : 0;
        const clientHeight = containerNode ? containerNode.clientHeight : 0;

        this.width = clientWidth > 0 ? clientWidth : 800;
        this.height = clientHeight > 0 ? clientHeight : 500;

        console.log(`Initializing ELK-based visualizer: ${this.width}x${this.height}`);
        console.log(`Initial state: ${this.initialState}`);
        console.log(`States: ${this.states.length}, Transitions: ${this.transitions.length}`);

        if (this.debugMode) {
            console.log('[DEBUG] Transition details:', this.transitions);
        }

        // Initialize ELK
        this.elk = new ELK();

        // Initialize helper modules (must be before initGraph)
        this.nodeBuilder = new NodeBuilder(this);
        this.linkBuilder = new LinkBuilder(this);
        this.layoutManager = new LayoutManager(this);
        this.renderer = new Renderer(this);
        this.pathCalculator = new PathCalculator(this);
        this.interactionHandler = new InteractionHandler(this);

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
        this.allLinks = this.buildLinks();

        // Initialize layout optimizer
        this.layoutOptimizer = new TransitionLayoutOptimizer(this.nodes, this.allLinks, this);

        // Compute layout
        await this.computeLayout();

        // Render
        this.render();

        // Center diagram in viewport
        this.centerDiagram();
    }

    async computeLayout() {
        if (this.debugMode) {
            console.log('Computing ELK layout...');
        }

        const elkGraph = this.buildELKGraph();
        const layouted = await this.elk.layout(elkGraph);

        this.applyELKLayout(layouted);

        if (this.debugMode) {
            console.log('ELK layout computed');
        }
    }

    centerDiagram() {
        const visibleNodes = this.getVisibleNodes();

        if (visibleNodes.length === 0) {
            console.log('[CENTER] No visible nodes, skipping center alignment');
            return;
        }

        // Calculate bounding box of all visible nodes
        let minX = Infinity, minY = Infinity;
        let maxX = -Infinity, maxY = -Infinity;

        visibleNodes.forEach(node => {
            if (node.x !== undefined && node.y !== undefined &&
                node.width !== undefined && node.height !== undefined) {
                const left = node.x - node.width / 2;
                const right = node.x + node.width / 2;
                const top = node.y - node.height / 2;
                const bottom = node.y + node.height / 2;

                minX = Math.min(minX, left);
                maxX = Math.max(maxX, right);
                minY = Math.min(minY, top);
                maxY = Math.max(maxY, bottom);
            }
        });

        const diagramWidth = maxX - minX;
        const diagramHeight = maxY - minY;
        const diagramCenterX = minX + diagramWidth / 2;
        const diagramCenterY = minY + diagramHeight / 2;

        // Calculate scale to fit diagram in viewport with padding
        const padding = 40; // 40px padding on each side
        const availableWidth = this.width - 2 * padding;
        const availableHeight = this.height - 2 * padding;

        const scaleX = availableWidth / diagramWidth;
        const scaleY = availableHeight / diagramHeight;
        const scale = Math.min(scaleX, scaleY, 1.0); // Don't zoom in beyond 1.0

        // Calculate transform to center the diagram
        const transform = d3.zoomIdentity
            .translate(this.width / 2, this.height / 2)
            .scale(scale)
            .translate(-diagramCenterX, -diagramCenterY);

        // Apply transform
        this.svg.transition()
            .duration(750)
            .call(this.zoom.transform, transform);

        // Update initialTransform so resetView() returns to centered state
        this.initialTransform = transform;

        console.log(`[CENTER] Diagram centered: bbox=(${minX.toFixed(1)}, ${minY.toFixed(1)}, ${maxX.toFixed(1)}, ${maxY.toFixed(1)}), scale=${scale.toFixed(2)}`);
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
    findCollapsedAncestor(nodeId, nodes) { return this.linkBuilder.findCollapsedAncestor(nodeId, nodes); }

    buildELKGraph() { return this.layoutManager.buildELKGraph(); }
    applyELKLayout(layouted) { return this.layoutManager.applyELKLayout(layouted); }
    updateCompoundBounds(compoundNode) { return this.layoutManager.updateCompoundBounds(compoundNode); }
    findCompoundParent(nodeId) { return this.layoutManager.findCompoundParent(nodeId); }
    findTopmostCompoundParent(nodeId) { return this.layoutManager.findTopmostCompoundParent(nodeId); }
    getAllDescendantIds(parentId) { return this.layoutManager.getAllDescendantIds(parentId); }

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
    cancelPendingHighlights() { return this.interactionHandler.cancelPendingHighlights(); }
    highlightLink(linkId, duration) { return this.interactionHandler.highlightLink(linkId, duration); }
    highlightLabel(linkId, duration) { return this.interactionHandler.highlightLabel(linkId, duration); }
    scheduleHighlightRemoval(linkId, duration) { return this.interactionHandler.scheduleHighlightRemoval(linkId, duration); }
    focusOnTransition(transitionId) { return this.interactionHandler.focusOnTransition(transitionId); }
    resize() { return this.interactionHandler.resize(); }
    resetView() { return this.interactionHandler.resetView(); }
    toggleCompoundState(stateId) { return this.interactionHandler.toggleCompoundState(stateId); }
}
