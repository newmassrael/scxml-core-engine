#!/usr/bin/env python3
"""
SCXML Static Code Generator (Python + Jinja2)

Generates state machine code from W3C SCXML files.
Dispatches to language-specific generators (C++, Kotlin, etc.).

Usage:
    python codegen.py input.scxml -o output_dir [--language cpp|kotlin]

Default language is C++ for backward compatibility with existing build systems.
"""

import sys
import argparse
from pathlib import Path

from generators import get_generator, supported_languages


def main():
    parser = argparse.ArgumentParser(
        description='Generate state machine code from W3C SCXML files'
    )
    parser.add_argument('scxml_file', help='Input SCXML file')
    parser.add_argument('-o', '--output-dir', default='.',
                        help='Output directory for generated files')
    parser.add_argument('-t', '--template-dir', default=None,
                        help='Template directory (default: language-specific)')
    parser.add_argument('-l', '--language', default='cpp',
                        choices=supported_languages(),
                        help='Target language (default: cpp)')
    parser.add_argument('--as-child', action='store_true',
                        help='Generate as invoked child (force template generation)')
    parser.add_argument('--write-deps', metavar='DEPFILE',
                        help='Write Makefile dependency file for CMake DEPFILE')

    args = parser.parse_args()

    # Check input file exists
    if not Path(args.scxml_file).exists():
        print(f"Error: SCXML file not found: {args.scxml_file}", file=sys.stderr)
        return 1

    # Create language-specific generator and run
    generator = get_generator(args.language, template_dir=args.template_dir)
    success = generator.generate(
        args.scxml_file,
        args.output_dir,
        as_child=args.as_child,
        depfile_path=args.write_deps
    )

    return 0 if success else 1


if __name__ == '__main__':
    sys.exit(main())
