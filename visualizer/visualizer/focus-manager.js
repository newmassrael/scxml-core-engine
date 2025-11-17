// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Transition Focus Manager - Centralized management for transition focus and diagram centering
 *
 * Responsibilities:
 * - Transition active state management (permanent)
 * - Transition temporary highlighting (with timeout)
 * - Diagram centering (initial load + focus on active states)
 * - ID generation and matching consistency
 */

class TransitionFocusManager {
    constructor(visualizer) {
        this.visualizer = visualizer;

        // Debug mode control for console logging
        this.debugMode = visualizer.debugMode || false;

        // Active state tracking
        this.activeTransition = null;

        // Timeout tracking for highlight removal
        this.svgHighlightTimeout = null;
        this.panelHighlightTimeout = null;

        if (this.debugMode) console.log('[FocusManager] Initialized');
    }

    /**
     * Main entry point: Focus on a transition
     * @param {Object} transition - Transition object {source, target, event, id}
     * @param {Object} options - Focus options
     * @param {boolean} options.permanent - If true, set as active (persistent), else temporary highlight
     * @param {number} options.duration - Highlight duration in ms (default: 2000)
     * @param {boolean} options.scroll - Auto-scroll into view (default: true)
     */
    focusTransition(transition, options = {}) {
        const {
            permanent = false,
            duration = 2000,
            scroll = true
        } = options;

        if (this.debugMode) console.log('[FocusManager] focusTransition():', { transition, options });

        if (permanent) {
            this.setActive(transition);
        } else {
            this.highlightTemporary(transition, duration);
        }

        if (scroll) {
            this.scrollToTransition(transition);
        }
    }

    /**
     * Set transition as permanently active
     */
    setActive(transition) {
        if (this.debugMode) console.log('[FocusManager] setActive():', transition);

        // Store active transition
        this.activeTransition = transition;
        this.visualizer.activeTransition = transition;

        const panel = document.getElementById('transition-list-panel');
        const transitionId = this.getTransitionId(transition);

        // Clear all previous active states
        this.clearActiveStates(panel);

        // Set active state on matching elements
        if (transitionId) {
            this.setActivePanel(panel, transitionId);
            this.setActiveSVG(transitionId);
        }
    }

    /**
     * Clear active transition state
     */
    clearActive() {
        if (this.debugMode) console.log('[FocusManager] clearActive()');

        this.activeTransition = null;
        this.visualizer.activeTransition = null;

        const panel = document.getElementById('transition-list-panel');

        // Clear panel active state
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
            if (this.debugMode) console.log('[FocusManager] Panel active transition cleared');
        }

