/**
 * Invoke formatter utilities for SCXML visualizer
 * Provides detailed formatting for W3C SCXML invoke elements
 */

const InvokeFormatter = (function() {
    'use strict';

    // Truncation limits for display optimization (matching ActionFormatter)
    const TRUNCATION = {
        TYPE: 50,          // Invoke type display limit
        SRC: 40,           // Source path display limit
        NAMELIST: 40,      // Namelist display limit
        CONTENT: 30,       // Content preview limit
        PARAMS: 50,        // Params display limit
        FINALIZE: 30       // Finalize script preview limit
    };

    // Detail line prefix for hierarchical display (matching ActionFormatter)
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
     * Format invoke information
     * W3C SCXML 6.4: Invoke external state machines or services
     * @param {Object} invokeData - Invoke data object from state
     * @returns {Object} {main: string, details: Array<string>}
     */
    function formatInvokeInfo(invokeData) {
        // Skip if no invoke data
        if (!invokeData || !invokeData.hasInvoke) {
            return { main: '', details: [] };
        }

        const icon = '🚀';

        // Determine type (static or dynamic)
        const type = invokeData.invokeType || invokeData.invokeTypeExpr || '?';
        const isDynamicType = isEmpty(invokeData.invokeType) && !isEmpty(invokeData.invokeTypeExpr);

        // Simplify common SCXML type for display
        const typeDisplay = type === 'http://www.w3.org/TR/scxml/'
            ? 'SCXML'
            : (type.length > TRUNCATION.TYPE ? type.substring(0, TRUNCATION.TYPE) + '...' : type);

        // Build main line
        const main = isDynamicType
            ? `${icon} ${typeDisplay} (dynamic)`
            : `${icon} ${typeDisplay}`;

        const details = [];

        // W3C SCXML 6.4.1: Type attribute (show if not default SCXML)
        if (type && type !== 'http://www.w3.org/TR/scxml/') {
            const suffix = isDynamicType ? ' (dynamic)' : '';
            details.push(`${DETAIL_PREFIX}type: ${typeDisplay}${suffix}`);
        }

        // W3C SCXML 6.4.2: ID or idlocation attribute
        if (!isEmpty(invokeData.invokeId)) {
            details.push(`${DETAIL_PREFIX}id: ${invokeData.invokeId}`);
        } else if (!isEmpty(invokeData.invokeIdLocation)) {
            details.push(`${DETAIL_PREFIX}idlocation: ${invokeData.invokeIdLocation}`);
        }

        // W3C SCXML 6.4.3: Source (static or dynamic)
        const src = invokeData.invokeSrc || invokeData.invokeSrcExpr;
        if (!isEmpty(src)) {
            const isDynamicSrc = isEmpty(invokeData.invokeSrc) && !isEmpty(invokeData.invokeSrcExpr);
            const suffix = isDynamicSrc ? ' (dynamic)' : '';
            const truncated = src.length > TRUNCATION.SRC
                ? src.substring(0, TRUNCATION.SRC) + '...'
                : src;
            details.push(`${DETAIL_PREFIX}src: ${truncated}${suffix}`);
        }

        // W3C SCXML 6.4.4: Content (inline SCXML or dynamic expression)
        if (!isEmpty(invokeData.invokeContent)) {
            // Show preview of inline content
            const contentPreview = invokeData.invokeContent.trim().substring(0, TRUNCATION.CONTENT);
            const hasMore = invokeData.invokeContent.trim().length > TRUNCATION.CONTENT;
            details.push(`${DETAIL_PREFIX}content: <scxml...> (inline)`);
        } else if (!isEmpty(invokeData.invokeContentExpr)) {
            const truncated = invokeData.invokeContentExpr.length > TRUNCATION.CONTENT
                ? invokeData.invokeContentExpr.substring(0, TRUNCATION.CONTENT) + '...'
                : invokeData.invokeContentExpr;
            details.push(`${DETAIL_PREFIX}contentexpr: ${truncated}`);
        }

        // W3C SCXML 6.4.5: Params (name-value pairs to pass to child)
        if (hasItems(invokeData.invokeParams)) {
            const paramStrs = invokeData.invokeParams.map(p => {
                const name = p.name || '?';
                const expr = p.expr || p.location || '?';
                return `${name}=${expr}`;
            });
            const paramsStr = paramStrs.join(', ');
            const truncated = paramsStr.length > TRUNCATION.PARAMS
                ? paramsStr.substring(0, TRUNCATION.PARAMS) + '...'
                : paramsStr;
            details.push(`${DETAIL_PREFIX}params: ${truncated}`);
        }

        // W3C SCXML 6.4.6: Namelist (variable names to pass)
        if (!isEmpty(invokeData.invokeNamelist)) {
            const truncated = invokeData.invokeNamelist.length > TRUNCATION.NAMELIST
                ? invokeData.invokeNamelist.substring(0, TRUNCATION.NAMELIST) + '...'
                : invokeData.invokeNamelist;
            details.push(`${DETAIL_PREFIX}namelist: ${truncated}`);
        }

        // W3C SCXML 6.4.7: AutoForward (automatic event forwarding)
        if (invokeData.invokeAutoForward === true) {
            details.push(`${DETAIL_PREFIX}autoforward: true`);
        }

        // W3C SCXML 6.5: Finalize (script to execute when child sends events)
        if (!isEmpty(invokeData.invokeFinalize)) {
            const preview = invokeData.invokeFinalize.trim().substring(0, TRUNCATION.FINALIZE);
            const truncated = invokeData.invokeFinalize.trim().length > TRUNCATION.FINALIZE
                ? preview + '...'
                : preview;
            details.push(`${DETAIL_PREFIX}finalize: ${truncated}`);
        }

        return { main, details };
    }

    // Public API
    return {
        formatInvokeInfo: formatInvokeInfo
    };
})();
