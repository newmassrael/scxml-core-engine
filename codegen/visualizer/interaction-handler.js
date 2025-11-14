// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Interaction Handler - Handles user interactions, highlighting, animations
 */

class InteractionHandler {
    constructor(visualizer) {
        this.visualizer = visualizer;
    }

    updateLinks(useGreedy = false) {
        if (this.visualizer.debugMode) {
            console.log(`[UPDATE LINKS] Called with useGreedy=${useGreedy}`);
        }
        if (!this.visualizer.linkElements || !this.visualizer.allLinks) {
            if (this.visualizer.debugMode) {
                console.log('[UPDATE LINKS] Early return: linkElements or allLinks missing');
            }
            return;
        }

        // **Get visibleLinks BEFORE optimization for filtering**
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, this.visualizer.nodes);

        // Check if any node is being dragged
        const anyNodeDragging = this.visualizer.nodes.some(n => n.isDragging);

        if (anyNodeDragging || useGreedy) {
            // const mode = useGreedy ? 'GREEDY (fast)' : 'CSP (optimal)';
            // console.log(`[DRAG UPDATE] Re-running optimizer (${mode})...`);

            // Clear all routing
            this.visualizer.allLinks.forEach(link => {
                delete link.routing;
            });

            // **ADAPTIVE ALGORITHM SELECTION**
            // - useGreedy=true: Fast greedy for real-time drag (1-5ms)
            // - useGreedy=false: Optimal CSP for final result (50-200ms)
            // Use visibleLinks for optimization (only visible nodes)
            this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(visibleLinks, this.visualizer.nodes, useGreedy);

            // Sync routing back to original allLinks
            visibleLinks.forEach(vlink => {
                if (vlink.routing && vlink.originalLink) {
                    vlink.originalLink.routing = vlink.routing;
                }
            });

            // Calculate midY for new routing
            visibleLinks.forEach(link => {
                const sourceNode = this.visualizer.nodes.find(n => n.id === (link.visualSource || link.source));
                const targetNode = this.visualizer.nodes.find(n => n.id === (link.visualTarget || link.target));
                if (sourceNode && targetNode) {
                    this.visualizer.calculateLinkDirections(sourceNode, targetNode, link);
                }
            });

            // console.log('[DRAG UPDATE] Re-optimization complete');
        }

        // Pass 2: Rebind linkElements with updated visibleLinks data
        // CRITICAL: linkElements was created by render() with old data
        // We need to rebind with new visibleLinks that have updated routing
        this.visualizer.linkElements = this.visualizer.linkElements
            .data(visibleLinks, d => d.id);

        // Pass 3: Render with updated directions
        if (this.visualizer.debugMode) {
            console.log(`[UPDATE LINKS] Updating ${this.visualizer.linkElements.size()} link paths`);
        }
        this.visualizer.linkElements.attr('d', d => this.visualizer.getLinkPath(d));

        // Update transition labels if they exist
        if (this.visualizer.transitionLabels) {
            // Rebind with updated visibleLinks data
            this.visualizer.transitionLabels = this.visualizer.transitionLabels
                .data(visibleLinks, d => d.id);
            
            this.visualizer.transitionLabels
                .attr('x', d => this.visualizer.getTransitionLabelPosition(d).x)
                .attr('y', d => this.visualizer.getTransitionLabelPosition(d).y);
        }

        // Update node visuals with latest positions from this.visualizer.nodes
        // Performance: Create Map for O(1) lookup instead of O(n) find()
        const nodeMap = new Map(this.visualizer.nodes.map(n => [n.id, n]));
        
        if (this.visualizer.nodeElements) {
            if (this.visualizer.debugMode) {
                const nodeCount = this.visualizer.nodeElements.size();
                console.log(`[UPDATE LINKS] Updating ${nodeCount} node DOM positions`);
            }
            this.visualizer.nodeElements.each(function(nodeData) {
                const latestNode = nodeMap.get(nodeData.id);
                if (latestNode) {
                    d3.select(this).attr('transform', `translate(${latestNode.x}, ${latestNode.y})`);
                }
            });
        } else {
            if (this.visualizer.debugMode) {
                console.log('[UPDATE LINKS] nodeElements not found!');
            }
        }

