// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include "common/Logger.h"
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>

namespace RSM {

/**
 * @brief Helper functions for W3C SCXML external file loading
 *
 * Single Source of Truth for file loading logic shared between:
 * - Python code generator (scxml_parser.py - build-time)
 * - Interpreter engine (DataModelParser.cpp - runtime)
 * - StateMachine (StateMachine.cpp - runtime)
 *
 * W3C SCXML References:
 * - 5.2.2: Data Model - src attribute for external content
 * - 3.3: External SCXML file loading
 */
class FileLoadingHelper {
public:
    /**
     * @brief Normalize file path by removing URI prefix
     *
     * Single Source of Truth for path normalization.
     * W3C SCXML 5.2.2: src attribute may use "file:" URI scheme.
     *
     * @param srcPath Source path (may include "file:" prefix)
     * @return Normalized file path without URI prefix
     */
    static std::string normalizePath(const std::string &srcPath) {
        // W3C SCXML 5.2.2: Remove "file:" or "file://" prefix
        if (srcPath.find("file://") == 0) {
            return srcPath.substr(7);  // Remove "file://"
        } else if (srcPath.find("file:") == 0) {
            return srcPath.substr(5);  // Remove "file:"
        }
        return srcPath;
    }

    /**
     * @brief Load file content from disk
     *
     * Single Source of Truth for file loading logic.
     * Used by both Interpreter (runtime) and Python codegen (build-time).
     *
     * W3C SCXML 5.2.2: Content from external file via src attribute.
     *
     * @param filePath Path to file (already normalized)
     * @param content Output parameter for file content
     * @return true if file loaded successfully, false on error
     */
    static bool loadFileContent(const std::string &filePath, std::string &content) {
        std::ifstream file(filePath);
        if (!file.is_open()) {
            LOG_ERROR("FileLoadingHelper: Failed to open file: {}", filePath);
            return false;
        }

        std::stringstream buffer;
        buffer << file.rdbuf();
        content = buffer.str();

        // W3C SCXML 5.2.2: Trim whitespace for consistency
        // Remove leading/trailing whitespace
        size_t start = content.find_first_not_of(" \t\r\n");
        size_t end = content.find_last_not_of(" \t\r\n");

        if (start != std::string::npos && end != std::string::npos) {
            content = content.substr(start, end - start + 1);
        } else if (start == std::string::npos) {
            content = "";  // All whitespace
        }

        return true;
    }

    /**
     * @brief Load and normalize file content from src attribute
     *
     * Convenience function combining path normalization and file loading.
     *
     * @param srcAttribute Value of src attribute (may include "file:" prefix)
     * @param content Output parameter for file content
     * @return true if file loaded successfully, false on error
     */
    static bool loadFromSrc(const std::string &srcAttribute, std::string &content) {
        std::string normalizedPath = normalizePath(srcAttribute);
        return loadFileContent(normalizedPath, content);
    }

    /**
     * @brief W3C SCXML 6.4.3: Load SCXML file for invoke srcexpr
     *
     * Single Source of Truth for srcexpr invoke file loading shared between:
     * - Interpreter engine: StateMachine invoke processing
     * - AOT engine: Generated code for srcexpr invoke (entry_exit_actions.jinja2)
     *
     * ARCHITECTURE.md Zero Duplication: Removes inline file I/O from generated AOT code.
     * This Helper method encapsulates the W3C SCXML 6.4.3 file loading logic.
     *
     * W3C SCXML 6.4.3: srcexpr evaluates to URI that identifies the SCXML file to invoke.
     * Protocol stripping: "file:path" → "path" (W3C SCXML file: URI scheme)
     *
     * W3C SCXML Standard Behavior:
     * - Relative paths are resolved relative to parent SCXML file location
     * - Example: parent="/app/workflows/main.scxml", child="sub.scxml" → "/app/workflows/sub.scxml"
     *
     * Differences from loadFromSrc():
     * - Throws exceptions instead of returning bool (for generated code error handling)
     * - Does NOT trim whitespace (SCXML files need exact content preservation)
     * - Validates non-empty content
     * - Resolves relative paths based on parent SCXML location (W3C standard)
     *
     * @param filePath File path or URI to load (may include "file:" protocol)
     * @param parentScxmlPath Optional parent SCXML file path for relative resolution (W3C standard)
     * @return SCXML file content as string (exact content, no trimming)
     * @throws std::runtime_error if file cannot be opened, is empty, or read fails
     *
     * Usage (AOT generated code with parent path):
     * @code
     * std::string scxmlContent = ::RSM::FileLoadingHelper::loadScxmlFile(filePath, parentScxmlPath);
     * auto child = ::RSM::StateMachine::createFromSCXMLString(scxmlContent, invokeId);
     * @endcode
     *
     * Usage (Interpreter):
     * @code
     * std::string scxmlContent = FileLoadingHelper::loadScxmlFile(srcUri, parentPath);
     * auto child = StateMachine::createFromSCXMLString(scxmlContent, invokeId);
     * @endcode
     */
    static std::string loadScxmlFile(const std::string &filePath, const std::string &parentScxmlPath = "") {
        // W3C SCXML 6.4.3: Strip "file:" protocol prefix if present
        std::string actualPath = normalizePath(filePath);

        // Security validation: Reject empty paths
        if (actualPath.empty()) {
            LOG_ERROR("FileLoadingHelper: Empty file path after protocol stripping");
            throw std::runtime_error("Empty SCXML file path");
        }

        // W3C SCXML 6.4: Resolve child SCXML relative to parent SCXML location
        std::ifstream scxmlFile(actualPath);

        // If direct path fails and path is relative, resolve relative to parent
        if (!scxmlFile.is_open() && !std::filesystem::path(actualPath).is_absolute()) {
            // W3C SCXML Standard: Child SCXML must be resolved relative to parent SCXML file location
            if (!parentScxmlPath.empty()) {
                std::filesystem::path parentPath(parentScxmlPath);
                std::filesystem::path parentDir = parentPath.parent_path();
                std::filesystem::path resolvedPath = parentDir / actualPath;

                scxmlFile.open(resolvedPath);
                if (scxmlFile.is_open()) {
                    actualPath = resolvedPath.string();
                    LOG_DEBUG("FileLoadingHelper: Resolved child SCXML relative to parent: {} (parent: {})", actualPath,
                              parentScxmlPath);
                }
            } else {
                LOG_ERROR("FileLoadingHelper: Relative path '{}' requires parent SCXML path for W3C SCXML compliance",
                          actualPath);
            }
        }

        if (!scxmlFile.is_open()) {
            LOG_ERROR("FileLoadingHelper: Failed to open SCXML file: {}", actualPath);
            throw std::runtime_error("Failed to open SCXML file: " + actualPath);
        }

        // Read entire file content WITHOUT trimming (SCXML needs exact content)
        std::stringstream buffer;
        buffer << scxmlFile.rdbuf();
        scxmlFile.close();

        std::string content = buffer.str();
        LOG_DEBUG("FileLoadingHelper: Loaded {} bytes from SCXML file: {}", content.size(), actualPath);

        // Validate non-empty content
        if (content.empty()) {
            LOG_ERROR("FileLoadingHelper: Empty SCXML file content: {}", actualPath);
            throw std::runtime_error("Empty SCXML file: " + actualPath);
        }

        return content;
    }

