// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Panel Controller - Handles bottom panel resizing and minimize/maximize
 */

// Configuration constants
const PANEL_CONFIG = {
    MIN_HEIGHT: 100,
    MAX_HEIGHT: 800,
    MINIMIZED_HEIGHT: 40,
    DEFAULT_HEIGHT: 600,
    STORAGE_KEY: 'scxml-visualizer-panel-height',
    DEBUG_MODE: false,  // Set to true to enable debug logging
    START_MINIMIZED: false  // Start with panel expanded by default
};

class PanelController {
    constructor() {
        this.panel = document.getElementById('bottom-panels-container');
        this.resizeHandle = document.getElementById('panel-resize-handle');
        this.toggleBtn = document.getElementById('panel-toggle-btn');

        this.isResizing = false;
        this.isMinimized = false;
        this.rafId = null;
        this.pendingHeight = null;

        // Load saved height from localStorage or use default
        this.lastHeight = this.loadSavedHeight();

        this.log('Panel Controller initialized', {
            panel: !!this.panel,
            resizeHandle: !!this.resizeHandle,
            toggleBtn: !!this.toggleBtn,
            initialHeight: this.lastHeight
        });

        this.init();
    }

    /**
     * Conditional logging helper
     */
    log(...args) {
        if (PANEL_CONFIG.DEBUG_MODE) {
            console.log('[Panel Controller]', ...args);
        }
    }

    /**
     * Load saved panel height from localStorage
     * @returns {number} Saved height or default height
     */
    loadSavedHeight() {
        try {
            const saved = localStorage.getItem(PANEL_CONFIG.STORAGE_KEY);
            if (saved) {
                const height = parseInt(saved);
                if (!isNaN(height) && height >= PANEL_CONFIG.MIN_HEIGHT && height <= PANEL_CONFIG.MAX_HEIGHT) {
                    return height;
                }
            }
        } catch (e) {
            // localStorage might not be available
        }

        // Fallback to computed style or default
        if (this.panel) {
            const currentHeight = parseInt(getComputedStyle(this.panel).maxHeight);
            return isNaN(currentHeight) ? PANEL_CONFIG.DEFAULT_HEIGHT : currentHeight;
        }

        return PANEL_CONFIG.DEFAULT_HEIGHT;
    }

    /**
     * Save panel height to localStorage
     * @param {number} height - Height to save
     */
    saveHeight(height) {
        try {
            localStorage.setItem(PANEL_CONFIG.STORAGE_KEY, height.toString());
            this.log('Height saved to localStorage:', height);
        } catch (e) {
            // localStorage might not be available
        }
    }

    /**
     * Initialize event listeners
     */
    init() {
        if (!this.panel || !this.resizeHandle || !this.toggleBtn) {
            console.error('[Panel Controller] Required elements not found:', {
                panel: !!this.panel,
                resizeHandle: !!this.resizeHandle,
                toggleBtn: !!this.toggleBtn
            });
            return;
        }

        // Resize handle drag
        this.resizeHandle.addEventListener('mousedown', (e) => {
            // Don't start resize if clicking the toggle button
            if (e.target.closest('.panel-toggle-btn')) {
                return;
            }
            this.startResize(e);
        });

        document.addEventListener('mousemove', (e) => this.resize(e));
        document.addEventListener('mouseup', () => this.stopResize());

        // Toggle button click
        this.toggleBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            this.toggleMinimize();
        });

        // Apply initial minimized state if configured
        if (PANEL_CONFIG.START_MINIMIZED) {
            this.isMinimized = true;
            this.panel.style.maxHeight = `${PANEL_CONFIG.MINIMIZED_HEIGHT}px`;
            this.panel.classList.add('minimized');
            this.log('Panel started in minimized state');
        }

        this.log('Initialization complete');
    }

    /**
     * Start resize operation
     * @param {MouseEvent} e - Mouse event
     */
    startResize(e) {
        this.isResizing = true;
        this.startY = e.clientY;
        this.startHeight = parseInt(getComputedStyle(this.panel).maxHeight);

        this.log('Resize started:', { startY: this.startY, startHeight: this.startHeight });

        // Disable transition during resize for smooth dragging
        this.panel.classList.add('resizing');

        document.body.style.cursor = 'ns-resize';
        document.body.style.userSelect = 'none';
    }

    /**
     * Handle resize dragging with requestAnimationFrame optimization
     * @param {MouseEvent} e - Mouse event
     */
    resize(e) {
        if (!this.isResizing || this.isMinimized) return;

        // Calculate new height (resize from top, so subtract delta)
        const deltaY = this.startY - e.clientY;
        let newHeight = this.startHeight + deltaY;

        // Constrain height
        newHeight = Math.max(PANEL_CONFIG.MIN_HEIGHT, Math.min(PANEL_CONFIG.MAX_HEIGHT, newHeight));

        // Store pending height for requestAnimationFrame
        this.pendingHeight = newHeight;

        // Use requestAnimationFrame for smooth updates
        if (!this.rafId) {
            this.rafId = requestAnimationFrame(() => {
                if (this.pendingHeight !== null) {
                    this.panel.style.maxHeight = `${this.pendingHeight}px`;
                    this.lastHeight = this.pendingHeight;
                    this.pendingHeight = null;
                }
                this.rafId = null;
            });
        }
    }

    /**
     * Stop resize operation and save height
     */
    stopResize() {
        if (!this.isResizing) return;

        this.log('Resize stopped, final height:', this.lastHeight);

        // Save height to localStorage
        this.saveHeight(this.lastHeight);

        // Cancel any pending animation frame
        if (this.rafId) {
            cancelAnimationFrame(this.rafId);
            this.rafId = null;
        }

        // Re-enable transition
        this.panel.classList.remove('resizing');

        this.isResizing = false;
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
    }

    /**
     * Toggle panel minimize/maximize state
     */
    toggleMinimize() {
        this.isMinimized = !this.isMinimized;

        this.log('Toggle minimize:', {
            isMinimized: this.isMinimized,
            currentHeight: getComputedStyle(this.panel).maxHeight,
            lastHeight: this.lastHeight
        });

        if (this.isMinimized) {
            // Save current height before minimizing
            const currentHeight = parseInt(getComputedStyle(this.panel).maxHeight);
            if (!isNaN(currentHeight) && currentHeight > PANEL_CONFIG.MINIMIZED_HEIGHT) {
                this.lastHeight = currentHeight;
                this.saveHeight(this.lastHeight);
                this.log('Saved current height before minimizing:', this.lastHeight);
            }
            // Explicitly set minimized height with inline style (higher priority than CSS)
            this.panel.style.maxHeight = `${PANEL_CONFIG.MINIMIZED_HEIGHT}px`;
            this.panel.classList.add('minimized');
        } else {
            this.panel.classList.remove('minimized');
            // Restore previous height
            this.panel.style.maxHeight = `${this.lastHeight}px`;
            this.log('Restored panel to height:', this.lastHeight);
        }
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    window.panelController = new PanelController();
});
