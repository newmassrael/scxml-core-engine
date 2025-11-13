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

        if (DEBUG_MODE) {
            console.log('[DEBUG] Transition details:', this.transitions);
        }

        // Initialize ELK
        this.elk = new ELK();

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

        // Arrowhead marker
        this.svg.append('defs').append('marker')
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

        // Build data
        this.nodes = this.buildNodes();
        this.allLinks = this.buildLinks();

        // Initialize layout optimizer
        this.layoutOptimizer = new TransitionLayoutOptimizer(this.nodes, this.allLinks);

        // Compute layout
        await this.computeLayout();

        // Render
        this.render();
    }

    /**
     * Build nodes from states
     */
    buildNodes() {
        const nodes = [];

        // Add initial pseudo-node
        if (this.initialState) {
            nodes.push({
                id: '__initial__',
                type: 'initial-pseudo',
                label: '',
                children: [],
                collapsed: false
            });
        }

        this.states.forEach(state => {
            const node = {
                id: state.id,
                type: state.type,
                label: state.id,
                children: state.children || [],
                // W3C SCXML 3.4: Parallel states expanded by default to show concurrent regions
                collapsed: (state.type === 'compound'),  // Only compound states collapsed, parallel expanded
                onentry: state.onentry || [],
                onexit: state.onexit || [],
                // W3C SCXML 6.3: Invoke metadata for child SCXML navigation
                hasInvoke: state.hasInvoke || false,
                invokeSrc: state.invokeSrc || null,
                invokeSrcExpr: state.invokeSrcExpr || null,
                invokeId: state.invokeId || null
            };

            // Debug: Log children arrays for compound/parallel states
            if ((state.type === 'compound' || state.type === 'parallel') && node.children.length > 0) {
                console.log(`[buildNodes] ${state.id} (${state.type}) has children: ${node.children.join(', ')}`);
            }

            nodes.push(node);
        });

        return nodes;
    }

    /**
     * Build links from transitions
     */
    buildLinks() {
        const links = [];

        // Initial transition
        if (this.initialState) {
            links.push({
                id: `__initial___${this.initialState}`,
                source: '__initial__',
                target: this.initialState,
                linkType: 'initial'
            });
        }

        // Containment links
        this.states.forEach(state => {
            if (state.children && state.children.length > 0) {
                state.children.forEach(childId => {
                    links.push({
                        id: `${state.id}_contains_${childId}`,
                        source: state.id,
                        target: childId,
                        linkType: 'containment'
                    });
                });
            }
        });

        // History delegation
        this.states.forEach(state => {
            if (state.type === 'history' && state.defaultTarget) {
                links.push({
                    id: `${state.id}_delegates_${state.defaultTarget}`,
                    source: state.id,
                    target: state.defaultTarget,
                    linkType: 'delegation'
                });
            }
        });

        // Transition links
        this.transitions.forEach(transition => {
            links.push({
                id: transition.id,
                source: transition.source,
                target: transition.target,
                event: transition.event,
                cond: transition.cond,
                linkType: 'transition',
                actions: transition.actions || [],
                guards: transition.guards || []
            });
        });

        return links;
    }

    /**
     * Compute ELK layout
     */
    async computeLayout() {
        console.log('Computing ELK layout...');

        const elkGraph = this.buildELKGraph();
        const layouted = await this.elk.layout(elkGraph);

        this.applyELKLayout(layouted);

        console.log('ELK layout computed');
    }

    /**
     * Build ELK graph structure
     */
    buildELKGraph() {
        const graph = {
            id: 'root',
            layoutOptions: {
                'elk.algorithm': 'layered',
                'elk.direction': 'DOWN',
                'elk.spacing.nodeNode': ELK_LAYOUT_CONFIG.NODE_SPACING,
                'elk.layered.spacing.nodeNodeBetweenLayers': ELK_LAYOUT_CONFIG.LAYER_SPACING,
                'elk.edgeRouting': 'ORTHOGONAL',
                'elk.layered.unnecessaryBendpoints': 'false',
                'elk.layered.nodePlacement.favorStraightEdges': 'true',

                // Crossing minimization options
                'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',
                'elk.layered.crossingMinimization.greedySwitch.type': 'TWO_SIDED',
                'elk.layered.crossingMinimization.greedySwitch.activationThreshold': '40',

                // Edge straightening for better routing
                'elk.layered.nodePlacement.bk.edgeStraightening': 'IMPROVE_STRAIGHTNESS',

                // Edge spacing to reduce crossings
                'elk.layered.spacing.edgeNodeBetweenLayers': '15',
                'elk.layered.spacing.edgeEdgeBetweenLayers': '10',

                // Consider edge direction for better layout
                'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',

                // Hierarchical crossing minimization
                'elk.layered.crossingMinimization.hierarchicalSweepiness': '0.1'

                // Use default bottom-up layout: children computed first, then parents sized to fit
            },
            children: [],
            edges: []
        };

        const visibleNodes = this.getVisibleNodes();
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));

        // Recursive function to build ELK node with nested children
        const buildELKNode = (node, depth = 0) => {
            const indent = '  '.repeat(depth);
            console.log(`${indent}[buildELKNode] Building ${node.id} (${node.type}, collapsed=${node.collapsed})`);

            const elkNode = {
                id: node.id,
                width: this.getNodeWidth(node),
                height: this.getNodeHeight(node)
            };

            // Add children for expanded compounds
            if ((node.type === 'compound' || node.type === 'parallel') && !node.collapsed) {
                elkNode.children = [];

                console.log(`${indent}  ${node.id} has ${node.children.length} children: ${node.children.join(', ')}`);

                elkNode.layoutOptions = {
                    'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
                    'elk.padding': `[top=${ELK_LAYOUT_CONFIG.COMPOUND_PADDING_TOP},left=${ELK_LAYOUT_CONFIG.COMPOUND_PADDING_SIDE},bottom=${ELK_LAYOUT_CONFIG.COMPOUND_PADDING_SIDE},right=${ELK_LAYOUT_CONFIG.COMPOUND_PADDING_SIDE}]`
                };

                // Parallel states: arrange children horizontally (left-to-right)
                // W3C SCXML semantics: parallel children execute concurrently, visualized side-by-side
                if (node.type === 'parallel') {
                    elkNode.layoutOptions['elk.direction'] = 'RIGHT';
                    elkNode.layoutOptions['elk.spacing.nodeNode'] = ELK_LAYOUT_CONFIG.PARALLEL_CHILD_SPACING;
                }

                // Recursively build children
                node.children.forEach(childId => {
                    const childNode = this.nodes.find(n => n.id === childId);
                    if (childNode) {
                        if (visibleNodeIds.has(childId)) {
                            console.log(`${indent}    → Adding child ${childId} (visible)`);
                            elkNode.children.push(buildELKNode(childNode, depth + 1));
                        } else {
                            console.log(`${indent}    → Skipping child ${childId} (not visible)`);
                        }
                    } else {
                        console.warn(`${indent}    → Child ${childId} not found in this.nodes!`);
                    }
                });

                console.log(`${indent}  ${node.id} elkNode.children.length = ${elkNode.children.length}`);
            }

            return elkNode;
        };

        // Only add top-level visible nodes (nodes without visible parents)
        const topLevelNodes = visibleNodes.filter(node => {
            // Check if this node has a visible parent
            const hasVisibleParent = this.nodes.some(parent =>
                (parent.type === 'compound' || parent.type === 'parallel') &&
                !parent.collapsed &&
                parent.children &&
                parent.children.includes(node.id) &&
                visibleNodeIds.has(parent.id)
            );
            return !hasVisibleParent;
        });

        console.log(`[buildELKGraph] Total visible nodes: ${visibleNodes.length}, Top-level nodes: ${topLevelNodes.length}`);
        console.log(`  Top-level: ${topLevelNodes.map(n => n.id).join(', ')}`);

        topLevelNodes.forEach(node => {
            graph.children.push(buildELKNode(node));
        });

        // Add edges (only if both source and target are visible)
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);

        visibleLinks.forEach(link => {
            if (link.linkType === 'transition' || link.linkType === 'initial') {
                // Only add edge if both endpoints exist in visible nodes
                if (visibleNodeIds.has(link.source) && visibleNodeIds.has(link.target)) {
                    graph.edges.push({
                        id: link.id,
                        sources: [link.source],
                        targets: [link.target]
                    });
                }
            }
        });

        return graph;
    }

    /**
     * Get node width
     */
    getNodeWidth(node) {
        if (node.type === 'initial-pseudo') return 20;
        if (node.type === 'compound' || node.type === 'parallel') {
            return node.collapsed ? 120 : 300;
        }

        // Calculate width based on text content (generous estimates for ELK layout)
        let maxWidth = LAYOUT_CONSTANTS.STATE_MIN_WIDTH;

        // State ID length
        const idWidth = (node.id || '').length * 10 + 60;
        if (idWidth > maxWidth) maxWidth = idWidth;

        // Check onentry/onexit actions for text length (use canvas measurement for accuracy)
        const actions = [...(node.onentry || []), ...(node.onexit || [])];
        if (actions.length > 0) {
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            ctx.font = '13px sans-serif';

            actions.forEach(action => {
                const text = this.formatActionText(action);
                if (text) {
                    // "↓ entry / " = ~10 chars + action text
                    const fullText = `↓ entry / ${text}`;
                    const metrics = ctx.measureText(fullText);
                    const estimatedWidth = metrics.width + 80; // Add padding for margins, box, and scrollbar
                    if (estimatedWidth > maxWidth) {
                        maxWidth = estimatedWidth;
                    }
                }
            });
        }


        return Math.min(maxWidth, LAYOUT_CONSTANTS.STATE_MAX_WIDTH);
    }

    /**
     * Get node height
     */
    getNodeHeight(node) {
        if (node.type === 'initial-pseudo') return 20;
        if (node.type === 'compound' || node.type === 'parallel') {
            return node.collapsed ? 50 : 200;
        }

        // Calculate height based on number of actions
        const entryActions = (node.onentry || []).length;
        const exitActions = (node.onexit || []).length;
        const totalActions = entryActions + exitActions;

        if (totalActions === 0) return LAYOUT_CONSTANTS.STATE_MIN_HEIGHT;

        return LAYOUT_CONSTANTS.STATE_BASE_HEIGHT + (totalActions * LAYOUT_CONSTANTS.ACTION_HEIGHT);
    }

    /**
     * Get visible nodes
     */
    getVisibleNodes() {
        console.log('[getVisibleNodes] Checking visibility for all nodes...');
        const visibleIds = new Set();

        this.nodes.forEach(node => {
            // Recursive check: node is hidden if ANY ancestor is collapsed
            const collapsedAncestor = this.findCollapsedAncestor(node.id, this.nodes);
            
            if (collapsedAncestor) {
                console.log(`  ${node.id}: HIDDEN (ancestor ${collapsedAncestor.id} collapsed)`);
            } else {
                visibleIds.add(node.id);
                console.log(`  ${node.id}: VISIBLE (type=${node.type})`);
            }
        });

        const result = this.nodes.filter(n => visibleIds.has(n.id));
        console.log(`[getVisibleNodes] Result: ${result.map(n => n.id).join(', ')}`);
        
        // Debug: Show compound/parallel nodes and their children
        this.nodes.filter(n => n.type === 'compound' || n.type === 'parallel').forEach(n => {
            console.log(`  ${n.id} (${n.type}, collapsed=${n.collapsed}): children=${n.children ? n.children.join(', ') : 'none'}`);
        });
        
        return result;
    }

    /**
     * Get visible links
     */
    getVisibleLinks(allLinks, nodes) {
        return allLinks.filter(link => {
            const sourceNode = nodes.find(n => n.id === link.source);
            const targetNode = nodes.find(n => n.id === link.target);

            if (!sourceNode || !targetNode) return false;

            // Hide containment/delegation
            if (link.linkType === 'containment' || link.linkType === 'delegation') {
                return false;
            }

            // Check hidden states
            const sourceHidden = this.findCollapsedAncestor(link.source, nodes);
            const targetHidden = this.findCollapsedAncestor(link.target, nodes);

            // Both hidden in same compound
            if (sourceHidden && targetHidden && sourceHidden.id === targetHidden.id) {
                return false;
            }

            // Source hidden, target is collapsed parent
            if (sourceHidden && link.target === sourceHidden.id) {
                return false;
            }

            return true;
        });
    }

    /**
     * Find collapsed ancestor
     */
    findCollapsedAncestor(nodeId, nodes) {
        // First check direct parent
        for (const parent of nodes) {
            if ((parent.type === 'compound' || parent.type === 'parallel') &&
                parent.collapsed &&
                parent.children &&
                parent.children.includes(nodeId)) {
                return parent;
            }
        }
        
        // Recursive check: check if parent has a collapsed ancestor
        for (const parent of nodes) {
            if ((parent.type === 'compound' || parent.type === 'parallel') &&
                parent.children &&
                parent.children.includes(nodeId)) {
                const grandparent = this.findCollapsedAncestor(parent.id, nodes);
                if (grandparent) {
                    return grandparent; // Return the collapsed ancestor, not the parent
                }
            }
        }
        
        return null;
    }

    /**
     * Apply ELK layout
     */
    applyELKLayout(layouted) {
        console.log('Applying ELK layout to nodes...');
        console.log(`  this.nodes count: ${this.nodes.length}, nodes: ${this.nodes.map(n => `${n.id}(${n.type})`).join(', ')}`);

        // Debug: Log ELK result structure
        console.log('ELK result structure:');
        layouted.children.forEach(child => {
            console.log(`  ${child.id}: hasChildren=${!!child.children}, childCount=${child.children ? child.children.length : 0}`);
            if (child.children) {
                child.children.forEach(grandchild => {
                    console.log(`    └─ ${grandchild.id}: hasChildren=${!!grandchild.children}, childCount=${grandchild.children ? grandchild.children.length : 0}`);
                    if (grandchild.children) {
                        grandchild.children.forEach(ggrandchild => {
                            console.log(`       └─ ${ggrandchild.id}`);
                        });
                    }
                });
            }
        });

        const applyToNode = (elkNode, offsetX = 0, offsetY = 0, depth = 0) => {
            const indent = '  '.repeat(depth);
            const node = this.nodes.find(n => n.id === elkNode.id);
            if (node) {
                node.x = elkNode.x + offsetX + elkNode.width / 2;
                node.y = elkNode.y + offsetY + elkNode.height / 2;
                node.width = elkNode.width;
                node.height = elkNode.height;

                console.log(`${indent}  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height}, offset=(${offsetX}, ${offsetY})`);
            } else {
                console.warn(`${indent}  ELK node not found in this.nodes: ${elkNode.id} (possibly child state or collapsed)`);
            }

            if (elkNode.children) {
                console.log(`${indent}  ${elkNode.id} has ${elkNode.children.length} children in ELK result`);
                elkNode.children.forEach(child => {
                    applyToNode(child, elkNode.x + offsetX, elkNode.y + offsetY, depth + 1);
                });
            }
        };

        const collectELKNodeIds = (elkNode, ids = []) => {
            ids.push(elkNode.id);
            if (elkNode.children) {
                elkNode.children.forEach(child => collectELKNodeIds(child, ids));
            }
            return ids;
        };
        const elkNodeIds = [];
        layouted.children.forEach(child => collectELKNodeIds(child, elkNodeIds));
        console.log(`  ELK layout nodes: ${elkNodeIds.join(', ')}`);

        layouted.children.forEach(child => {
            applyToNode(child);
        });

        // Calculate bounding boxes for expanded compounds without coordinates
        // (ELK may not provide coordinates for nested hierarchy)
        console.log('Calculating bounding boxes for expanded compounds...');
        console.log(`  Total nodes: ${this.nodes.length}`);
        this.nodes.forEach(node => {
            if ((node.type === 'compound' || node.type === 'parallel') && 
                !node.collapsed && 
                (node.x === undefined || node.y === undefined)) {
                
                console.log(`  Processing ${node.id} (type=${node.type}, collapsed=${node.collapsed}, hasCoords=${node.x !== undefined && node.y !== undefined})`);
                
                // Get children coordinates
                if (!node.children || node.children.length === 0) {
                    console.warn(`  ${node.id}: No children array, skipping bounding box calculation`);
                    return;
                }
                
                const childNodes = node.children
                    .map(childId => this.nodes.find(n => n.id === childId))
                    .filter(child => child && child.x !== undefined && child.y !== undefined);
                
                console.log(`    Children: ${node.children.join(', ')}, with coords: ${childNodes.map(c => c.id).join(', ')}`);
                
                if (childNodes.length > 0) {
                    // Calculate bounding box with padding
                    const padding = PATH_CONSTANTS.COMPOUND_BOUNDS_PADDING;
                    const minX = Math.min(...childNodes.map(c => c.x - c.width/2)) - padding;
                    const maxX = Math.max(...childNodes.map(c => c.x + c.width/2)) + padding;
                    const minY = Math.min(...childNodes.map(c => c.y - c.height/2)) - padding;
                    const maxY = Math.max(...childNodes.map(c => c.y + c.height/2)) + padding;
                    
                    node.x = (minX + maxX) / 2;
                    node.y = (minY + maxY) / 2;
                    node.width = maxX - minX;
                    node.height = maxY - minY;
                    
                    console.log(`  ${node.id}: Calculated from children (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width.toFixed(1)}x${node.height.toFixed(1)}`);
                } else {
                    console.warn(`  ${node.id}: No children with coordinates, cannot calculate bounding box`);
                }
            }
        });

        // Apply ELK edge routing information BEFORE modifying node positions
        if (layouted.edges) {
            console.log('Applying ELK edge routing...');
            layouted.edges.forEach(elkEdge => {
                const link = this.allLinks.find(l => l.id === elkEdge.id);
                if (link && elkEdge.sections && elkEdge.sections.length > 0) {
                    link.elkSections = elkEdge.sections;
                    console.log(`  ${elkEdge.id}: ${elkEdge.sections.length} section(s)`);
                    elkEdge.sections.forEach((section, idx) => {
                        console.log(`    section ${idx}: start=(${section.startPoint.x.toFixed(1)}, ${section.startPoint.y.toFixed(1)}), end=(${section.endPoint.x.toFixed(1)}, ${section.endPoint.y.toFixed(1)})`);
                        if (section.bendPoints && section.bendPoints.length > 0) {
                            section.bendPoints.forEach((bp, bpIdx) => {
                                console.log(`      bendPoint ${bpIdx}: (${bp.x.toFixed(1)}, ${bp.y.toFixed(1)})`);
                            });
                        }
                    });
                }
            });
        }

        // Trust ELK (Eclipse Layout Kernel) hierarchical layout algorithm
        // ELK handles node positioning, overlap prevention, and hierarchical nesting
        // Manual alignment removed as it conflicts with ELK's optimized calculations
        // Reference: https://www.eclipse.org/elk/
        console.log('[LAYOUT] Using ELK calculated positions (no manual alignment)');

        // Invalidate ELK edge routing to use optimizer-calculated snap points
        // ELK routing is only used during initial layout, afterward we use optimizer routing
        this.allLinks.forEach(link => {
            if (link.elkSections) {
                delete link.elkSections;
            }
        });
        console.log('[LAYOUT] Invalidated ELK edge routing (will use optimizer routing)');

        // Optimize snap point assignments to minimize intersections
        console.log('Optimizing snap point assignments...');
        this.layoutOptimizer.optimizeSnapPointAssignments(this.allLinks, this.nodes);

        // Update all compound/parallel bounds to ensure they contain all children
        // Process in bottom-up order: children first, then parents
        console.log('Updating compound container bounds...');
        
        // Find all expanded compounds/parallels and their depths
        const compoundsWithDepth = [];
        const getDepth = (nodeId, visited = new Set()) => {
            if (visited.has(nodeId)) return 0; // Cycle detection
            visited.add(nodeId);
            
            const node = this.nodes.find(n => n.id === nodeId);
            if (!node || !node.children || node.children.length === 0) return 0;
            
            let maxChildDepth = 0;
            for (const childId of node.children) {
                const childDepth = getDepth(childId, visited);
                maxChildDepth = Math.max(maxChildDepth, childDepth);
            }
            return maxChildDepth + 1;
        };
        
        this.nodes.forEach(node => {
            if ((node.type === 'compound' || node.type === 'parallel') && !node.collapsed) {
                const depth = getDepth(node.id);
                compoundsWithDepth.push({ node, depth });
            }
        });
        
        // Sort by depth ascending (deepest children first, shallowest parents last)
        compoundsWithDepth.sort((a, b) => a.depth - b.depth);
        
        console.log(`  Processing ${compoundsWithDepth.length} compounds in bottom-up order:`);
        compoundsWithDepth.forEach(({ node, depth }) => {
            console.log(`    depth=${depth}: ${node.id}`);
        });
        
        // Update bounds in bottom-up order
        compoundsWithDepth.forEach(({ node }) => {
            this.updateCompoundBounds(node);
        });

        // Re-optimize snap points after compound bounds update
        // Compound bounds changes affect node positions and sizes
        if (compoundsWithDepth.length > 0) {
            console.log('Re-optimizing snap points after compound bounds update...');
            this.layoutOptimizer.optimizeSnapPointAssignments(this.allLinks, this.nodes);
        }

        console.log('Layout application complete');
    }

    /**
     * Update compound/parallel container bounds to contain all children
     * Ensures parent containers are always large enough for their children
     */
    updateCompoundBounds(compoundNode) {
        if (!compoundNode.children || compoundNode.children.length === 0) {
            return;
        }

        const childNodes = compoundNode.children
            .map(childId => this.nodes.find(n => n.id === childId))
            .filter(child => child && child.x !== undefined && child.y !== undefined);

        if (childNodes.length === 0) {
            console.warn(`[updateCompoundBounds] ${compoundNode.id}: No children with coordinates`);
            return;
        }

        // Debug: Log child positions
        console.log(`[updateCompoundBounds] ${compoundNode.id}: Processing ${childNodes.length} children:`);
        childNodes.forEach(child => {
            console.log(`  - ${child.id}: x=${child.x.toFixed(1)}, y=${child.y.toFixed(1)}, width=${child.width}, height=${child.height}`);
        });

        const padding = SCXMLVisualizer.COMPOUND_PADDING;
        const topPadding = SCXMLVisualizer.COMPOUND_TOP_PADDING;

        const minX = Math.min(...childNodes.map(c => c.x - (c.width || 0)/2)) - padding;
        const maxX = Math.max(...childNodes.map(c => c.x + (c.width || 0)/2)) + padding;
        const minY = Math.min(...childNodes.map(c => c.y - (c.height || 0)/2)) - topPadding;
        const maxY = Math.max(...childNodes.map(c => c.y + (c.height || 0)/2)) + padding;

        compoundNode.x = (minX + maxX) / 2;
        compoundNode.y = (minY + maxY) / 2;
        compoundNode.width = maxX - minX;
        compoundNode.height = maxY - minY;

        console.log(`[updateCompoundBounds] ${compoundNode.id}: Updated to (${compoundNode.x.toFixed(1)}, ${compoundNode.y.toFixed(1)}) size=${compoundNode.width.toFixed(1)}x${compoundNode.height.toFixed(1)}`);
    }

    /**
     * Find the immediate compound/parallel parent of a node
     * Returns null if node has no compound/parallel parent
     */
    findCompoundParent(nodeId) {
        for (const node of this.nodes) {
            if ((node.type === 'compound' || node.type === 'parallel') && 
                !node.collapsed && 
                node.children && 
                node.children.includes(nodeId)) {
                return node;
            }
        }
        return null;
    }

    /**
     * Find the topmost compound/parallel ancestor of a node
     * Returns null if node has no compound/parallel parent
     * Returns the highest ancestor in the hierarchy
     */
    findTopmostCompoundParent(nodeId) {
        let currentId = nodeId;
        let topmostParent = null;
        
        while (true) {
            const parent = this.findCompoundParent(currentId);
            if (!parent) {
                break;
            }
            topmostParent = parent;
            currentId = parent.id;
        }
        
        return topmostParent;
    }

    /**
     * Recursively collect all descendant IDs of a node
     * Returns array of all children, grandchildren, great-grandchildren, etc.
     */
    getAllDescendantIds(parentId) {
        const descendants = [];
        const parent = this.nodes.find(n => n.id === parentId);
        
        if (parent && parent.children) {
            parent.children.forEach(childId => {
                descendants.push(childId);
                // Recursively add grandchildren
                const grandchildren = this.getAllDescendantIds(childId);
                descendants.push(...grandchildren);
            });
        }
        
        return descendants;
    }

    /**
     * Render SVG
     */
    render() {
        console.log('[RENDER START] ========== Beginning render() ==========');
        // Store reference to visualizer instance for use in drag handlers
        const self = this;

        // Time-based throttling for link updates during drag
        let lastLinkUpdateTime = 0;
        const LINK_UPDATE_INTERVAL = 50; // 50ms = 20fps max for link updates

        // Clear
        this.zoomContainer.selectAll('*').remove();

        const visibleNodes = this.getVisibleNodes();
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);
        
        console.log(`[RENDER] visibleNodes: ${visibleNodes.map(n => n.id).join(', ')}`);

        // Compound containers (expanded)
        // Include compounds that have visible children, even if the compound itself is not in visibleNodes
        const compoundData = [];
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
        
        this.nodes.forEach(node => {
            if ((node.type === 'compound' || node.type === 'parallel') && !node.collapsed) {
                // Check if this compound has any visible children
                const hasVisibleChildren = node.children && node.children.some(childId => visibleNodeIds.has(childId));
                
                // Include if compound is visible OR has visible children
                if (visibleNodeIds.has(node.id) || hasVisibleChildren) {
                    // Ensure compound has coordinates (from bounding box calculation)
                    if (node.x !== undefined && node.y !== undefined && node.width !== undefined && node.height !== undefined) {
                        compoundData.push(node);
                        console.log(`  Including compound ${node.id}: inVisibleNodes=${visibleNodeIds.has(node.id)}, hasVisibleChildren=${hasVisibleChildren}`);
                    } else {
                        console.warn(`  Compound ${node.id} has visible children but no coordinates!`);
                    }
                }
            }
        });
        
        console.log(`[RENDER] compoundData: ${compoundData.map(n => `${n.id}(${n.type})`).join(', ')}`);

        this.compoundContainers = this.zoomContainer.append('g')
            .attr('class', 'compound-containers')
            .selectAll('rect')
            .data(compoundData)
            .enter().append('rect')
            .attr('class', 'compound-container')
            .attr('x', d => d.x - d.width/2)
            .attr('y', d => d.y - d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            .attr('data-state-id', d => d.id)
            .style('cursor', 'pointer')
            .call(d3.drag()
                .on('start', function(event, d) {
                    console.log(`[DRAG START COMPOUND] ${d.id} at (${d.x}, ${d.y})`);

                    // Store initial position for click vs drag detection
                    d.dragStartX = d.x;
                    d.dragStartY = d.y;

                    // Cancel background optimization
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                    }
                    
                    // Find the topmost parent to move the entire hierarchy
                    const topmostParent = self.findTopmostCompoundParent(d.id);
                    if (topmostParent) {
                        console.log(`[DRAG START COMPOUND] ${d.id} has topmost parent ${topmostParent.id}, will move entire hierarchy`);
                        d.dragParent = topmostParent;
                        
                        // Cache descendant IDs for topmost parent
                        d._cachedDescendants = self.getAllDescendantIds(topmostParent.id);
                    } else {
                        // No parent: this is the topmost
                        d.dragParent = null;
                        
                        // Cache descendant IDs for self
                        d._cachedDescendants = self.getAllDescendantIds(d.id);
                    }
                    
                    d3.select(this).raise();
                    d.isDragging = true;
                    self.isDraggingAny = true;
                })
                
                .on('drag', function(event, d) {
                    // Store delta before updating
                    const dx = event.dx;
                    const dy = event.dy;

                    // If this compound has a parent, move the parent instead (entire hierarchy)
                    if (d.dragParent) {
                        console.log(`[DRAG COMPOUND] ${d.id} has parent ${d.dragParent.id}, moving entire hierarchy`);
                        
                        // Update parent position (data only)
                        d.dragParent.x += dx;
                        d.dragParent.y += dy;
                        
                        // Update all descendants (including this compound) with single pass
                        if (d.dragParent.children) {
                            // Convert to Set for O(1) lookup performance
                            const allDescendantIds = new Set(d._cachedDescendants);
                            const parentChildrenSet = new Set(d.dragParent.children);
                            
                            // Single pass: update data positions for ALL descendants
                            self.nodes.forEach(node => {
                                // Update direct children of parent
                                if (parentChildrenSet.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                }
                            });
                    }
                    } else {
                        // No parent: move only this compound and its children
                        // console.log(`[DRAG COMPOUND] ${d.id} at (${d.x.toFixed(1)}, ${d.y.toFixed(1)}) dx=${dx.toFixed(1)} dy=${dy.toFixed(1)}`);
                        
                        // Update compound position (data only)
                        d.x += dx;
                        d.y += dy;

                        // Update children positions and visuals with single pass
                        if (d.children) {
                            // Convert to Set for O(1) lookup performance
                            const allDescendantIds = new Set(d._cachedDescendants);
                            const childrenSet = new Set(d.children);
                            
                            // Debug logs removed for performance
                            
                            // Single pass: update data positions for ALL descendants
                            self.nodes.forEach(node => {
                                // Update direct children
                                if (childrenSet.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    // Debug log removed
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    // Debug log removed
                                }
                            });
                        }
                    }

                    // Time-based throttling: max 20fps for link path updates (50ms interval)
                    const now = performance.now();
                    if (now - lastLinkUpdateTime >= LINK_UPDATE_INTERVAL) {
                        lastLinkUpdateTime = now;
                        self.updateLinksFast();
                    }
                })
                
                .on('end', function(event, d) {
                    console.log(`[DRAG END COMPOUND] ${d.id}`);
                    
                    // dragAnimationFrame removed (using time-based throttling instead)
                    
                    d.isDragging = false;
                    self.isDraggingAny = false;
                    
                    // Cleanup cached descendants
                    delete d._cachedDescendants;
                    
                    // Update bounds based on whether we moved parent or self
                    if (d.dragParent) {
                        console.log(`[DRAG END COMPOUND] Updating parent ${d.dragParent.id} bounds to contain all children`);
                        self.updateCompoundBounds(d.dragParent);
                        
                        // Update parent container visual
                        if (self.compoundContainers) {
                            self.compoundContainers.each(function(compoundData) {
                                if (compoundData.id === d.dragParent.id) {
                                    d3.select(this)
                                        .attr('x', compoundData.x - compoundData.width/2)
                                        .attr('y', compoundData.y - compoundData.height/2)
                                        .attr('width', compoundData.width)
                                        .attr('height', compoundData.height);
                                }
                            });
                        }
                        
                        // Clear dragParent reference
                        d.dragParent = null;
                    } else {
                        console.log(`[DRAG END COMPOUND] Updating ${d.id} bounds to contain all children`);
                        self.updateCompoundBounds(d);

                        // Final position with updated bounds
                        d3.select(this)
                            .attr('x', d.x - d.width/2)
                            .attr('y', d.y - d.height/2)
                            .attr('width', d.width)
                            .attr('height', d.height);
                    }
                    
                    // Immediate greedy optimization for instant UI feedback
                    self.updateLinksFast();

                    // Background CSP refinement for optimal solution quality
                    // Calculate drag distance to distinguish click vs drag
                    const dragDistance = Math.hypot(d.x - d.dragStartX, d.y - d.dragStartY);
                    const DRAG_THRESHOLD = 5; // pixels
                    const isDrag = dragDistance > DRAG_THRESHOLD;

                    if (isDrag) {
                        console.log(`[DRAG END COMPOUND] Node moved ${dragDistance.toFixed(0)}px, starting progressive optimization...`);
                    } else {
                        console.log(`[DRAG END COMPOUND] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), skipping optimization`);
                    }

                    // Only optimize if node was actually dragged
                    if (!isDrag) {
                        return; // Skip optimization for clicks
                    }

                    console.log('[DRAG END COMPOUND] Starting progressive optimization...');

                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                    }

                    self.backgroundOptimization = self.layoutOptimizer.optimizeSnapPointAssignmentsProgressive(
                        self.allLinks,
                        self.nodes,
                        d.id,  // Dragged compound node ID
                        (success) => {
                            if (success) {
                                console.log(`[DRAG END COMPOUND] Background CSP complete, updating visualization...`);

                                self.allLinks.forEach(link => {
                                    const sourceNode = self.nodes.find(n => n.id === link.source);
                                    const targetNode = self.nodes.find(n => n.id === link.target);
                                    if (sourceNode && targetNode) {
                                        self.calculateLinkDirections(sourceNode, targetNode, link);
                                    }
                                });

                                self.updateLinksOptimal();
                                console.log(`[DRAG END COMPOUND] CSP visualization update complete`);
                            } else {
                                console.log(`[DRAG END COMPOUND] Background CSP cancelled or failed, keeping greedy result`);
                            }

                            self.backgroundOptimization = null;
                        },
                        (iteration, totalIterations, score) => {
                            // Progressive update: called for each intermediate solution
                            console.log(`[DRAG END COMPOUND] Intermediate update (${iteration}/${totalIterations}): score=${score.toFixed(1)}`);

                            // Recalculate link directions
                            self.allLinks.forEach(link => {
                                const sourceNode = self.nodes.find(n => n.id === link.source);
                                const targetNode = self.nodes.find(n => n.id === link.target);
                                if (sourceNode && targetNode) {
                                    self.calculateLinkDirections(sourceNode, targetNode, link);
                                }
                            });

                            // Update visualization with intermediate solution
                            self.updateLinksOptimal();
                        },
                        500
                    );
                }))
            .on('click', (event, d) => {
                // Only toggle if not dragging
                if (!d.isDragging && event.defaultPrevented === false) {
                    event.stopPropagation();
                    this.toggleCompoundState(d.id);

                    // Design System: Panel + Diagram interaction (matches panel click behavior)
                    if (window.executionController) {
                        window.executionController.highlightStateInPanel(d.id);
                        window.executionController.focusState(d.id);
                    }
                }
            });
        
        console.log(`[RENDER] Rendered ${compoundData.length} compound containers`);
        compoundData.forEach(d => {
            console.log(`  ${d.id}: x=${d.x}, y=${d.y}, width=${d.width}, height=${d.height}`);
        });

        // Regular nodes
        const regularNodes = visibleNodes.filter(n =>
            n.type !== 'compound' && n.type !== 'parallel'
        );

        this.nodeElements = this.zoomContainer.append('g')
            .attr('class', 'nodes')
            .selectAll('g')
            .data(regularNodes)
            .enter().append('g')
            .attr('class', d => {
                if (d.type === 'atomic') return 'node state state-atomic';
                if (d.type === 'final') return 'node state state-final';
                if (d.type === 'history') return 'node state state-history';
                if (d.type === 'initial-pseudo') return 'node state-initial-pseudo';
                return 'node state';
            })
            .attr('data-state-id', d => d.id)
            .attr('transform', d => `translate(${d.x},${d.y})`)
            .call(d3.drag()
                .on('start', function(event, d) {
                    console.log(`[DRAG START] ${d.id} at (${d.x}, ${d.y})`);

                    // Store initial position for click vs drag detection
                    d.dragStartX = d.x;
                    d.dragStartY = d.y;

                    // Cancel any ongoing background optimization
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                        console.log(`[DRAG START] Cancelled background CSP optimization`);
                    }

                    // Check if this node has compound/parallel ancestors
                    // Find the topmost ancestor to move the entire hierarchy
                    const topmostParent = self.findTopmostCompoundParent(d.id);
                    if (topmostParent) {
                        console.log(`[DRAG START] ${d.id} has topmost parent ${topmostParent.id}, will move entire hierarchy`);
                        d.dragParent = topmostParent;
                        
                        // Cache descendant IDs for performance (avoid recalculating on every drag event)
                        d._cachedDescendants = self.getAllDescendantIds(topmostParent.id);
                    } else {
                        d.dragParent = null;
                    }

                    // Raise dragged element to front
                    d3.select(this).raise();
                    d.isDragging = true;
                    self.isDraggingAny = true;
                })
                .on('drag', function(event, d) {
                    // Store delta before any updates
                    const dx = event.dx;
                    const dy = event.dy;

                    // If node has a compound/parallel parent, move the parent instead
                    if (d.dragParent) {
                        console.log(`[DRAG] Moving parent ${d.dragParent.id} instead of child ${d.id}`);
                        
                        // Update parent position
                        d.dragParent.x += dx;
                        d.dragParent.y += dy;

                        // Update all children positions (including this node) with single pass
                        if (d.dragParent.children) {
                            // Convert to Set for O(1) lookup performance
                            const allDescendantIds = new Set(d._cachedDescendants);
                            const parentChildrenSet = new Set(d.dragParent.children);
                            
                            // Single pass: update data positions for ALL descendants
                            self.nodes.forEach(node => {
                                // Update direct children
                                if (parentChildrenSet.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                }
                            });
                        }
                    } else {
                        // No parent: move only this node (data only)
                        d.x += dx;
                        d.y += dy;
                    }

                    // Throttle link updates with RAF (for both cases)
                    if (self.dragOptimizationTimer) {
                        clearTimeout(self.dragOptimizationTimer);
                        self.dragOptimizationTimer = null;
                    }

                    // Time-based throttling: max 20fps for link path updates (50ms interval)
                    const now = performance.now();
                    if (now - lastLinkUpdateTime >= LINK_UPDATE_INTERVAL) {
                        lastLinkUpdateTime = now;
                        self.updateLinksFast();
                    }
                })
                .on('end', function(event, d) {
                    console.log(`[DRAG END] ${d.id} at (${d.x}, ${d.y})`);
                    d.isDragging = false;
                    self.isDraggingAny = false;

                    // Cancel any pending animation frame
                    // dragAnimationFrame removed (using time-based throttling instead)

                    // Clear debounce timer
                    if (self.dragOptimizationTimer) {
                        clearTimeout(self.dragOptimizationTimer);
                        self.dragOptimizationTimer = null;
                    }

                    // Cancel any ongoing background optimization
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                    }

                    // Update parent bounds if this node has a parent
                    if (d.dragParent) {
                        console.log(`[DRAG END] Updating parent ${d.dragParent.id} bounds to contain children`);
                        self.updateCompoundBounds(d.dragParent);

                        // Update parent container visual
                        if (self.compoundContainers) {
                            self.compoundContainers.each(function(compoundData) {
                                if (compoundData.id === d.dragParent.id) {
                                    d3.select(this)
                                        .attr('x', compoundData.x - compoundData.width/2)
                                        .attr('y', compoundData.y - compoundData.height/2)
                                        .attr('width', compoundData.width)
                                        .attr('height', compoundData.height);
                                }
                            });
                        }

                        // Cleanup cached descendants
                        delete d._cachedDescendants;
                        
                        // Clear dragParent reference
                        d.dragParent = null;
                    }

                    // **PROGRESSIVE OPTIMIZATION: Immediate greedy + background CSP**
                    // Calculate drag distance to distinguish click vs drag
                    const dragDistance = Math.hypot(d.x - d.dragStartX, d.y - d.dragStartY);
                    const DRAG_THRESHOLD = 5; // pixels
                    const isDrag = dragDistance > DRAG_THRESHOLD;

                    if (isDrag) {
                        console.log(`[DRAG END] Node moved ${dragDistance.toFixed(0)}px, starting progressive optimization...`);
                    } else {
                        console.log(`[DRAG END] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), skipping optimization`);
                    }

                    // Only optimize if node was actually dragged
                    if (!isDrag) {
                        return; // Skip optimization for clicks
                    }

                    // Start progressive optimization (returns immediately with greedy result)
                    // Pass dragged node ID for locality-aware optimization
                    self.backgroundOptimization = self.layoutOptimizer.optimizeSnapPointAssignmentsProgressive(
                        self.allLinks,
                        self.nodes,
                        d.id,  // Dragged node ID for distance-based prioritization
                        (success) => {
                            if (success) {
                                console.log(`[DRAG END] Background CSP complete, updating visualization...`);

                                // Calculate midY for new CSP routing
                                self.allLinks.forEach(link => {
                                    const sourceNode = self.nodes.find(n => n.id === link.source);
                                    const targetNode = self.nodes.find(n => n.id === link.target);
                                    if (sourceNode && targetNode) {
                                        self.calculateLinkDirections(sourceNode, targetNode, link);
                                    }
                                });

                                // Update visualization with CSP-optimized paths
                                self.updateLinksOptimal();

                                console.log(`[DRAG END] CSP visualization update complete`);
                            } else {
                                console.log(`[DRAG END] Background CSP cancelled or failed, keeping greedy result`);
                            }

                            self.backgroundOptimization = null;
                        },
                        (iteration, totalIterations, score) => {
                            // Progressive update: called for each intermediate solution
                            console.log(`[DRAG END] Intermediate update (${iteration}/${totalIterations}): score=${score.toFixed(1)}`);

                            // Recalculate link directions
                            self.allLinks.forEach(link => {
                                const sourceNode = self.nodes.find(n => n.id === link.source);
                                const targetNode = self.nodes.find(n => n.id === link.target);
                                if (sourceNode && targetNode) {
                                    self.calculateLinkDirections(sourceNode, targetNode, link);
                                }
                            });

                            // Update visualization with intermediate solution
                            self.updateLinksOptimal();
                        },
                        500 // 500ms debounce
                    );

                    // Calculate midY for immediate greedy routing
                    self.allLinks.forEach(link => {
                        const sourceNode = self.nodes.find(n => n.id === link.source);
                        const targetNode = self.nodes.find(n => n.id === link.target);
                        if (sourceNode && targetNode) {
                            self.calculateLinkDirections(sourceNode, targetNode, link);
                        }
                    });

                    // Immediate update to render greedy paths (fast feedback)
                    self.updateLinksOptimal();

                    console.log(`[DRAG END] Immediate greedy rendering complete`);
                }));

        // Shapes
        this.nodeElements.filter(d => d.type === 'initial-pseudo')
            .append('circle')
            .attr('r', 10)
            .attr('class', 'initial-pseudo-circle');

        this.nodeElements.filter(d => d.type === 'atomic' || d.type === 'final')
            .append('rect')
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            // State rect positioning: centered at (0,0) in local coords, so x = -width/2
            .attr('x', d => -d.width/2)
            .attr('y', d => -d.height/2)
            .attr('rx', 5)
            .style('cursor', 'pointer')
            .on('click', (event, d) => {
                event.stopPropagation();

                // W3C SCXML 6.3: Invoke navigation - if state has invoke, navigate to child
                if (d.hasInvoke) {
                    console.log(`State ${d.id} has invoke - dispatching state-navigate event`);
                    
                    // Dispatch custom event for navigation
                    const navEvent = new CustomEvent('state-navigate', {
                        detail: {
                            stateId: d.id,
                            invokeSrc: d.invokeSrc,
                            invokeSrcExpr: d.invokeSrcExpr,
                            invokeId: d.invokeId
                        }
                    });
                    document.dispatchEvent(navEvent);
                    return;
                }

                // Debug: Log state coordinates
                const bounds = this.getNodeBounds(d);
                console.log('=== STATE CLICKED ===');
                console.log(`State: ${d.id} (type: ${d.type})`);
                console.log(`Center: (${d.x}, ${d.y})`);
                console.log(`Bounds:`, bounds);
                console.log(`  Left: ${bounds.left}, Right: ${bounds.right}`);
                console.log(`  Top: ${bounds.top}, Bottom: ${bounds.bottom}`);
                console.log(`  Width: ${bounds.right - bounds.left}, Height: ${bounds.bottom - bounds.top}`);
                console.log('====================');

                // Design System: Panel + Diagram interaction (matches panel click behavior)
                if (window.executionController) {
                    window.executionController.highlightStateInPanel(d.id);
                    window.executionController.focusState(d.id);
                }
            });

        this.nodeElements.filter(d => d.type === 'history')
            .append('circle')
            .attr('r', 20)
            .attr('class', 'history-circle');

        // Collapsed compounds
        const collapsedCompounds = visibleNodes.filter(n =>
            (n.type === 'compound' || n.type === 'parallel') && n.collapsed
        );

        this.collapsedElements = this.zoomContainer.append('g')
            .attr('class', 'collapsed-compounds')
            .selectAll('rect')
            .data(collapsedCompounds)
            .enter().append('rect')
            .attr('class', 'compound-collapsed')
            .attr('data-state-id', d => d.id)
            .attr('x', d => d.x - d.width/2)
            .attr('y', d => d.y - d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            .attr('rx', 5)
            .style('cursor', 'pointer')
            .call(d3.drag()
                .on('start', function(event, d) {
                    console.log(`[DRAG START COLLAPSED] ${d.id} at (${d.x}, ${d.y})`);

                    // Store initial position for click vs drag detection
                    d.dragStartX = d.x;
                    d.dragStartY = d.y;

                    // Cancel any ongoing background optimization
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                        console.log(`[DRAG START COLLAPSED] Cancelled background CSP optimization`);
                    }

                    // Raise dragged element to front
                    d3.select(this).raise();
                    d.isDragging = true;
                    self.isDraggingAny = true;
                })
                .on('drag', function(event, d) {
                    // Use delta movement (dx, dy) to avoid flickering
                    d.x += event.dx;
                    d.y += event.dy;

                    const element = this;

                    // Cancel previous animation frame if exists
                    if (dragAnimationFrame) {
                        cancelAnimationFrame(dragAnimationFrame);
                    }

                    // Update visual position synchronously for perfect synchronization
                    d3.select(element)
                        .attr('x', d.x - d.width/2)
                        .attr('y', d.y - d.height/2);

                    // Update label position if exists
                    self.zoomContainer.selectAll('.collapsed-label')
                        .filter(label => label.id === d.id)
                        .attr('x', d.x)
                        .attr('y', d.y);

                    // Time-based throttling: max 20fps for link path updates (50ms interval)
                    const now = performance.now();
                    if (now - lastLinkUpdateTime >= LINK_UPDATE_INTERVAL) {
                        lastLinkUpdateTime = now;
                        self.updateLinksFast();
                    }
                })
                .on('end', function(event, d) {
                    console.log(`[DRAG END COLLAPSED] ${d.id} at (${d.x}, ${d.y})`);

                    // Calculate drag distance to distinguish click vs drag
                    const dragDistance = Math.hypot(d.x - d.dragStartX, d.y - d.dragStartY);
                    const DRAG_THRESHOLD = 5; // pixels
                    const isDrag = dragDistance > DRAG_THRESHOLD;

                    if (!isDrag) {
                        console.log(`[DRAG END COLLAPSED] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), skipping optimization`);
                        d.isDragging = false;
                        self.isDraggingAny = false;
                        return; // Skip optimization for clicks
                    }

                    console.log(`[DRAG END COLLAPSED] Node moved ${dragDistance.toFixed(0)}px, starting progressive optimization...`);

                    // Cancel pending animation frame
                    // dragAnimationFrame removed (using time-based throttling instead)

                    d.isDragging = false;
                    self.isDraggingAny = false;

                    // Final position update
                    d3.select(this)
                        .attr('x', d.x - d.width/2)
                        .attr('y', d.y - d.height/2);

                    // Update label final position
                    self.zoomContainer.selectAll('.collapsed-label')
                        .filter(label => label.id === d.id)
                        .attr('x', d.x)
                        .attr('y', d.y);

                    // Immediate greedy optimization for instant UI feedback
                    console.log('[DRAG END COLLAPSED] Immediate greedy optimization...');
                    self.updateLinksFast();

                    // Background CSP refinement for optimal solution quality
                    console.log('[DRAG END COLLAPSED] Starting background CSP refinement...');
                    
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                    }
                    
                    self.backgroundOptimization = self.layoutOptimizer.optimizeSnapPointAssignmentsProgressive(
                        self.allLinks,
                        self.nodes,
                        d.id,  // Dragged collapsed node ID
                        (success) => {
                            if (success) {
                                console.log('[DRAG END COLLAPSED] Background CSP complete, updating visualization...');

                                // Calculate midY for new CSP routing
                                self.allLinks.forEach(link => {
                                    const sourceNode = self.nodes.find(n => n.id === link.source);
                                    const targetNode = self.nodes.find(n => n.id === link.target);
                                    if (sourceNode && targetNode) {
                                        self.calculateLinkDirections(sourceNode, targetNode, link);
                                    }
                                });

                                // Update visualization with CSP-optimized paths
                                self.updateLinksOptimal();

                                console.log('[DRAG END COLLAPSED] CSP visualization update complete');
                            } else {
                                console.log('[DRAG END COLLAPSED] Background CSP cancelled or failed, keeping greedy result');
                            }

                            self.backgroundOptimization = null;
                        },
                        (iteration, totalIterations, score) => {
                            // Progressive update: called for each intermediate solution
                            console.log(`[DRAG END COLLAPSED] Intermediate update (${iteration}/${totalIterations}): score=${score.toFixed(1)}`);

                            // Recalculate link directions
                            self.allLinks.forEach(link => {
                                const sourceNode = self.nodes.find(n => n.id === link.source);
                                const targetNode = self.nodes.find(n => n.id === link.target);
                                if (sourceNode && targetNode) {
                                    self.calculateLinkDirections(sourceNode, targetNode, link);
                                }
                            });

                            // Update visualization with intermediate solution
                            self.updateLinksOptimal();
                        },
                        500 // 500ms debounce
                    );
                }))
            .on('click', (event, d) => {
                // Only toggle if not dragging
                if (!d.isDragging && event.defaultPrevented === false) {
                    event.stopPropagation();
                    this.toggleCompoundState(d.id);

                    // Design System: Panel + Diagram interaction (matches panel click behavior)
                    if (window.executionController) {
                        window.executionController.highlightStateInPanel(d.id);
                        window.executionController.focusState(d.id);
                    }
                }
            });

        // W3C SCXML 6.3: Invoke Badge for states with child SCXML
        this.nodeElements.filter(d => d.hasInvoke && (d.type === 'atomic' || d.type === 'final'))
            .each(function(d) {
                const group = d3.select(this);
                
                // Badge position: top-right corner of state rect
                const badgeX = d.width/2 - 18;
                const badgeY = -d.height/2 + 18;
                
                // Badge circle (blue background)
                group.append('circle')
                    .attr('class', 'invoke-badge-circle')
                    .attr('cx', badgeX)
                    .attr('cy', badgeY)
                    .attr('r', 12)
                    .attr('fill', '#0969da')
                    .attr('stroke', 'white')
                    .attr('stroke-width', 2);
                
                // Badge icon (down arrow or custom symbol)
                group.append('text')
                    .attr('class', 'invoke-badge-icon')
                    .attr('x', badgeX)
                    .attr('y', badgeY)
                    .attr('text-anchor', 'middle')
                    .attr('dominant-baseline', 'middle')
                    .attr('font-size', '14px')
                    .attr('font-weight', 'bold')
                    .attr('fill', 'white')
                    .text('⤵');  // Unicode down-right arrow
                
                // Add has-invoke class to parent for CSS styling
                group.classed('has-invoke', true);
                
                console.log(`Added invoke badge to state: ${d.id}`);
            });

        // Labels - State ID with onentry/onexit actions (getBBox precision)
        this.nodeElements.filter(d => d.type !== 'initial-pseudo' && d.type !== 'history')
            .each(function(d) {
                const group = d3.select(this);
                let yOffset = -d.height/2 + 26; // Start from top
                // Calculate left margin using helper function
                // Text positioned at 10% from left edge (separator line at 5%)
                const leftMargin = self.getActionTextLeftMargin(d.width);

// State ID (bold, centered, larger font)
                group.append('text')
                    .attr('x', 0)
                    .attr('y', yOffset)
                    .attr('text-anchor', 'middle')
                    .attr('font-weight', 'bold')
                    .attr('font-size', '16px')
                    .attr('fill', '#1f2328')
                    .text(d.id);

                // Separator line if there are actions (stronger line)
                const hasActions = (d.onentry && d.onentry.length > 0) || (d.onexit && d.onexit.length > 0);
                if (hasActions) {
                    yOffset += 14;
                    group.append('line')
                        .attr('x1', -d.width/2 + (d.width * 0.05))
                        .attr('y1', yOffset)
                        .attr('x2', d.width/2 - (d.width * 0.05))
                        .attr('y2', yOffset)
                        .attr('stroke', '#d0d7de')
                        .attr('stroke-width', 2);
                    yOffset += 20; // Space below separator
                }

                // Entry actions with precise background boxes (layered approach)
                if (d.onentry && d.onentry.length > 0) {
                    yOffset = self.renderActionTexts({
                        prefix: '↓ entry',
                        color: '#2e7d32',
                        actions: d.onentry,
                        yOffset: yOffset,
                        stateData: d,
                        group: group,
                        leftMargin: leftMargin
                    });
                }

                // Exit actions with precise background boxes (layered approach)
                if (d.onexit && d.onexit.length > 0) {
                    yOffset = self.renderActionTexts({
                        prefix: '↑ exit',
                        color: '#c62828',
                        actions: d.onexit,
                        yOffset: yOffset,
                        stateData: d,
                        group: group,
                        leftMargin: leftMargin
                    });
                }
            });

        this.nodeElements.filter(d => d.type === 'history')
            .append('text')
            .attr('dy', 5)
            .attr('class', 'history-label')
            .text('H');

        this.zoomContainer.append('g')
            .attr('class', 'collapsed-labels')
            .selectAll('text')
            .data(collapsedCompounds)
            .enter().append('text')
            .attr('x', d => d.x)
            .attr('y', d => d.y)
            .attr('text-anchor', 'middle')
            .attr('dy', 5)
            .text(d => d.id);

        this.compoundLabels = this.zoomContainer.append('g')
            .attr('class', 'compound-labels')
            .selectAll('text')
            .data(compoundData)
            .enter().append('text')
            .attr('x', d => d.x - d.width/2 + 10)
            .attr('y', d => d.y - d.height/2 + 20)
            .attr('text-anchor', 'start')
            .text(d => d.id);

        // TWO-PASS ALGORITHM FOR ACCURATE SNAP POSITIONING
        // Pass 1: Calculate actual directions for all links (with collision avoidance)
        if (this.debugMode) console.log('[TWO-PASS] Pass 1: Calculating directions for all links...');
        visibleLinks.forEach(link => {
            const sourceNode = this.nodes.find(n => n.id === link.source);
            const targetNode = this.nodes.find(n => n.id === link.target);

            // **Calculate midY for z-path collision avoidance**
            // This ensures routing.midY is set for all links
            if (sourceNode && targetNode) {
                this.calculateLinkDirections(sourceNode, targetNode, link);
            }
        });
        if (this.debugMode) console.log('[TWO-PASS] Pass 1 complete');

        // Links (drawn AFTER nodes so they appear on top)
        // Pass 2: Render with snap positions based on confirmed directions
        if (this.debugMode) console.log('[TWO-PASS] Pass 2: Rendering links with snap positions...');
        
        const linkGroups = this.zoomContainer.append('g')
            .attr('class', 'links')
            .selectAll('g')
            .data(visibleLinks)
            .enter().append('g')
            .attr('class', 'link-group');
        
        // Path elements
        this.linkElements = linkGroups.append('path')
            .attr('class', d => {
                if (d.linkType === 'initial') return 'link-initial';
                return 'transition';
            })
            .attr('d', d => this.getLinkPath(d))
            .style('marker-end', 'url(#arrowhead)')
            .style('cursor', d => d.linkType === 'transition' ? 'pointer' : 'default')
            .on('click', (event, d) => {
                if (d.linkType === 'transition') {
                    // Show transition animation on diagram (temporary)
                    this.highlightTransition(d);
                    this.focusOnTransition(d);

                    // Dispatch event for execution-controller to update detail panel
                    document.dispatchEvent(new CustomEvent('transition-click', { detail: d }));
                }
            });
        
        // Transition labels (event, condition, actions)
        this.transitionLabels = linkGroups
            .filter(d => d.linkType === 'transition' && (d.event || d.cond)) // Show labels for transitions with events or guards
            .append('text')
            .attr('class', 'transition-label')
            .attr('x', d => this.getTransitionLabelPosition(d).x)
            .attr('y', d => this.getTransitionLabelPosition(d).y)
            .attr('text-anchor', 'middle')
            .attr('dominant-baseline', 'middle')
            .style('font-size', '11px')
            .style('font-family', 'monospace')
            .style('fill', '#0969da')
            .style('pointer-events', 'none')
            .text(d => this.getTransitionLabelText(d));

        // Snap point visualization (enabled with ?show-snap)
        if (this.showSnapPoints) {
            this.renderSnapPoints(visibleNodes);
        }

        console.log(`Rendered ${visibleNodes.length} nodes, ${visibleLinks.length} links`);

        // Debug: Check actual DOM elements created
        console.log('DOM elements check:');
        console.log('  Link paths:', this.linkElements.size());
        console.log('  Node groups:', this.nodeElements ? this.nodeElements.size() : 0);
        console.log('  Collapsed compounds:', this.collapsedElements ? this.collapsedElements.size() : 0);
        console.log('  Compound containers:', this.compoundContainers ? this.compoundContainers.size() : 0);

        // Render transition list
        this.renderTransitionList();

        console.log('[RENDER END] ========== Completed render() ==========');
    }

    /**
     * Render snap points visualization for debugging
     */
    /**
     * Generate snap points data array from visible nodes
     * Shared by renderSnapPoints() and updateSnapPointPositions()
     */
    generateSnapPointsData(visibleNodes) {
        const snapPointsData = [];

        // Iterate through all visible nodes (exclude initial-pseudo)
        visibleNodes.filter(n => n.type !== 'initial-pseudo').forEach(node => {
            const cx = node.x || 0;
            const cy = node.y || 0;
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            let nodeSnapIndex = 0;  // Index counter for this node

            const edges = ['top', 'bottom', 'left', 'right'];

            edges.forEach(edge => {
                // Collect snap points on this edge from optimized links or confirmed directions
                const edgeSnapPoints = [];

                this.allLinks.forEach(link => {
                    // **Use routing for snap visualization**
                    // Only transition and initial links should have routing (containment/delegation links are structural, not routed)
                    if (link.routing) {
                        if (link.source === node.id && link.routing.sourceEdge === edge) {
                            edgeSnapPoints.push({
                                point: link.routing.sourcePoint,
                                link: link
                            });
                        }

                        if (link.target === node.id && link.routing.targetEdge === edge) {
                            edgeSnapPoints.push({
                                point: link.routing.targetPoint,
                                link: link
                            });
                        }
                        return;
                    }

                    // **SKIP containment and delegation links** (they don't have routing)
                    // Visualizer layout: containment is hierarchical structure, not routing path
                    if (link.linkType === 'containment' || link.linkType === 'delegation') {
                        return;
                    }

                    // **FALLBACK: This should not happen in new architecture**
                    // Optimizer always runs before rendering, so routing should always exist for transition/initial links
                    console.warn(`[RENDER SNAP] ${link.source}→${link.target}: No routing found!`);
                });

                const hasInitial = this.layoutOptimizer.hasInitialTransitionOnEdge(node.id, edge);

                if (edgeSnapPoints.length === 0) {
                    // No connections on this edge: show center point
                    let x, y;
                    if (edge === 'top') {
                        x = cx;
                        y = cy - halfHeight;
                    } else if (edge === 'bottom') {
                        x = cx;
                        y = cy + halfHeight;
                    } else if (edge === 'left') {
                        x = cx - halfWidth;
                        y = cy;
                    } else if (edge === 'right') {
                        x = cx + halfWidth;
                        y = cy;
                    }

                    const snapData = {
                        x: x,
                        y: y,
                        index: nodeSnapIndex++,
                        nodeId: node.id,
                        edge: edge,
                        hasInitial: hasInitial,
                        hasConnection: false,
                        isInitialConnection: false
                    };
                    console.log(`[SNAP INDEX] ${node.id} #${snapData.index}: ${edge} center at (${x.toFixed(1)}, ${y.toFixed(1)})`);
                    snapPointsData.push(snapData);
                } else {
                    // Has connections: show actual snap points
                    // Sort by position
                    edgeSnapPoints.sort((a, b) => {
                        if (edge === 'top' || edge === 'bottom') {
                            return a.point.x - b.point.x;
                        } else {
                            return a.point.y - b.point.y;
                        }
                    });

                    edgeSnapPoints.forEach(sp => {
                        const snapData = {
                            x: sp.point.x,
                            y: sp.point.y,
                            index: nodeSnapIndex++,
                            nodeId: node.id,
                            edge: edge,
                            hasInitial: hasInitial,
                            hasConnection: true,
                            isInitialConnection: sp.link.linkType === 'initial'
                        };
                        const linkName = sp.link.source === node.id ?
                            `${sp.link.source}→${sp.link.target}` :
                            `${sp.link.source}→${sp.link.target}`;
                        console.log(`[SNAP INDEX] ${node.id} #${snapData.index}: ${edge} (${linkName}) at (${sp.point.x.toFixed(1)}, ${sp.point.y.toFixed(1)})`);
                        snapPointsData.push(snapData);
                    });
                }
            });
        });

        return snapPointsData;
    }

    /**
     * Render snap points (initial render)
     */
    renderSnapPoints(visibleNodes) {
        // Generate snap points data
        const snapPointsData = this.generateSnapPointsData(visibleNodes);

        // Remove old snap points before rendering new ones
        const oldSnapGroups = this.zoomContainer.selectAll('g.snap-points');
        const removedCount = oldSnapGroups.size();
        oldSnapGroups.remove();
        console.log(`[RENDER SNAP] Removed ${removedCount} old snap-points groups`);

        // Render snap point circles
        const snapGroup = this.zoomContainer.append('g').attr('class', 'snap-points');
        console.log(`[RENDER SNAP] Creating snap group, data points: ${snapPointsData.length}`);

        // Store references for later updates
        this.snapPointCircles = snapGroup.selectAll('circle.snap-point')
            .data(snapPointsData)
            .enter()
            .append('circle')
            .attr('class', 'snap-point')
            .attr('cx', d => {
                // Debug mode: log snap circle coordinates
                if (this.debugMode) {
                    console.log(`[SNAP CIRCLE] ${d.nodeId} #${d.index}: cx=${d.x.toFixed(1)}, cy=${d.y.toFixed(1)}`);
                }
                return d.x;
            })
            .attr('cy', d => d.y)
            .attr('r', 5)
            .style('fill', d => {
                if (d.isInitialConnection) return '#00ff00';  // Green for initial transitions
                if (d.hasConnection) return '#ffa500';  // Orange for connections
                return '#87ceeb';  // Sky blue for available positions
            })
            .style('stroke', '#000')
            .style('stroke-width', 1)
            .style('pointer-events', 'none');

        // Render index labels
        this.snapPointLabels = snapGroup.selectAll('text.snap-index')
            .data(snapPointsData)
            .enter()
            .append('text')
            .attr('class', 'snap-index')
            .attr('x', d => d.x)
            .attr('y', d => d.y - 10)
            .attr('text-anchor', 'middle')
            .attr('dominant-baseline', 'middle')
            .style('font-size', '11px')
            .style('font-family', 'monospace')
            .style('font-weight', 'bold')
            .style('fill', d => {
                if (d.isInitialConnection) return '#00aa00';
                if (d.hasConnection) return '#ff8800';
                return '#0066cc';
            })
            .style('pointer-events', 'none')
            .text(d => d.index + 1);

        console.log(`Rendered ${snapPointsData.length} snap points`);
    }

    /**
     * Fast update of snap point positions using D3 enter/update/exit pattern
     * Efficiently handles snap point count changes without full DOM re-creation
     */
    updateSnapPointPositions() {
        if (!this.showSnapPoints) {
            return;
        }

        // Generate latest snap points data
        const visibleNodes = this.getVisibleNodes();
        const snapPointsData = this.generateSnapPointsData(visibleNodes);

        // Get or create snap group
        let snapGroup = this.zoomContainer.select('g.snap-points');
        if (snapGroup.empty()) {
            snapGroup = this.zoomContainer.append('g').attr('class', 'snap-points');
        }

        // Key function for data binding (ensures correct element tracking)
        const keyFn = d => `${d.nodeId}-${d.edge}-${d.index}`;

        // === UPDATE CIRCLES ===
        const circles = snapGroup.selectAll('circle.snap-point')
            .data(snapPointsData, keyFn);

        // Update existing circles (only change position)
        circles
            .attr('cx', d => d.x)
            .attr('cy', d => d.y);

        // Enter new circles
        circles.enter()
            .append('circle')
            .attr('class', 'snap-point')
            .attr('cx', d => d.x)
            .attr('cy', d => d.y)
            .attr('r', 5)
            .style('fill', d => {
                if (d.isInitialConnection) return '#00ff00';  // Green for initial transitions
                if (d.hasConnection) return '#ffa500';  // Orange for connections
                return '#87ceeb';  // Sky blue for available positions
            })
            .style('stroke', '#000')
            .style('stroke-width', 1)
            .style('pointer-events', 'none');

        // Exit old circles
        circles.exit().remove();

        // === UPDATE LABELS ===
        const labels = snapGroup.selectAll('text.snap-index')
            .data(snapPointsData, keyFn);

        // Update existing labels (only change position)
        labels
            .attr('x', d => d.x)
            .attr('y', d => d.y - 10);

        // Enter new labels
        labels.enter()
            .append('text')
            .attr('class', 'snap-index')
            .attr('x', d => d.x)
            .attr('y', d => d.y - 10)
            .attr('text-anchor', 'middle')
            .attr('dominant-baseline', 'middle')
            .style('font-size', '11px')
            .style('font-family', 'monospace')
            .style('font-weight', 'bold')
            .style('fill', d => {
                if (d.isInitialConnection) return '#00aa00';
                if (d.hasConnection) return '#ff8800';
                return '#0066cc';
            })
            .style('pointer-events', 'none')
            .text(d => d.index + 1);

        // Exit old labels
        labels.exit().remove();

        // Update stored references
        this.snapPointCircles = snapGroup.selectAll('circle.snap-point');
        this.snapPointLabels = snapGroup.selectAll('text.snap-index');
    }

    /**
     * Get direction string from edge and isSource flag
     */
    getDirectionForEdge(edge, isSource) {
        if (edge === 'top') {
            return isSource ? 'to-top' : 'from-top';
        } else if (edge === 'bottom') {
            return isSource ? 'to-bottom' : 'from-bottom';
        } else if (edge === 'left') {
            return isSource ? 'to-left' : 'from-left';
        } else if (edge === 'right') {
            return isSource ? 'to-right' : 'from-right';
        }
        return '';
    }

    /**
     * Analyze link connections for each node to enable smart snapping
     */

    /**
     * Format action for state label
     * W3C SCXML 3.7: Format actions concisely for state diagrams
     */
    /**
     * Calculate left margin for action text positioning
     * Text starts at 10% from left edge (separator line at 5%)
     * 
     * @param {number} width - State width
     * @returns {number} Left margin in local coordinates
     */
    getActionTextLeftMargin(width) {
        return -width/2 + (width * LAYOUT_CONSTANTS.TEXT_LEFT_MARGIN_PERCENT);
    }

    /**
     * Render action text with background box
     * DRY: Shared by entry and exit action rendering
     * 
     * @param {object} config - Configuration object
     * @param {string} config.prefix - Text prefix ('↓ entry' or '↑ exit')
     * @param {string} config.color - Text color
     * @param {Array} config.actions - Actions to render
     * @param {number} config.yOffset - Current Y position
     * @param {object} config.stateData - State data (d)
     * @param {object} config.group - D3 group element
     * @param {number} config.leftMargin - Left margin
     * @returns {number} Updated yOffset
     */
    renderActionTexts(config) {
        const { prefix, color, actions, stateData, group, leftMargin } = config;
        let yOffset = config.yOffset;
        const self = this;

        actions.forEach(action => {
            const actionText = self.formatActionText(action);
            if (actionText) {
                const actionGroup = group.append('g');
                const fullText = `${prefix} / ${actionText}`;

                // Create background layer first (will be drawn behind)
                const bgLayer = actionGroup.append('g').attr('class', 'bg-layer');

                // Create text layer second (will be drawn on top)
                const textLayer = actionGroup.append('g').attr('class', 'text-layer');

                // Text x position: leftMargin + additional padding
                const textX = leftMargin + LAYOUT_CONSTANTS.TEXT_PADDING;
                const textElement = textLayer.append('text')
                    .attr('x', textX)
                    .attr('y', yOffset)
                    .attr('text-anchor', 'start')
                    .attr('dominant-baseline', 'middle')
                    .attr('font-size', '13px')
                    .attr('fill', color)
                    .text(fullText);

                // Get dimensions using getBBox (wait for render)
                const textNode = textElement.node();

                // Force layout flush to ensure getBBox returns accurate values
                textNode.getBoundingClientRect();

                let bbox;
                try {
                    bbox = textNode.getBBox();
                    // Validate getBBox result
                    if (!bbox || bbox.width === 0 || bbox.height === 0) {
                        // getBBox returned zero, forcing reflow (normal during initial render)
                        group.node().getBoundingClientRect();
                        bbox = textNode.getBBox();
                    }
                } catch (e) {
                    console.error(`[getBBox] ${stateData.id}: Error getting bbox: ${e.message}`);
                    bbox = {x: 0, y: 0, width: 0, height: 0};
                }

                // Calculate box dimensions and position
                let boxWidth, boxHeight, boxX, boxY;
                const boxPaddingH = 12;
                const boxPaddingV = 6;

                if (bbox.width === 0 || bbox.height === 0) {
                    // Improved fallback: use canvas measurement for accuracy
                    const canvas = document.createElement('canvas');
                    const ctx = canvas.getContext('2d');
                    ctx.font = '13px sans-serif';
                    const metrics = ctx.measureText(fullText);
                    boxWidth = metrics.width;
                    boxHeight = 15;
                } else {
                    // Use getBBox for accurate dimensions (width and height only)
                    boxWidth = bbox.width;
                    boxHeight = bbox.height;
                }

                // Box position: align with text start position (textX)
                // Text starts at textX with text-anchor='start', so box should start at textX - padding
                boxX = textX - 6;
                boxY = yOffset - (boxHeight + boxPaddingV) / 2;

                // Add background box to bg layer (behind text)
                bgLayer.append('rect')
                    .attr('x', boxX)
                    .attr('y', boxY)
                    .attr('width', boxWidth + boxPaddingH)
                    .attr('height', boxHeight + boxPaddingV)
                    .attr('fill', prefix.includes('entry') ? '#f0fdf4' : '#fef2f2')
                    .attr('rx', 4)
                    .attr('opacity', 0.6);

                yOffset += boxHeight + boxPaddingV + 4; // Move to next action (reduced spacing)
            }
        });

        return yOffset;
    }

    formatActionText(action) {
        if (!action || !action.actionType) return '';

        if (action.actionType === 'assign') {
            return `${action.location}=${action.expr}`;
        } else if (action.actionType === 'log') {
            return `log(${action.label || action.expr || ''})`;
        } else if (action.actionType === 'send') {
            // W3C SCXML 6.2: Format send with key attributes
            let parts = [];
            if (action.event) {
                parts.push(`event=${action.event}`);
            } else if (action.eventexpr) {
                parts.push(`eventexpr=${action.eventexpr}`);
            }
            if (action.target) {
                parts.push(`target=${action.target}`);
            }
            if (action.delay) {
                parts.push(`delay=${action.delay}`);
            }
            return parts.length > 0 ? `send(${parts.join(', ')})` : 'send()';
        } else if (action.actionType === 'raise') {
            return `raise(${action.event})`;
        } else if (action.actionType === 'script') {
            // W3C SCXML 5.9: Show truncated script content
            if (action.content) {
                const preview = action.content.substring(0, 30);
                return preview.length < action.content.length ? `script(${preview}...)` : `script(${preview})`;
            }
            return 'script';
        } else if (action.actionType === 'if') {
            // W3C SCXML 3.12.1: Show all branch conditions with key actions
            if (action.branches && action.branches.length > 0) {
                const branchTexts = action.branches.map((branch, i) => {
                    // Format condition part
                    let condPart = '';
                    if (branch.isElse) {
                        condPart = 'else';
                    } else if (i === 0) {
                        condPart = `if(${branch.condition || ''})`;
                    } else {
                        condPart = `elseif(${branch.condition || ''})`;
                    }

                    // Extract and format key actions (prioritize raise for clarity)
                    let actionPart = '';
                    if (branch.actions && branch.actions.length > 0) {
                        // Check for raise actions first (most important for state machine logic)
                        const raiseActions = branch.actions.filter(a => a.actionType === 'raise');
                        if (raiseActions.length > 0) {
                            const events = raiseActions.map(a => a.event).join(',');
                            actionPart = ` → ${events}`;
                        } else {
                            // Show first 2 actions for other types
                            const actionTypes = branch.actions
                                .slice(0, 2)
                                .map(a => {
                                    if (a.actionType === 'assign') return `${a.location}=..`;
                                    if (a.actionType === 'send') return `send(${a.event || '...'})`;
                                    return a.actionType;
                                })
                                .join(', ');
                            const more = branch.actions.length > 2 ? '...' : '';
                            actionPart = ` → ${actionTypes}${more}`;
                        }
                    }

                    return condPart + actionPart;
                });
                return branchTexts.join(' / ');
            }
            // Fallback for missing branches data
            const condText = action.cond || '';
            return `if(${condText})`;
        } else if (action.actionType === 'foreach') {
            return `foreach(${action.item || ''} in ${action.array || ''})`;
        } else if (action.actionType === 'cancel') {
            // W3C SCXML 6.3: Format cancel with sendid
            if (action.sendid) {
                return `cancel(${action.sendid})`;
            } else if (action.sendidexpr) {
                return `cancel(${action.sendidexpr})`;
            }
            return 'cancel()';
        }

        return action.actionType;
    }

    /**
     * Get transition label text (event, condition, actions)
     * Format: "event [cond] / action1, action2"
     */
    getTransitionLabelText(transition) {
        let label = '';
        
        // Event name
        if (transition.event) {
            label = transition.event;
        }

        // Condition (guard)
        if (transition.cond) {
            label += (label ? ' ' : '') + `[${transition.cond}]`;
        }
        
        // Actions
        const actions = [];
        if (transition.actions) {
            transition.actions.forEach(action => {
                if (action.type === 'assign') {
                    actions.push(`${action.location}=${action.expr}`);
                } else if (action.type === 'log') {
                    actions.push(`log(${action.label || action.expr})`);
                } else if (action.type === 'send') {
                    actions.push(`send(${action.event})`);
                } else if (action.type === 'raise') {
                    actions.push(`raise(${action.event})`);
                }
            });
        }
        
        if (actions.length > 0) {
            label += ` / ${actions.join(', ')}`;
        }
        
        return label;
    }
    
    /**
     * Get transition label position (on the actual path)
     */
    getTransitionLabelPosition(transition) {
        // Use routing information to get actual path coordinates
        if (transition.routing && transition.routing.sourcePoint && transition.routing.targetPoint) {
            const start = transition.routing.sourcePoint;
            const end = transition.routing.targetPoint;
            const sourceEdge = transition.routing.sourceEdge;
            const targetEdge = transition.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

            // Calculate the middle segment of the path based on edge types
            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: VHV path (vertical-horizontal-vertical)
                // Middle segment is horizontal: from (sx, y1) to (tx, y1)
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else {
                    y1 = sy + MIN_SEGMENT;
                }

                // Return midpoint of horizontal segment, slightly above
                return {
                    x: (sx + tx) / 2,
                    y: y1 - 8
                };
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: HVH path (horizontal-vertical-horizontal)
                // Middle segment is vertical: from (x1, sy) to (x1, ty)
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else {
                    x1 = sx - MIN_SEGMENT;
                }

                // Return midpoint of vertical segment, to the right
                return {
                    x: x1 + 10,
                    y: (sy + ty) / 2
                };
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: V→H path
                // Place label near the corner point
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else {
                    y1 = sy + MIN_SEGMENT;
                }

                return {
                    x: (sx + tx) / 2,
                    y: y1 - 8
                };
            } else {
                // Source horizontal, target vertical: H→V path
                // Place label near the corner point
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else {
                    x1 = sx - MIN_SEGMENT;
                }

                return {
                    x: x1 + 10,
                    y: (sy + ty) / 2
                };
            }
        }

        // Fallback to simple midpoint
        const sourceNode = this.nodes.find(n => n.id === transition.source);
        const targetNode = this.nodes.find(n => n.id === transition.target);

        if (!sourceNode || !targetNode) {
            return { x: 0, y: 0 };
        }

        return {
            x: (sourceNode.x + targetNode.x) / 2,
            y: (sourceNode.y + targetNode.y) / 2 - 8
        };
    }
    
    // **REMOVED: analyzeLinkConnections() is obsolete**
    // Two-pass algorithm uses link.routing instead of prediction-based edge analysis

    /**
     * Determine which side of the node faces the given point
     */
    getNodeSide(node, toX, toY) {
        const cx = node.x || 0;
        const cy = node.y || 0;
        const dx = toX - cx;
        const dy = toY - cy;

        // Use angle to determine side
        const angle = Math.atan2(dy, dx) * 180 / Math.PI;

        // -45 to 45: right, 45 to 135: bottom, 135 to 180 or -180 to -135: left, -135 to -45: top
        if (angle >= -45 && angle < 45) return 'right';
        if (angle >= 45 && angle < 135) return 'bottom';
        if (angle >= 135 || angle < -135) return 'left';
        return 'top';
    }

    /**
     * Calculate intersection point between line and node boundary with smart snapping
     */
    getNodeBoundaryPoint(node, fromX, fromY, link, isSource, connections) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        // Direction vector from node center to target
        const dx = fromX - cx;
        const dy = fromY - cy;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance === 0) return { x: cx, y: cy };

        // Normalize direction
        const ndx = dx / distance;
        const ndy = dy / distance;

        // Get node dimensions based on type
        if (node.type === 'initial-pseudo') {
            // Circle with radius 10
            return {
                x: cx + ndx * 10,
                y: cy + ndy * 10
            };
        } else if (node.type === 'history') {
            // Circle with radius 20
            return {
                x: cx + ndx * 20,
                y: cy + ndy * 20
            };
        } else if (node.type === 'atomic' || node.type === 'final') {
            // Rectangle with smart snapping (use actual node dimensions)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            // Determine which side this connection is on
            const side = this.getNodeSide(node, fromX, fromY);

            // Get snap position on this side
            let snapX = cx, snapY = cy;

            if (connections && connections[node.id]) {
                const sideConnections = connections[node.id][side];
                const index = sideConnections.findIndex(c =>
                    c.link.id === link.id && c.isSource === isSource
                );

                if (index >= 0 && sideConnections.length > 0) {
                    const count = sideConnections.length;
                    const position = (index + 1) / (count + 1); // Divide side into (count+1) segments

                    if (side === 'top') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy - halfHeight;
                    } else if (side === 'bottom') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy + halfHeight;
                    } else if (side === 'left') {
                        snapX = cx - halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    } else if (side === 'right') {
                        snapX = cx + halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    }

                    // Snap point is exactly on boundary
                    return { x: snapX, y: snapY };
                }
            }

            // Fallback: no snapping, use angle-based intersection
            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        } else if (node.type === 'compound' || node.type === 'parallel') {
            // Use node's width and height
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    /**
     * Get incoming direction of last segment for a point in orthogonal path
     */
    getOrthogonalIncomingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'from-top' : 'from-bottom';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'from-left' : 'from-right';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // Last segment is vertical: (end.x, midY) → (end.x, end.y)
        return end.y > midY ? 'from-top' : 'from-bottom';
    }

    /**
     * Get outgoing direction of first segment for a point in orthogonal path
     */
    getOrthogonalOutgoingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'to-bottom' : 'to-top';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'to-right' : 'to-left';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // First segment is vertical: (start.x, start.y) → (start.x, midY)
        return midY > start.y ? 'to-bottom' : 'to-top';
    }

    /**
     * Get boundary point based on actual segment direction with smart snapping
     */
    getOrthogonalBoundaryPoint(node, direction, link = null, isSource = true, connections = null) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            const halfWidth = PATH_CONSTANTS.INITIAL_NODE_HALF_WIDTH;
            const halfHeight = 20;

            // **PRIORITY: Use routing if available (don't let direction override it)**
            if (link && link.routing) {
                const optPoint = isSource ? link.routing.sourcePoint : link.routing.targetPoint;

                if (optPoint) {
                    // Use the optimized snap point directly, no fallback needed
                    return { x: optPoint.x, y: optPoint.y };
                }
            }

            // Map direction to side
            let side = null;
            if (direction === 'from-top' || direction === 'to-top') {
                side = 'top';
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                side = 'bottom';
            } else if (direction === 'from-left' || direction === 'to-left') {
                side = 'left';
            } else if (direction === 'from-right' || direction === 'to-right') {
                side = 'right';
            }

            // Smart snapping: use layout optimizer to calculate optimal snap position
            if (side && link) {
                const snapResult = this.layoutOptimizer.calculateSnapPosition(
                    node.id,
                    side,
                    link.id,
                    direction
                );

                if (snapResult) {
                    return { x: snapResult.x, y: snapResult.y };
                }

                // If blocked by initial transition, try alternative edges
                if (this.layoutOptimizer.hasInitialTransitionOnEdge(node.id, side)) {
                    console.log(`[FALLBACK] ${node.id} ${side}: blocked, trying alternative edge`);

                    // Try alternative edges based on original direction
                    const alternatives = [];
                    if (side === 'top' || side === 'bottom') {
                        alternatives.push('left', 'right', side === 'top' ? 'bottom' : 'top');
                    } else {
                        alternatives.push('top', 'bottom', side === 'left' ? 'right' : 'left');
                    }

                    // Try each alternative
                    for (const altSide of alternatives) {
                        if (!this.layoutOptimizer.hasInitialTransitionOnEdge(node.id, altSide)) {
                            console.log(`[FALLBACK] ${node.id}: using ${altSide} instead of ${side}`);

                            // Calculate proper snap position for alternative edge
                            const altDirection = isSource ? `to-${altSide}` : `from-${altSide}`;
                            const altSnapResult = this.layoutOptimizer.calculateSnapPosition(
                                node.id,
                                altSide,
                                link.id,
                                altDirection
                            );

                            // **DO NOT MODIFY routing - it's read-only!**
                            // Return the alternative snap position without modifying routing

                            if (altSnapResult) {
                                return { x: altSnapResult.x, y: altSnapResult.y };
                            }

                            // Fallback to edge center if snap calculation fails
                            if (altSide === 'top') {
                                return { x: cx, y: cy - halfHeight };
                            } else if (altSide === 'bottom') {
                                return { x: cx, y: cy + halfHeight };
                            } else if (altSide === 'left') {
                                return { x: cx - halfWidth, y: cy };
                            } else if (altSide === 'right') {
                                return { x: cx + halfWidth, y: cy };
                            }
                        }
                    }
                }
            }

            // Fallback: no smart snapping, use edge center
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - halfHeight };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + halfHeight };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - halfWidth, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + halfWidth, y: cy };
            }
        } else if (node.type === 'initial-pseudo') {
            const radius = 10;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        } else if (node.type === 'history') {
            const radius = 20;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    /**
     * Get node's bounding box
     */
    getNodeBounds(node) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            // Use actual node dimensions (not hardcoded 60x40)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        } else if (node.type === 'initial-pseudo') {
            return {
                left: cx - 10,
                right: cx + 10,
                top: cy - 10,
                bottom: cy + 10
            };
        } else if (node.type === 'history') {
            return {
                left: cx - 20,
                right: cx + 20,
                top: cy - 20,
                bottom: cy + 20
            };
        } else if (node.type === 'compound' || node.type === 'parallel') {
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        }

        return { left: cx, right: cx, top: cy, bottom: cy };
    }

    /**
     * Check if horizontal line at given Y intersects with node's bounding box
     */
    horizontalLineIntersectsNode(y, xStart, xEnd, node) {
        const bounds = this.getNodeBounds(node);

        // Check if y is within node's vertical range
        if (y < bounds.top || y > bounds.bottom) {
            return false;
        }

        // Check if horizontal line segment overlaps with node's horizontal range
        const lineLeft = Math.min(xStart, xEnd);
        const lineRight = Math.max(xStart, xEnd);

        return !(lineRight < bounds.left || lineLeft > bounds.right);
    }

    /**
     * Check if vertical line segment intersects a node
     */
    verticalLineIntersectsNode(x, yStart, yEnd, node) {
        const bounds = this.getNodeBounds(node);

        // Check if x is within node's horizontal bounds
        if (x < bounds.left || x > bounds.right) {
            return false;
        }

        // Check if vertical segment overlaps node's vertical bounds
        const lineTop = Math.min(yStart, yEnd);
        const lineBottom = Math.max(yStart, yEnd);

        return !(lineBottom < bounds.top || lineTop > bounds.bottom);
    }

    /**
     * Get all nodes that might collide with the path (excluding source and target)
     */
    getObstacleNodes(sourceNode, targetNode) {
        return this.nodes.filter(node => 
            node.id !== sourceNode.id && 
            node.id !== targetNode.id &&
            node.type !== 'initial'  // Initial state markers are small, ignore them
        );
    }

    /**
     * Find a collision-free horizontal Y coordinate
     */
    findCollisionFreeY(sx, sy, tx, ty, obstacles) {
        const candidates = [];
        const margin = 15;

        // Strategy 1: Try midpoint
        const midY = (sy + ty) / 2;
        candidates.push(midY);

        // Strategy 2: Try routing above all obstacles
        const maxTop = Math.max(
            ...obstacles.map(node => this.getNodeBounds(node).top),
            sy, ty
        );
        candidates.push(maxTop - margin);

        // Strategy 3: Try routing below all obstacles
        const minBottom = Math.min(
            ...obstacles.map(node => this.getNodeBounds(node).bottom),
            sy, ty
        );
        candidates.push(minBottom + margin);

        // Strategy 4: Try routing at source height
        candidates.push(sy);

        // Strategy 5: Try routing at target height
        candidates.push(ty);

        // Test each candidate and find first that doesn't collide
        for (const candidateY of candidates) {
            let hasCollision = false;

            // Check horizontal segment collision
            for (const obstacle of obstacles) {
                if (this.horizontalLineIntersectsNode(candidateY, sx, tx, obstacle)) {
                    hasCollision = true;
                    break;
                }
            }

            if (!hasCollision) {
                // Also verify vertical segments don't collide
                const verticalCollision = obstacles.some(obstacle =>
                    this.verticalLineIntersectsNode(sx, sy, candidateY, obstacle) ||
                    this.verticalLineIntersectsNode(tx, candidateY, ty, obstacle)
                );

                if (!verticalCollision) {
                    return candidateY;
                }
            }
        }

        // Fallback: return midpoint (better than nothing)
        return midY;
    }

    /**
     * Calculate link directions (for Two-Pass algorithm - Pass 1)
     * Same logic as createOrthogonalPath but only returns directions
     */
    calculateLinkDirections(sourceNode, targetNode, link) {
        // **PRIORITY: If routing exists, calculate midY and store in routing**
        // Don't run collision avoidance again - optimizer already calculated optimal path
        if (link.routing && link.routing.sourceEdge && link.routing.targetEdge) {
            const sy = link.routing.sourcePoint.y;
            const ty = link.routing.targetPoint.y;
            const midY = (sy + ty) / 2;

            // Store midY in routing for z-path collision avoidance
            link.routing.midY = midY;
            return;
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        if (link.linkType !== 'containment' && link.linkType !== 'delegation') {
            console.warn(`[CALC DIR WARNING] ${link.source}→${link.target}: No routing found! Optimizer should have run first.`);
        }
    }

    /**
     * Create ORTHOGONAL path with comprehensive collision avoidance
     */
    createOrthogonalPath(sourceNode, targetNode, link, connections) {
        // **OPTIMIZED SNAP POINTS: Use routing if available**
        if (link.routing) {
            const start = link.routing.sourcePoint;
            const end = link.routing.targetPoint;
            const sourceEdge = link.routing.sourceEdge;
            const targetEdge = link.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            // Debug mode: log path coordinates
            if (this.debugMode) {
                console.log(`[PATH DEBUG] ${link.source}→${link.target}: source=(${sx.toFixed(1)}, ${sy.toFixed(1)}), target=(${tx.toFixed(1)}, ${ty.toFixed(1)})`);
            }

            const dx = Math.abs(tx - sx);
            const dy = Math.abs(ty - sy);

            // Check if direct line (horizontal or vertical alignment)
            if (dx < 1 || dy < 1) {
                // Direct line
                return `M ${sx} ${sy} L ${tx} ${ty}`;
            }

            // Create orthogonal path based on edge directions with minimum segment lengths
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');
            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;  // Minimum horizontal/vertical segment length

            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: vertical-horizontal-vertical (5 points)
                // Ensure minimum vertical segments from source and target
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to target x → vertical MIN_SEGMENT → end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${tx} ${y1} L ${tx} ${y2} L ${tx} ${ty}`;
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: horizontal-vertical-horizontal (5 points)
                // Ensure minimum horizontal segments from source and target
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to target y → horizontal MIN_SEGMENT → end
                return `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${ty} L ${x2} ${ty} L ${tx} ${ty}`;
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: vertical-then-horizontal (5 points)
                // Ensure minimum segments
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to x2 → horizontal MIN_SEGMENT to end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${x2} ${y1} L ${x2} ${ty} L ${tx} ${ty}`;
            } else {
                // Source horizontal, target vertical: horizontal-then-vertical (5 points)
                // Ensure minimum segments
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to y2 → vertical MIN_SEGMENT to end
                return `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${y2} L ${tx} ${y2} L ${tx} ${ty}`;
            }
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        if (link.linkType !== 'containment' && link.linkType !== 'delegation') {
            console.warn(`[PATH WARNING] ${link.source}→${link.target}: No routing found! Falling back to node centers.`);
        }

        // Draw direct line as emergency fallback
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;
        return `M ${sx} ${sy} L ${tx} ${ty}`;
    }

    /**
     * Get link path using ELK routing information
     */
    getLinkPath(link) {
        console.log(`[GET LINK PATH] Called for ${link.source}→${link.target}`);
        // Get source and target nodes
        const sourceNode = this.nodes.find(n => n.id === link.source);
        const targetNode = this.nodes.find(n => n.id === link.target);

        if (!sourceNode || !targetNode) {
            console.log(`[GET LINK PATH] Source or target node not found`);
            return 'M 0 0';
        }

        // **TWO-PASS: No need for analyzeLinkConnections(), optimizer uses link.routing**
        const connections = null;

        // If either node is being dragged, use dynamic orthogonal path recalculation
        if (sourceNode.isDragging || targetNode.isDragging) {
            // Create ORTHOGONAL path with direction-aware boundary snapping
            return this.createOrthogonalPath(sourceNode, targetNode, link, connections);
        }

        // Use ELK edge routing only if available (only during initial ELK layout)
        if (link.elkSections && link.elkSections.length > 0) {
            const section = link.elkSections[0];

            // Calculate boundary points for start and end
            let startPoint, endPoint;

            if (section.bendPoints && section.bendPoints.length > 0) {
                // If there are bend points, calculate boundary to first/last bend point
                const firstBend = section.bendPoints[0];
                const lastBend = section.bendPoints[section.bendPoints.length - 1];

                startPoint = this.getNodeBoundaryPoint(sourceNode, firstBend.x, firstBend.y, link, true, connections);
                endPoint = this.getNodeBoundaryPoint(targetNode, lastBend.x, lastBend.y, link, false, connections);

                // Build path: start boundary → bend points → end boundary
                let path = `M ${startPoint.x} ${startPoint.y}`;
                section.bendPoints.forEach(point => {
                    path += ` L ${point.x} ${point.y}`;
                });
                path += ` L ${endPoint.x} ${endPoint.y}`;

                return path;
            } else {
                // No bend points, direct line with boundary calculation
                const sx = sourceNode.x || 0;
                const sy = sourceNode.y || 0;
                const tx = targetNode.x || 0;
                const ty = targetNode.y || 0;

                startPoint = this.getNodeBoundaryPoint(sourceNode, tx, ty, link, true, connections);
                endPoint = this.getNodeBoundaryPoint(targetNode, sx, sy, link, false, connections);

                return `M ${startPoint.x} ${startPoint.y} L ${endPoint.x} ${endPoint.y}`;
            }
        }

        // Fallback to orthogonal path (after ELK routing is invalidated)
        // This ensures all paths use routing for consistent routing
        return this.createOrthogonalPath(sourceNode, targetNode, link, connections);
    }

    /**
     * Update link paths after drag
     */
    updateLinks(useGreedy = false) {
        // console.log(`[UPDATE LINKS] Called with useGreedy=${useGreedy}`);
        if (!this.linkElements || !this.allLinks) {
            // console.log('[UPDATE LINKS] Early return: linkElements or allLinks missing');
            return;
        }

        // **INVALIDATE ROUTING: Clear routing for dragged nodes only**
        // This allows dynamic recalculation during drag, while preserving routing otherwise
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);

        // Check if any node is being dragged
        const anyNodeDragging = this.nodes.some(n => n.isDragging);

        if (anyNodeDragging || useGreedy) {
            // const mode = useGreedy ? 'GREEDY (fast)' : 'CSP (optimal)';
            // console.log(`[DRAG UPDATE] Re-running optimizer (${mode})...`);

            // Clear all routing
            this.allLinks.forEach(link => {
                delete link.routing;
            });

            // **ADAPTIVE ALGORITHM SELECTION**
            // - useGreedy=true: Fast greedy for real-time drag (1-5ms)
            // - useGreedy=false: Optimal CSP for final result (50-200ms)
            this.layoutOptimizer.optimizeSnapPointAssignments(this.allLinks, this.nodes, useGreedy);

            // Calculate midY for new routing
            visibleLinks.forEach(link => {
                const sourceNode = this.nodes.find(n => n.id === link.source);
                const targetNode = this.nodes.find(n => n.id === link.target);
                if (sourceNode && targetNode) {
                    this.calculateLinkDirections(sourceNode, targetNode, link);
                }
            });

            // console.log('[DRAG UPDATE] Re-optimization complete');
        }

        // Pass 2: Render with updated directions
        this.linkElements.attr('d', d => this.getLinkPath(d));

        // Update transition labels if they exist
        if (this.transitionLabels) {
            this.transitionLabels
                .attr('x', d => this.getTransitionLabelPosition(d).x)
                .attr('y', d => this.getTransitionLabelPosition(d).y);
        }

        // Update node visuals with latest positions from this.nodes
        // Performance: Create Map for O(1) lookup instead of O(n) find()
        const nodeMap = new Map(this.nodes.map(n => [n.id, n]));
        
        if (this.nodeElements) {
            this.nodeElements.each(function(nodeData) {
                const latestNode = nodeMap.get(nodeData.id);
                if (latestNode) {
                    d3.select(this).attr('transform', `translate(${latestNode.x}, ${latestNode.y})`);
                }
            });
        }

        // Update compound container visuals with latest positions
        if (this.compoundContainers) {
            this.compoundContainers.each(function(compoundData) {
                const latestNode = nodeMap.get(compoundData.id);
                if (latestNode) {
                    d3.select(this)
                        .attr('x', latestNode.x - latestNode.width/2)
                        .attr('y', latestNode.y - latestNode.height/2);
                }
            });
        }

        // Update compound label positions with latest positions
        if (this.compoundLabels) {
            this.compoundLabels.each(function(labelData) {
                const latestNode = nodeMap.get(labelData.id);
                if (latestNode) {
                    d3.select(this)
                        .attr('x', latestNode.x - latestNode.width/2 + 10)
                        .attr('y', latestNode.y - latestNode.height/2 + 20);
                }
            });
        }

        // Update snap points visualization if enabled
        if (this.showSnapPoints) {
            // Fast update: only change positions, no DOM re-creation
            this.updateSnapPointPositions();
        }
    }

    /**
     * Fast update for real-time drag (uses greedy algorithm)
     */
    updateLinksFast() {
        this.updateLinks(true); // useGreedy=true
    }

    /**
     * Optimal update for final positioning (uses CSP solver)
     */
    updateLinksOptimal() {
        this.updateLinks(false); // useGreedy=false
    }

    /**
     * Toggle compound state
     */
    async toggleCompoundState(stateId) {
        const state = this.nodes.find(n => n.id === stateId);
        if (!state) return;

        state.collapsed = !state.collapsed;
        console.log(`Toggled ${stateId}: ${state.collapsed ? 'collapsed' : 'expanded'}`);

        await this.computeLayout();
        this.render();
    }

    /**
     * Highlight active states
     */
    highlightActiveStates(activeStateIds) {
        console.log(`[highlightActiveStates] Called with:`, activeStateIds);
        this.activeStates = new Set(activeStateIds);

        // Auto-expand compound/parallel states that are active or have active children
        let needsReLayout = false;
        this.nodes.forEach(node => {
            if ((node.type === 'compound' || node.type === 'parallel') && node.collapsed) {
                // Check if this node is active OR has any active children
                const isActive = this.activeStates.has(node.id);
                const hasActiveChildren = node.children && node.children.some(childId => this.activeStates.has(childId));

                if (isActive || hasActiveChildren) {
                    console.log(`  → Auto-expanding ${node.id} (${node.type}): isActive=${isActive}, hasActiveChildren=${hasActiveChildren}`);
                    node.collapsed = false;
                    needsReLayout = true;
                }
            }
        });

        // Re-layout if any compound/parallel was expanded
        if (needsReLayout) {
            console.log(`  → Triggering re-layout due to auto-expansion`);
            this.computeLayout().then(() => {
                this.render();
                // Re-highlight after re-render
                this.highlightActiveStatesVisual();
            });
            return;
        }

        this.highlightActiveStatesVisual();
    }

    highlightActiveStatesVisual() {
        console.log(`[highlightActiveStatesVisual] Applying visual highlights`);
        console.log(`  nodeElements exists: ${this.nodeElements ? 'yes' : 'no'}, size: ${this.nodeElements ? this.nodeElements.size() : 0}`);
        console.log(`  collapsedElements exists: ${this.collapsedElements ? 'yes' : 'no'}, size: ${this.collapsedElements ? this.collapsedElements.size() : 0}`);
        console.log(`  compoundContainers exists: ${this.compoundContainers ? 'yes' : 'no'}, size: ${this.compoundContainers ? this.compoundContainers.size() : 0}`);

        if (this.nodeElements) {
            this.nodeElements.classed('active', d => {
                const isActive = this.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating node: ${d.id} (type: ${d.type})`);
                }
                return isActive;
            });
        }

        if (this.collapsedElements) {
            this.collapsedElements.classed('active', d => {
                const isActive = this.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating collapsed: ${d.id}`);
                }
                return isActive;
            });
        }

        if (this.compoundContainers) {
            this.compoundContainers.classed('active', d => {
                const isActive = this.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating compound container: ${d.id}`);
                }
                return isActive;
            });
        }
    }

    /**
     * Animate transition (DEPRECATED - CSS handles animation now)
     *
     * Zero Duplication Principle: CSS handles all animation through .highlighted class
     * This method is kept for backward compatibility but does nothing.
     * Animation is now automatically triggered by highlightTransition() via CSS.
     */
    animateTransition(transition) {
        // No-op: CSS handles animation via .highlighted class
        // See visualizer.css: .transition.highlighted { animation: transitionPulse ... }
        console.log('[DEPRECATED] animateTransition() called - CSS handles animation now');
    }


    /**
     * Render transition list in info panel
     */
    renderTransitionList() {
        const panel = document.getElementById('transition-list-panel');
        if (!panel) return;

        if (!this.transitions || this.transitions.length === 0) {
            panel.innerHTML = '<div class="transition-hint">No transitions</div>';
            return;
        }

        let html = '<div class="transition-list">';
        html += '<div class="transition-list-header">All Transitions</div>';

        this.transitions.forEach((transition, index) => {
            const transitionId = `${transition.source}-${transition.target}`;
            const eventText = transition.event || '(eventless)';

            html += `
                <div class="transition-list-item" data-transition-id="${transitionId}" data-transition-index="${index}">
                    <div class="transition-list-source-target">
                        <strong>${transition.source}</strong> → <strong>${transition.target}</strong>
                    </div>
                    <div class="transition-list-event">${eventText}</div>
                </div>
            `;
        });

        html += '</div>';
        panel.innerHTML = html;

        // Add click handlers
        const self = this;
        panel.querySelectorAll('.transition-list-item').forEach(item => {
            item.addEventListener('click', function() {
                const index = parseInt(this.getAttribute('data-transition-index'));
                const transition = self.transitions[index];

                // Show transition animation on diagram (temporary)
                self.highlightTransition(transition);
                self.focusOnTransition(transition);

                // Design System: Panel highlight animation (matches State Actions panel)
                this.classList.add('panel-highlighted');
                setTimeout(() => {
                    this.classList.remove('panel-highlighted');
                }, 3000);  // 3s animation duration

                // Dispatch event for execution-controller to update detail panel
                document.dispatchEvent(new CustomEvent('transition-click', { detail: transition }));
            });
        });
    }

    /**
     * Set active transition (permanent highlight for last executed transition - like state.active)
     * This is different from click - it shows which transition was last executed
     */
    setActiveTransition(transition) {
        console.log('[SET ACTIVE TRANSITION] Setting active transition:', transition);

        const panel = document.getElementById('transition-list-panel');
        const transitionId = transition ? `${transition.source}-${transition.target}` : null;

        // Clear all previous active states in panel
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
        }

        // Clear all previous active states in diagram (SVG)
        if (this.linkElements) {
            this.linkElements.classed('active', false);
        }
        if (this.transitionLabels) {
            this.transitionLabels.classed('active', false);
        }

        // Set active state on matching item (panel + diagram)
        if (transitionId) {
            // Panel: Set active state
            if (panel) {
                const activeItem = panel.querySelector(`[data-transition-id="${transitionId}"]`);
                if (activeItem) {
                    activeItem.classList.add('active');
                    console.log('[SET ACTIVE TRANSITION] Panel active state set on:', transitionId);
                }
            }

            // Diagram: Set active state (permanent - like state.active)
            if (this.linkElements) {
                this.linkElements.each(function(d) {
                    const linkId = `${d.source}-${d.target}`;
                    if (linkId === transitionId) {
                        d3.select(this).classed('active', true);
                        console.log('[SET ACTIVE TRANSITION] Diagram active state set on:', linkId);
                    }
                });
            }

            // Diagram label: Set active state
            if (this.transitionLabels) {
                this.transitionLabels.each(function(d) {
                    const linkId = `${d.source}-${d.target}`;
                    if (linkId === transitionId) {
                        d3.select(this).classed('active', true);
                        console.log('[SET ACTIVE TRANSITION] Label active state set on:', linkId);
                    }
                });
            }
        }
    }

    /**
     * Clear all transition highlights (SVG diagram + panel)
     * Note: Does NOT clear active state in transition list (active = last executed)
     * Design System: Cancel pending timeout for immediate response
     */
    clearTransitionHighlights() {
        console.log('[CLEAR HIGHLIGHT] Clearing transition highlights (SVG + panel)');

        // Cancel pending SVG highlight timeout (immediate cancellation on step backward)
        if (this.transitionHighlightTimeout) {
            clearTimeout(this.transitionHighlightTimeout);
            this.transitionHighlightTimeout = null;
            console.log('[CLEAR HIGHLIGHT] Cancelled pending SVG highlight timeout');
        }

        // Cancel pending panel highlight timeout (immediate cancellation on step backward)
        if (this.transitionPanelHighlightTimeout) {
            clearTimeout(this.transitionPanelHighlightTimeout);
            this.transitionPanelHighlightTimeout = null;
            console.log('[CLEAR HIGHLIGHT] Cancelled pending panel highlight timeout');
        }

        // Clear SVG diagram highlights
        if (this.linkElements) {
            this.linkElements.classed('highlighted', false);
        }

        if (this.transitionLabels) {
            this.transitionLabels.classed('highlighted', false);
        }

        // Clear panel highlights
        const panel = document.getElementById('transition-list-panel');
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('panel-highlighted');
            });
            console.log('[CLEAR HIGHLIGHT] Panel highlights cleared');
        }

        console.log('[CLEAR HIGHLIGHT] All highlights cleared (active state preserved)');
    }

    /**
     * Clear active transition state (used on reset or step backward to initial)
     */
    clearActiveTransition() {
        console.log('[CLEAR ACTIVE] Clearing active transition state');

        // Clear panel active state
        const panel = document.getElementById('transition-list-panel');
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
            console.log('[CLEAR ACTIVE] Panel active transition cleared');
        }

        // Clear diagram active state (SVG - permanent .active class)
        if (this.linkElements) {
            this.linkElements.classed('active', false);
            console.log('[CLEAR ACTIVE] Diagram active transition cleared');
        }
        if (this.transitionLabels) {
            this.transitionLabels.classed('active', false);
        }

        // Clear diagram highlights (SVG - temporary .highlighted class)
        this.clearTransitionHighlights();

        // Clear detail panel
        const detailPanel = document.getElementById('transition-detail-panel');
        if (detailPanel) {
            detailPanel.innerHTML = '<div class="transition-hint">Click a transition to view details</div>';
        }
    }

    /**
     * Highlight a specific transition (temporary - like state.focused)
     * Shows clicked transition with animation, auto-removed after 2 seconds
     * Design System: Consistent timeout cancellation (like State Actions DOM re-render)
     */
    highlightTransition(transition) {
        console.log('[HIGHLIGHT] highlightTransition() called with:', transition);

        if (!this.linkElements) {
            console.log('[HIGHLIGHT] No linkElements - aborting');
            return;
        }

        this.cancelPendingHighlights();
        const transitionId = `${transition.source}-${transition.target}`;

        this.clearTransitionHighlights();
        this.highlightLink(transitionId);
        this.highlightLabel(transitionId);
        this.scheduleHighlightRemoval();

        console.log('[HIGHLIGHT] highlightTransition() complete');
    }

    /**
     * Cancel pending highlight timeout (for immediate UI response)
     * @private
     */
    cancelPendingHighlights() {
        if (this.transitionHighlightTimeout) {
            clearTimeout(this.transitionHighlightTimeout);
            this.transitionHighlightTimeout = null;
            console.log('[HIGHLIGHT] Cancelled previous highlight timeout');
        }
    }

    /**
     * Highlight transition link in diagram
     * @private
     */
    highlightLink(transitionId) {
        console.log(`[HIGHLIGHT] Looking for transition ID: ${transitionId}`);
        console.log(`[HIGHLIGHT] Available linkElements count: ${this.linkElements.size()}`);

        let foundLink = false;
        this.linkElements.each(function(d) {
            const linkId = `${d.source}-${d.target}`;
            console.log(`[HIGHLIGHT] Checking link: ${linkId} (type: ${d.linkType})`);
            if (linkId === transitionId) {
                console.log(`[HIGHLIGHT] Match found! Highlighting link: ${linkId}`);
                d3.select(this).classed('highlighted', true);
                foundLink = true;
            }
        });

        if (!foundLink) {
            console.log(`[HIGHLIGHT] No match - transition ${transitionId} not found in linkElements`);
            if (this.debugMode) {
                console.log('[HIGHLIGHT] All available transitions:');
                this.linkElements.each(function(d) {
                    console.log(`  - ${d.source} → ${d.target} (type: ${d.linkType}, id: ${d.id})`);
                });
            }
        }
    }

    /**
     * Highlight transition label in diagram
     * @private
     */
    highlightLabel(transitionId) {
        if (!this.transitionLabels) {
            return;
        }

        console.log(`[HIGHLIGHT] Available transitionLabels: ${this.transitionLabels.size()}`);

        let foundLabel = false;
        this.transitionLabels.each(function(d) {
            const linkId = `${d.source}-${d.target}`;
            if (linkId === transitionId) {
                console.log(`[HIGHLIGHT] Label found for: ${linkId}`);
                d3.select(this).classed('highlighted', true);
                foundLabel = true;
            }
        });

        if (!foundLabel) {
            console.log(`[HIGHLIGHT] No label found for transition ${transitionId}`);
        }
    }

    /**
     * Schedule automatic removal of highlight after 2 seconds
     * @private
     */
    scheduleHighlightRemoval() {
        const self = this;
        // Auto-remove highlight after 2 seconds (matches FOCUS_HIGHLIGHT_DURATION in execution-controller.js)
        // Store timeout ID for cancellation (immediate response on step backward)
        this.transitionHighlightTimeout = setTimeout(() => {
            self.clearTransitionHighlights();
            self.transitionHighlightTimeout = null;
            console.log('[HIGHLIGHT] Auto-removed temporary highlight after 2s');
        }, 2000);
    }

    /**
     * Focus on a specific transition (pan and zoom)
     */
    focusOnTransition(transition) {
        const sourceNode = this.nodes.find(n => n.id === transition.source);
        const targetNode = this.nodes.find(n => n.id === transition.target);

        // Validate nodes exist
        if (!sourceNode || !targetNode) {
            console.warn('[FOCUS] Source or target node not found:', transition);
            return;
        }

        // Validate coordinates are valid numbers
        if (!Number.isFinite(sourceNode.x) || !Number.isFinite(sourceNode.y) ||
            !Number.isFinite(targetNode.x) || !Number.isFinite(targetNode.y)) {
            console.warn('[FOCUS] Invalid node coordinates:', {
                source: { id: sourceNode.id, x: sourceNode.x, y: sourceNode.y },
                target: { id: targetNode.id, x: targetNode.x, y: targetNode.y }
            });
            return;
        }

        // Calculate center point between source and target
        const centerX = (sourceNode.x + targetNode.x) / 2;
        const centerY = (sourceNode.y + targetNode.y) / 2;

        // Calculate distance between nodes
        const dx = targetNode.x - sourceNode.x;
        const dy = targetNode.y - sourceNode.y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        // Handle same-position nodes (prevent division by zero)
        if (Math.abs(dx) < 0.1 && Math.abs(dy) < 0.1) {
            console.log('[FOCUS] Same-position nodes, using default zoom');
            const transform = d3.zoomIdentity
                .translate(this.width / 2, this.height / 2)
                .scale(1.0)
                .translate(-centerX, -centerY);

            this.svg.transition()
                .duration(750)
                .call(this.zoom.transform, transform);
            return;
        }

        // Calculate zoom level to fit both nodes
        const padding = 100;
        const zoomLevel = Math.min(
            this.width / (Math.abs(dx) + padding * 2),
            this.height / (Math.abs(dy) + padding * 2),
            2.0  // Max zoom
        );

        // Validate zoom level is finite
        if (!Number.isFinite(zoomLevel) || zoomLevel <= 0) {
            console.warn('[FOCUS] Invalid zoom level:', zoomLevel);
            return;
        }

        // Apply transform
        const transform = d3.zoomIdentity
            .translate(this.width / 2, this.height / 2)
            .scale(zoomLevel)
            .translate(-centerX, -centerY);

        this.svg.transition()
            .duration(750)
            .call(this.zoom.transform, transform);
    }

    /**
     * Resize
     */
    resize() {
        const containerNode = this.container.node();
        if (!containerNode) return;

        const newWidth = containerNode.clientWidth;
        const newHeight = containerNode.clientHeight;

        if (newWidth > 0 && newHeight > 0) {
            this.width = newWidth;
            this.height = newHeight;

            this.svg.attr('viewBox', `0 0 ${this.width} ${this.height}`);

            console.log(`Resized to ${this.width}x${this.height}`);
        }
    }

    /**
     * Reset view
     */
    resetView() {
        this.svg.transition()
            .duration(750)
            .call(this.zoom.transform, this.initialTransform);
    }
}
