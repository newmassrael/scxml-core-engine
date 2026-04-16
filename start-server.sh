#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

# Local development server with GitHub Pages URL structure
# Usage: ./start-server.sh [port]

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PORT="${1:-8000}"

# Kill existing process on port
kill_port() {
    if command -v fuser &> /dev/null; then
        fuser -k "$PORT/tcp" 2>/dev/null && echo "Killed existing process on port $PORT" || true
    elif command -v lsof &> /dev/null; then
        lsof -ti:"$PORT" | xargs kill -9 2>/dev/null && echo "Killed existing process on port $PORT" || true
    fi
    sleep 1
}

# Create symlinks for GitHub Pages URL compatibility
create_symlinks() {
    echo "Creating symlinks for GitHub Pages URL structure..."

    # visualizer/resources -> ../resources
    if [ ! -e "$SCRIPT_DIR/visualizer/resources" ]; then
        ln -sf ../resources "$SCRIPT_DIR/visualizer/resources"
        echo "  Created: visualizer/resources -> ../resources"
    fi

    # visualizer/tools -> ../tools
    if [ ! -e "$SCRIPT_DIR/visualizer/tools" ]; then
        ln -sf ../tools "$SCRIPT_DIR/visualizer/tools"
        echo "  Created: visualizer/tools -> ../tools"
    fi

    # visualizer/doom -> ../examples/doom_wasm/build (only if build exists)
    if [ -d "$SCRIPT_DIR/examples/doom_wasm/build" ]; then
        if [ ! -e "$SCRIPT_DIR/visualizer/doom" ]; then
            ln -sf ../examples/doom_wasm/build "$SCRIPT_DIR/visualizer/doom"
            echo "  Created: visualizer/doom -> ../examples/doom_wasm/build"
        fi
    else
        echo "  Skipped: visualizer/doom (build examples/doom_wasm first)"
    fi
}

# Remove symlinks on exit
cleanup() {
    echo ""
    echo "Cleaning up symlinks..."
    rm -f "$SCRIPT_DIR/visualizer/doom"
    rm -f "$SCRIPT_DIR/visualizer/resources"
    rm -f "$SCRIPT_DIR/visualizer/tools"
    echo "Done."
}

# Set trap to cleanup on exit
trap cleanup EXIT

# Kill existing process on port
kill_port

# Create symlinks
create_symlinks

echo ""
echo "Starting local server on port $PORT..."
echo ""
echo "URLs (same as GitHub Pages):"
echo "  Visualizer: http://localhost:$PORT/visualizer/visualizer.html"
echo "  Codegen:    http://localhost:$PORT/visualizer/codegen.html"
if [ -d "$SCRIPT_DIR/examples/doom_wasm/build" ]; then
echo "  DOOM:       http://localhost:$PORT/visualizer/doom/"
fi
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Start server from project root
cd "$SCRIPT_DIR"
python3 -m http.server "$PORT"