    /**
     * @brief Load external script with security validation
     *
     * Single Source of Truth for W3C SCXML 5.8 external script loading.
     * Used by both Python code generator and Interpreter engine.
     *
     * ARCHITECTURE.md Zero Duplication: Shared logic between:
     * - Python code generator (scxml_parser.py:_parse_global_scripts)
     * - Interpreter engine (ActionParser.cpp:parseActionNode)
     *
     * W3C SCXML 5.8: External scripts resolved relative to SCXML file location.
     * Security: Prevents path traversal attacks (e.g., "../../etc/passwd").
     *
     * Algorithm:
     * 1. Normalize src path (remove "file:" prefix)
     * 2. Resolve path relative to SCXML file base directory
     * 3. Security validation: ensure resolved path is within SCXML directory tree
     * 4. Load file content or reject document (W3C SCXML 5.8 requirement)
     *
     * @param srcPath Script source path (from src attribute)
     * @param scxmlBasePath Base directory of SCXML file
     * @param content Output parameter for script content
     * @param errorMsg Output parameter for error message
     * @return true if loaded successfully, false if security violation or file not found
     */
    static bool loadExternalScript(const std::string &srcPath, const std::string &scxmlBasePath, std::string &content,
                                   std::string &errorMsg) {
        namespace fs = std::filesystem;

        // Step 1: Normalize path (remove "file:" prefix if present)
        std::string normalizedSrc = normalizePath(srcPath);

        // Step 2: Resolve path relative to SCXML file location
        fs::path scriptPath;
        try {
            if (scxmlBasePath.empty()) {
                // No base path set - use srcPath as-is
                scriptPath = fs::absolute(normalizedSrc);
            } else {
                scriptPath = fs::path(scxmlBasePath) / normalizedSrc;
                scriptPath = fs::absolute(scriptPath);
            }
        } catch (const std::exception &e) {
            errorMsg = "Failed to resolve script path: " + normalizedSrc + ". Error: " + e.what();
            LOG_ERROR("FileLoadingHelper: {}", errorMsg);
            return false;
        }

        // Step 3: Security validation - prevent path traversal attacks
        if (!scxmlBasePath.empty()) {
            try {
                fs::path scxmlDir = fs::absolute(fs::path(scxmlBasePath));

                // Normalize paths for comparison (lexically_normal resolves ".." and ".")
                auto scriptPathNorm = scriptPath.lexically_normal();
                auto scxmlDirNorm = scxmlDir.lexically_normal();

                // Check if script path is within allowed directory tree
                // Use lexically_relative to check if path is truly within directory
                auto relativePath = scriptPathNorm.lexically_relative(scxmlDirNorm);

                // If relative path starts with "..", it's outside the allowed directory
                if (relativePath.empty() || relativePath.string().find("..") == 0) {
                    errorMsg = "Security violation: Script path '" + srcPath + "' resolves outside SCXML directory. " +
                               "Resolved to: " + scriptPath.string() + ", SCXML dir: " + scxmlDir.string();
                    LOG_ERROR("FileLoadingHelper: {}", errorMsg);
                    return false;
                }
            } catch (const std::exception &e) {
                errorMsg = "Security validation failed for script path: " + srcPath + ". Error: " + e.what();
                LOG_ERROR("FileLoadingHelper: {}", errorMsg);
                return false;
            }
        }

        // Step 4: Load file content
        if (!loadFileContent(scriptPath.string(), content)) {
            // W3C SCXML 5.8: Document MUST be rejected if script cannot be loaded
            errorMsg = "W3C SCXML 5.8: External script file not found: '" + srcPath + "' (resolved to " +
                       scriptPath.string() + "). Document is non-conformant and MUST be rejected.";
            LOG_ERROR("FileLoadingHelper: {}", errorMsg);
            return false;
        }

        LOG_INFO("FileLoadingHelper: W3C SCXML 5.8 - Loaded external script: {} (resolved to {})", srcPath,
                 scriptPath.string());
        return true;
    }
};

}  // namespace RSM
