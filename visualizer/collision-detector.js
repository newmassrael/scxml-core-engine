// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Collision Detector - Prevents non-child states from overlapping with compound states
 *
 * When compound states expand (either during drag or bounds update), this module
 * detects and resolves collisions by pushing away overlapping external states.
 */

class CollisionDetector {
    constructor(visualizer) {
        this.visualizer = visualizer;
        this.debugMode = visualizer.debugMode;
        
        // Track affected states for selective DOM updates
        this.affectedStateIds = new Set();
        
        // Throttle DOM updates with requestAnimationFrame
        this.pendingDOMUpdate = false;
    }

    /**
     * Push away states that overlap with compound container
     * Called during drag and after bounds update
     * @param {Object} compoundNode - Compound state to check collisions against
     * @param {number} dragDx - Drag delta X (direction of expansion)
     * @param {number} dragDy - Drag delta Y (direction of expansion)
     * @param {boolean} dragTogether - If true, drag touching states together instead of pushing away
     */
    pushAwayOverlappingStates(compoundNode, dragDx = 0, dragDy = 0, dragTogether = false, dampingFactor = 0.4) {
        const startTime = performance.now();

        const COLLISION_MARGIN = 5; // Minimum spacing between states
        
        // Set default dimensions for nodes without width/height (e.g., initial-pseudo)
        const getNodeBounds = (node) => {
            let width = node.width;
            let height = node.height;
            
            // Initial-pseudo: circle with radius 10 → bounding box 20x20
            if (node.type === 'initial-pseudo') {
                width = width || 20;
                height = height || 20;
            }
            // History: circle with radius 20 → bounding box 40x40
            else if (node.type === 'history') {
                width = width || 40;
                height = height || 40;
            }
            
            return {
                left: node.x - (width || 60) / 2 - COLLISION_MARGIN,
                right: node.x + (width || 60) / 2 + COLLISION_MARGIN,
                top: node.y - (height || 40) / 2 - COLLISION_MARGIN,
                bottom: node.y + (height || 40) / 2 + COLLISION_MARGIN,
                centerX: node.x,
                centerY: node.y,
                width: width || 60,
                height: height || 40
            };
        };

        // Clear affected states from previous check
        this.affectedStateIds.clear();

        // Get compound bounds with margin using helper
        const compoundNodeBounds = getNodeBounds(compoundNode);
        const compoundBounds = {
            left: compoundNodeBounds.left,
            right: compoundNodeBounds.right,
            top: compoundNodeBounds.top,
            bottom: compoundNodeBounds.bottom,
            centerX: compoundNodeBounds.centerX,
            centerY: compoundNodeBounds.centerY
        };

        // Get all states that are NOT children of this compound
        const externalStates = this.getExternalStates(compoundNode);

        if (this.debugMode) {
            const mode = dragTogether ? 'drag together' : 'push away';
            logger.debug(`[CollisionDetector] ${compoundNode.id}: Checking ${externalStates.length} external states (drag direction: ${dragDx.toFixed(1)}, ${dragDy.toFixed(1)}, mode: ${mode})`);
        }

        let affectedCount = 0;

        externalStates.forEach(state => {
            // Get state bounds using helper (handles initial-pseudo, history, and regular nodes)
            const stateBounds = getNodeBounds(state);

            // Check if state overlaps with compound
            const overlapsX = stateBounds.right > compoundBounds.left && stateBounds.left < compoundBounds.right;
            const overlapsY = stateBounds.bottom > compoundBounds.top && stateBounds.top < compoundBounds.bottom;

            if (overlapsX && overlapsY) {
                if (dragTogether) {
                    // Drag together mode: move state by same delta as compound
                    if (this.debugMode) {
                        logger.debug(`[CollisionDetector] Dragging ${state.id} together with ${compoundNode.id} by (${dragDx.toFixed(1)}, ${dragDy.toFixed(1)})`);
                    }
                    this.applyPush(state, dragDx, dragDy);
                } else {
                    // Push away mode: calculate push vector considering drag direction
                    const pushVector = this.calculatePushVector(compoundBounds, stateBounds, dragDx, dragDy);
                    
                    // Debug: log detailed collision info
                    if (this.debugMode) {
                        logger.debug(`[COLLISION DEBUG] ${state.id} vs ${compoundNode.id}:`);
                        logger.debug(`  State bounds: left=${stateBounds.left.toFixed(1)}, right=${stateBounds.right.toFixed(1)}, top=${stateBounds.top.toFixed(1)}, bottom=${stateBounds.bottom.toFixed(1)}`);
                        logger.debug(`  Compound bounds: left=${compoundBounds.left.toFixed(1)}, right=${compoundBounds.right.toFixed(1)}, top=${compoundBounds.top.toFixed(1)}, bottom=${compoundBounds.bottom.toFixed(1)}`);
                        logger.debug(`  Push vector: x=${pushVector.x.toFixed(1)}, y=${pushVector.y.toFixed(1)}, direction=${pushVector.direction}`);
                    }
                    
                    // For expansion (dampingFactor = 1.0), move directly to boundary - no gradual push
                    // For drag (dampingFactor < 1.0), apply smooth damping
                    let finalX, finalY;
                    if (dampingFactor >= 1.0) {
                        // Direct positioning: calculate final position outside compound bounds
                        // This ensures one-shot resolution with no iterations needed
                        finalX = pushVector.x;
                        finalY = pushVector.y;
                    } else {
                        // Smooth damping for drag operations
                        finalX = pushVector.x * dampingFactor;
                        finalY = pushVector.y * dampingFactor;
                    }

                    if (this.debugMode) {
                        logger.debug(`[CollisionDetector] Collision: ${state.id} overlaps ${compoundNode.id}, pushing ${pushVector.direction} by (${finalX.toFixed(1)}, ${finalY.toFixed(1)})`);
                    }

                    this.applyPush(state, finalX, finalY);
                }
                
                // Track affected state for selective DOM update
                this.affectedStateIds.add(state.id);
                affectedCount++;
            }
        });

        const endTime = performance.now();
        const duration = endTime - startTime;

        if (affectedCount > 0 && this.debugMode) {
            const action = dragTogether ? 'Dragged together' : 'Pushed away';
            logger.debug(`[CollisionDetector] ${action} ${affectedCount} states with ${compoundNode.id} (took ${duration.toFixed(2)}ms)`);
        }

        // Warn if collision detection is too slow
        if (duration > 5) {
            logger.warn(`[CollisionDetector] SLOW: ${compoundNode.id} collision detection took ${duration.toFixed(2)}ms (${externalStates.length} states checked, ${affectedCount} affected)`);
        }

        return affectedCount;
    }

