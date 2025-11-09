// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * SCXML State Diagram Visualizer using D3.js
 *
 * Renders SCXML state machines as interactive force-directed graphs
 * with support for hierarchical states, parallel regions, and transitions.
 */

// Debug mode: enable with ?debug URL parameter (e.g., visualizer.html?debug#test=144)
const DEBUG_MODE = new URLSearchParams(window.location.search).has('debug');

class SCXMLVisualizer {
    constructor(containerId, scxmlStructure) {
        this.container = d3.select(`#${containerId}`);
        this.states = scxmlStructure.states || [];
        this.transitions = scxmlStructure.transitions || [];
        this.initialState = scxmlStructure.initial || '';
        this.activeStates = new Set();

        // Get container dimensions dynamically
        const containerNode = this.container.node();
        const clientWidth = containerNode ? containerNode.clientWidth : 0;
        const clientHeight = containerNode ? containerNode.clientHeight : 0;
        
        // Ensure minimum dimensions (container might not be rendered yet)
        this.width = clientWidth > 0 ? clientWidth : 800;
        this.height = clientHeight > 0 ? clientHeight : 500;
        
        console.log(`📐 Initial diagram size: ${this.width}x${this.height}`);

        if (DEBUG_MODE) {
            console.log('%c[DEBUG] SCXMLVisualizer initialized', 'color: #1a7f37; font-weight: bold');
            console.log('  States:', this.states.length, this.states.map(s => s.id));
            console.log('  Transitions:', this.transitions.length);
            console.log('  Initial state:', this.initialState);
        }

        this.initGraph();
        
        // Delay initial resize to ensure container is fully rendered
        setTimeout(() => {
            this.resize();
        }, 100);
    }

