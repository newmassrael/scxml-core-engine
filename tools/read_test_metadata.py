#!/usr/bin/env python3
"""
Read W3C SCXML test metadata and extract description.

Usage:
    python3 read_test_metadata.py <metadata_file_path>

Output:
    Prints the test description to stdout
"""

import sys
import os


def read_metadata_description(metadata_path):
    """
    Read metadata.txt and extract description field.

    Args:
        metadata_path: Path to metadata.txt file

    Returns:
        Description string
    """
    if not os.path.exists(metadata_path):
        print(f"ERROR: Metadata file not found: {metadata_path}", file=sys.stderr)
        sys.exit(1)

    description = None

    with open(metadata_path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if line.startswith('description:'):
                # Extract description (everything after "description: ")
                description = line.split('description:', 1)[1].strip()
                break

    if description is None:
        print(f"ERROR: No description field found in {metadata_path}", file=sys.stderr)
        sys.exit(1)

    return description


def main():
    if len(sys.argv) != 2:
        print("Usage: read_test_metadata.py <metadata_file_path>", file=sys.stderr)
        sys.exit(1)

    metadata_path = sys.argv[1]
    description = read_metadata_description(metadata_path)

    # Print description to stdout (will be captured by CMake)
    print(description)


if __name__ == '__main__':
    main()
