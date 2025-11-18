// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Layout Manager - Handles ELK layout computation and application
 */

class LayoutManager {
    constructor(visualizer) {
        this.visualizer = visualizer;
    }

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

        const visibleNodes = this.visualizer.getVisibleNodes();
        const visibleNodeIds = new Set(visibleNodes.map(n => n.id));

        // Recursive function to build ELK node with nested children
        const buildELKNode = (node, depth = 0) => {
            const indent = '  '.repeat(depth);
            logger.debug(`${indent}[buildELKNode] Building ${node.id} (${node.type}, collapsed=${node.collapsed})`);

            const elkNode = {
                id: node.id,
                width: this.visualizer.getNodeWidth(node),
                height: this.visualizer.getNodeHeight(node)
            };

            // Add children for expanded compounds
            if (SCXMLVisualizer.isCompoundOrParallel(node) && !node.collapsed) {
                elkNode.children = [];

                logger.debug(`${indent}  ${node.id} has ${node.children.length} children: ${node.children.join(', ')}`);

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
                    const childNode = this.visualizer.nodes.find(n => n.id === childId);
                    if (childNode) {
                        if (visibleNodeIds.has(childId)) {
                            logger.debug(`${indent}    → Adding child ${childId} (visible)`);
                            elkNode.children.push(buildELKNode(childNode, depth + 1));
                        } else {
                            logger.debug(`${indent}    → Skipping child ${childId} (not visible)`);
                        }
                    } else {
                        logger.warn(`${indent}    → Child ${childId} not found in this.visualizer.nodes!`);
                    }
                });

