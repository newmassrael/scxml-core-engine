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

class SCXMLVisualizer {
    constructor(containerId, scxmlStructure) {
        this.container = d3.select(`#${containerId}`);
        this.states = scxmlStructure.states || [];
        this.transitions = scxmlStructure.transitions || [];
        this.initialState = scxmlStructure.initial || '';
        this.activeStates = new Set();
        
        // Debug mode from URL parameter (?debug)
        this.debugMode = DEBUG_MODE;

        // Container dimensions
        const containerNode = this.container.node();
        const clientWidth = containerNode ? containerNode.clientWidth : 0;
        const clientHeight = containerNode ? containerNode.clientHeight : 0;

        this.width = clientWidth > 0 ? clientWidth : 800;
        this.height = clientHeight > 0 ? clientHeight : 500;

        console.log(`Initializing ELK-based visualizer: ${this.width}x${this.height}`);
        console.log(`Initial state: ${this.initialState}`);

        if (DEBUG_MODE) {
            console.log('States:', this.states.length);
            console.log('Transitions:', this.transitions.length);
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
                collapsed: (state.type === 'compound' || state.type === 'parallel')
            };
            console.log(`[buildNodes] ${state.id}: type=${state.type}, children=${node.children.length}, collapsed=${node.collapsed}`);
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
                'elk.spacing.nodeNode': '80',
                'elk.layered.spacing.nodeNodeBetweenLayers': '100',
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
            },
            children: [],
            edges: []
        };

        const visibleNodes = this.getVisibleNodes();

        visibleNodes.forEach(node => {
            const elkNode = {
                id: node.id,
                width: this.getNodeWidth(node),
                height: this.getNodeHeight(node)
            };

            // Add children for expanded compounds
            if ((node.type === 'compound' || node.type === 'parallel') && !node.collapsed) {
                elkNode.children = [];
                elkNode.layoutOptions = {
                    'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
                    'elk.padding': '[top=50,left=25,bottom=25,right=25]'
                };

                node.children.forEach(childId => {
                    const childNode = this.nodes.find(n => n.id === childId);
                    if (childNode) {
                        elkNode.children.push({
                            id: childId,
                            width: this.getNodeWidth(childNode),
                            height: this.getNodeHeight(childNode)
                        });
                    }
                });
            }

            graph.children.push(elkNode);
        });

        // Add edges (only if both source and target are visible)
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));

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
        return 60;
    }

    /**
     * Get node height
     */
    getNodeHeight(node) {
        if (node.type === 'initial-pseudo') return 20;
        if (node.type === 'compound' || node.type === 'parallel') {
            return node.collapsed ? 50 : 200;
        }
        return 40;
    }

    /**
     * Get visible nodes
     */
    getVisibleNodes() {
        const visibleIds = new Set();

        this.nodes.forEach(node => {
            const isHidden = this.nodes.some(parent =>
                (parent.type === 'compound' || parent.type === 'parallel') &&
                parent.collapsed &&
                parent.children &&
                parent.children.includes(node.id)
            );

            if (!isHidden) {
                visibleIds.add(node.id);
            }
        });

        return this.nodes.filter(n => visibleIds.has(n.id));
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
        for (const parent of nodes) {
            if ((parent.type === 'compound' || parent.type === 'parallel') &&
                parent.collapsed &&
                parent.children &&
                parent.children.includes(nodeId)) {
                return parent;
            }
        }
        return null;
    }

    /**
     * Apply ELK layout
     */
    applyELKLayout(layouted) {
        console.log('Applying ELK layout to nodes...');

        const applyToNode = (elkNode, offsetX = 0, offsetY = 0) => {
            const node = this.nodes.find(n => n.id === elkNode.id);
            if (node) {
                node.x = elkNode.x + offsetX + elkNode.width / 2;
                node.y = elkNode.y + offsetY + elkNode.height / 2;
                node.width = elkNode.width;
                node.height = elkNode.height;

                console.log(`  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height}`);
            }

            if (elkNode.children) {
                elkNode.children.forEach(child => {
                    applyToNode(child, elkNode.x + offsetX, elkNode.y + offsetY);
                });
            }
        };

        layouted.children.forEach(child => {
            applyToNode(child);
        });

        // Apply ELK edge routing information
        if (layouted.edges) {
            console.log('Applying ELK edge routing...');
            layouted.edges.forEach(elkEdge => {
                const link = this.allLinks.find(l => l.id === elkEdge.id);
                if (link && elkEdge.sections && elkEdge.sections.length > 0) {
                    link.elkSections = elkEdge.sections;
                    console.log(`  ${elkEdge.id}: ${elkEdge.sections.length} section(s)`);
                }
            });
        }

        console.log('Layout application complete');
    }

    /**
     * Render SVG
     */
    render() {
        // Store reference to visualizer instance for use in drag handlers
        const self = this;

        // RequestAnimationFrame throttling for smooth drag
        let dragAnimationFrame = null;

        // Clear
        this.zoomContainer.selectAll('*').remove();

        const visibleNodes = this.getVisibleNodes();
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);

        // Compound containers (expanded)
        const compoundData = visibleNodes.filter(n =>
            (n.type === 'compound' || n.type === 'parallel') && !n.collapsed
        );

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
            .on('click', (event, d) => {
                event.stopPropagation();
                this.toggleCompoundState(d.id);
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
            .attr('transform', d => `translate(${d.x},${d.y})`)
            .call(d3.drag()
                .on('start', function(event, d) {
                    console.log(`[DRAG START] ${d.id} at (${d.x}, ${d.y})`);
                    // Raise dragged element to front
                    d3.select(this).raise();
                    d.isDragging = true;
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

                    // Schedule update on next animation frame for smooth 60fps
                    dragAnimationFrame = requestAnimationFrame(() => {
                        // Update visual transform
                        d3.select(element).attr('transform', `translate(${d.x},${d.y})`);

                        // Update connected links
                        self.updateLinks();
                    });
                })
                .on('end', function(event, d) {
                    console.log(`[DRAG END] ${d.id} at (${d.x}, ${d.y})`);
                    d.isDragging = false;
                    d.hasMoved = true;  // Mark node as moved to invalidate ELK routing

                    // Cancel any pending animation frame
                    if (dragAnimationFrame) {
                        cancelAnimationFrame(dragAnimationFrame);
                        dragAnimationFrame = null;
                    }

                    // Final update to ensure links are correctly positioned
                    self.updateLinks();
                }));

        // Shapes
        this.nodeElements.filter(d => d.type === 'initial-pseudo')
            .append('circle')
            .attr('r', 10)
            .attr('class', 'initial-pseudo-circle');

        this.nodeElements.filter(d => d.type === 'atomic' || d.type === 'final')
            .append('rect')
            .attr('width', 60)
            .attr('height', 40)
            .attr('x', -30)
            .attr('y', -20)
            .attr('rx', 5)
            .style('cursor', 'pointer')
            .on('click', (event, d) => {
                event.stopPropagation();
                
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
            .attr('x', d => d.x - d.width/2)
            .attr('y', d => d.y - d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            .attr('rx', 5)
            .style('cursor', 'pointer')
            .on('click', (event, d) => {
                event.stopPropagation();
                this.toggleCompoundState(d.id);
            });

        // Labels
        this.nodeElements.filter(d => d.type !== 'initial-pseudo' && d.type !== 'history')
            .append('text')
            .attr('dy', 5)
            .text(d => d.id);

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

        this.zoomContainer.append('g')
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

            // **FIXED: Always calculate directions, not just for moved nodes**
            // This ensures Pass 1 works on initial render
            if (sourceNode && targetNode) {
                // Calculate actual directions that will be used
                link.confirmedDirections = this.calculateLinkDirections(sourceNode, targetNode, link);
            } else {
                // Missing source or target - skip this link
                link.confirmedDirections = null;
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
                    this.showTransitionInfo(d);
                }
            });
        
        // Transition labels (event, condition, actions)
        this.transitionLabels = linkGroups
            .filter(d => d.linkType === 'transition' && d.event) // Only show labels for transitions with events
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

        console.log(`Rendered ${visibleNodes.length} nodes, ${visibleLinks.length} links`);

        // Debug: Check actual DOM elements created
        console.log('DOM elements check:');
        console.log('  Link paths:', this.linkElements.size());
        console.log('  Node groups:', this.nodeElements ? this.nodeElements.size() : 0);
        console.log('  Collapsed compounds:', this.collapsedElements ? this.collapsedElements.size() : 0);
        console.log('  Compound containers:', this.compoundContainers ? this.compoundContainers.size() : 0);
    }

    /**
     * Analyze link connections for each node to enable smart snapping
     */
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
            label += ` [${transition.cond}]`;
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
     * Get transition label position (midpoint of path)
     */
    getTransitionLabelPosition(transition) {
        const sourceNode = this.nodes.find(n => n.id === transition.source);
        const targetNode = this.nodes.find(n => n.id === transition.target);
        
        if (!sourceNode || !targetNode) {
            return { x: 0, y: 0 };
        }
        
        // Calculate midpoint between source and target
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;
        
        // For Z-paths, use the horizontal segment's midpoint
        if (transition.confirmedDirections && transition.confirmedDirections.midY !== null) {
            const midY = transition.confirmedDirections.midY;
            return {
                x: (sx + tx) / 2,
                y: midY - 5  // Slightly above the path
            };
        }
        
        // For direct lines, use simple midpoint
        return {
            x: (sx + tx) / 2,
            y: (sy + ty) / 2 - 5  // Slightly above the path
        };
    }
    
    // **REMOVED: analyzeLinkConnections() is obsolete**
    // Two-pass algorithm uses link.confirmedDirections instead of prediction-based edge analysis

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
            // Rectangle 60x40 with smart snapping
            const halfWidth = 30;
            const halfHeight = 20;

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
            const halfWidth = 30;
            const halfHeight = 20;

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
            return {
                left: cx - 30,
                right: cx + 30,
                top: cy - 20,
                bottom: cy + 20
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
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;

        const dx = Math.abs(tx - sx);
        const dy = Math.abs(ty - sy);
        const alignmentThreshold = 30;

        // Direct alignment cases (no midY needed)
        if (dx < 1 || dy < 1) {
            if (dx < 1) {
                return {
                    outgoingDir: ty > sy ? 'to-bottom' : 'to-top',
                    incomingDir: ty > sy ? 'from-top' : 'from-bottom',
                    midY: null  // Direct vertical line
                };
            } else {
                return {
                    outgoingDir: tx > sx ? 'to-right' : 'to-left',
                    incomingDir: tx > sx ? 'from-left' : 'from-right',
                    midY: null  // Direct horizontal line
                };
            }
        }

        // Horizontal alignment
        if (dy < alignmentThreshold) {
            return {
                outgoingDir: tx > sx ? 'to-right' : 'to-left',
                incomingDir: tx > sx ? 'from-left' : 'from-right',
                midY: null  // Direct horizontal line
            };
        }

        // Vertical alignment
        if (dx < alignmentThreshold) {
            return {
                outgoingDir: ty > sy ? 'to-bottom' : 'to-top',
                incomingDir: ty > sy ? 'from-top' : 'from-bottom',
                midY: null  // Direct vertical line
            };
        }

        // Z-path with collision avoidance
        const obstacles = this.getObstacleNodes(sourceNode, targetNode);
        let midY = this.findCollisionFreeY(sx, sy, tx, ty, obstacles);

        const targetBounds = this.getNodeBounds(targetNode);
        const sourceBounds = this.getNodeBounds(sourceNode);
        const margin = 15;

        const targetCollision = this.horizontalLineIntersectsNode(midY, sx, tx, targetNode);
        const sourceCollision = this.horizontalLineIntersectsNode(midY, sx, tx, sourceNode);

        if (targetCollision || sourceCollision) {
            const candidates = [
                targetBounds.top - margin,
                targetBounds.bottom + margin,
                sourceBounds.top - margin,
                sourceBounds.bottom + margin
            ];

            let foundSafe = false;
            for (const candidate of candidates) {
                const avoidsTarget = !this.horizontalLineIntersectsNode(candidate, sx, tx, targetNode);
                const avoidsSource = !this.horizontalLineIntersectsNode(candidate, sx, tx, sourceNode);

                if (avoidsTarget && avoidsSource) {
                    midY = candidate;
                    foundSafe = true;
                    break;
                }
            }

            if (!foundSafe) {
                const distToTargetTop = Math.abs(targetBounds.top - margin - sy);
                const distToTargetBottom = Math.abs(targetBounds.bottom + margin - sy);
                const distToSourceTop = Math.abs(sourceBounds.top - margin - ty);
                const distToSourceBottom = Math.abs(sourceBounds.bottom + margin - ty);

                const maxDist = Math.max(distToTargetTop, distToTargetBottom, distToSourceTop, distToSourceBottom);

                if (maxDist === distToTargetTop) {
                    midY = targetBounds.top - margin;
                } else if (maxDist === distToTargetBottom) {
                    midY = targetBounds.bottom + margin;
                } else if (maxDist === distToSourceTop) {
                    midY = sourceBounds.top - margin;
                } else {
                    midY = sourceBounds.bottom + margin;
                }
            }
        }

        // Return the actual directions that will be used
        return {
            outgoingDir: midY > sy ? 'to-bottom' : 'to-top',
            incomingDir: ty > midY ? 'from-top' : 'from-bottom',
            midY: midY  // Store for debugging
        };
    }

    /**
     * Create ORTHOGONAL path with comprehensive collision avoidance
     */
    createOrthogonalPath(sourceNode, targetNode, link, connections) {
        // **TWO-PASS ALGORITHM: Use confirmed directions from Pass 1 if available**
        if (link.confirmedDirections) {
            if (this.debugMode) {
                console.log(`[PASS 2] Using confirmed directions for ${link.source}→${link.target}:`, link.confirmedDirections);
            }

            const outgoingDir = link.confirmedDirections.outgoingDir;
            const incomingDir = link.confirmedDirections.incomingDir;
            const midY = link.confirmedDirections.midY;

            // Get boundary points using confirmed directions
            const start = this.getOrthogonalBoundaryPoint(sourceNode, outgoingDir, link, true, connections);
            const end = this.getOrthogonalBoundaryPoint(targetNode, incomingDir, link, false, connections);

            // Create path based on confirmed directions
            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            // Check if it's a direct line or Z-path
            if (midY === null) {
                // Direct line (horizontal or vertical alignment)
                return `M ${sx} ${sy} L ${tx} ${ty}`;
            } else {
                // Z-shaped path with confirmed midY
                return `M ${sx} ${sy} L ${sx} ${midY} L ${tx} ${midY} L ${tx} ${ty}`;
            }
        }

        // **FALLBACK: If no confirmed directions, calculate on-the-fly (ELK routing)**
        // Get initial rough positions
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;

        const dx = Math.abs(tx - sx);
        const dy = Math.abs(ty - sy);

        // Threshold for considering nodes "horizontally aligned" or "vertically aligned"
        const alignmentThreshold = 30;

        // If points are already aligned (same x or same y), use direct path
        if (dx < 1 || dy < 1) {
            // Determine directions for direct line
            let outgoingDir, incomingDir;

            if (dx < 1) {
                // Vertical line
                outgoingDir = ty > sy ? 'to-bottom' : 'to-top';
                incomingDir = ty > sy ? 'from-top' : 'from-bottom';
            } else {
                // Horizontal line
                outgoingDir = tx > sx ? 'to-right' : 'to-left';
                incomingDir = tx > sx ? 'from-left' : 'from-right';
            }

            const start = this.getOrthogonalBoundaryPoint(sourceNode, outgoingDir, link, true, connections);
            const end = this.getOrthogonalBoundaryPoint(targetNode, incomingDir, link, false, connections);

            return `M ${start.x} ${start.y} L ${end.x} ${end.y}`;
        }

        // SMART ROUTING: If nodes are nearly horizontally aligned, use side connections
        if (dy < alignmentThreshold) {
            console.log(`[PATH] Nodes are horizontally aligned (dy=${dy}), using side connection`);

            const outgoingDir = tx > sx ? 'to-right' : 'to-left';
            const incomingDir = tx > sx ? 'from-left' : 'from-right';

            const start = this.getOrthogonalBoundaryPoint(sourceNode, outgoingDir, link, true, connections);
            const end = this.getOrthogonalBoundaryPoint(targetNode, incomingDir, link, false, connections);

            return `M ${start.x} ${start.y} L ${end.x} ${end.y}`;
        }

        // SMART ROUTING: If nodes are nearly vertically aligned, use top/bottom connections
        if (dx < alignmentThreshold) {
            console.log(`[PATH] Nodes are vertically aligned (dx=${dx}), using top/bottom connection`);

            const outgoingDir = ty > sy ? 'to-bottom' : 'to-top';
            const incomingDir = ty > sy ? 'from-top' : 'from-bottom';

            const start = this.getOrthogonalBoundaryPoint(sourceNode, outgoingDir, link, true, connections);
            const end = this.getOrthogonalBoundaryPoint(targetNode, incomingDir, link, false, connections);

            return `M ${start.x} ${start.y} L ${end.x} ${end.y}`;
        }

        // Get all obstacle nodes (excluding source and target)
        const obstacles = this.getObstacleNodes(sourceNode, targetNode);

        // Find collision-free Y coordinate for horizontal segment
        let midY = this.findCollisionFreeY(sx, sy, tx, ty, obstacles);

        const targetBounds = this.getNodeBounds(targetNode);
        const sourceBounds = this.getNodeBounds(sourceNode);
        const margin = 15;

        // CRITICAL: Check if horizontal segment passes through TARGET or SOURCE
        // Try to find a Y that avoids BOTH in one step
        const targetCollision = this.horizontalLineIntersectsNode(midY, sx, tx, targetNode);
        const sourceCollision = this.horizontalLineIntersectsNode(midY, sx, tx, sourceNode);

        if (targetCollision || sourceCollision) {
            console.log(`[PATH] Horizontal segment at y=${midY} intersects ${targetCollision ? 'target' : ''} ${sourceCollision ? 'source' : ''}`);
            
            // Calculate candidate positions that avoid BOTH
            const candidates = [
                targetBounds.top - margin,      // Above target
                targetBounds.bottom + margin,   // Below target
                sourceBounds.top - margin,      // Above source
                sourceBounds.bottom + margin    // Below source
            ];
            
            // Find first candidate that avoids both source and target
            let foundSafe = false;
            for (const candidate of candidates) {
                const avoidsTarget = !this.horizontalLineIntersectsNode(candidate, sx, tx, targetNode);
                const avoidsSource = !this.horizontalLineIntersectsNode(candidate, sx, tx, sourceNode);
                
                if (avoidsTarget && avoidsSource) {
                    midY = candidate;
                    console.log(`[PATH] Found safe Y=${midY} (avoids both)`);
                    foundSafe = true;
                    break;
                }
            }
            
            // Fallback: choose the one that's farthest from both
            if (!foundSafe) {
                const distToTargetTop = Math.abs(targetBounds.top - margin - sy);
                const distToTargetBottom = Math.abs(targetBounds.bottom + margin - sy);
                const distToSourceTop = Math.abs(sourceBounds.top - margin - ty);
                const distToSourceBottom = Math.abs(sourceBounds.bottom + margin - ty);
                
                const maxDist = Math.max(distToTargetTop, distToTargetBottom, distToSourceTop, distToSourceBottom);
                
                if (maxDist === distToTargetTop) {
                    midY = targetBounds.top - margin;
                } else if (maxDist === distToTargetBottom) {
                    midY = targetBounds.bottom + margin;
                } else if (maxDist === distToSourceTop) {
                    midY = sourceBounds.top - margin;
                } else {
                    midY = sourceBounds.bottom + margin;
                }
                
                console.log(`[PATH] Fallback: Using Y=${midY} (farthest option)`);
            }
        }

        // Determine directions based on adjusted path
        let outgoingDir = midY > sy ? 'to-bottom' : 'to-top';
        let incomingDir = ty > midY ? 'from-top' : 'from-bottom';

        // Get boundary points based on actual segment directions
        let start = this.getOrthogonalBoundaryPoint(sourceNode, outgoingDir, link, true, connections);
        let end = this.getOrthogonalBoundaryPoint(targetNode, incomingDir, link, false, connections);

        // Additional check: verify vertical segments don't pass through nodes
        // Note: We only adjust if absolutely necessary, since horizontal check should have handled most cases
        const finalVerticalCollision = this.verticalLineIntersectsNode(end.x, midY, end.y, targetNode);
        const firstVerticalCollision = this.verticalLineIntersectsNode(start.x, start.y, midY, sourceNode);

        if (finalVerticalCollision || firstVerticalCollision) {
            console.log(`[PATH] Vertical segment collision detected (final:${finalVerticalCollision}, first:${firstVerticalCollision})`);
            console.log(`[PATH] This usually means source/target are vertically overlapping - path may not be perfect`);
            // Don't adjust - would cause infinite loop. Accept imperfect path.
        }

        // Create Z-shaped path that avoids all rectangles
        return `M ${start.x} ${start.y} L ${start.x} ${midY} L ${end.x} ${midY} L ${end.x} ${end.y}`;
    }

    /**
     * Get link path using ELK routing information
     */
    getLinkPath(link) {
        // Get source and target nodes
        const sourceNode = this.nodes.find(n => n.id === link.source);
        const targetNode = this.nodes.find(n => n.id === link.target);

        if (!sourceNode || !targetNode) {
            return 'M 0 0';
        }

        // **TWO-PASS: No need for analyzeLinkConnections(), optimizer uses link.confirmedDirections**
        const connections = null;

        // If either node has been moved (dragged), invalidate ELK routing
        // and always use current node positions with ORTHOGONAL routing
        if (sourceNode.hasMoved || targetNode.hasMoved ||
            sourceNode.isDragging || targetNode.isDragging) {
            // Create ORTHOGONAL path with direction-aware boundary snapping
            return this.createOrthogonalPath(sourceNode, targetNode, link, connections);
        }

        // Use ELK edge routing only if nodes haven't been moved
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

        // Fallback to simple straight line using current node positions with boundary calculation
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;

        const sourcePoint = this.getNodeBoundaryPoint(sourceNode, tx, ty, link, true, connections);
        const targetPoint = this.getNodeBoundaryPoint(targetNode, sx, sy, link, false, connections);

        return `M ${sourcePoint.x} ${sourcePoint.y} L ${targetPoint.x} ${targetPoint.y}`;
    }

    /**
     * Update link paths after drag
     */
    updateLinks() {
        if (!this.linkElements || !this.allLinks) {
            return;
        }

        // **TWO-PASS ALGORITHM: Recalculate directions for moved nodes (Pass 1)**
        const visibleLinks = this.getVisibleLinks(this.allLinks, this.nodes);
        
        if (this.debugMode) console.log('[TWO-PASS DRAG] Pass 1: Recalculating directions after drag...');
            visibleLinks.forEach(link => {
                const sourceNode = this.nodes.find(n => n.id === link.source);
                const targetNode = this.nodes.find(n => n.id === link.target);

                if (sourceNode && targetNode &&
                    (sourceNode.hasMoved || targetNode.hasMoved ||
                     sourceNode.isDragging || targetNode.isDragging)) {
                    // Recalculate actual directions for moved nodes
                    link.confirmedDirections = this.calculateLinkDirections(sourceNode, targetNode, link);
                }
            });

            // Pass 2: Render with updated directions
            this.linkElements.attr('d', d => this.getLinkPath(d));
            
            // Update transition labels if they exist
            if (this.transitionLabels) {
                this.transitionLabels
                    .attr('x', d => this.getTransitionLabelPosition(d).x)
                    .attr('y', d => this.getTransitionLabelPosition(d).y);
            }
            
            if (this.debugMode) console.log('[TWO-PASS DRAG] Pass 2 complete');
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
     * Animate transition
     */
    animateTransition(transition) {
        if (!transition || !transition.source || !transition.target) {
            return;
        }

        const linkId = `${transition.source}_${transition.target}`;
        const link = this.linkElements.filter(d => d.id === linkId);

        if (link.empty()) {
            console.warn(`Transition link not found: ${linkId}`);
            return;
        }

        link.classed('animating', true);

        setTimeout(() => {
            link.classed('animating', false);
        }, 2000);
    }

    /**
     * Show transition info
     */
    showTransitionInfo(transition) {
        const panel = document.getElementById('transition-info-panel');
        if (!panel) return;

        // Debug: Log transition path coordinates
        const sourceNode = this.nodes.find(n => n.id === transition.source);
        const targetNode = this.nodes.find(n => n.id === transition.target);
        
        if (sourceNode && targetNode) {
            console.log('=== TRANSITION CLICKED ===');
            console.log(`Source: ${transition.source} → Target: ${transition.target}`);
            console.log(`Source node: (${sourceNode.x}, ${sourceNode.y})`);
            console.log(`Target node: (${targetNode.x}, ${targetNode.y})`);
            
            // Get the actual path being drawn
            const pathD = this.getLinkPath(transition);
            console.log('Path data:', pathD);
            
            // Parse path to show individual segments
            const pathSegments = pathD.split(/(?=[ML])/);
            console.log('Path segments:');
            pathSegments.forEach((seg, i) => {
                console.log(`  ${i}: ${seg}`);
            });
            
            // Show target bounds
            const targetBounds = this.getNodeBounds(targetNode);
            console.log('Target bounds:', targetBounds);
            
            // Show source bounds
            const sourceBounds = this.getNodeBounds(sourceNode);
            console.log('Source bounds:', sourceBounds);
            
            console.log('======================');
        }

        let html = `
            <div class="transition-detail">
                <div class="transition-source-target">
                    <strong>${transition.source}</strong> → <strong>${transition.target}</strong>
                </div>
        `;

        if (transition.event) {
            html += `<div class="transition-event">Event: <code>${transition.event}</code></div>`;
        }

        if (transition.cond) {
            html += `<div class="transition-cond">Condition: <code>${transition.cond}</code></div>`;
        }

        if (transition.actions && transition.actions.length > 0) {
            html += `<div class="transition-actions"><strong>Actions:</strong><ul>`;
            transition.actions.forEach(action => {
                html += `<li>${action}</li>`;
            });
            html += `</ul></div>`;
        }

        if (transition.guards && transition.guards.length > 0) {
            html += `<div class="transition-guards"><strong>Guards:</strong><ul>`;
            transition.guards.forEach(guard => {
                html += `<li><code>${guard}</code></li>`;
            });
            html += `</ul></div>`;
        }

        html += `</div>`;
        panel.innerHTML = html;
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