    /**
     * Get all states that are external to (not children of) the compound
     * @param {Object} compoundNode - Compound state
     * @returns {Array} External states
     */
    getExternalStates(compoundNode) {
        // Exclude descendants (children, grandchildren, etc.)
        const descendantIds = new Set(this.visualizer.getAllDescendantIds(compoundNode.id));
        descendantIds.add(compoundNode.id);

        // **CRITICAL: Exclude ancestors (parent, grandparent, etc.)**
        // Without this, s11 would push s1 (its parent) causing erratic position jumps
        const ancestorIds = new Set();
        let current = compoundNode.id;
        while (true) {
            const parent = this.visualizer.findCompoundParent(current);
            if (!parent) break;
            ancestorIds.add(parent.id);
            current = parent.id;
        }

        return this.visualizer.nodes.filter(node => {
            if (descendantIds.has(node.id)) return false;
            if (ancestorIds.has(node.id)) return false;  // Don't push ancestors
            if (node.collapsed && SCXMLVisualizer.isCompoundOrParallel(node)) return false;
            return node.x !== undefined && node.y !== undefined;
        });
    }

    /**
     * Calculate push vector considering drag direction for natural boundary sliding
     * @param {Object} compoundBounds - Compound bounding box
     * @param {Object} stateBounds - State bounding box
     * @param {number} dragDx - Drag delta X (direction of expansion)
     * @param {number} dragDy - Drag delta Y (direction of expansion)
     * @returns {Object} Push vector {x, y, direction}
     */
    calculatePushVector(compoundBounds, stateBounds, dragDx = 0, dragDy = 0) {
        // Calculate overlap distances in each direction
        const overlapLeft = compoundBounds.right - stateBounds.left;
        const overlapRight = stateBounds.right - compoundBounds.left;
        const overlapTop = compoundBounds.bottom - stateBounds.top;
        const overlapBottom = stateBounds.bottom - compoundBounds.top;

        // Determine which edge of compound is pushing the state
        // State's relative position to compound center
        const stateRelativeX = stateBounds.centerX - compoundBounds.centerX;
        const stateRelativeY = stateBounds.centerY - compoundBounds.centerY;

        let pushX = 0;
        let pushY = 0;
        let direction = '';

        // If drag direction is significant, prioritize pushing in drag direction
        const dragMagnitude = Math.sqrt(dragDx * dragDx + dragDy * dragDy);

        if (dragMagnitude > 0.1) {
            // Drag-directed pushing: push along the direction of compound expansion

            // Determine primary drag axis
            const isDragHorizontal = Math.abs(dragDx) > Math.abs(dragDy);

            if (isDragHorizontal) {
                // Horizontal drag dominates
                if (dragDx > 0 && stateRelativeX > 0) {
                    // Dragging right, state is on right side → push right
                    pushX = overlapRight;
                    direction = 'right';
                } else if (dragDx < 0 && stateRelativeX < 0) {
                    // Dragging left, state is on left side → push left
                    pushX = -overlapLeft;
                    direction = 'left';
                } else {
                    // State is on opposite side, use minimum overlap
                    if (Math.abs(stateRelativeX) * overlapTop < Math.abs(stateRelativeY) * overlapRight) {
                        // Closer to top/bottom edge → push vertically
                        pushY = stateRelativeY > 0 ? overlapBottom : -overlapTop;
                        direction = stateRelativeY > 0 ? 'down' : 'up';
                    } else {
                        // Closer to left/right edge → push horizontally
                        pushX = stateRelativeX > 0 ? overlapRight : -overlapLeft;
                        direction = stateRelativeX > 0 ? 'right' : 'left';
                    }
                }

                // Slide along boundary if there's vertical drag component
                if (Math.abs(dragDy) > 0.1 && pushX !== 0) {
                    // Add vertical sliding proportional to drag direction
                    pushY = dragDy * 0.5; // Damped sliding for smooth motion
                }
            } else {
                // Vertical drag dominates
                if (dragDy > 0 && stateRelativeY > 0) {
                    // Dragging down, state is below → push down
                    pushY = overlapBottom;
                    direction = 'down';
                } else if (dragDy < 0 && stateRelativeY < 0) {
                    // Dragging up, state is above → push up
                    pushY = -overlapTop;
                    direction = 'up';
                } else {
                    // State is on opposite side, use minimum overlap
                    if (Math.abs(stateRelativeY) * overlapLeft < Math.abs(stateRelativeX) * overlapTop) {
                        // Closer to left/right edge → push horizontally
                        pushX = stateRelativeX > 0 ? overlapRight : -overlapLeft;
                        direction = stateRelativeX > 0 ? 'right' : 'left';
                    } else {
                        // Closer to top/bottom edge → push vertically
                        pushY = stateRelativeY > 0 ? overlapBottom : -overlapTop;
                        direction = stateRelativeY > 0 ? 'down' : 'up';
                    }
                }

                // Slide along boundary if there's horizontal drag component
                if (Math.abs(dragDx) > 0.1 && pushY !== 0) {
                    // Add horizontal sliding proportional to drag direction
                    pushX = dragDx * 0.5; // Damped sliding for smooth motion
                }
            }
        } else {
            // No drag direction (expansion) → push outward from compound center
            // Use center-to-center direction to avoid oscillation
            
            // Calculate distances to each edge
            const distToLeft = stateBounds.centerX - compoundBounds.left;
            const distToRight = compoundBounds.right - stateBounds.centerX;
            const distToTop = stateBounds.centerY - compoundBounds.top;
            const distToBottom = compoundBounds.bottom - stateBounds.centerY;
            
            // Find closest edge
            const minDist = Math.min(distToLeft, distToRight, distToTop, distToBottom);
            
            // Push through closest edge (full distance + state half-size + margin)
            const stateHalfWidth = (stateBounds.right - stateBounds.left) / 2;
            const stateHalfHeight = (stateBounds.bottom - stateBounds.top) / 2;
            
            if (minDist === distToLeft) {
                // Push left: move state center to left of compound.left
                pushX = -(distToLeft + stateHalfWidth);
                direction = 'left';
            } else if (minDist === distToRight) {
                // Push right: move state center to right of compound.right
                pushX = distToRight + stateHalfWidth;
                direction = 'right';
            } else if (minDist === distToTop) {
                // Push up: move state center above compound.top
                pushY = -(distToTop + stateHalfHeight);
                direction = 'up';
            } else {
                // Push down: move state center below compound.bottom
                pushY = distToBottom + stateHalfHeight;
                direction = 'down';
            }
        }

        return { x: pushX, y: pushY, direction };
    }

