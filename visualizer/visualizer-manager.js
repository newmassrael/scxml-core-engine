// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Visualizer Manager - Encapsulates visualizer lifecycle management
 * 
 * Prevents global namespace pollution by managing visualizer instances
 * internally instead of polluting window object.
 */
class VisualizerManager {
    constructor() {
        this.parentVisualizer = null;
        this.childVisualizers = new Map();
        this.resizeHandlerBound = this.handleResize.bind(this);
        this.setupResizeHandler();
    }

    /**
     * Set the parent visualizer instance
     */
    setParent(visualizer) {
        this.parentVisualizer = visualizer;
    }

    /**
     * Add a child visualizer instance
     */
    addChild(sessionId, visualizer) {
        this.childVisualizers.set(sessionId, visualizer);
    }

    /**
     * Get child visualizer by session ID
     */
    getChild(sessionId) {
        return this.childVisualizers.get(sessionId);
    }

    /**
     * Check if child visualizer exists
     */
    hasChild(sessionId) {
        return this.childVisualizers.has(sessionId);
    }

    /**
     * Get all visualizers (parent + children)
     */
    getAllVisualizers() {
        const all = [];
        if (this.parentVisualizer) {
            all.push(this.parentVisualizer);
        }
        all.push(...this.childVisualizers.values());
        return all;
    }

    /**
     * Setup window resize handler
     */
    setupResizeHandler() {
        let resizeTimeout;
        
        const debouncedResize = () => {
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(() => {
                this.handleResize();
            }, 250);
        };

        window.addEventListener('resize', debouncedResize);
        
        // Store for cleanup
        this.resizeHandler = debouncedResize;
    }

    /**
     * Handle window resize - update all visualizers
     */
    handleResize() {
        this.getAllVisualizers().forEach(visualizer => {
            if (visualizer && visualizer.resize) {
                visualizer.resize();
            }
        });
    }

    /**
     * Cleanup - remove event listeners and clear references
     */
    destroy() {
        if (this.resizeHandler) {
            window.removeEventListener('resize', this.resizeHandler);
        }
        this.parentVisualizer = null;
        this.childVisualizers.clear();
    }
}
