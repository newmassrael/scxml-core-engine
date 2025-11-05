#!/usr/bin/env python3
"""
Update C++ header files with centralized dual license headers.

Updates StaticExecutionEngine.h and all Helper files with
the dual license header from license_config.py.
"""

import sys
import os
import re
from pathlib import Path
from license_config import get_engine_dual_license_header


def find_existing_license_header(content: str) -> tuple[int, int]:
    """
    Find the start and end positions of existing license header.
    
    Returns:
        (start_pos, end_pos) or (-1, -1) if no header found
    """
    # Look for license header pattern
    # Headers start with /** and contain license keywords
    pattern = r'/\*\*[\s\S]*?(?:RSM Execution Engine|DUAL LICENSED|MIT License|Commercial License)[\s\S]*?\*/'
    
    match = re.search(pattern, content)
    if match:
        return match.start(), match.end()
    
    return -1, -1


def update_file_license(file_path: Path, new_header: str) -> bool:
    """
    Update a single file's license header.
    
    Args:
        file_path: Path to C++ header file
        new_header: New license header text
        
    Returns:
        True if file was updated, False otherwise
    """
    try:
        # Read file content
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Find existing header
        start_pos, end_pos = find_existing_license_header(content)
        
        if start_pos == -1:
            # No existing header - add at top
            print(f"  No existing header found, adding at top: {file_path.name}")
            new_content = new_header + "\n\n" + content
        else:
            # Replace existing header
            print(f"  Replacing existing header: {file_path.name}")
            new_content = content[:start_pos] + new_header + content[end_pos:]
        
        # Write updated content
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(new_content)
        
        return True
        
    except Exception as e:
        print(f"  ✗ Error updating {file_path}: {e}", file=sys.stderr)
        return False


def main():
    """Update all C++ engine header files with dual license."""
    
    # Get project root (3 levels up from tools/codegen/)
    script_dir = Path(__file__).parent
    project_root = script_dir.parent.parent
    
    # Get centralized license header
    new_header = get_engine_dual_license_header()
    
    print("Updating C++ engine header files with dual license...")
    print(f"Project root: {project_root}")
    print()
    
    # Files to update
    files_to_update = []
    
    # StaticExecutionEngine.h
    static_engine = project_root / "rsm" / "include" / "static" / "StaticExecutionEngine.h"
    if static_engine.exists():
        files_to_update.append(static_engine)
    else:
        print(f"Warning: StaticExecutionEngine.h not found at {static_engine}")
    
    # All Helper files in rsm/include/common/
    common_dir = project_root / "rsm" / "include" / "common"
    if common_dir.exists():
        helper_files = list(common_dir.glob("*Helper.h"))
        files_to_update.extend(helper_files)
        print(f"Found {len(helper_files)} Helper files in {common_dir}")
    else:
        print(f"Warning: Common directory not found at {common_dir}")
    
    if not files_to_update:
        print("No files to update!")
        return 1
    
    print(f"\nUpdating {len(files_to_update)} files...")
    print()
    
    # Update each file
    success_count = 0
    for file_path in files_to_update:
        if update_file_license(file_path, new_header):
            success_count += 1
    
    print()
    print(f"✓ Updated {success_count}/{len(files_to_update)} files")
    
    if success_count < len(files_to_update):
        print(f"✗ Failed to update {len(files_to_update) - success_count} files")
        return 1
    
    return 0


if __name__ == '__main__':
    sys.exit(main())
