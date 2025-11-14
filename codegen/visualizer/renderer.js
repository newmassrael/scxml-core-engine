// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Renderer - Handles SVG rendering, snap points, and action texts
 */

class Renderer {
    constructor(visualizer) {
        this.visualizer = visualizer;
    }

    render() {
        console.log('[RENDER START] ========== Beginning render() ==========');
        // Store reference to visualizer instance for use in drag handlers
        const self = this.visualizer;

        // Time-based throttling for link updates during drag
        let lastLinkUpdateTime = 0;
        let lastCollapsedLinkUpdateTime = 0; // Throttling for collapsed node drag
        const LINK_UPDATE_INTERVAL = 50; // 50ms = 20fps max for link updates

        // Clear
        this.visualizer.zoomContainer.selectAll('*').remove();

        const visibleNodes = this.visualizer.getVisibleNodes();
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, this.visualizer.nodes);
        
        if (this.visualizer.debugMode) {
            console.log(`[RENDER] visibleNodes: ${visibleNodes.map(n => n.id).join(', ')}`);
        }

        // Compound containers (expanded)
        // Include compounds that have visible children, even if the compound itself is not in visibleNodes
        const compoundData = [];
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));
        
        this.visualizer.nodes.forEach(node => {
            if (this.visualizer.constructor.isCompoundOrParallel(node) && !node.collapsed) {
                // Check if this compound has any visible children
                const hasVisibleChildren = node.children && node.children.some(childId => visibleNodeIds.has(childId));
                
                // Include if compound is visible OR has visible children
                if (visibleNodeIds.has(node.id) || hasVisibleChildren) {
                    // Ensure compound has coordinates (from bounding box calculation)
                    if (node.x !== undefined && node.y !== undefined && node.width !== undefined && node.height !== undefined) {
                        compoundData.push(node);
                        if (this.visualizer.debugMode) {
                            console.log(`  Including compound ${node.id}: inVisibleNodes=${visibleNodeIds.has(node.id)}, hasVisibleChildren=${hasVisibleChildren}`);
                        }
                    } else {
                        console.warn(`  Compound ${node.id} has visible children but no coordinates!`);
                    }
                }
            }
        });
        
        if (this.visualizer.debugMode) {
            console.log(`[RENDER] compoundData: ${compoundData.map(n => `${n.id}(${n.type})`).join(', ')}`);
        }

        // D3 data join pattern: select existing containers, bind data, handle enter/update/exit
        const containerElements = this.visualizer.zoomContainer.selectAll('rect.compound-container')
            .data(compoundData, d => d.id);

        // Remove old containers (exit)
        containerElements.exit().remove();

        // Add new containers (enter)
        const containerEnter = containerElements.enter().append('rect')
            .attr('class', d => {
                let classes = 'compound-container';
                // Apply active class if this node is in activeStates
                if (self.activeStates && self.activeStates.has(d.id)) {
                    classes += ' active';
                }
                return classes;
            })
            .attr('x', d => d.x - d.width/2)
            .attr('y', d => d.y - d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            .attr('data-state-id', d => d.id)
            .style('cursor', 'pointer')
            .call(d3.drag()
                .on('start', function(event, d) {
                    if (self.debugMode) {
                        console.log(`[DRAG START COMPOUND] ${d.id} at (${d.x}, ${d.y})`);
                    }

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
                        if (self.debugMode) {
                            console.log(`[DRAG START COMPOUND] ${d.id} has topmost parent ${topmostParent.id}, will move entire hierarchy`);
                        }
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

                    // Keep snap points on top (above dragged node)
                    const snapGroup = self.zoomContainer.select('g.snap-points');
                    if (!snapGroup.empty()) {
                        snapGroup.raise();
                    }
                })

                .on('drag', function(event, d) {
                    // Store delta before updating
                    const dx = event.dx;
                    const dy = event.dy;

                    // If this compound has a parent, move the parent instead (entire hierarchy)
                    if (d.dragParent) {
                        if (self.debugMode) {
                            console.log(`[DRAG COMPOUND] ${d.id} has parent ${d.dragParent.id}, moving entire hierarchy`);
                        }
                        
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
                                    
                                    // Update collapsed child's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(function(d) { return d.id === node.id; })
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    
                                    // Update collapsed descendant's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(collapsed => collapsed.id === node.id)
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
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
                            
                            // Single pass: update data positions for ALL descendants
                            self.nodes.forEach(node => {
                                // Update direct children
                                if (childrenSet.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    
                                    // Update collapsed child's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(function(d) { return d.id === node.id; })
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    
                                    // Update collapsed descendant's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(collapsed => collapsed.id === node.id)
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
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
                    if (self.debugMode) {
                        console.log(`[DRAG END COMPOUND] ${d.id}`);
                    }
                    
                    // dragAnimationFrame removed (using time-based throttling instead)
                    
                    d.isDragging = false;
                    self.isDraggingAny = false;
                    
                    // Cleanup cached descendants
                    delete d._cachedDescendants;
                    
                    // Update bounds based on whether we moved parent or self
                    if (d.dragParent) {
                        if (self.debugMode) {
                            console.log(`[DRAG END COMPOUND] Updating parent ${d.dragParent.id} bounds to contain all children`);
                        }
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
                        if (self.debugMode) {
                            console.log(`[DRAG END COMPOUND] Updating ${d.id} bounds to contain all children`);
                        }
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
                        if (self.debugMode) {
                            console.log(`[DRAG END COMPOUND] Node moved ${dragDistance.toFixed(0)}px, starting progressive optimization...`);
                        }
                    } else {
                        if (self.debugMode) {
                            console.log(`[DRAG END COMPOUND] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), toggling state`);
                        }

                        // D3 drag prevents click events, so manually toggle on click
                        self.toggleCompoundState(d.id);
                        return; // Skip optimization for clicks
                    }

                    // Only optimize if node was actually dragged
                    if (!isDrag) {
                        return;
                    }

                    if (self.debugMode) {
                        console.log('[DRAG END COMPOUND] Starting progressive optimization...');
                    }

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
                                if (self.debugMode) {
                                    console.log(`[DRAG END COMPOUND] Background CSP complete, updating visualization...`);
                                }

                                self.allLinks.forEach(link => {
                                    const sourceNode = self.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = self.nodes.find(n => n.id === (link.visualTarget || link.target));
                                    if (sourceNode && targetNode) {
                                        self.calculateLinkDirections(sourceNode, targetNode, link);
                                    }
                                });

                                self.updateLinksOptimal();
                                if (self.debugMode) {
                                    console.log(`[DRAG END COMPOUND] CSP visualization update complete`);
                                }
                            } else {
                                if (self.debugMode) {
                                    console.log(`[DRAG END COMPOUND] Background CSP cancelled or failed, keeping greedy result`);
                                }
                            }

                            self.backgroundOptimization = null;
                        },
                        (iteration, totalIterations, score) => {
                            // Progressive update: called for each intermediate solution
                            if (self.debugMode) {
                                console.log(`[DRAG END COMPOUND] Intermediate update (${iteration}/${totalIterations}): score=${score.toFixed(1)}`);
                            }

                            // Recalculate link directions
                            self.allLinks.forEach(link => {
                                const sourceNode = self.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = self.nodes.find(n => n.id === (link.visualTarget || link.target));
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
                    this.visualizer.toggleCompoundState(d.id);

                    // Design System: Panel + Diagram interaction (matches panel click behavior)
                    if (window.executionController) {
                        window.executionController.highlightStateInPanel(d.id);
                        window.executionController.focusState(d.id);
                    }
                }
            });

        // Merge enter + update selections
        this.visualizer.compoundContainers = containerEnter.merge(containerElements);

        // Update existing containers (positions may have changed)
        this.visualizer.compoundContainers
            .attr('x', d => d.x - d.width/2)
            .attr('y', d => d.y - d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height);
        
        if (this.visualizer.debugMode) {
            console.log(`[RENDER] Rendered ${compoundData.length} compound containers`);
            compoundData.forEach(d => {
                    console.log(`  ${d.id}: x=${d.x}, y=${d.y}, width=${d.width}, height=${d.height}`);
            });
        }

        // Regular nodes
        const regularNodes = visibleNodes.filter(n =>
            n.type !== 'compound' && n.type !== 'parallel'
        );

        this.visualizer.nodeElements = this.visualizer.zoomContainer.append('g')
            .attr('class', 'nodes')
            .selectAll('g')
            .data(regularNodes)
            .enter().append('g')
            .attr('class', d => {
                let classes = 'node state';
                if (d.type === 'atomic') classes += ' state-atomic';
                else if (d.type === 'final') classes += ' state-final';
                else if (d.type === 'history') classes += ' state-history';
                else if (d.type === 'initial-pseudo') classes += ' state-initial-pseudo';
                
                // Apply active class if this node is in activeStates
                if (self.activeStates && self.activeStates.has(d.id)) {
                    classes += ' active';
                }
                
                return classes;
            })
            .attr('data-state-id', d => d.id)
            .attr('transform', d => `translate(${d.x},${d.y})`)
            .call(d3.drag()
                .on('start', function(event, d) {
                    if (self.debugMode) {
                        console.log(`[DRAG START] ${d.id} at (${d.x}, ${d.y})`);
                    }

                    // Store initial position for click vs drag detection
                    d.dragStartX = d.x;
                    d.dragStartY = d.y;

                    // Cancel any ongoing background optimization
                    if (self.backgroundOptimization) {
                        self.backgroundOptimization.cancel();
                        self.backgroundOptimization = null;
                        if (self.debugMode) {
                            console.log(`[DRAG START] Cancelled background CSP optimization`);
                        }
                    }

                    // Check if this node has compound/parallel ancestors
                    // Find the topmost ancestor to move the entire hierarchy
                    const topmostParent = self.findTopmostCompoundParent(d.id);
                    if (topmostParent) {
                        if (self.debugMode) {
                            console.log(`[DRAG START] ${d.id} has topmost parent ${topmostParent.id}, will move entire hierarchy`);
                        }
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

                    // Keep snap points on top (above dragged compound)
                    const snapGroup = self.zoomContainer.select('g.snap-points');
                    if (!snapGroup.empty()) {
                        snapGroup.raise();
                    }
                })
                .on('drag', function(event, d) {
                    // Store delta before any updates
                    const dx = event.dx;
                    const dy = event.dy;

                    // If node has a compound/parallel parent, move the parent instead
                    if (d.dragParent) {
                        if (self.debugMode) {
                            console.log(`[DRAG] Moving parent ${d.dragParent.id} instead of child ${d.id}`);
                        }
                        
                        // Update parent position
                        d.dragParent.x += dx;
                        d.dragParent.y += dy;
                        
                        // Update parent's DOM element if collapsed
                        if (d.dragParent.collapsed) {
                            self.zoomContainer.selectAll('g.collapsed-compound')
                                .filter(function(collapsed) { return collapsed.id === d.dragParent.id; })
                                .attr('transform', `translate(${d.dragParent.x}, ${d.dragParent.y})`);
                        }

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
                                    
                                    // Update collapsed child's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(function(d) { return d.id === node.id; })
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
                                }
                                // Update grandchildren and deeper (not direct children)
                                else if (allDescendantIds.has(node.id)) {
                                    node.x += dx;
                                    node.y += dy;
                                    
                                    // Update collapsed descendant's DOM elements
                                    if (node.collapsed && self.constructor.isCompoundOrParallel(node)) {
                                        self.zoomContainer.selectAll('g.collapsed-compound')
                                            .filter(collapsed => collapsed.id === node.id)
                                            .attr('transform', `translate(${node.x}, ${node.y})`);
                                    }
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
                    if (self.debugMode) {
                        console.log(`[DRAG END] ${d.id} at (${d.x}, ${d.y})`);
                    }
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
                        if (self.debugMode) {
                            console.log(`[DRAG END] Updating parent ${d.dragParent.id} bounds to contain children`);
                        }
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
                        if (self.debugMode) {
                            console.log(`[DRAG END] Node moved ${dragDistance.toFixed(0)}px, starting progressive optimization...`);
                        }
                    } else {
                        if (self.debugMode) {
                            console.log(`[DRAG END] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), skipping optimization`);
                        }
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
                                if (self.debugMode) {
                                    console.log(`[DRAG END] Background CSP complete, updating visualization...`);
                                }

                                // Calculate midY for new CSP routing
                                self.allLinks.forEach(link => {
                                    const sourceNode = self.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = self.nodes.find(n => n.id === (link.visualTarget || link.target));
                                    if (sourceNode && targetNode) {
                                        self.calculateLinkDirections(sourceNode, targetNode, link);
                                    }
                                });

                                // Update visualization with CSP-optimized paths
                                self.updateLinksOptimal();

                                if (self.debugMode) {
                                    console.log(`[DRAG END] CSP visualization update complete`);
                                }
                            } else {
                                if (self.debugMode) {
                                    console.log(`[DRAG END] Background CSP cancelled or failed, keeping greedy result`);
                                }
                            }

                            self.backgroundOptimization = null;
                        },
                        (iteration, totalIterations, score) => {
                            // Progressive update: called for each intermediate solution
                            if (self.debugMode) {
                                console.log(`[DRAG END] Intermediate update (${iteration}/${totalIterations}): score=${score.toFixed(1)}`);
                            }

                            // Recalculate link directions
                            self.allLinks.forEach(link => {
                                const sourceNode = self.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = self.nodes.find(n => n.id === (link.visualTarget || link.target));
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
                        const sourceNode = self.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = self.nodes.find(n => n.id === (link.visualTarget || link.target));
                        if (sourceNode && targetNode) {
                            self.calculateLinkDirections(sourceNode, targetNode, link);
                        }
                    });

                    // Immediate update to render greedy paths (fast feedback)
                    self.updateLinksOptimal();

                    if (self.debugMode) {
                        console.log(`[DRAG END] Immediate greedy rendering complete`);
                    }
                }));

        // Shapes
        this.visualizer.nodeElements.filter(d => d.type === 'initial-pseudo')
            .append('circle')
            .attr('r', 10)
            .attr('class', 'initial-pseudo-circle');

        this.visualizer.nodeElements.filter(d => d.type === 'atomic' || d.type === 'final')
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
                const bounds = this.visualizer.getNodeBounds(d);
                if (self.debugMode) {
                    console.log('=== STATE CLICKED ===');
                    console.log(`State: ${d.id} (type: ${d.type})`);
                    console.log(`Center: (${d.x}, ${d.y})`);
                    console.log(`Bounds:`, bounds);
                    console.log(`  Left: ${bounds.left}, Right: ${bounds.right}`);
                    console.log(`  Top: ${bounds.top}, Bottom: ${bounds.bottom}`);
                    console.log(`  Width: ${bounds.right - bounds.left}, Height: ${bounds.bottom - bounds.top}`);
                    console.log('====================');
                }

                // Design System: Panel + Diagram interaction (matches panel click behavior)
                if (window.executionController) {
                    window.executionController.highlightStateInPanel(d.id);
                    window.executionController.focusState(d.id);
                }
            });

        this.visualizer.nodeElements.filter(d => d.type === 'history')
            .append('circle')
            .attr('r', 20)
            .attr('class', 'history-circle');

// W3C SCXML 6.3: Invoke Badge for states with child SCXML
        this.visualizer.nodeElements.filter(d => d.hasInvoke && (d.type === 'atomic' || d.type === 'final'))
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
                
                if (self.debugMode) {
                    console.log(`Added invoke badge to state: ${d.id}`);
                }
            });

        // Labels - State ID with onentry/onexit actions (getBBox precision)
        this.visualizer.nodeElements.filter(d => d.type !== 'initial-pseudo' && d.type !== 'history')
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

        this.visualizer.nodeElements.filter(d => d.type === 'history')
            .append('text')
            .attr('dy', 5)
            .attr('class', 'history-label')
            .text('H');

this.visualizer.compoundLabels = this.visualizer.zoomContainer.append('g')
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
        if (this.visualizer.debugMode) console.log('[TWO-PASS] Pass 1: Calculating directions for all links...');
        visibleLinks.forEach(link => {
            const sourceNode = this.visualizer.nodes.find(n => n.id === (link.visualSource || link.source));
                                    const targetNode = this.visualizer.nodes.find(n => n.id === (link.visualTarget || link.target));

            // **Calculate midY for z-path collision avoidance**
            // This ensures routing.midY is set for all links
            if (sourceNode && targetNode) {
                this.visualizer.calculateLinkDirections(sourceNode, targetNode, link);
            }
        });
        if (this.visualizer.debugMode) console.log('[TWO-PASS] Pass 1 complete');

        // Links (drawn AFTER nodes so they appear on top)
        // Pass 2: Render with snap positions based on confirmed directions
        if (this.visualizer.debugMode) console.log('[TWO-PASS] Pass 2: Rendering links with snap positions...');
        
        const linkGroups = this.visualizer.zoomContainer.append('g')
            .attr('class', 'links')
            .selectAll('g')
            .data(visibleLinks)
            .enter().append('g')
            .attr('class', 'link-group');
        
        // Path elements
        this.visualizer.linkElements = linkGroups.append('path')
            .attr('class', d => {
                if (d.linkType === 'initial') return 'link-initial';
                return 'transition';
            })
            .attr('d', d => this.visualizer.getLinkPath(d))
            .style('marker-end', 'url(#arrowhead)')
            .style('cursor', d => d.linkType === 'transition' ? 'pointer' : 'default')
            .on('click', (event, d) => {
                if (d.linkType === 'transition') {
                    // Show transition animation on diagram (temporary)
                    this.visualizer.highlightTransition(d);
                    this.visualizer.focusOnTransition(d);

                    // Dispatch event for execution-controller to update detail panel
                    document.dispatchEvent(new CustomEvent('transition-click', { detail: d }));
                }
            });
        
        // Transition labels (event, condition, actions)
        this.visualizer.transitionLabels = linkGroups
            .filter(d => d.linkType === 'transition' && (d.event || d.cond)) // Show labels for transitions with events or guards
            .append('text')
            .attr('class', 'transition-label')
            .attr('x', d => this.visualizer.getTransitionLabelPosition(d).x)
            .attr('y', d => this.visualizer.getTransitionLabelPosition(d).y)
            .attr('text-anchor', 'middle')
            .attr('dominant-baseline', 'middle')
            .style('font-size', '11px')
            .style('font-family', 'monospace')
            .style('fill', '#0969da')
            .style('pointer-events', 'none')
            .text(d => this.visualizer.getTransitionLabelText(d));

        // Collapsed compound states (rendered AFTER links/labels for proper z-order)
        const collapsedCompounds = visibleNodes.filter(d =>
            (d.type === 'compound' || d.type === 'parallel') &&
            d.collapsed &&
            d.x !== undefined && d.y !== undefined &&
            d.width !== undefined && d.height !== undefined
        );

        const collapsedElements = this.visualizer.zoomContainer.selectAll('g.collapsed-compound')
            .data(collapsedCompounds, d => d.id);

        // Remove old collapsed compounds
        collapsedElements.exit().remove();

        // Add new collapsed compounds
        const collapsedEnter = collapsedElements.enter()
            .append('g')
            .attr('class', d => {
                let classes = 'collapsed-compound';
                // Apply active class if this node is in activeStates
                if (self.activeStates && self.activeStates.has(d.id)) {
                    classes += ' active';
                }
                return classes;
            })
            .attr('transform', d => `translate(${d.x}, ${d.y})`);

        // Collapsed compound rect (dashed border)
        collapsedEnter.append('rect')
            .attr('x', d => -d.width/2)
            .attr('y', d => -d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height)
            .attr('rx', 5)
            .attr('ry', 5)
            .attr('class', 'collapsed-state-rect')
            .style('cursor', 'pointer');

        // Collapsed compound label
        collapsedEnter.append('text')
            .attr('class', 'state-label')
            .attr('x', 0)
            .attr('y', 0)
            .attr('text-anchor', 'middle')
            .attr('dominant-baseline', 'middle')
            .style('font-size', '14px')
            .style('font-weight', 'bold')
            .style('pointer-events', 'none')
            .text(d => d.id);

        // Update existing collapsed compounds
        const collapsedMerge = collapsedEnter.merge(collapsedElements);

        collapsedMerge.attr('transform', d => `translate(${d.x}, ${d.y})`);

        collapsedMerge.select('rect')
            .attr('x', d => -d.width/2)
            .attr('y', d => -d.height/2)
            .attr('width', d => d.width)
            .attr('height', d => d.height);

        collapsedMerge.select('text')
            .text(d => d.id);

        // Drag behavior for collapsed compounds
        const collapsedDrag = d3.drag()
            .on('start', function(event, d) {
                if (self.debugMode) {
                    console.log(`[DRAG START COLLAPSED] ${d.id} at (${d.x}, ${d.y})`);
                }
                d.dragStartX = d.x;
                d.dragStartY = d.y;
                
                // Cache descendant IDs for efficient updates
                d._cachedDescendants = self.getAllDescendantIds(d.id);
                
                // Update isDragging on actual node in self.nodes array
                const node = self.nodes.find(n => n.id === d.id);
                if (node) {
                    node.isDragging = true;
                }
                
                d3.select(this).raise().classed('dragging', true);
                d.isDragging = true;
                self.isDraggingAny = true;

                // Keep snap points on top (above dragged collapsed compound)
                const snapGroup = self.zoomContainer.select('g.snap-points');
                if (!snapGroup.empty()) {
                    snapGroup.raise();
                }
            })
            .on('drag', function(event, d) {
                const dx = event.dx;
                const dy = event.dy;
                d.x += dx;
                d.y += dy;
                
                // Update children positions (hidden but data must be synced for link calculations)
                if (d.children) {
                    const allDescendantIds = new Set(d._cachedDescendants);
                    const childrenSet = new Set(d.children);
                    
                    self.nodes.forEach(node => {
                        if (childrenSet.has(node.id) || allDescendantIds.has(node.id)) {
                            node.x += dx;
                            node.y += dy;

                            // Update collapsed child's DOM elements
                            if (node.collapsed && (node.type === 'compound' || node.type === 'parallel')) {
                                self.zoomContainer.selectAll('g.collapsed-compound')
                                    .filter(function(d) { return d.id === node.id; })
                                    .attr('transform', `translate(${node.x}, ${node.y})`);
                            }
                        }
                    });
                }
                
                d3.select(this).attr('transform', `translate(${d.x}, ${d.y})`);

                // Throttle link updates to 20fps for smooth drag performance
                const now = performance.now();
                if (now - lastCollapsedLinkUpdateTime >= LINK_UPDATE_INTERVAL) {
                    lastCollapsedLinkUpdateTime = now;
                    self.updateLinksFast();
                }
            })
            .on('end', function(event, d) {
                // Update isDragging on actual node in self.nodes array
                const node = self.nodes.find(n => n.id === d.id);
                if (node) {
                    node.isDragging = false;
                }
                
                d3.select(this).classed('dragging', false);
                d.isDragging = false;
                self.isDraggingAny = false;
                
                // Cleanup cached descendants
                delete d._cachedDescendants;
                
                // Calculate drag distance to distinguish click vs drag
                const dragDistance = Math.hypot(d.x - d.dragStartX, d.y - d.dragStartY);
                const DRAG_THRESHOLD = 5;
                const isDrag = dragDistance > DRAG_THRESHOLD;
                
                if (self.debugMode) {
                    console.log(`[DRAG END COLLAPSED] ${d.id} at (${d.x}, ${d.y}), distance=${dragDistance.toFixed(0)}px`);
                }
                
                if (!isDrag) {
                    if (self.debugMode) {
                        console.log(`[DRAG END COLLAPSED] Click detected (${dragDistance.toFixed(0)}px < ${DRAG_THRESHOLD}px threshold), toggling state`);
                    }
                    self.toggleCompoundState(d.id);
                    return;
                }
                
                // Immediate greedy optimization
                self.updateLinksFast();
                if (self.debugMode) {
                    console.log(`[DRAG END COLLAPSED] Node moved ${dragDistance.toFixed(0)}px, starting optimization...`);
                }
            });

        collapsedMerge.call(collapsedDrag);

        this.visualizer.collapsedElements = collapsedMerge;

        // Snap point visualization (enabled with ?show-snap)
        if (this.visualizer.showSnapPoints) {
            this.visualizer.renderSnapPoints(visibleNodes);
        }

        if (this.visualizer.debugMode) {
            console.log(`Rendered ${visibleNodes.length} nodes, ${visibleLinks.length} links`);
            console.log('DOM elements check:');
            console.log('  Link paths:', this.visualizer.linkElements.size());
            console.log('  Node groups:', this.visualizer.nodeElements ? this.visualizer.nodeElements.size() : 0);
            console.log('  Collapsed compounds:', this.visualizer.collapsedElements ? this.visualizer.collapsedElements.size() : 0);
            console.log('  Compound containers:', this.visualizer.compoundContainers ? this.visualizer.compoundContainers.size() : 0);
        }

        // Render transition list
        this.visualizer.renderTransitionList();

        // Re-apply active state highlights after render
        if (this.visualizer.activeStates && this.visualizer.activeStates.size > 0) {
            this.visualizer.highlightActiveStatesVisual();
        }

        // Re-apply active transition after render (renderTransitionList recreates HTML)
        if (this.visualizer.activeTransition) {
            this.visualizer.setActiveTransition(this.visualizer.activeTransition);
        }

        console.log('[RENDER END] ========== Completed render() ==========');
    }

    generateSnapPointsData(visibleNodes, visibleLinks = null) {
        const snapPointsData = [];
        
        // Use visibleLinks if provided (for collapsed state support), otherwise use allLinks
        const links = visibleLinks || this.visualizer.allLinks;

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

                links.forEach(link => {
                    // **SKIP containment and delegation links** (they don't have routing)
                    // Visualizer layout: containment is hierarchical structure, not routing path
                    if (link.linkType === 'containment' || link.linkType === 'delegation') {
                        return;
                    }

                    // **Use routing for snap visualization**
                    // Only transition and initial links should have routing
                    if (!link.routing) {
                        // Skip links without routing (e.g., hidden links in collapsed states)
                        return;
                    }

                    // Use visual redirect for collapsed states
                    const visualSource = link.visualSource || link.source;
                    const visualTarget = link.visualTarget || link.target;

                    if (visualSource === node.id && link.routing.sourceEdge === edge) {
                        edgeSnapPoints.push({
                            point: link.routing.sourcePoint,
                            link: link
                        });
                    }

                    if (visualTarget === node.id && link.routing.targetEdge === edge) {
                        edgeSnapPoints.push({
                            point: link.routing.targetPoint,
                            link: link
                        });
                    }
                });

                const hasInitial = this.visualizer.layoutOptimizer.hasInitialTransitionOnEdge(node.id, edge);

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
                    if (this.visualizer.debugMode) {
                        console.log(`[SNAP INDEX] ${node.id} #${snapData.index}: ${edge} center at (${x.toFixed(1)}, ${y.toFixed(1)})`);
                    }
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
                        const visualSource = sp.link.visualSource || sp.link.source;
                        const visualTarget = sp.link.visualTarget || sp.link.target;
                        const linkName = visualSource === node.id ?
                            `${visualSource}→${visualTarget}` :
                            `${visualSource}→${visualTarget}`;
                        if (this.visualizer.debugMode) {
                            console.log(`[SNAP INDEX] ${node.id} #${snapData.index}: ${edge} (${linkName}) at (${sp.point.x.toFixed(1)}, ${sp.point.y.toFixed(1)})`);
                        }
                        snapPointsData.push(snapData);
                    });
                }
            });
        });

        return snapPointsData;
    }

    renderSnapPoints(visibleNodes, visibleLinks = null) {
        // Generate snap points data
        const snapPointsData = this.visualizer.generateSnapPointsData(visibleNodes, visibleLinks);

        // Remove old snap points before rendering new ones
        const oldSnapGroups = this.visualizer.zoomContainer.selectAll('g.snap-points');
        const removedCount = oldSnapGroups.size();
        oldSnapGroups.remove();
        if (this.visualizer.debugMode) {
            console.log(`[RENDER SNAP] Removed ${removedCount} old snap-points groups`);
        }

        // Render snap point circles
        const snapGroup = this.visualizer.zoomContainer.append('g').attr('class', 'snap-points');
        snapGroup.raise(); // Force snap points to top of z-order (above collapsed compounds)
        if (this.visualizer.debugMode) {
            console.log(`[RENDER SNAP] Creating snap group, data points: ${snapPointsData.length}`);
        }

        // Store references for later updates
        this.visualizer.snapPointCircles = snapGroup.selectAll('circle.snap-point')
            .data(snapPointsData)
            .enter()
            .append('circle')
            .attr('class', 'snap-point')
            .attr('cx', d => {
                // Debug mode: log snap circle coordinates
                if (this.visualizer.debugMode) {
                    if (self.debugMode) {
                        console.log(`[SNAP CIRCLE] ${d.nodeId} #${d.index}: cx=${d.x.toFixed(1)}, cy=${d.y.toFixed(1)}`);
                    }
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
        this.visualizer.snapPointLabels = snapGroup.selectAll('text.snap-index')
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

        if (this.visualizer.debugMode) {
            console.log(`Rendered ${snapPointsData.length} snap points`);
        }
    }

    updateSnapPointPositions() {
        if (!this.visualizer.showSnapPoints) {
            return;
        }

        // Generate latest snap points data
        const visibleNodes = this.visualizer.getVisibleNodes();
        const snapPointsData = this.visualizer.generateSnapPointsData(visibleNodes);

        // Get or create snap group
        let snapGroup = this.visualizer.zoomContainer.select('g.snap-points');
        if (snapGroup.empty()) {
            snapGroup = this.visualizer.zoomContainer.append('g').attr('class', 'snap-points');
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
        this.visualizer.snapPointCircles = snapGroup.selectAll('circle.snap-point');
        this.visualizer.snapPointLabels = snapGroup.selectAll('text.snap-index');
    }

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

    getActionTextLeftMargin(width) {
        return -width/2 + (width * LAYOUT_CONSTANTS.TEXT_LEFT_MARGIN_PERCENT);
    }

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
}