        // Clear SVG active state
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('active', false);
        }
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.classed('active', false);
        }
        if (this.debugMode) console.log('[FocusManager] SVG active transition cleared');
    }

    /**
     * Temporarily highlight transition (auto-remove after duration)
     */
    highlightTemporary(transition, duration = 2000) {
        if (this.debugMode) console.log('[FocusManager] highlightTemporary():', { transition, duration });

        if (!this.visualizer.linkElements) {
            if (this.debugMode) console.log('[FocusManager] No linkElements - aborting');
            return;
        }

        this.cancelPendingHighlights();
        const transitionId = this.getTransitionId(transition);

        this.clearHighlights();
        this.highlightSVG(transitionId);
        this.highlightPanel(transitionId);
        this.scheduleHighlightRemoval(transitionId, duration);

        if (this.debugMode) console.log('[FocusManager] Temporary highlight complete');
    }

    /**
     * Clear all temporary highlights
     */
    clearHighlights() {
        if (this.debugMode) console.log('[FocusManager] clearHighlights()');

        this.cancelPendingHighlights();

        // Clear SVG highlights
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('highlighted', false);
        }
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.each(function() {
                const labelElement = this.querySelector('.transition-label');
                if (labelElement) {
                    d3.select(labelElement).classed('highlighted', false);
                }
            });
        }

        // Clear panel highlights
        const panel = document.getElementById('transition-list-panel');
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('panel-highlighted');
            });
            if (this.debugMode) console.log('[FocusManager] Panel highlights cleared');
        }

        if (this.debugMode) console.log('[FocusManager] All highlights cleared (active state preserved)');
    }

    /**
     * Center diagram in viewport
     * @param {Set|Array} targetStates - Optional: specific states to focus on (defaults to active states)
     */
    centerDiagram(targetStates = null) {
        // Get current container dimensions (using shared helper)
        const { width: currentWidth, height: currentHeight } = this.getContainerDimensions();

        const visibleNodes = this.visualizer.getVisibleNodes();

        if (visibleNodes.length === 0) {
            if (this.debugMode) console.log('[FocusManager] No visible nodes, skipping center alignment');
            return;
        }

        // Focus on target states if provided, otherwise use active states
        let nodesToFocus = visibleNodes;

        if (targetStates && targetStates.size > 0) {
            const targetArray = Array.isArray(targetStates) ? targetStates : Array.from(targetStates);
            nodesToFocus = visibleNodes.filter(node => targetArray.includes(node.id));

            if (nodesToFocus.length === 0) {
                nodesToFocus = visibleNodes;
                if (this.debugMode) console.log('[FocusManager] Target states not visible, using all nodes');
            } else {
                if (this.debugMode) console.log(`[FocusManager] Focusing on ${nodesToFocus.length} target state(s)`);
            }
        } else if (this.visualizer.activeStates && this.visualizer.activeStates.size > 0) {
            nodesToFocus = visibleNodes.filter(node => this.visualizer.activeStates.has(node.id));

            if (nodesToFocus.length === 0) {
                nodesToFocus = visibleNodes;
                if (this.debugMode) console.log('[FocusManager] Active states not visible, using all nodes');
            } else {
                if (this.debugMode) console.log(`[FocusManager] Focusing on ${nodesToFocus.length} active state(s)`);
            }
        }

        // Calculate bounding box of target nodes
        let minX = Infinity, minY = Infinity;
        let maxX = -Infinity, maxY = -Infinity;

        nodesToFocus.forEach(node => {
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

        // Get bottom panel height (using shared helper)
        const bottomPanelHeight = this.getBottomPanelHeight();

        // Calculate scale to fit diagram in viewport with padding
        const padding = LAYOUT_CONSTANTS.VIEWPORT_PADDING;
        const availableWidth = currentWidth - 2 * padding;
        const availableHeight = currentHeight - bottomPanelHeight - 2 * padding;

        const scaleX = availableWidth / diagramWidth;
        const scaleY = availableHeight / diagramHeight;
        const scale = Math.min(scaleX, scaleY, 1.0); // Don't zoom in beyond 1.0

        // Get viewport center (using shared helper)
        const viewportCenter = this.getViewportCenter();
        const viewportCenterX = viewportCenter.x;
        const viewportCenterY = viewportCenter.y;

        const transform = d3.zoomIdentity
            .translate(viewportCenterX, viewportCenterY)
            .scale(scale)
            .translate(-diagramCenterX, -diagramCenterY);

        // Apply transform
        this.visualizer.svg.transition()
            .duration(750)
            .call(this.visualizer.zoom.transform, transform);

        // Update initialTransform so resetView() returns to centered state
        this.visualizer.initialTransform = transform;

        if (this.debugMode) console.log(`[FocusManager] Diagram centered: bbox=(${minX.toFixed(1)}, ${minY.toFixed(1)}, ${maxX.toFixed(1)}, ${maxY.toFixed(1)}), scale=${scale.toFixed(2)}, bottomPanelHeight=${bottomPanelHeight}px`);
        if (this.debugMode) console.log(`[FocusManager] Container (cached): ${this.visualizer.width}x${this.visualizer.height}, Container (actual): ${currentWidth}x${currentHeight}`);
        if (this.debugMode) console.log(`[FocusManager] Diagram: ${diagramWidth.toFixed(1)}x${diagramHeight.toFixed(1)}, DiagramCenter: (${diagramCenterX.toFixed(1)}, ${diagramCenterY.toFixed(1)})`);
        if (this.debugMode) console.log(`[FocusManager] ViewportCenter: (${viewportCenterX.toFixed(1)}, ${viewportCenterY.toFixed(1)}), Available: ${availableWidth.toFixed(1)}x${availableHeight.toFixed(1)}`);
        if (this.debugMode) console.log(`[FocusManager] ScaleX: ${scaleX.toFixed(3)}, ScaleY: ${scaleY.toFixed(3)}, Final: ${scale.toFixed(3)}`);
    }

    // ========================================
    // Internal Helper Methods
    // ========================================

    /**
     * Get consistent transition ID
     */
    getTransitionId(transition) {
        if (!transition) return null;
        // Always use source_target format (C++ transition.id is just an index)
        return `${transition.source}_${transition.target}`;
    }

    /**
     * Get bottom panel height (excluded from viewport calculations)
     */
    getBottomPanelHeight() {
        const bottomPanel = document.getElementById('bottom-panels-container');
        return bottomPanel ? bottomPanel.offsetHeight : 0;
    }

    /**
     * Get current container dimensions (with fallback to cached values)
     * @returns {{width: number, height: number}} Container dimensions
     */
    getContainerDimensions() {
        const containerNode = this.visualizer.container.node();
        const width = containerNode && containerNode.clientWidth > 0
            ? containerNode.clientWidth
            : this.visualizer.width;
        const height = containerNode && containerNode.clientHeight > 0
            ? containerNode.clientHeight
            : this.visualizer.height;
        
        return { width, height };
    }

    /**
     * Get viewport center coordinates (accounting for bottom panel)
     * @returns {{x: number, y: number}} Viewport center
     */
    getViewportCenter() {
        const { width, height } = this.getContainerDimensions();
        const bottomPanelHeight = this.getBottomPanelHeight();

        return {
            x: width / 2,
            y: (height - bottomPanelHeight) / 2
        };
    }

    /**
     * Focus viewport on a specific transition
     * @param {Object} transition - Transition object
     */
    focusOnTransition(transition) {
        const transitionId = this.getTransitionId(transition);

        // Try to find and focus on transition label (text) first
        if (transitionId) {
            const labelElement = document.querySelector(`.transition-label[data-transition-id="${transitionId}"]`);

            if (labelElement) {
                const foreignObject = labelElement.closest('foreignObject');
                if (foreignObject) {
                    const x = parseFloat(foreignObject.getAttribute('x'));
                    const y = parseFloat(foreignObject.getAttribute('y'));
                    const width = parseFloat(foreignObject.getAttribute('width'));
                    const height = parseFloat(foreignObject.getAttribute('height'));

                    // Validate label coordinates
                    if (Number.isFinite(x) && Number.isFinite(y) && Number.isFinite(width) && Number.isFinite(height)) {
                        // Focus on label center
                        const centerX = x + width / 2;
                        const centerY = y + height / 2;

                        if (this.debugMode) console.log('[FocusManager] Focusing on transition label at:', { centerX, centerY });

                        // Use shared viewport center (accounts for bottom panel)
                        const viewportCenter = this.getViewportCenter();

                        const transform = d3.zoomIdentity
                            .translate(viewportCenter.x, viewportCenter.y)
                            .scale(1.5)  // Comfortable zoom for reading text
                            .translate(-centerX, -centerY);

                        this.visualizer.svg.transition()
                            .duration(750)
                            .call(this.visualizer.zoom.transform, transform);
                        return;
                    }
                }
            }
        }

        // Fallback: Focus on midpoint between source and target nodes
        const sourceNode = this.visualizer.nodes.find(n => n.id === transition.source);
        const targetNode = this.visualizer.nodes.find(n => n.id === transition.target);

        // Validate nodes exist
        if (!sourceNode || !targetNode) {
            console.warn('[FocusManager] Source or target node not found:', transition);
            return;
        }

        // Validate coordinates are valid numbers
        if (!Number.isFinite(sourceNode.x) || !Number.isFinite(sourceNode.y) ||
            !Number.isFinite(targetNode.x) || !Number.isFinite(targetNode.y)) {
            console.warn('[FocusManager] Invalid node coordinates:', {
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

        // Use shared viewport center (accounts for bottom panel)
        const viewportCenter = this.getViewportCenter();

        // Handle same-position nodes (prevent division by zero)
        if (Math.abs(dx) < 0.1 && Math.abs(dy) < 0.1) {
            if (this.debugMode) console.log('[FocusManager] Same-position nodes, using default zoom');
            const transform = d3.zoomIdentity
                .translate(viewportCenter.x, viewportCenter.y)
                .scale(1.0)
                .translate(-centerX, -centerY);

            this.visualizer.svg.transition()
                .duration(750)
                .call(this.visualizer.zoom.transform, transform);
            return;
        }

        // Calculate zoom level to fit both nodes (use bottom-panel-aware viewport)
        const { width: currentWidth, height: currentHeight } = this.getContainerDimensions();
        const bottomPanelHeight = this.getBottomPanelHeight();
        const availableHeight = currentHeight - bottomPanelHeight;

        const padding = 100;
        const zoomLevel = Math.min(
            currentWidth / (Math.abs(dx) + padding * 2),
            availableHeight / (Math.abs(dy) + padding * 2),
            2.0  // Max zoom
        );

        // Validate zoom level is finite
        if (!Number.isFinite(zoomLevel) || zoomLevel <= 0) {
            console.warn('[FocusManager] Invalid zoom level:', zoomLevel);
            return;
        }

        // Apply transform
        const transform = d3.zoomIdentity
            .translate(viewportCenter.x, viewportCenter.y)
            .scale(zoomLevel)
            .translate(-centerX, -centerY);

        this.visualizer.svg.transition()
            .duration(750)
            .call(this.visualizer.zoom.transform, transform);

        if (this.debugMode) console.log(`[FocusManager] Focused on transition ${transition.source} → ${transition.target}, zoom: ${zoomLevel.toFixed(2)}`);
    }

    /**
     * Clear all active states
     */
    clearActiveStates(panel) {
        // Clear panel
        if (panel) {
            panel.querySelectorAll('.transition-list-item').forEach(item => {
                item.classList.remove('active');
            });
        }

        // Clear SVG
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.classed('active', false);
        }
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.classed('active', false);
        }
    }

    /**
     * Set active state on panel item
     */
    setActivePanel(panel, transitionId) {
        if (!panel) return;

        const activeItem = panel.querySelector(`[data-transition-id="${transitionId}"]`);
        if (activeItem) {
            activeItem.classList.add('active');

            // Auto-scroll into view
            activeItem.scrollIntoView({
                behavior: 'smooth',
                block: 'nearest'
            });

            if (this.debugMode) console.log(`[FocusManager] Panel active state set on: ${transitionId}`);
        }
    }

    /**
     * Set active state on SVG elements
     */
    setActiveSVG(transitionId) {
        // Set active on links
        if (this.visualizer.linkElements) {
            this.visualizer.linkElements.each(function(d) {
                const tid = d3.select(this).attr('data-transition-id');
                if (tid === transitionId) {
                    d3.select(this).classed('active', true);
                }
            });
            if (this.debugMode) console.log(`[FocusManager] SVG active state set on: ${transitionId}`);
        }

        // Set active on labels
        if (this.visualizer.transitionLabels) {
            this.visualizer.transitionLabels.each(function() {
                const labelElement = this.querySelector('.transition-label');
                if (labelElement && labelElement.getAttribute('data-transition-id') === transitionId) {
                    d3.select(labelElement).classed('active', true);
                }
            });
            if (this.debugMode) console.log(`[FocusManager] Label active state set on: ${transitionId}`);
        }
    }

    /**
     * Highlight link (wrapper for highlightSVG - legacy compatibility)
     */
    highlightLink(transitionId) {
        this.highlightSVG(transitionId);
    }

    /**
     * Highlight label (wrapper for highlightSVG - legacy compatibility)
     */
    highlightLabel(transitionId) {
        this.highlightSVG(transitionId);
    }

    /**
     * Highlight SVG elements
     */
    highlightSVG(transitionId) {
        if (this.debugMode) console.log(`[FocusManager] highlightSVG(): ${transitionId}`);

        // Highlight links
        const matchingLinks = this.visualizer.linkElements.filter(function() {
            return d3.select(this).attr('data-transition-id') === transitionId;
        });

        if (matchingLinks.size() > 0) {
            matchingLinks.classed('highlighted', true);
            if (this.debugMode) console.log(`[FocusManager] SVG highlighted ${matchingLinks.size()} link(s)`);
        } else {
            if (this.debugMode) console.log(`[FocusManager] No SVG match for: ${transitionId}`);
        }

        // Highlight labels
        let foundLabel = false;
        this.visualizer.transitionLabels.each(function() {
            const labelElement = this.querySelector('.transition-label');
            if (labelElement && labelElement.getAttribute('data-transition-id') === transitionId) {
                d3.select(labelElement).classed('highlighted', true);
                foundLabel = true;
            }
        });

        if (foundLabel) {
            if (this.debugMode) console.log(`[FocusManager] Label highlighted: ${transitionId}`);
        } else {
            if (this.debugMode) console.log(`[FocusManager] No label match for: ${transitionId}`);
        }
    }

    /**
     * Highlight panel item
     */
    highlightPanel(transitionId) {
        const panel = document.getElementById('transition-list-panel');
        if (!panel) return;

        const item = panel.querySelector(`[data-transition-id="${transitionId}"]`);
        if (item) {
            item.classList.add('panel-highlighted');
            if (this.debugMode) console.log(`[FocusManager] Panel highlighted: ${transitionId}`);
        }
    }

    /**
     * Scroll transition into view
     */
    scrollToTransition(transition) {
        const transitionId = this.getTransitionId(transition);
        if (!transitionId) return;

        const panel = document.getElementById('transition-list-panel');
        if (panel) {
            const item = panel.querySelector(`[data-transition-id="${transitionId}"]`);
            if (item) {
                item.scrollIntoView({
                    behavior: 'smooth',
                    block: 'nearest'
                });
            }
        }
    }

    /**
     * Schedule highlight removal after duration
     */
    scheduleHighlightRemoval(transitionId, duration) {
        this.svgHighlightTimeout = setTimeout(() => {
            // Remove SVG highlights
            if (this.visualizer.linkElements) {
                this.visualizer.linkElements.classed('highlighted', false);
            }
            if (this.visualizer.transitionLabels) {
                this.visualizer.transitionLabels.each(function() {
                    const labelElement = this.querySelector('.transition-label');
                    if (labelElement) {
                        d3.select(labelElement).classed('highlighted', false);
                    }
                });
            }
            if (this.debugMode) console.log(`[FocusManager] Auto-removed SVG highlight after ${duration}ms`);
        }, duration);

        this.panelHighlightTimeout = setTimeout(() => {
            // Remove panel highlights
            const panel = document.getElementById('transition-list-panel');
            if (panel) {
                panel.querySelectorAll('.transition-list-item').forEach(item => {
                    item.classList.remove('panel-highlighted');
                });
            }
            if (this.debugMode) console.log(`[FocusManager] Auto-removed panel highlight after ${duration}ms`);
        }, duration);
    }

    /**
     * Cancel pending highlight removal timeouts
     */
    cancelPendingHighlights() {
        if (this.svgHighlightTimeout) {
            clearTimeout(this.svgHighlightTimeout);
            this.svgHighlightTimeout = null;
            if (this.debugMode) console.log('[FocusManager] Cancelled pending SVG highlight timeout');
        }
        if (this.panelHighlightTimeout) {
            clearTimeout(this.panelHighlightTimeout);
            this.panelHighlightTimeout = null;
            if (this.debugMode) console.log('[FocusManager] Cancelled pending panel highlight timeout');
        }
    }
}
