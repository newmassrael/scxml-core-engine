/**
 * Action formatter utilities for SCXML visualizer
 * Provides smart, context-aware formatting for SCXML actions (send, raise, assign, etc.)
 */

const ActionFormatter = (function() {
    'use strict';

    // Truncation limits for display optimization
    const TRUNCATION = {
        CONTENT: 30,      // W3C SCXML 6.2.3 - content display limit for readability
        PARAMS: 40,       // W3C SCXML 6.2.4 - params display limit (can show more data)
        EXPRESSION: 30,   // General expression display limit (assign, log, etc.)
        LOG_TEXT: 30      // Log message display limit
    };

    // Detail line prefix for hierarchical display
    const DETAIL_PREFIX = '   ↳ ';

    /**
     * Check if a value is empty (null, undefined, or empty string)
     * @param {*} value - Value to check
     * @returns {boolean} True if value is empty
     */
    function isEmpty(value) {
        return value === undefined || value === null || value === '';
    }

    /**
     * Check if an array has items
     * @param {Array} arr - Array to check
     * @returns {boolean} True if array exists and has items
     */
    function hasItems(arr) {
        return arr && Array.isArray(arr) && arr.length > 0;
    }

    /**
     * Format send action main line
     * @param {Object} action - Send action object
     * @returns {string} Formatted main line (e.g., "📤 Send event: event1")
     */
    function formatSendAction(action) {
        const icon = '📤';
        const eventName = action.event || action.eventexpr || '?';
        const isDynamic = !action.event && action.eventexpr;

        if (isDynamic) {
            return `${icon} Send event: ${eventName} (dynamic)`;
        }
        return `${icon} Send event: ${eventName}`;
    }

    /**
     * Format send action detail lines
     * @param {Object} action - Send action object
     * @returns {Array<string>} Array of detail lines (e.g., ["↳ content: 123", "↳ delay: 5s"])
     */
    function formatSendDetails(action) {
        const details = [];

        // W3C SCXML 6.2.3: <send> content attribute for message payload
        if (!isEmpty(action.content)) {
            const contentStr = String(action.content);
            const truncated = contentStr.length > TRUNCATION.CONTENT
                ? contentStr.substring(0, TRUNCATION.CONTENT) + '...'
                : contentStr;
            details.push(`${DETAIL_PREFIX}content: ${truncated}`);
        } else if (!isEmpty(action.contentexpr)) {
            details.push(`${DETAIL_PREFIX}contentexpr: ${action.contentexpr}`);
        }

        if (!isEmpty(action.data)) {
            details.push(`${DETAIL_PREFIX}data: ${action.data}`);
        }

        // W3C SCXML 6.2.4: <param> elements for name-value pairs in send
        if (hasItems(action.params)) {
            const paramStrs = action.params.map(p => `${p.name}=${p.expr || '?'}`);
            const paramsStr = paramStrs.join(', ');
            const truncated = paramsStr.length > TRUNCATION.PARAMS
                ? paramsStr.substring(0, TRUNCATION.PARAMS) + '...'
                : paramsStr;
            details.push(`${DETAIL_PREFIX}params: ${truncated}`);
    }

        // W3C SCXML 6.2.5: namelist attribute for variable names to send
        if (!isEmpty(action.namelist)) {
            details.push(`${DETAIL_PREFIX}namelist: ${action.namelist}`);
        }

        // W3C SCXML 6.2.1: target attribute for event destination (only show if not default)
        const target = action.target || action.targetexpr;
        if (!isEmpty(target) && target !== '#_internal') {
            const isDynamic = isEmpty(action.target) && !isEmpty(action.targetexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}target: ${target}${suffix}`);
        }

        // W3C SCXML 6.2.6: delay/delayexpr attributes for delayed event delivery (static or dynamic)
        const delay = action.delay || action.delayexpr;
        if (!isEmpty(delay)) {
            const isDynamic = isEmpty(action.delay) && !isEmpty(action.delayexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}delay: ${delay}${suffix}`);
        }

        // W3C SCXML 6.2.2: type attribute for Event I/O Processor selection (only show if not default)
        const type = action.type || action.typeexpr;
        if (!isEmpty(type) && type !== 'scxml' && type !== 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor') {
            const isDynamic = isEmpty(action.type) && !isEmpty(action.typeexpr);
            const suffix = isDynamic ? ' (dynamic)' : '';
            // Simplify BasicHTTP processor type for display
            const displayType = type === 'http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor' ? 'BasicHTTP' : type;
            details.push(`${DETAIL_PREFIX}type: ${displayType}${suffix}`);
        }

        // W3C SCXML 6.2.7: sendid attribute for unique event identification and cancellation
        if (!isEmpty(action.sendid)) {
            details.push(`${DETAIL_PREFIX}id: ${action.sendid}`);
        } else if (!isEmpty(action.idlocation)) {
            details.push(`${DETAIL_PREFIX}idlocation: ${action.idlocation}`);
        }

        return details;
    }

    /**
     * Format raise action
     * @param {Object} action - Raise action object
     * @returns {string} Formatted action text
     */
    function formatRaiseAction(action) {
        const icon = '📢';
        const eventName = action.event || '?';
        return `${icon} Raise: ${eventName}`;
    }

    /**
     * Format assign action
     * @param {Object} action - Assign action object
     * @returns {string} Formatted action text
     */
    function formatAssignAction(action) {
        const icon = '💾';
        const location = action.location || '?';
        const expr = action.expr || '?';
        const truncatedExpr = expr.length > TRUNCATION.EXPRESSION
            ? expr.substring(0, TRUNCATION.EXPRESSION) + '...'
            : expr;
        return `${icon} Assign: ${location} = ${truncatedExpr}`;
    }

    /**
     * Format log action
     * @param {Object} action - Log action object
     * @returns {string} Formatted action text
     */
    function formatLogAction(action) {
        const text = action.label || action.expr || '?';
        const truncated = text.length > TRUNCATION.LOG_TEXT
            ? text.substring(0, TRUNCATION.LOG_TEXT) + '...'
            : text;
        return `📝 Log: ${truncated}`;
    }

    /**
     * Format cancel action
     * @param {Object} action - Cancel action object
     * @returns {string} Formatted action text
     */
    function formatCancelAction(action) {
        const id = action.sendid || action.sendidexpr || '?';
        const isDynamic = !action.sendid && action.sendidexpr;
        const suffix = isDynamic ? ' (dynamic)' : '';
        return `🚫 Cancel: ${id}${suffix}`;
    }

    /**
     * Format script action
     * W3C SCXML 5.9: Execute ECMAScript code
     * @param {Object} action - Script action object
     * @returns {string} Formatted action text
     */
    function formatScriptAction(action) {
        const icon = '📜';
        const content = action.content || action.src || '';

        if (!content) {
            return `${icon} Script: (empty)`;
        }

        // Show preview of script content
        const preview = content.trim().substring(0, TRUNCATION.EXPRESSION);
        const truncated = content.trim().length > TRUNCATION.EXPRESSION
            ? preview + '...'
            : preview;

        // If it's a source file reference
        if (action.src) {
            return `${icon} Script: src="${truncated}"`;
        }

        return `${icon} Script: ${truncated}`;
    }

    /**
     * Format foreach action main line
     * W3C SCXML 3.12.2: Iteration over arrays
     * @param {Object} action - Foreach action object
     * @returns {string} Formatted main line (e.g., "🔁 foreach: item in array")
     */
    function formatForeachAction(action) {
        const icon = '🔁';
        const item = action.item || '<missing>';
        const array = action.array || '<missing>';
        const index = action.index;

        let text = `${icon} foreach: ${item} in ${array}`;
        if (index) {
            text += ` (index: ${index})`;
        }

        return text;
    }

    /**
     * Format foreach action detail lines
     * Shows nested actions if present
     * @param {Object} action - Foreach action object
     * @returns {Array<string>} Array of detail lines for nested actions
     */
    function formatForeachDetails(action) {
        const details = [];
        const actions = action.actions || [];

        // If foreach has nested actions, show them
        if (actions.length > 0) {
            const nestedPrefix = '      ↳ ';  // Same as if/elseif nested actions

            actions.forEach(a => {
                const type = a.actionType || a.type || '?';
                let actionText = '';

                if (type === 'raise') {
                    actionText = `raise ${a.event || '?'}`;
                } else if (type === 'assign') {
                    const location = a.location || '?';
                    const expr = a.expr || '?';
                    actionText = `assign ${location} = ${expr}`;
                } else if (type === 'send') {
                    actionText = `send ${a.event || a.eventexpr || '?'}`;
                } else if (type === 'log') {
                    actionText = `log ${a.label || a.expr || ''}`;
                } else if (type === 'cancel') {
                    actionText = 'cancel';
                } else {
                    actionText = type;
                }

                details.push(`${nestedPrefix}${actionText}`);
            });
        }

        return details;
    }

    /**
     * Format if action main line
     * W3C SCXML 3.12.1: Conditional execution with if/elseif/else structure
     * @param {Object} action - If action object
     * @returns {string} Formatted main line (e.g., "🔀 if/elseif/else [3 branches]")
     */
    function formatIfAction(action) {
        const icon = '🔀';
        const branches = action.branches || [];
        const branchCount = branches.length;

        // No branches case (shouldn't happen, but handle gracefully)
        if (branchCount === 0) {
            const cond = action.cond || '?';
            const truncatedCond = cond.length > TRUNCATION.EXPRESSION
                ? cond.substring(0, TRUNCATION.EXPRESSION) + '...'
                : cond;
            return `${icon} if [${truncatedCond}]`;
        }

        // Build branch type summary (if/elseif/else)
        const branchTypes = branches.map((b, i) => {
            if (b.isElse) return 'else';
            if (i === 0) return 'if';
            return 'elseif';
        }).join('/');

        return `${icon} ${branchTypes} [${branchCount} branch${branchCount > 1 ? 'es' : ''}]`;
    }

    /**
     * Format if action detail lines
     * Shows each branch with its condition and nested actions summary
     * @param {Object} action - If action object
     * @returns {Array<string>} Array of detail lines for each branch
     */
    function formatIfDetails(action) {
        const details = [];
        const branches = action.branches || [];

        // Debug: Log action structure to help diagnose
        if (branches.length > 0 && console && console.log) {
            console.log('[ActionFormatter] if action structure:', {
                actionCond: action.cond,
                branchCount: branches.length,
                branches: branches.map((b, i) => ({
                    index: i,
                    cond: b.cond,
                    isElse: b.isElse,
                    actionCount: (b.actions || []).length
                }))
            });
        }

        branches.forEach((branch, idx) => {
            // Build branch header
            let branchType;
            if (branch.isElse) {
                branchType = 'else:';
            } else {
                // Get condition from branch (C++ uses branch.condition for ALL branches including if)
                // Try both field names (cond/condition) for compatibility
                const cond = branch.cond || branch.condition || '?';
                const truncatedCond = cond.length > TRUNCATION.EXPRESSION
                    ? cond.substring(0, TRUNCATION.EXPRESSION) + '...'
                    : cond;
                
                // Determine branch type by index
                if (idx === 0) {
                    branchType = `if: [${truncatedCond}]`;
                } else {
                    branchType = `elseif: [${truncatedCond}]`;
                }
            }

            // Add branch header line
            details.push(`${DETAIL_PREFIX}${branchType}`);

            // Add each nested action on separate line with deeper indent
            const actions = branch.actions || [];
            // Use non-breaking spaces to preserve indent in SVG rendering
            const nestedPrefix = '      ↳ ';  // 6 non-breaking spaces + arrow
            
            actions.forEach(a => {
                const type = a.actionType || a.type || '?';
                let actionText = '';
                
                if (type === 'raise') {
                    actionText = `raise ${a.event || '?'}`;
                } else if (type === 'assign') {
                    const location = a.location || '?';
                    const expr = a.expr || '?';
                    actionText = `assign ${location} = ${expr}`;
                } else if (type === 'send') {
                    actionText = `send ${a.event || a.eventexpr || '?'}`;
                } else if (type === 'log') {
                    actionText = 'log';
                } else if (type === 'cancel') {
                    actionText = 'cancel';
                } else {
                    actionText = type;
                }
                
                details.push(`${nestedPrefix}${actionText}`);
            });
        });

        return details;
    }

    /**
     * Format any action type
     * @param {Object} action - Action object
     * @returns {Object} {main: string, details: Array<string>}
     */
    function formatAction(action) {
        // Input validation
        if (!action || typeof action !== 'object') {
            return { main: '⚙️ [Invalid action]', details: [] };
        }

        const actionType = action.actionType || action.type;

        // Handle missing action type
        if (!actionType) {
            return { main: '⚙️ [Unknown action type]', details: [] };
        }

        let main = '';
        let details = [];

        try {
            switch (actionType) {
                case 'send':
                    main = formatSendAction(action);
                    details = formatSendDetails(action);
                    break;
                case 'raise':
                    main = formatRaiseAction(action);
                    break;
                case 'assign':
                    main = formatAssignAction(action);
                    break;
                case 'log':
                    main = formatLogAction(action);
                    break;
                case 'cancel':
                    main = formatCancelAction(action);
                    break;
                case 'script':
                    main = formatScriptAction(action);
                    break;
                case 'foreach':
                    main = formatForeachAction(action);
                    details = formatForeachDetails(action);
                    break;
                case 'if':
                    main = formatIfAction(action);
                    details = formatIfDetails(action);
                    break;
                default:
                    main = `⚙️ ${actionType}`;
            }
        } catch (error) {
            // Graceful error handling - log to console but return safe fallback
            console.error('ActionFormatter error:', error, 'for action:', action);
            main = `⚙️ ${actionType} [Format error]`;
            details = [];
        }

        return { main, details };
    }

    // Public API
    return {
        formatAction: formatAction,
        formatSendAction: formatSendAction,
        formatSendDetails: formatSendDetails,
        formatRaiseAction: formatRaiseAction,
        formatAssignAction: formatAssignAction,
        formatLogAction: formatLogAction,
        formatCancelAction: formatCancelAction,
        formatScriptAction: formatScriptAction,
        formatForeachAction: formatForeachAction,
        formatForeachDetails: formatForeachDetails,
        formatIfAction: formatIfAction,
        formatIfDetails: formatIfDetails
    };
})();
