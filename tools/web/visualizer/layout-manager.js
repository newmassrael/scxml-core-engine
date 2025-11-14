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
            console.log(`${indent}[buildELKNode] Building ${node.id} (${node.type}, collapsed=${node.collapsed})`);

            const elkNode = {
                id: node.id,
                width: this.visualizer.getNodeWidth(node),
                height: this.visualizer.getNodeHeight(node)
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
                    const childNode = this.visualizer.nodes.find(n => n.id === childId);
                    if (childNode) {
                        if (visibleNodeIds.has(childId)) {
                            console.log(`${indent}    → Adding child ${childId} (visible)`);
                            elkNode.children.push(buildELKNode(childNode, depth + 1));
                        } else {
                            console.log(`${indent}    → Skipping child ${childId} (not visible)`);
                        }
                    } else {
                        console.warn(`${indent}    → Child ${childId} not found in this.visualizer.nodes!`);
                    }
                });

                console.log(`${indent}  ${node.id} elkNode.children.length = ${elkNode.children.length}`);
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

        console.log(`[buildELKGraph] Total visible nodes: ${visibleNodes.length}, Top-level nodes: ${topLevelNodes.length}`);
        console.log(`  Top-level: ${topLevelNodes.map(n => n.id).join(', ')}`);

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
        console.log('Applying ELK layout to nodes...');
        console.log(`  this.visualizer.nodes count: ${this.visualizer.nodes.length}, nodes: ${this.visualizer.nodes.map(n => `${n.id}(${n.type})`).join(', ')}`);

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
            const node = this.visualizer.nodes.find(n => n.id === elkNode.id);
            if (node) {
                node.x = elkNode.x + offsetX + elkNode.width / 2;
                node.y = elkNode.y + offsetY + elkNode.height / 2;
                node.width = elkNode.width;
                node.height = elkNode.height;

                console.log(`${indent}  ${node.id}: (${node.x.toFixed(1)}, ${node.y.toFixed(1)}) size=${node.width}x${node.height}, offset=(${offsetX}, ${offsetY})`);
            } else {
                console.warn(`${indent}  ELK node not found in this.visualizer.nodes: ${elkNode.id} (possibly child state or collapsed)`);
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
        console.log(`  Total nodes: ${this.visualizer.nodes.length}`);
        this.visualizer.nodes.forEach(node => {
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
                    .map(childId => this.visualizer.nodes.find(n => n.id === childId))
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
                const link = this.visualizer.allLinks.find(l => l.id === elkEdge.id);
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
        this.visualizer.allLinks.forEach(link => {
            if (link.elkSections) {
                delete link.elkSections;
            }
        });
        console.log('[LAYOUT] Invalidated ELK edge routing (will use optimizer routing)');

        // Optimize snap point assignments to minimize intersections
        console.log('Optimizing snap point assignments...');
        this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(this.visualizer.allLinks, this.visualizer.nodes);

        // Update all compound/parallel bounds to ensure they contain all children
        // Process in bottom-up order: children first, then parents
        console.log('Updating compound container bounds...');
        
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
            this.visualizer.updateCompoundBounds(node);
        });

        // Re-optimize snap points after compound bounds update
        // Compound bounds changes affect node positions and sizes
        if (compoundsWithDepth.length > 0) {
            console.log('Re-optimizing snap points after compound bounds update...');
            this.visualizer.layoutOptimizer.optimizeSnapPointAssignments(this.visualizer.allLinks, this.visualizer.nodes);
        }

        console.log('Layout application complete');
    }

    updateCompoundBounds(compoundNode) {
        if (!compoundNode.children || compoundNode.children.length === 0) {
            return;
        }

        const childNodes = compoundNode.children
            .map(childId => this.visualizer.nodes.find(n => n.id === childId))
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

    findCompoundParent(nodeId) {
        for (const node of this.visualizer.nodes) {
            if ((node.type === 'compound' || node.type === 'parallel') && 
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