    /**
     * Initialize D3 force-directed graph
     */
    initGraph() {
        // Clear previous content
        this.container.selectAll('*').remove();

        // Create SVG with responsive viewBox
        this.svg = this.container
            .append('svg')
            .attr('viewBox', `0 0 ${this.width} ${this.height}`)
            .attr('preserveAspectRatio', 'xMidYMid meet');  // Center and scale to fit

        // Create zoom container (all content goes inside this)
        this.zoomContainer = this.svg.append('g').attr('class', 'zoom-container');
        
        // Add zoom/pan behavior
        this.zoom = d3.zoom()
            .scaleExtent([0.1, 4])
            .on('zoom', (event) => {
                this.zoomContainer.attr('transform', event.transform);
            });
        
        this.svg.call(this.zoom);
        
        // Store initial transform for reset
        this.initialTransform = d3.zoomIdentity;

        // Define arrowhead marker
        this.svg.append('defs').append('marker')
            .attr('id', 'arrowhead')
            .attr('viewBox', '0 -5 10 10')
            .attr('refX', 7)  // Arrow tip extends 3 units beyond path endpoint
            .attr('refY', 0)
            .attr('markerWidth', 6)
            .attr('markerHeight', 6)
            .attr('orient', 'auto')
            .append('path')
            .attr('d', 'M0,-5L10,0L0,5')
            .attr('fill', '#57606a');

        // Debug: Add circles to visualize path endpoints
        this.debugCircles = this.zoomContainer.append('g').attr('class', 'debug-circles');

        // Build node and link data
        const nodes = this.buildNodes();
        const links = this.buildLinks();

        // Create force simulation (adjusted for comfortable state spacing)
        this.simulation = d3.forceSimulation(nodes)
            .force('link', d3.forceLink(links).id(d => d.id).distance(d => {
                // Initial transition should be shorter, normal transitions well-spaced
                // Check both isInitial flag and source id (D3 might convert source to object)
                const sourceId = typeof d.source === 'object' ? d.source.id : d.source;
                const isInitial = d.isInitial || sourceId === '__initial__';
                const distance = isInitial ? 40 : 150;
                
                // Debug log for initial transition
                if (isInitial) {
                    console.log('🎯 Initial transition detected:', {
                        sourceId,
                        targetId: typeof d.target === 'object' ? d.target.id : d.target,
                        distance,
                        hasIsInitialFlag: d.isInitial
                    });
                }
                
                return distance;
            }))
            .force('charge', d3.forceManyBody().strength(-300))
            .force('center', d3.forceCenter(this.width / 2, this.height / 2))
            .force('collision', d3.forceCollide().radius(d => {
                // Initial pseudo-node is smaller, normal states need more spacing
                return d.type === 'initial-pseudo' ? 20 : 60;
            }))
            .alpha(1) // Start with maximum energy for better initial centering
            .alphaDecay(0.02); // Slower decay for smoother animation

        // Create link elements
        this.linkElements = this.zoomContainer.append('g')
            .attr('class', 'links')
            .selectAll('path')
            .data(links)
            .enter().append('path')
            .attr('class', 'transition')
            .attr('data-id', d => `${d.source.id}_${d.target.id}`);

        // Add event labels to transitions
        this.linkLabels = this.zoomContainer.append('g')
            .attr('class', 'link-labels')
            .selectAll('text')
            .data(links)
            .enter().append('text')
            .attr('class', 'transition-label')
            .text(d => d.event || '')
            .attr('text-anchor', 'middle')
            .style('font-size', '12px')
            .style('fill', '#57606a')
            .style('pointer-events', 'none');

        // Create node elements
        this.nodeElements = this.zoomContainer.append('g')
            .attr('class', 'nodes')
            .selectAll('g')
            .data(nodes)
            .enter().append('g')
            .attr('class', d => `state state-${d.type}`)
            .call(this.drag(this.simulation));

        // Add rectangles to regular nodes (not initial-pseudo)
        this.nodeElements.filter(d => d.type !== 'initial-pseudo')
            .append('rect')
            .attr('width', d => d.type === 'compound' || d.type === 'parallel' ? 90 : 60)
            .attr('height', 40)
            .attr('x', d => -(d.type === 'compound' || d.type === 'parallel' ? 45 : 30))
            .attr('y', -20);

        // Add circles to initial-pseudo nodes (W3C/UML standard)
        this.nodeElements.filter(d => d.type === 'initial-pseudo')
            .append('circle')
            .attr('r', 10)
            .attr('fill', '#1a7f37')
            .attr('class', 'initial-pseudo-circle');

        // Add text labels to regular nodes (not initial-pseudo)
        this.nodeElements.filter(d => d.type !== 'initial-pseudo')
            .append('text')
            .attr('dy', 5)
            .text(d => d.id);

        // Debug flag to log only once
        let debugLogged = false;

        // Update positions on simulation tick
        this.simulation.on('tick', () => {
            // Draw arc paths with arrow padding
            const self = this;

            if (DEBUG_MODE && !debugLogged && self.simulation.alpha() < 0.1) {
                console.log('%c[DEBUG] Simulation settled, logging path details...', 'color: #0969da; font-weight: bold');
            }

            this.linkElements.attr('d', function(d, i) {
                // Get source and target info
                const sourceId = typeof d.source === 'object' ? d.source.id : d.source;
                const targetId = typeof d.target === 'object' ? d.target.id : d.target;

                // Calculate direction vector
                const dx = d.target.x - d.source.x;
                const dy = d.target.y - d.source.y;
                const distance = Math.sqrt(dx * dx + dy * dy);

                if (distance === 0) return '';

                // Unit vector
                const ux = dx / distance;
                const uy = dy / distance;

                // Node sizes for padding
                const sourceNode = self.states.find(s => s.id === sourceId);
                const targetNode = self.states.find(s => s.id === targetId);

                // Calculate exact distance to rectangle boundary along direction vector
                const getRectangleBoundaryDistance = (width, height, ux, uy) => {
                    // Distance to hit vertical edges (left/right)
                    const t_x = (width / 2) / Math.abs(ux);
                    // Distance to hit horizontal edges (top/bottom)
                    const t_y = (height / 2) / Math.abs(uy);
                    // First edge hit
                    return Math.min(t_x, t_y);
                };

                const getNodeRadius = (node, id) => {
                    if (id === '__initial__') return 10;
                    if (!node) return 40;
                    const width = (node.type === 'compound' || node.type === 'parallel') ? 120 : 80;
                    const height = 50;
                    // Calculate exact intersection with rectangle boundary
                    return getRectangleBoundaryDistance(width, height, ux, uy);
                };

                const sourceRadius = getNodeRadius(sourceNode, sourceId);
                const targetRadius = getNodeRadius(targetNode, targetId);

                // Arrow marker offset (matches marker refX for exact alignment)
                const arrowOffset = 7;

                // Calculate padded start and end points
                const startX = d.source.x + ux * sourceRadius;
                const startY = d.source.y + uy * sourceRadius;
                const endX = d.target.x - ux * (targetRadius + arrowOffset);
                const endY = d.target.y - uy * (targetRadius + arrowOffset);

                // Calculate arc for padded points (larger radius = gentler curve)
                const pdx = endX - startX;
                const pdy = endY - startY;
                const dr = Math.sqrt(pdx * pdx + pdy * pdy) / 1.6;

                // Debug logging (only once after simulation settles)
                if (DEBUG_MODE && !debugLogged && self.simulation.alpha() < 0.1) {
                    console.log(`%c[Path ${i}] ${sourceId} → ${targetId}`, 'color: #8250df; font-weight: bold');
                    console.table({
                        'Source Node': { x: d.source.x.toFixed(2), y: d.source.y.toFixed(2), id: sourceId },
                        'Target Node': { x: d.target.x.toFixed(2), y: d.target.y.toFixed(2), id: targetId },
                        'Path Start': { x: startX.toFixed(2), y: startY.toFixed(2), note: 'After source padding' },
                        'Path End': { x: endX.toFixed(2), y: endY.toFixed(2), note: 'Before target padding' }
                    });
                    console.log('  Direction vector:', { dx: dx.toFixed(2), dy: dy.toFixed(2), distance: distance.toFixed(2) });
                    console.log('  Unit vector:', { ux: ux.toFixed(4), uy: uy.toFixed(4) });
                    console.log('  Padding:', { sourceRadius: sourceRadius.toFixed(2), targetRadius: targetRadius.toFixed(2), arrowOffset });
                    console.log('  Arc radius (curvature):', dr.toFixed(2));
                    console.log('  SVG Path:', `M${startX.toFixed(2)},${startY.toFixed(2)}A${dr.toFixed(2)},${dr.toFixed(2)} 0 0,1 ${endX.toFixed(2)},${endY.toFixed(2)}`);

                    if (i === self.linkElements.size() - 1) {
                        debugLogged = true;
                        console.log('%c[DEBUG] All paths logged', 'color: #1a7f37; font-weight: bold');

                        // Add visual debugging markers
                        self.addDebugMarkers();
                    }
                }

                return `M${startX},${startY}A${dr},${dr} 0 0,1 ${endX},${endY}`;
            });

            // Update link label positions using actual path midpoint
            this.linkLabels.each(function(d, i) {
                // Get corresponding path element by index
                const pathElement = self.linkElements.nodes()[i];
                if (pathElement) {
                    try {
                        const length = pathElement.getTotalLength();
                        const point = pathElement.getPointAtLength(length / 2);
                        d3.select(this)
                            .attr('x', point.x)
                            .attr('y', point.y);
                    } catch (e) {
                        // Fallback to straight line midpoint
                        d3.select(this)
                            .attr('x', (d.source.x + d.target.x) / 2)
                            .attr('y', (d.source.y + d.target.y) / 2);
                    }
                }
            });

            this.nodeElements.attr('transform', d => `translate(${d.x},${d.y})`);
        });

        // Add zoom and pan
        const zoom = d3.zoom()
            .scaleExtent([0.5, 3])
            .on('zoom', (event) => {
                // Apply transform to zoom container only
                this.zoomContainer.attr('transform', event.transform);
            });

        this.container.select('svg').call(zoom);
    }