    /**
     * Apply push to state and all its descendants
     * @param {Object} state - State to push
     * @param {number} pushX - Push distance X
     * @param {number} pushY - Push distance Y
     */
    applyPush(state, pushX, pushY) {
        // Push the state
        state.x += pushX;
        state.y += pushY;

        // If this state is also a compound with children, push all descendants
        if (SCXMLVisualizer.isCompoundOrParallel(state) && state.children && state.children.length > 0) {
            const descendantIds = new Set(this.visualizer.getAllDescendantIds(state.id));
            let descendantCount = 0;

            this.visualizer.nodes.forEach(node => {
                if (descendantIds.has(node.id)) {
                    node.x += pushX;
                    node.y += pushY;
                    
                    // Track descendants as affected for DOM update
                    this.affectedStateIds.add(node.id);
                    descendantCount++;
                }
            });

            if (this.debugMode) {
                logger.debug(`[CollisionDetector] Also pushed ${descendantCount} descendants of ${state.id}`);
            }
        }
    }

    /**
     * Update DOM positions of pushed states (throttled with requestAnimationFrame)
     * Called after collision resolution to sync visual with data
     * @param {Set<string>} affectedStateIds - Optional set of state IDs to update (if null, updates all)
     */
    updatePushedStatesDOM(affectedStateIds = null) {
        // Use requestAnimationFrame to throttle DOM updates (max 60fps)
        if (this.pendingDOMUpdate) {
            return; // Already scheduled
        }
        
        this.pendingDOMUpdate = true;
        
        requestAnimationFrame(() => {
            const startTime = performance.now();
            
            // Use affectedStateIds from collision detection if not provided
            const statesToUpdate = affectedStateIds || this.affectedStateIds;
            
            if (statesToUpdate.size === 0) {
                this.pendingDOMUpdate = false;
                return; // Nothing to update
            }
            
            if (this.debugMode) {
                logger.debug(`[CollisionDetector] Updating ${statesToUpdate.size} affected states in DOM`);
            }

            // Update only affected node positions with smooth transition
            if (this.visualizer.nodeElements) {
                this.visualizer.nodeElements
                    .filter(d => statesToUpdate.has(d.id))
                    .transition()
                    .duration(150)  // 150ms smooth transition
                    .ease(d3.easeQuadOut)  // Ease out for natural deceleration
                    .attr('transform', d => `translate(${d.x},${d.y})`);
            }

            // Update only affected compound container positions with smooth transition
            if (this.visualizer.compoundContainers) {
                this.visualizer.compoundContainers
                    .filter(d => statesToUpdate.has(d.id))
                    .transition()
                    .duration(150)
                    .ease(d3.easeQuadOut)
                    .attr('x', d => d.x - d.width/2)
                    .attr('y', d => d.y - d.height/2);
            }

            // Update only affected compound label positions with smooth transition
            if (this.visualizer.compoundLabels) {
                this.visualizer.compoundLabels
                    .filter(d => statesToUpdate.has(d.id))
                    .transition()
                    .duration(150)
                    .ease(d3.easeQuadOut)
                    .attr('x', d => d.x - d.width/2 + 10)
                    .attr('y', d => d.y - d.height/2 + 20);
            }

            // Update only affected collapsed compound positions
            if (this.visualizer.zoomContainer) {
                this.visualizer.zoomContainer.selectAll('g.collapsed-compound')
                    .filter(d => statesToUpdate.has(d.id))
                    .attr('transform', d => `translate(${d.x}, ${d.y})`);
            }

            const endTime = performance.now();
            const duration = endTime - startTime;
            
            this.pendingDOMUpdate = false;

            // Warn if DOM update is too slow
            if (duration > 10) {
                logger.warn(`[CollisionDetector] SLOW DOM UPDATE: took ${duration.toFixed(2)}ms (${statesToUpdate.size} states)`);
            } else if (this.debugMode) {
                logger.debug(`[CollisionDetector] DOM update completed in ${duration.toFixed(2)}ms (${statesToUpdate.size} states)`);
            }
        });
    }

    /**
     * Centralized drag collision handler - handles all drag collision scenarios
     * Call this from any drag handler to ensure consistent collision behavior
     * @param {Object} draggedNode - Node being dragged
     * @param {number} dx - Drag delta X
     * @param {number} dy - Drag delta Y
     */
    handleDragCollision(draggedNode, dx, dy) {
        // Store drag direction for updateCompoundBounds
        draggedNode._dragDx = dx;
        draggedNode._dragDy = dy;
        
        // Scenario 1: Dragging a child of a compound
        // Parent expands, push away external states overlapping with parent
        if (draggedNode.dragParent) {
            this.pushAwayOverlappingStates(draggedNode.dragParent, dx, dy, false);
            this.updatePushedStatesDOM();
            return; // Parent handles collision, no need to check child
        }
        
        // Scenario 2: Dragging a standalone node (no parent)
        // Push away states overlapping with the dragged node itself
        this.pushAwayOverlappingStates(draggedNode, dx, dy, false);
        this.updatePushedStatesDOM();
    }
}
