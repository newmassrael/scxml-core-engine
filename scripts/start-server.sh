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

    # web/visualizer/resources -> ../resources
    if [ ! -e "$REPO_ROOT/web/visualizer/resources" ]; then
        ln -sf ../../resources "$REPO_ROOT/web/visualizer/resources"
        echo "  Created: web/visualizer/resources -> ../resources"
    fi

    # web/visualizer/tools -> ../tools
    if [ ! -e "$REPO_ROOT/web/visualizer/tools" ]; then
        ln -sf ../../tools "$REPO_ROOT/web/visualizer/tools"
        echo "  Created: web/visualizer/tools -> ../tools"
    fi

    # web/visualizer/doom -> ../examples/doom_wasm/build (only if build exists)
    if [ -d "$REPO_ROOT/examples/doom_wasm/build" ]; then
        if [ ! -e "$REPO_ROOT/web/visualizer/doom" ]; then
            ln -sf ../../examples/doom_wasm/build "$REPO_ROOT/web/visualizer/doom"
            echo "  Created: web/visualizer/doom -> ../examples/doom_wasm/build"
        fi
    else
        echo "  Skipped: web/visualizer/doom (build examples/doom_wasm first)"
    fi
}

# Remove symlinks on exit
cleanup() {
    echo ""
    echo "Cleaning up symlinks..."
    rm -f "$REPO_ROOT/web/visualizer/doom"
    rm -f "$REPO_ROOT/web/visualizer/resources"
    rm -f "$REPO_ROOT/web/visualizer/tools"
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

# Serve from web/ so /visualizer/ matches the GitHub Pages URL structure
cd "$REPO_ROOT/web"
python3 -m http.server "$PORT"