    /**
     * Build node data from SCXML states
     */
    buildNodes() {
        const nodes = this.states.map(state => ({
            id: state.id,
            type: state.type || 'atomic',
            initial: state.initial,
            children: state.children || []
        }));

        // Add initial pseudo-node (W3C/UML statechart standard)
        if (this.initialState) {
            nodes.push({
                id: '__initial__',
                type: 'initial-pseudo'
                // Let force simulation position it naturally near initial state
            });
        }

        return nodes;
    }

    /**
     * Build link data from SCXML transitions
     */
    buildLinks() {
        const links = this.transitions.map(trans => ({
            source: trans.source,
            target: trans.target,
            event: trans.event || '',
            guard: trans.guard || '',
            id: `${trans.source}_${trans.target}`
        }));

        // Add initial transition (from pseudo-node to initial state)
        if (this.initialState) {
            links.unshift({
                source: '__initial__',
                target: this.initialState,
                event: '',
                guard: '',
                id: '__initial_transition__',
                isInitial: true  // Mark as initial transition for styling
            });
        }

        return links;
    }

    /**
     * Create curved arc for transition links with linknum-based curvature
     * Handles bidirectional transitions (e.g., A→B and B→A)
     * Uses D3.js multiple-links pattern with elliptical arc
     */
    linkArc(d) {
        const dx = d.target.x - d.source.x;
        const dy = d.target.y - d.source.y;

        // Check if this is a reverse transition (target < source alphabetically)
        const sourceId = typeof d.source === 'object' ? d.source.id : d.source;
        const targetId = typeof d.target === 'object' ? d.target.id : d.target;
        const isReverse = sourceId > targetId;

        // Assign linknum for bidirectional links
        // Forward direction (A→B where A < B): linknum = 1
        // Reverse direction (B→A where B > A): linknum = 2
        const linknum = isReverse ? 2 : 1;

        // Calculate arc radius inversely proportional to linknum
        // Smaller linknum = larger radius = gentler curve
        // Larger linknum = smaller radius = sharper curve
        const dr = Math.sqrt(dx * dx + dy * dy) / (linknum * 0.8);

        // Use elliptical arc with varying curvature
        return `M${d.source.x},${d.source.y}A${dr},${dr} 0 0,1 ${d.target.x},${d.target.y}`;
    }