                logger.debug(`${indent}  ${node.id} elkNode.children.length = ${elkNode.children.length}`);
            }

            return elkNode;
        };

        // Only add top-level visible nodes (nodes without visible parents)
        const topLevelNodes = visibleNodes.filter(node => {
            // Check if this node has a visible parent
            const hasVisibleParent = this.visualizer.nodes.some(parent =>
                (parent.type === 'compound' || parent.type === 'parallel') &&
                !parent.collapsed &&
                parent.children &&
                parent.children.includes(node.id) &&
                visibleNodeIds.has(parent.id)
            );
            return !hasVisibleParent;
        });

        logger.debug(`[buildELKGraph] Total visible nodes: ${visibleNodes.length}, Top-level nodes: ${topLevelNodes.length}`);
        logger.debug(`  Top-level: ${topLevelNodes.map(n => n.id).join(', ')}`);

        topLevelNodes.forEach(node => {
            graph.children.push(buildELKNode(node));
        });

        // Add edges (only if both source and target are visible)
        const visibleLinks = this.visualizer.getVisibleLinks(this.visualizer.allLinks, this.visualizer.nodes);

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

    applyELKLayout(layouted) {
        logger.debug('Applying ELK layout to nodes...');
        logger.debug(`  this.visualizer.nodes count: ${this.visualizer.nodes.length}, nodes: ${this.visualizer.nodes.map(n => `${n.id}(${n.type})`).join(', ')}`);

        // Debug: Log ELK result structure
        logger.debug('ELK result structure:');
        layouted.children.forEach(child => {
            logger.debug(`  ${child.id}: hasChildren=${!!child.children}, childCount=${child.children ? child.children.length : 0}`);
            if (child.children) {
                child.children.forEach(grandchild => {
                    logger.debug(`    └─ ${grandchild.id}: hasChildren=${!!grandchild.children}, childCount=${grandchild.children ? grandchild.children.length : 0}`);
                    if (grandchild.children) {
                        grandchild.children.forEach(ggrandchild => {
                            logger.debug(`       └─ ${ggrandchild.id}`);
                        });
                    }
                });
            }
        });

        const applyToNode = (elkNode, offsetX = 0, offsetY = 0, depth = 0) => {
            const indent = '  '.repeat(depth);
            const node = this.visualizer.nodes.find(n => n.id === elkNode.id);
            if (node) {
                node.x = elkNode.x + offsetX + elkNode.width / 2;
                node.y = elkNode.y + offsetY + elkNode.height / 2;

                // Preserve collapsed node size - don't overwrite with ELK's expanded layout
                if (node.collapsed && SCXMLVisualizer.isCompoundOrParallel(node)) {
                    // Initialize size if not set (first render of collapsed node)
                    if (node.width === undefined || node.height === undefined) {
                        node.width = this.visualizer.getNodeWidth(node);
                        node.height = this.visualizer.getNodeHeight(node);
                        logger.debug(`${indent}  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height} (collapsed, size initialized), offset=(${offsetX}, ${offsetY})`);
                    } else {
                        logger.debug(`${indent}  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height} (collapsed, size preserved), offset=(${offsetX}, ${offsetY})`);
                    }
                } else {
                    // Get the originally calculated width/height
                    const originalWidth = this.visualizer.getNodeWidth(node);
                    const originalHeight = this.visualizer.getNodeHeight(node);

                    // Use original calculated dimensions instead of ELK's adjusted values
                    // to ensure text content fits properly
                    node.width = originalWidth;
                    node.height = originalHeight;

                    logger.debug(`${indent}  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height} (original calc, ELK wanted ${elkNode.width}x${elkNode.height}), offset=(${offsetX}, ${offsetY})`);
                }
            } else {
                logger.warn(`${indent}  ELK node not found in this.visualizer.nodes: ${elkNode.id} (possibly child state or collapsed)`);
            }

            if (elkNode.children) {
                logger.debug(`${indent}  ${elkNode.id} has ${elkNode.children.length} children in ELK result`);
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
        logger.debug(`  ELK layout nodes: ${elkNodeIds.join(', ')}`);

        layouted.children.forEach(child => {
            applyToNode(child);
        });

        // Calculate bounding boxes for expanded compounds without coordinates
        // (ELK may not provide coordinates for nested hierarchy)
        logger.debug('Calculating bounding boxes for expanded compounds...');
        logger.debug(`  Total nodes: ${this.visualizer.nodes.length}`);
        this.visualizer.nodes.forEach(node => {
            if ((node.type === 'compound' || node.type === 'parallel') && 
                !node.collapsed && 
                (node.x === undefined || node.y === undefined)) {
                
                logger.debug(`  Processing ${node.id} (type=${node.type}, collapsed=${node.collapsed}, hasCoords=${node.x !== undefined && node.y !== undefined})`);
                
                // Get children coordinates
                if (!node.children || node.children.length === 0) {
                    logger.warn(`  ${node.id}: No children array, skipping bounding box calculation`);
                    return;
                }
                
                const allChildNodes = node.children
                    .map(childId => this.visualizer.nodes.find(n => n.id === childId))
                    .filter(child => child);

                const childNodesWithCoords = allChildNodes
                    .filter(child => child.x !== undefined && child.y !== undefined);

                logger.debug(`    Children: ${node.children.join(', ')}, with coords: ${childNodesWithCoords.map(c => c.id).join(', ')}`);

                if (childNodesWithCoords.length > 0) {
                    // Calculate bounding box with padding
                    const padding = PATH_CONSTANTS.COMPOUND_BOUNDS_PADDING;
                    const minX = Math.min(...childNodesWithCoords.map(c => c.x - c.width/2)) - padding;
                    const maxX = Math.max(...childNodesWithCoords.map(c => c.x + c.width/2)) + padding;
                    const minY = Math.min(...childNodesWithCoords.map(c => c.y - c.height/2)) - padding;
                    const maxY = Math.max(...childNodesWithCoords.map(c => c.y + c.height/2)) + padding;

                    node.x = (minX + maxX) / 2;
                    node.y = (minY + maxY) / 2;
                    node.width = maxX - minX;
                    node.height = maxY - minY;

                    logger.debug(`  ${node.id}: Calculated from children (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width.toFixed(1)}x${node.height.toFixed(1)}`);
                } else if (childNodesWithCoords.length === 0 && allChildNodes.length > 0 && allChildNodes.some(c => c.x === undefined || c.y === undefined)) {
                    // All children exist but none have coordinates - this is expected when ELK doesn't layout nested hierarchies
                    logger.debug(`  ${node.id}: Children not yet laid out by ELK, skipping bounding box calculation`);
                } else if (childNodesWithCoords.length < allChildNodes.length) {
                    // Some children have coordinates but not all - this is unexpected
                    logger.warn(`  ${node.id}: Partial child coordinates (${childNodesWithCoords.length}/${allChildNodes.length}), cannot calculate reliable bounding box`);
                }
            }
        });

        // Apply ELK edge routing information BEFORE modifying node positions
        if (layouted.edges) {
            logger.debug('Applying ELK edge routing...');
            layouted.edges.forEach(elkEdge => {
                const link = this.visualizer.allLinks.find(l => l.id === elkEdge.id);
                if (link && elkEdge.sections && elkEdge.sections.length > 0) {
                    link.elkSections = elkEdge.sections;
                    logger.debug(`  ${elkEdge.id}: ${elkEdge.sections.length} section(s)`);
                    elkEdge.sections.forEach((section, idx) => {
                        logger.debug(`    section ${idx}: start=(${section.startPoint.x.toFixed(1)}, ${section.startPoint.y.toFixed(1)}), end=(${section.endPoint.x.toFixed(1)}, ${section.endPoint.y.toFixed(1)})`);
                        if (section.bendPoints && section.bendPoints.length > 0) {
                            section.bendPoints.forEach((bp, bpIdx) => {
                                logger.debug(`      bendPoint ${bpIdx}: (${bp.x.toFixed(1)}, ${bp.y.toFixed(1)})`);
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
        logger.debug('[LAYOUT] Using ELK calculated positions (no manual alignment)');

        // Invalidate ELK edge routing to use optimizer-calculated snap points
        // ELK routing is only used during initial layout, afterward we use optimizer routing
        this.visualizer.allLinks.forEach(link => {
            if (link.elkSections) {
                delete link.elkSections;
            }
        });
        logger.debug('[LAYOUT] Invalidated ELK edge routing (will use optimizer routing)');

        // Optimize snap point assignments to minimize intersections
        logger.debug('Optimizing snap point assignments...');
        this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(this.visualizer.allLinks, this.visualizer.nodes);

        // Update all compound/parallel bounds to ensure they contain all children
        // Process in bottom-up order: children first, then parents
        logger.debug('Updating compound container bounds...');
        
        // Find all expanded compounds/parallels and their depths
        const compoundsWithDepth = [];
        const getDepth = (nodeId, visited = new Set()) => {
            if (visited.has(nodeId)) return 0; // Cycle detection
            visited.add(nodeId);
            
            const node = this.visualizer.nodes.find(n => n.id === nodeId);
            if (!node || !node.children || node.children.length === 0) return 0;
            
            let maxChildDepth = 0;
            for (const childId of node.children) {
                const childDepth = getDepth(childId, visited);
                maxChildDepth = Math.max(maxChildDepth, childDepth);
            }
            return maxChildDepth + 1;
        };
        
        this.visualizer.nodes.forEach(node => {
            if (SCXMLVisualizer.isCompoundOrParallel(node) && !node.collapsed) {
                const depth = getDepth(node.id);
                compoundsWithDepth.push({ node, depth });
            }
        });
        
        // Sort by depth ascending (deepest children first, shallowest parents last)
        compoundsWithDepth.sort((a, b) => a.depth - b.depth);
        
        logger.debug(`  Processing ${compoundsWithDepth.length} compounds in bottom-up order:`);
        compoundsWithDepth.forEach(({ node, depth }) => {
            logger.debug(`    depth=${depth}: ${node.id}`);
        });
        
        // Update bounds in bottom-up order
        compoundsWithDepth.forEach(({ node }) => {
            this.visualizer.updateCompoundBounds(node);
        });

        // Re-optimize snap points after compound bounds update
        // Compound bounds changes affect node positions and sizes
        if (compoundsWithDepth.length > 0) {
            logger.debug('Re-optimizing snap points after compound bounds update...');
            this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(this.visualizer.allLinks, this.visualizer.nodes);
        }

        logger.debug('Layout application complete');
    }

    updateCompoundBounds(compoundNode) {
        // Skip collapsed nodes - they use fixed minimum size, not child-based bounds
        if (compoundNode.collapsed) {
            logger.debug(`[updateCompoundBounds] ${compoundNode.id}: Skipped (collapsed)`);
            return;
        }

        if (!compoundNode.children || compoundNode.children.length === 0) {
            return;
        }

        const allChildNodes = compoundNode.children
            .map(childId => this.visualizer.nodes.find(n => n.id === childId))
            .filter(child => child);

        const childNodesWithCoords = allChildNodes
            .filter(child => child.x !== undefined && child.y !== undefined);

        if (childNodesWithCoords.length === 0) {
            // All children exist but none have coordinates - expected when ELK doesn't layout nested hierarchies
            if (allChildNodes.length > 0) {
                logger.debug(`[updateCompoundBounds] ${compoundNode.id}: Children not yet laid out by ELK, skipping bounds calculation`);
            }
            return;
        } else if (childNodesWithCoords.length < allChildNodes.length) {
            // Some children have coordinates but not all - unexpected
            logger.warn(`[updateCompoundBounds] ${compoundNode.id}: Partial child coordinates (${childNodesWithCoords.length}/${allChildNodes.length})`);
        }

        const childNodes = childNodesWithCoords;

        // Debug: Log child positions
        logger.debug(`[updateCompoundBounds] ${compoundNode.id}: Processing ${childNodes.length} children:`);
        childNodes.forEach(child => {
            logger.debug(`  - ${child.id}: x=${child.x.toFixed(1)}, y=${child.y.toFixed(1)}, width=${child.width}, height=${child.height}`);
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

        logger.debug(`[updateCompoundBounds] ${compoundNode.id}: Updated to (${compoundNode.x.toFixed(1)}, ${compoundNode.y.toFixed(1)}) size=${compoundNode.width.toFixed(1)}x${compoundNode.height.toFixed(1)}`);

        // Push away overlapping non-child states after bounds update
        // Use stored drag direction if available (from drag handlers) for natural sliding
        if (this.visualizer.collisionDetector) {
            const dragDx = compoundNode._dragDx || 0;
            const dragDy = compoundNode._dragDy || 0;
            this.visualizer.collisionDetector.pushAwayOverlappingStates(compoundNode, dragDx, dragDy);
        }
    }

    findCompoundParent(nodeId) {
        for (const node of this.visualizer.nodes) {
            if (SCXMLVisualizer.isCompoundOrParallel(node) && 
                !node.collapsed && 
                node.children && 
                node.children.includes(nodeId)) {
                return node;
            }
        }
        return null;
    }

    findTopmostCompoundParent(nodeId) {
        let currentId = nodeId;
        let topmostParent = null;
        
        while (true) {
            const parent = this.visualizer.findCompoundParent(currentId);
            if (!parent) {
                break;
            }
            topmostParent = parent;
            currentId = parent.id;
        }
        
        return topmostParent;
    }

    getAllDescendantIds(parentId) {
        const descendants = [];
        const parent = this.visualizer.nodes.find(n => n.id === parentId);
        
        if (parent && parent.children) {
            parent.children.forEach(childId => {
                descendants.push(childId);
                // Recursively add grandchildren
                const grandchildren = this.visualizer.getAllDescendantIds(childId);
                descendants.push(...grandchildren);
            });
        }
        
        return descendants;
    }
}
