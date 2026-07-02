#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

# Local development server with GitHub Pages URL structure
# Usage: ./start-server.sh [port]

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
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
    if [ ! -e "$REPO_ROOT/visualizer/resources" ]; then
        ln -sf ../resources "$REPO_ROOT/visualizer/resources"
        echo "  Created: visualizer/resources -> ../resources"
    fi

    # visualizer/tools -> ../tools
    if [ ! -e "$REPO_ROOT/visualizer/tools" ]; then
        ln -sf ../tools "$REPO_ROOT/visualizer/tools"
        echo "  Created: visualizer/tools -> ../tools"
    fi

    # visualizer/doom -> ../examples/doom_wasm/build (only if build exists)
    if [ -d "$REPO_ROOT/examples/doom_wasm/build" ]; then
        if [ ! -e "$REPO_ROOT/visualizer/doom" ]; then
            ln -sf ../examples/doom_wasm/build "$REPO_ROOT/visualizer/doom"
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
    rm -f "$REPO_ROOT/visualizer/doom"
    rm -f "$REPO_ROOT/visualizer/resources"
    rm -f "$REPO_ROOT/visualizer/tools"
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
if [ -d "$REPO_ROOT/examples/doom_wasm/build" ]; then
echo "  DOOM:       http://localhost:$PORT/visualizer/doom/"
fi
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Start server from project root
cd "$REPO_ROOT"
python3 -m http.server "$PORT"