    /**
     * Enable dragging of nodes
     */
    drag(simulation) {
        function dragstarted(event) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            event.subject.fx = event.subject.x;
            event.subject.fy = event.subject.y;
        }

        function dragged(event) {
            event.subject.fx = event.x;
            event.subject.fy = event.y;
        }

        function dragended(event) {
            if (!event.active) simulation.alphaTarget(0);
            event.subject.fx = null;
            event.subject.fy = null;
        }

        return d3.drag()
            .on('start', dragstarted)
            .on('drag', dragged)
            .on('end', dragended);
    }

    /**
     * Highlight active states
     * @param {Array<string>} activeStateIds - Array of active state IDs
     */
    highlightActiveStates(activeStateIds) {
        this.activeStates = new Set(activeStateIds);

        this.nodeElements.classed('active', d => this.activeStates.has(d.id));
    }

    /**
     * Animate transition execution
     * @param {Object} transition - Transition object {source, target, event, id}
     */
    animateTransition(transition) {
        if (!transition || !transition.source || !transition.target) {
            return;
        }

        // Find the link element
        const linkId = `${transition.source}_${transition.target}`;
        const link = this.linkElements.filter(d => d.id === linkId);

        if (link.empty()) {
            console.warn(`Transition link not found: ${linkId}`);
            return;
        }

        if (DEBUG_MODE) {
            console.log(`%c[ANIMATION] ${transition.source} → ${transition.target}`, 'color: #1a7f37; font-weight: bold');
        }

        // Add animating class
        link.classed('animating', true);

        // Create arrow animation
        const pathNode = link.node();
        const pathLength = pathNode.getTotalLength();

        if (DEBUG_MODE) {
            console.log('  Path length:', pathLength.toFixed(2));
            console.log('  Path d attribute:', pathNode.getAttribute('d'));
            const start = pathNode.getPointAtLength(0);
            const mid = pathNode.getPointAtLength(pathLength / 2);
            const end = pathNode.getPointAtLength(pathLength);
            console.log('  Animation route:');
            console.log('    Start:', { x: start.x.toFixed(2), y: start.y.toFixed(2) });
            console.log('    Mid:', { x: mid.x.toFixed(2), y: mid.y.toFixed(2) });
            console.log('    End:', { x: end.x.toFixed(2), y: end.y.toFixed(2) });
        }

        const arrow = this.zoomContainer.append('circle')
            .attr('r', 5)
            .attr('class', 'transition-arrow')
            .attr('fill', '#1a7f37');

        // Animate arrow along path
        arrow.transition()
            .duration(500)
            .attrTween('transform', () => {
                return (t) => {
                    const point = pathNode.getPointAtLength(t * pathLength);
                    return `translate(${point.x},${point.y})`;
                };
            })
            .on('end', () => {
                arrow.remove();
                link.classed('animating', false);
            });
    }

    /**
     * Reset visualization to initial state
     */
    reset() {
        this.activeStates.clear();
        this.nodeElements.classed('active', false);
        this.simulation.alpha(1).restart();
    }

    /**
     * Destroy visualization and clean up
     */
    destroy() {
        if (this.simulation) {
            this.simulation.stop();
        }
        this.container.selectAll('*').remove();
    }

    /**
     * Resize visualization to fit container
     * Call this when container size changes
     */
    resize() {
        const containerNode = this.container.node();
        if (!containerNode) return;

        const newWidth = containerNode.clientWidth;
        const newHeight = containerNode.clientHeight;

        // Allow resize even if dimensions are similar (for initial centering)
        const isInitialResize = this.width === 800 || this.width === 0;
        
        if (newWidth === this.width && newHeight === this.height && !isInitialResize) {
            return; // No change
        }

        this.width = newWidth > 0 ? newWidth : this.width;
        this.height = newHeight > 0 ? newHeight : this.height;

        // Update SVG viewBox
        this.svg.attr('viewBox', `0 0 ${this.width} ${this.height}`);

        // Reset zoom to center (important for initial render)
        if (this.zoom && this.svg) {
            const transform = d3.zoomIdentity
                .translate(0, 0)
                .scale(1);
            this.svg.call(this.zoom.transform, transform);
        }

        // Update force simulation center
        if (this.simulation) {
            this.simulation.force('center', d3.forceCenter(this.width / 2, this.height / 2));
            this.simulation.alpha(0.5).restart(); // Higher alpha for better centering
        }

        console.log(`📐 Resized diagram: ${this.width}x${this.height}`);
    }
}
