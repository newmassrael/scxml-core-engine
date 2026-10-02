// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The built screen, served from a folder.

use std::io;
use std::path::{Component, Path, PathBuf};

/// A folder of static files and nothing outside it.
#[derive(Debug, Clone)]
pub struct Assets {
    root: PathBuf,
}

impl Assets {
    pub fn new(dir: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Assets {
            root: dir.as_ref().canonicalize()?,
        })
    }

    /// The file a request path names, and its media type. `None` for a path that
    /// names no file, and for one that tries to leave the folder.
    pub fn get(&self, url_path: &str) -> Option<(Vec<u8>, &'static str)> {
        let path = url_path.split(['?', '#']).next().unwrap_or("");
        let relative = if path == "/" { "/index.html" } else { path };
        // The screen's own files are plain names. An escape (`%2e%2e`) or a
        // Windows separator is a request for something else, so it is refused
        // rather than decoded.
        if relative.contains('%') || relative.contains('\\') {
            return None;
        }
        let relative = Path::new(relative.trim_start_matches('/'));
        if !relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        {
            return None;
        }
        let file = self.root.join(relative).canonicalize().ok()?;
        if !file.starts_with(&self.root) || !file.is_file() {
            return None;
        }
        let bytes = std::fs::read(&file).ok()?;
        Some((bytes, media_type(&file)))
    }
}

fn media_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
