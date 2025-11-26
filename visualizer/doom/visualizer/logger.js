// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Logger with Tag-based Filtering
 *
 * URL Parameters:
 * - ?debug                          Enable debug logs
 * - ?debug&verbose                  Show all logs (no filtering)
 * - ?debug&tags=CSP,OPTIMIZE        Show only CSP and OPTIMIZE logs (whitelist)
 * - ?debug&notags=CSP,LABEL         Hide CSP and LABEL logs (blacklist)
 * - ?debug&loglevel=warn            Set minimum log level (debug/info/warn/error)
 *
 * Examples:
 * - ?test=234&debug                           All debug logs
 * - ?test=234&debug&tags=Navigation          Only navigation logs
 * - ?test=234&debug&notags=CSP,LABEL POS     Exclude CSP and LABEL POS
 * - ?test=234&debug&verbose                  All logs (ignore filters)
 */

(function() {
    // Get location from window (main thread) or self (Worker)
    const globalLocation = typeof window !== 'undefined' ? window.location : self.location;
    const params = new URLSearchParams(globalLocation.search);
    const isDebugMode = params.has('debug');
    const verboseMode = params.has('verbose');

    // URL parameter filters
    // ?debug&tags=CSP,OPTIMIZE      -> Show only CSP, OPTIMIZE (whitelist)
    // ?debug&notags=CSP,LABEL        -> Hide CSP, LABEL (blacklist)
    const enableTags = params.get('tags')?.split(',').map(t => t.trim()) || [];
    const disableTags = params.get('notags')?.split(',').map(t => t.trim()) || [];

    // Log levels
    const LOG_LEVELS = {
        DEBUG: 0,
        INFO: 1,
        WARN: 2,
        ERROR: 3
    };

    const currentLevel = LOG_LEVELS[params.get('loglevel')?.toUpperCase()] ?? LOG_LEVELS.DEBUG;

    /**
     * Check if log message should be displayed
     * @param {string} message - Log message
     * @returns {boolean}
     */
    function shouldLog(message) {
        if (!isDebugMode) return false;

        // Verbose mode: show all logs
        if (verboseMode) return true;

        // Extract tag from message: "[TAG] ..." format
        const tagMatch = typeof message === 'string' ? message.match(/^\[([^\]]+)\]/) : null;
        const tag = tagMatch ? tagMatch[1] : null;

        // Whitelist mode (if tags parameter exists)
        if (enableTags.length > 0) {
            return tag && enableTags.some(t => tag.includes(t));
        }

        // Blacklist mode (if notags parameter exists)
        if (disableTags.length > 0) {
            return !tag || !disableTags.some(t => tag.includes(t));
        }

        // Default: show all
        return true;
    }

    /**
     * Global Logger object
     * Uses self for compatibility with both main thread and Web Workers
     * (In main thread, self === window; in Workers, self === WorkerGlobalScope)
     */
    const globalContext = typeof window !== 'undefined' ? window : self;

    // Assign to both globalContext and self to ensure accessibility
    globalContext.logger = self.logger = {
        debug(...args) {
            if (currentLevel <= LOG_LEVELS.DEBUG) {
                const msg = typeof args[0] === 'string' ? args[0] : '';
                if (shouldLog(msg)) {
                    console.log(...args);
                }
            }
        },

        info(...args) {
            if (currentLevel <= LOG_LEVELS.INFO) {
                console.info(...args);
            }
        },

        warn(...args) {
            if (currentLevel <= LOG_LEVELS.WARN) {
                const msg = typeof args[0] === 'string' ? args[0] : '';
                if (shouldLog(msg)) {
                    console.warn(...args);
                }
            }
        },

        error(...args) {
            if (currentLevel <= LOG_LEVELS.ERROR) {
                console.error(...args);
            }
        },

        /**
         * Get current logger configuration
         * @returns {Object} Logger config
         */
        getConfig() {
            return {
                debugMode: isDebugMode,
                verboseMode: verboseMode,
                enableTags: enableTags,
                disableTags: disableTags,
                logLevel: Object.keys(LOG_LEVELS).find(k => LOG_LEVELS[k] === currentLevel)
            };
        }
    };

    // Log initialization
    if (isDebugMode) {
        const config = globalContext.logger.getConfig();
        console.log('[Logger] Initialized:', config);

        if (enableTags.length > 0) {
            console.log('[Logger] Whitelist mode - showing tags:', enableTags);
        }
        if (disableTags.length > 0) {
            console.log('[Logger] Blacklist mode - hiding tags:', disableTags);
        }
        if (verboseMode) {
            console.log('[Logger] Verbose mode - showing all logs');
        }
    }
})();
