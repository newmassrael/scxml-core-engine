/**
 * The codegen WASM (`sce-build-wasm`), loaded once for the whole page.
 *
 * The GUI's structure, the annotation overlay and the browser code
 * generator all come from this one module. Each used to import it on its
 * own; three loaders of one module are three places to keep in step, and
 * the structure — the one the GUI cannot draw without — must not depend on
 * which of them happened to run first.
 *
 * ⚠ Failure policy is the CALLER's, not this module's: the overlay is
 * optional (an unmarked diagram is honest), the structure is not (a GUI
 * drawing some other reader's interpretation is exactly what the Rust
 * structure replaced). So `load()` rejects, and each caller decides.
 */
const SceBuildWasm = (() => {
    let ready = null;

    /** The initialised module; the same promise for every caller. */
    function load(base = 'wasm/') {
        if (!ready) {
            ready = (async () => {
                const module = await import(`./${base}sce_build.js`);
                await module.default(`./${base}sce_build_bg.wasm`);
                return module;
            })();
        }
        return ready;
    }

    /**
     * The statechart structure the GUI draws, built from the Rust model
     * (`sce_build::gui_structure`). Throws with the product's own message
     * when the product refuses the document — never falls back to another
     * reader's structure.
     */
    async function guiStructure(scxmlContent, scxmlName) {
        const module = await load();
        return JSON.parse(module.gui_structure(scxmlContent, scxmlName || 'untitled'));
    }

    return { load, guiStructure };
})();

if (typeof window !== 'undefined') {
    window.SceBuildWasm = SceBuildWasm;
}