        // Update compound container visuals with latest positions
        if (this.visualizer.compoundContainers) {
            this.visualizer.compoundContainers.each(function(compoundData) {
                const latestNode = nodeMap.get(compoundData.id);
                if (latestNode) {
                    d3.select(this)
                        .attr('x', latestNode.x - latestNode.width/2)
                        .attr('y', latestNode.y - latestNode.height/2);
                }
            });
        }

        // Update compound label positions with latest positions
        if (this.visualizer.compoundLabels) {
            this.visualizer.compoundLabels.each(function(labelData) {
                const latestNode = nodeMap.get(labelData.id);
                if (latestNode) {
                    d3.select(this)
                        .attr('x', latestNode.x - latestNode.width/2 + 10)
                        .attr('y', latestNode.y - latestNode.height/2 + 20);
                }
            });
        }

        // Update snap points visualization if enabled
        if (this.visualizer.showSnapPoints) {
            // Fast update: only change positions, no DOM re-creation
            this.visualizer.updateSnapPointPositions();
        }
    }

    updateLinksFast() {
        this.visualizer.updateLinks(true); // useGreedy=true
    }

    updateLinksOptimal() {
        this.visualizer.updateLinks(false); // useGreedy=false
    }

    highlightActiveStates(activeStateIds) {
        console.log(`[highlightActiveStates] Called with:`, activeStateIds);
        this.visualizer.activeStates = new Set(activeStateIds);

        // Auto-expand compound/parallel states that are active or have active children
        let needsReLayout = false;
        this.visualizer.nodes.forEach(node => {
            if (this.visualizer.constructor.isCompoundOrParallel(node) && node.collapsed) {
                // Check if this node is active OR has any active children
                const isActive = this.visualizer.activeStates.has(node.id);
                const hasActiveChildren = node.children && node.children.some(childId => this.visualizer.activeStates.has(childId));

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
            this.visualizer.computeLayout().then(() => {
                this.visualizer.render();
                // Re-highlight after re-render
                this.visualizer.highlightActiveStatesVisual();
            });
            return;
        }

        this.visualizer.highlightActiveStatesVisual();
    }

    highlightActiveStatesVisual() {
        console.log(`[highlightActiveStatesVisual] Applying visual highlights`);
        console.log(`  nodeElements exists: ${this.visualizer.nodeElements ? 'yes' : 'no'}, size: ${this.visualizer.nodeElements ? this.visualizer.nodeElements.size() : 0}`);
        console.log(`  collapsedElements exists: ${this.visualizer.collapsedElements ? 'yes' : 'no'}, size: ${this.visualizer.collapsedElements ? this.visualizer.collapsedElements.size() : 0}`);
        console.log(`  compoundContainers exists: ${this.visualizer.compoundContainers ? 'yes' : 'no'}, size: ${this.visualizer.compoundContainers ? this.visualizer.compoundContainers.size() : 0}`);

        if (this.visualizer.nodeElements) {
            this.visualizer.nodeElements.classed('active', d => {
                const isActive = this.visualizer.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating node: ${d.id} (type: ${d.type})`);
                }
                return isActive;
            });
        }

        if (this.visualizer.collapsedElements) {
            this.visualizer.collapsedElements.classed('active', d => {
                const isActive = this.visualizer.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating collapsed: ${d.id}`);
                }
                return isActive;
            });
        }

        if (this.visualizer.compoundContainers) {
            this.visualizer.compoundContainers.classed('active', d => {
                const isActive = this.visualizer.activeStates.has(d.id);
                if (isActive) {
                    console.log(`  → Activating compound container: ${d.id}`);
                }
                return isActive;
            });
        }
    }

    animateTransition(transition) {
        // No-op: CSS handles animation via .highlighted class
        // See visualizer.css: .transition.highlighted { animation: transitionPulse ... }
        console.log('[DEPRECATED] animateTransition() called - CSS handles animation now');
    }

    renderTransitionList() {
        const panel = document.getElementById('transition-list-panel');
        if (!panel) return;

        if (!this.visualizer.transitions || this.visualizer.transitions.length === 0) {
            panel.innerHTML = '<div class="transition-hint">No transitions</div>';
            return;
        }

        let html = '<div class="transition-list">';
        html += '<div class="transition-list-header">All Transitions</div>';

        this.visualizer.transitions.forEach((transition, index) => {
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
                const transition = self.visualizer.transitions[index];

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

    setActiveTransition(transition) {
        console.log('[SET ACTIVE TRANSITION] Setting active transition:', transition);

        // Store active transition for re-application after render
        this.visualizer.activeTransition = transition;

        const panel = document.getElementById('transition-list-panel');
        const transitionId = transition ? `${transition.source}-${transition.target}` : null;

        // Clear all previous active states in panel
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
        }

        // Clear all previous active states in diagram (SVG)
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('active', false);
        }
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.classed('active', false);
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
            if (this.visualizer.linkElements) {
                this.visualizer.linkElements.each(function(d) {
                    const linkId = `${d.source}-${d.target}`;
                    if (linkId === transitionId) {
                        d3.select(this).classed('active', true);
                        console.log('[SET ACTIVE TRANSITION] Diagram active state set on:', linkId);
                    }
                });
            }

            // Diagram label: Set active state
            if (this.visualizer.transitionLabels) {
                this.visualizer.transitionLabels.each(function(d) {
                    const linkId = `${d.source}-${d.target}`;
                    if (linkId === transitionId) {
                        d3.select(this).classed('active', true);
                        console.log('[SET ACTIVE TRANSITION] Label active state set on:', linkId);
                    }
                });
            }
        }
    }

    clearTransitionHighlights() {
        console.log('[CLEAR HIGHLIGHT] Clearing transition highlights (SVG + panel)');

        // Cancel pending SVG highlight timeout (immediate cancellation on step backward)
        if (this.visualizer.transitionHighlightTimeout) {
            clearTimeout(this.visualizer.transitionHighlightTimeout);
            this.visualizer.transitionHighlightTimeout = null;
            console.log('[CLEAR HIGHLIGHT] Cancelled pending SVG highlight timeout');
        }

        // Cancel pending panel highlight timeout (immediate cancellation on step backward)
        if (this.visualizer.transitionPanelHighlightTimeout) {
            clearTimeout(this.visualizer.transitionPanelHighlightTimeout);
            this.visualizer.transitionPanelHighlightTimeout = null;
            console.log('[CLEAR HIGHLIGHT] Cancelled pending panel highlight timeout');
        }

        // Clear SVG diagram highlights
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('highlighted', false);
        }

        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.classed('highlighted', false);
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

    clearActiveTransition() {
        console.log('[CLEAR ACTIVE] Clearing active transition state');

        // Clear stored active transition
        this.visualizer.activeTransition = null;

        // Clear panel active state
        const panel = document.getElementById('transition-list-panel');
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
            console.log('[CLEAR ACTIVE] Panel active transition cleared');
        }

        // Clear diagram active state (SVG - permanent .active class)
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('active', false);
            console.log('[CLEAR ACTIVE] Diagram active transition cleared');
        }
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.classed('active', false);
        }

        // Clear diagram highlights (SVG - temporary .highlighted class)
        this.visualizer.clearTransitionHighlights();

        // Clear detail panel
        const detailPanel = document.getElementById('transition-detail-panel');
        if (detailPanel) {
            detailPanel.innerHTML = '<div class="transition-hint">Click a transition to view details</div>';
        }
    }

    highlightTransition(transition) {
        console.log('[HIGHLIGHT] highlightTransition() called with:', transition);

        if (!this.visualizer.linkElements) {
            console.log('[HIGHLIGHT] No linkElements - aborting');
            return;
        }

        this.visualizer.cancelPendingHighlights();
        const transitionId = `${transition.source}-${transition.target}`;

        this.visualizer.clearTransitionHighlights();
        this.visualizer.highlightLink(transitionId);
        this.visualizer.highlightLabel(transitionId);
        this.visualizer.scheduleHighlightRemoval();

        console.log('[HIGHLIGHT] highlightTransition() complete');
    }

    cancelPendingHighlights() {
        if (this.visualizer.transitionHighlightTimeout) {
            clearTimeout(this.visualizer.transitionHighlightTimeout);
            this.visualizer.transitionHighlightTimeout = null;
            console.log('[HIGHLIGHT] Cancelled previous highlight timeout');
        }
    }

    highlightLink(transitionId) {
        console.log(`[HIGHLIGHT] Looking for transition ID: ${transitionId}`);
        console.log(`[HIGHLIGHT] Available linkElements count: ${this.visualizer.linkElements.size()}`);

        let foundLink = false;
        this.visualizer.linkElements.each(function(d) {
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
            if (this.visualizer.debugMode) {
                console.log('[HIGHLIGHT] All available transitions:');
                this.visualizer.linkElements.each(function(d) {
                    console.log(`  - ${d.source} → ${d.target} (type: ${d.linkType}, id: ${d.id})`);
                });
            }
        }
    }

    highlightLabel(transitionId) {
        if (!this.visualizer.transitionLabels) {
            return;
        }

        console.log(`[HIGHLIGHT] Available transitionLabels: ${this.visualizer.transitionLabels.size()}`);

        let foundLabel = false;
        this.visualizer.transitionLabels.each(function(d) {
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

    scheduleHighlightRemoval() {
        const self = this;
        // Auto-remove highlight after 2 seconds (matches FOCUS_HIGHLIGHT_DURATION in execution-controller.js)
        // Store timeout ID for cancellation (immediate response on step backward)
        this.visualizer.transitionHighlightTimeout = setTimeout(() => {
            self.clearTransitionHighlights();
            self.transitionHighlightTimeout = null;
            console.log('[HIGHLIGHT] Auto-removed temporary highlight after 2s');
        }, 2000);
    }

    focusOnTransition(transition) {
        const sourceNode = this.visualizer.nodes.find(n => n.id === transition.source);
        const targetNode = this.visualizer.nodes.find(n => n.id === transition.target);

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
                .translate(this.visualizer.width / 2, this.visualizer.height / 2)
                .scale(1.0)
                .translate(-centerX, -centerY);

            this.visualizer.svg.transition()
                .duration(750)
                .call(this.visualizer.zoom.transform, transform);
            return;
        }

        // Calculate zoom level to fit both nodes
        const padding = 100;
        const zoomLevel = Math.min(
            this.visualizer.width / (Math.abs(dx) + padding * 2),
            this.visualizer.height / (Math.abs(dy) + padding * 2),
            2.0  // Max zoom
        );

        // Validate zoom level is finite
        if (!Number.isFinite(zoomLevel) || zoomLevel <= 0) {
            console.warn('[FOCUS] Invalid zoom level:', zoomLevel);
            return;
        }

        // Apply transform
        const transform = d3.zoomIdentity
            .translate(this.visualizer.width / 2, this.visualizer.height / 2)
            .scale(zoomLevel)
            .translate(-centerX, -centerY);

        this.visualizer.svg.transition()
            .duration(750)
            .call(this.visualizer.zoom.transform, transform);
    }

    resize() {
        const containerNode = this.visualizer.container.node();
        if (!containerNode) return;

        const newWidth = containerNode.clientWidth;
        const newHeight = containerNode.clientHeight;

        if (newWidth > 0 && newHeight > 0) {
            this.visualizer.width = newWidth;
            this.visualizer.height = newHeight;

            this.visualizer.svg.attr('viewBox', `0 0 ${this.visualizer.width} ${this.visualizer.height}`);

            console.log(`Resized to ${this.visualizer.width}x${this.visualizer.height}`);
        }
    }

    resetView() {
        this.visualizer.svg.transition()
            .duration(750)
            .call(this.visualizer.zoom.transform, this.visualizer.initialTransform);
    }

    async toggleCompoundState(stateId) {
        const state = this.visualizer.nodes.find(n => n.id === stateId);
        if (!state) return;

        state.collapsed = !state.collapsed;
        console.log(`Toggled ${stateId}: ${state.collapsed ? 'collapsed' : 'expanded'}`);

        // Update size based on collapsed state (preserve position)
        state.width = this.visualizer.getNodeWidth(state);
        state.height = this.visualizer.getNodeHeight(state);
        console.log(`Updated ${stateId} size: ${state.width}x${state.height}`);

        // Update compound bounds (both expand/collapse) and propagate to parent
        if (!state.collapsed) {
            console.log(`  → Expanded: updating compound bounds to fit children`);
            
            // Check if children have positions
            const children = state.children
                ?.map(childId => this.visualizer.nodes.find(n => n.id === childId))
                .filter(child => child);
            
            const childrenWithCoords = children?.filter(child => 
                child.x !== undefined && child.y !== undefined
            );
            
            if (children && children.length > 0 && (!childrenWithCoords || childrenWithCoords.length === 0)) {
                console.log(`  → Children missing coordinates, assigning default positions`);
                // Assign default positions relative to parent
                const padding = this.visualizer.constructor.COMPOUND_PADDING;
                const topPadding = this.visualizer.constructor.COMPOUND_TOP_PADDING;
                const childSpacing = 30;
                let yOffset = state.y - state.height / 2 + topPadding;
                
                children.forEach((child, idx) => {
                    child.x = state.x;
                    child.y = yOffset;
                    yOffset += (child.height || LAYOUT_CONSTANTS.STATE_MIN_HEIGHT) + childSpacing;
                    console.log(`    → Assigned ${child.id}: (${child.x}, ${child.y})`);
                });
            }
            
            this.visualizer.updateCompoundBounds(state);
        }
        
        // Always update parent bounds when child size changes
        console.log(`  → Updating parent compound bounds after size change`);
        const parent = this.visualizer.nodes.find(p =>
            this.visualizer.constructor.isCompoundOrParallel(p) &&
            p.children &&
            p.children.includes(stateId)
        );
        if (parent) {
            console.log(`  → Found parent: ${parent.id}, updating bounds`);
            this.visualizer.updateCompoundBounds(parent);
            console.log(`  → Parent ${parent.id} updated to size: ${parent.width}x${parent.height}`);
        }

        // Re-render to show/hide children with new size
        this.visualizer.render();

        // Update transition paths with new snap points
        // Clear routing to force recalculation
        this.visualizer.allLinks.forEach(link => {
            delete link.routing;
        });

        // Get visible nodes (collapsed ancestors hide their children)
        const visibleNodes = this.visualizer.getVisibleNodes();
        console.log(`[TOGGLE] Visible nodes after toggle: ${visibleNodes.map(n => n.id).join(', ')}`);

        // Get visible links with visual redirect applied
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, visibleNodes);
        console.log(`[TOGGLE] Visible links after filter: ${visibleLinks.length} links`);
        visibleLinks.forEach(link => {
            const vs = link.visualSource || link.source;
            const vt = link.visualTarget || link.target;
            if (vs !== link.source || vt !== link.target) {
                console.log(`[TOGGLE] Visual redirect: ${link.source}→${link.target} becomes ${vs}→${vt}`);
            }
        });

        // Re-optimize snap points with visibleLinks and visible nodes
        this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(visibleLinks, visibleNodes, false);

        // Sync routing back to original allLinks
        visibleLinks.forEach(vlink => {
            if (vlink.routing && vlink.originalLink) {
                vlink.originalLink.routing = vlink.routing;
            }
        });

        // Re-render snap points with updated routing info
        this.visualizer.renderSnapPoints(visibleNodes, visibleLinks);
        console.log(`[TOGGLE] Re-rendered snap points after optimization`);

        // Recalculate directions for visible links only
        visibleLinks.forEach(link => {
            const visualSourceId = link.visualSource || link.source;
            const visualTargetId = link.visualTarget || link.target;
            const sourceNode = visibleNodes.find(n => n.id === visualSourceId);
            const targetNode = visibleNodes.find(n => n.id === visualTargetId);
            if (sourceNode && targetNode) {
                this.visualizer.calculateLinkDirections(sourceNode, targetNode, link);
            }
        });

        // Update link visuals: rebind data with updated routing info, then update paths
        // IMPORTANT: linkElements was created by render() with old visibleLinks
        // We need to rebind with new visibleLinks that have updated routing
        this.visualizer.linkElements = this.visualizer.linkElements
            .data(visibleLinks, d => d.id);  // Rebind with new data (use id as key)
        
        this.visualizer.linkElements.attr('d', d => this.visualizer.getLinkPath(d));
        
        console.log(`[TOGGLE] Updated ${this.visualizer.linkElements.size()} link paths`);
    }
}
