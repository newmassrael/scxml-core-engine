"""
SCE Code Generators Package

Language-specific code generators for W3C SCXML state machines.
Each generator extends BaseCodeGenerator with language-specific
template rendering and output file generation.

Supported languages:
  - cpp: C++ (header-only .h + .inl, CRTP policy pattern)
"""

from generators.cpp_generator import CppCodeGenerator


# Registry of available language generators
_GENERATORS = {
    'cpp': CppCodeGenerator,
}


def get_generator(language: str, template_dir=None):
    """
    Factory function to create a language-specific code generator.

    Args:
        language: Target language identifier (e.g., 'cpp', 'kotlin')
        template_dir: Override template directory (optional)

    Returns:
        Configured code generator instance

    Raises:
        ValueError: If language is not supported
    """
    generator_cls = _GENERATORS.get(language)
    if generator_cls is None:
        supported = ', '.join(sorted(_GENERATORS.keys()))
        raise ValueError(
            f"Unsupported language: '{language}'. "
            f"Supported languages: {supported}"
        )
    return generator_cls(template_dir=template_dir)


def supported_languages():
    """Return list of supported target language identifiers."""
    return sorted(_GENERATORS.keys())
