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
            logger.debug(`[UPDATE LINKS] Called with useGreedy=${useGreedy}`);
        }
        if (!this.visualizer.linkElements || !this.visualizer.allLinks) {
            if (this.visualizer.debugMode) {
                logger.debug('[UPDATE LINKS] Early return: linkElements or allLinks missing');
            }
            return;
        }

        // **Get visibleLinks BEFORE optimization for filtering**
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, this.visualizer.nodes);

        // Check if any node is being dragged
        const anyNodeDragging = this.visualizer.nodes.some(n => n.isDragging);

        if (anyNodeDragging || useGreedy) {
            // const mode = useGreedy ? 'GREEDY (fast)' : 'CSP (optimal)';
            // logger.debug(`[DRAG UPDATE] Re-running optimizer (${mode})...`);

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

            // logger.debug('[DRAG UPDATE] Re-optimization complete');
        }

        // Pass 2: Rebind linkElements with updated visibleLinks data
        // CRITICAL: linkElements was created by render() with old data
        // We need to rebind with new visibleLinks that have updated routing
        this.visualizer.linkElements = this.visualizer.linkElements
            .data(visibleLinks, d => d.id);

        // Pass 3: Render with updated directions
        if (this.visualizer.debugMode) {
            logger.debug(`[UPDATE LINKS] Updating ${this.visualizer.linkElements.size()} link paths`);
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
                logger.debug(`[UPDATE LINKS] Updating ${nodeCount} node DOM positions`);
            }
            this.visualizer.nodeElements.each(function(nodeData) {
                const latestNode = nodeMap.get(nodeData.id);
                if (latestNode) {
                    d3.select(this).attr('transform', `translate(${latestNode.x}, ${latestNode.y})`);
                }
            });
        } else {
            if (this.visualizer.debugMode) {
                logger.debug('[UPDATE LINKS] nodeElements not found!');
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
        logger.debug(`[highlightActiveStates] Called with:`, activeStateIds);
        this.visualizer.activeStates = new Set(activeStateIds);

        // Auto-expand compound/parallel states that are active or have active children
        let needsReLayout = false;
        this.visualizer.nodes.forEach(node => {
            if (this.visualizer.constructor.isCompoundOrParallel(node) && node.collapsed) {
                // Check if this node is active OR has any active children
                const isActive = this.visualizer.activeStates.has(node.id);
                const hasActiveChildren = node.children && node.children.some(childId => this.visualizer.activeStates.has(childId));

                if (isActive || hasActiveChildren) {
                    logger.debug(`  → Auto-expanding ${node.id} (${node.type}): isActive=${isActive}, hasActiveChildren=${hasActiveChildren}`);
                    node.collapsed = false;
                    needsReLayout = true;
                }
            }
        });

        // Re-layout if any compound/parallel was expanded
        if (needsReLayout) {
            logger.debug(`  → Triggering re-layout due to auto-expansion`);
            this.visualizer.computeLayout().then(() => {
                this.visualizer.render();
                // Re-highlight after re-render
                this.visualizer.highlightActiveStatesVisual();
                // Auto-center on active states
                this.visualizer.focusManager.centerDiagram(activeStateIds);
            });
            return;
        }

        this.visualizer.highlightActiveStatesVisual();
        // Auto-center on active states
        this.visualizer.focusManager.centerDiagram(activeStateIds);
    }

    highlightActiveStatesVisual() {
        if (this.visualizer.nodeElements) {
            this.visualizer.nodeElements.classed('active', d => {
                return this.visualizer.activeStates.has(d.id);
            });
        }

        if (this.visualizer.collapsedElements) {
            this.visualizer.collapsedElements.classed('active', d => {
                return this.visualizer.activeStates.has(d.id);
            });
        }

        if (this.visualizer.compoundContainers) {
            this.visualizer.compoundContainers.classed('active', d => {
                return this.visualizer.activeStates.has(d.id);
            });
        }
    }

    animateTransition(transition) {
        // No-op: CSS handles animation via .highlighted class
        // See visualizer.css: .transition.highlighted { animation: transitionPulse ... }
        logger.debug('[DEPRECATED] animateTransition() called - CSS handles animation now');
    }

    renderTransitionList() {
        const panel = document.getElementById('transition-list-panel');
        if (!panel) return;

        if (!this.visualizer.transitions || this.visualizer.transitions.length === 0) {
            panel.innerHTML = '<div class="transition-hint">No transitions</div>';
            return;
        }

        let html = '';

        // W3C SCXML Appendix D.2: Get conflict resolution data for badge display
        const enabledTransitions = this.visualizer.enabledTransitions || [];
        const optimalTransitions = this.visualizer.optimalTransitions || [];

        this.visualizer.transitions.forEach((transition, index) => {
            // Use shared utility function from utils.js (Single Source of Truth)
            const transitionId = getTransitionId(transition);
            const eventText = transition.event || '(eventless)';

            // W3C SCXML Appendix D.2: Check if this transition is in enabled/optimal sets
            const isEnabled = enabledTransitions.some(et =>
                et.source === transition.source && et.target === transition.target);
            const isOptimal = optimalTransitions.some(ot =>
                ot.source === transition.source && ot.target === transition.target);
            const isPreempted = isEnabled && !isOptimal;

            // Build badge HTML
            let badgesHtml = '';
            if (isOptimal) {
                badgesHtml += '<span class="badge badge-selected">Selected</span>';
            }
            if (isPreempted) {
                badgesHtml += '<span class="badge badge-preempted">Preempted</span>';
            }
            if (isEnabled && !isOptimal && !isPreempted) {
                badgesHtml += '<span class="badge badge-enabled">Enabled</span>';
            }

            // Add condition and actions for better distinction
            let detailsHtml = '';

            // Condition (guard)
            if (transition.cond) {
                detailsHtml += `<div class="transition-list-condition">🔍 [${transition.cond}]</div>`;
            }

            // Actions (using ActionFormatter for consistent display)
            if (transition.actions && transition.actions.length > 0) {
                logger.debug('[InteractionHandler] Processing actions:', transition.actions);
                logger.debug('[InteractionHandler] ActionFormatter available:', typeof ActionFormatter);
                transition.actions.forEach(action => {
                    logger.debug('[InteractionHandler] Processing action:', action);
                    const formatted = ActionFormatter.formatAction(action);
                    logger.debug('[InteractionHandler] Formatted result:', formatted);

                    // Main action line
                    detailsHtml += `<div class="transition-list-action">${formatted.main}</div>`;

                    // Detail lines (for send actions with content, params, etc.)
                    if (formatted.details && formatted.details.length > 0) {
                        logger.debug('[InteractionHandler] Adding details:', formatted.details);
                        formatted.details.forEach(detail => {
                            detailsHtml += `<div class="action-detail-line">${detail}</div>`;
                        });
                    }
                });
            }

            html += `
                <div class="transition-list-item" data-transition-id="${transitionId}" data-transition-index="${index}">
                    <div class="transition-list-source-target">
                        <strong>${transition.source}</strong> → <strong>${transition.target}</strong>
                        ${badgesHtml}
                    </div>
                    <div class="transition-list-event">${eventText}</div>
                    ${detailsHtml}
                </div>
            `;
        });

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
                }, 3000);  // 3s (PANEL_HIGHLIGHT_DURATION in controller-core.js)
            });
        });
    }

    setActiveTransition(transition) {
        // Delegated to FocusManager for centralized focus management
        return this.visualizer.focusManager.setActive(transition);
    }

    clearTransitionHighlights() {
        // Delegated to FocusManager for centralized focus management
        return this.visualizer.focusManager.clearHighlights();
    }

    clearActiveTransition() {
        // Delegated to FocusManager for centralized focus management
        this.visualizer.focusManager.clearActive();
        this.visualizer.focusManager.clearHighlights();
    }

    highlightTransition(transition, duration = 2000) {
        // Delegated to FocusManager for centralized focus management
        return this.visualizer.focusManager.highlightTemporary(transition, duration);
    }



    focusOnTransition(transition) {
        // Delegated to FocusManager for centralized focus management
        return this.visualizer.focusManager.focusOnTransition(transition);
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

            logger.debug(`Resized to ${this.visualizer.width}x${this.visualizer.height}`);
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
        logger.debug(`Toggled ${stateId}: ${state.collapsed ? 'collapsed' : 'expanded'}`);

        // Update size based on collapsed state (preserve position)
        state.width = this.visualizer.getNodeWidth(state);
        state.height = this.visualizer.getNodeHeight(state);
        logger.debug(`Updated ${stateId} size: ${state.width}x${state.height}`);

        // Update compound bounds (both expand/collapse) and propagate to parent
        if (!state.collapsed) {
            logger.debug(`  → Expanded: updating compound bounds to fit children`);
            
            // Check if children have positions
            const children = state.children
                ?.map(childId => this.visualizer.nodes.find(n => n.id === childId))
                .filter(child => child);
            
            const childrenWithCoords = children?.filter(child => 
                child.x !== undefined && child.y !== undefined
            );
            
            if (children && children.length > 0 && (!childrenWithCoords || childrenWithCoords.length === 0)) {
                logger.debug(`  → Children missing coordinates, assigning default positions`);
                // Assign default positions relative to parent
                const padding = this.visualizer.constructor.COMPOUND_PADDING;
                const topPadding = this.visualizer.constructor.COMPOUND_TOP_PADDING;
                const childSpacing = 30;
                let yOffset = state.y - state.height / 2 + topPadding;
                
                children.forEach((child, idx) => {
                    child.x = state.x;
                    child.y = yOffset;
                    yOffset += (child.height || LAYOUT_CONSTANTS.STATE_MIN_HEIGHT) + childSpacing;
                    logger.debug(`    → Assigned ${child.id}: (${child.x}, ${child.y})`);
                });
            }
            
            this.visualizer.updateCompoundBounds(state);
            
            // Push away overlapping states when expanding (direct positioning - one shot)
            if (this.visualizer.collisionDetector) {
                // Direct positioning: states moved immediately to boundary, not gradual push
                // Only need 1-2 iterations (2nd for cascading collisions)
                let iteration = 0;
                let affectedCount = 0;
                const MAX_ITERATIONS = 3; // 1 for main, 1-2 for cascade
                
                do {
                    // Use 100% damping (1.0) for expansion to fully resolve overlaps
                    // Drag uses 40% damping (default) for smooth following
                    affectedCount = this.visualizer.collisionDetector.pushAwayOverlappingStates(state, 0, 0, false, 1.0);
                    iteration++;
                    if (this.visualizer.debugMode) {
                        logger.debug(`  → Collision iteration ${iteration}: pushed ${affectedCount} states`);
                    }
                } while (affectedCount > 0 && iteration < MAX_ITERATIONS);
                
                if (affectedCount > 0 && this.visualizer.debugMode) {
                    logger.warn(`  ⚠️ Still ${affectedCount} overlapping states after ${MAX_ITERATIONS} iterations`);
                }
                
                // Don't call updatePushedStatesDOM() here - render() will handle it
                // updatePushedStatesDOM() uses requestAnimationFrame which conflicts with immediate render()
            }
        }
        
        // Always update parent bounds when child size changes
        logger.debug(`  → Updating parent compound bounds after size change`);
        const parent = this.visualizer.nodes.find(p =>
            this.visualizer.constructor.isCompoundOrParallel(p) &&
            p.children &&
            p.children.includes(stateId)
        );
        if (parent) {
            logger.debug(`  → Found parent: ${parent.id}, updating bounds`);
            this.visualizer.updateCompoundBounds(parent);
            logger.debug(`  → Parent ${parent.id} updated to size: ${parent.width}x${parent.height}`);
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
        logger.debug(`[TOGGLE] Visible nodes after toggle: ${visibleNodes.map(n => n.id).join(', ')}`);

        // Get visible links with visual redirect applied
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, visibleNodes);
        logger.debug(`[TOGGLE] Visible links after filter: ${visibleLinks.length} links`);
        visibleLinks.forEach(link => {
            const vs = link.visualSource || link.source;
            const vt = link.visualTarget || link.target;
            if (vs !== link.source || vt !== link.target) {
                logger.debug(`[TOGGLE] Visual redirect: ${link.source}→${link.target} becomes ${vs}→${vt}`);
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
        logger.debug(`[TOGGLE] Re-rendered snap points after optimization`);

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

        logger.debug(`[TOGGLE] Updated ${this.visualizer.linkElements.size()} link paths`);

        // Update transition labels if they exist
        if (this.visualizer.transitionLabels) {
            // Rebind with updated visibleLinks data
            this.visualizer.transitionLabels = this.visualizer.transitionLabels
                .data(visibleLinks, d => d.id);

            this.visualizer.transitionLabels
                .attr('x', d => this.visualizer.getTransitionLabelPosition(d).x)
                .attr('y', d => this.visualizer.getTransitionLabelPosition(d).y);

            logger.debug(`[TOGGLE] Updated ${this.visualizer.transitionLabels.size()} transition label positions`);
        }
    }
}
