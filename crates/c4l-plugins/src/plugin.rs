//! Plugin discovery and manifest loading.
//!
//! Maps from: ECC .claude-plugin/plugin.json pattern

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
/// Plugin manifest loaded from plugin.json or package.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_dir: Option<PathBuf>,
    /// Root directory of the plugin.
    #[serde(skip)]
    pub root: Option<PathBuf>,
}

/// Discover plugins from a list of directories.
///
/// Scans for: plugin.json, package.json, or .claude-plugin/plugin.json
pub fn discover_plugins(dirs: &[PathBuf]) -> Vec<PluginManifest> {
    let mut plugins = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        // Direct plugin.json
        let plugin_json = dir.join("plugin.json");
        if plugin_json.exists() {
            if let Some(p) = load_plugin_manifest(&plugin_json, dir) {
                plugins.push(p);
                continue;
            }
        }

        // .claude-plugin/plugin.json
        let claude_plugin = dir.join(".claude-plugin").join("plugin.json");
        if claude_plugin.exists() {
            if let Some(p) = load_plugin_manifest(&claude_plugin, dir) {
                plugins.push(p);
                continue;
            }
        }

        // Scan subdirectories for plugins
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let pj = path.join("plugin.json");
                    let pkg = path.join("package.json");
                    let cp = path.join(".claude-plugin").join("plugin.json");

                    if pj.exists() {
                        if let Some(p) = load_plugin_manifest(&pj, &path) {
                            plugins.push(p);
                        }
                    } else if cp.exists() {
                        if let Some(p) = load_plugin_manifest(&cp, &path) {
                            plugins.push(p);
                        }
                    } else if pkg.exists() {
                        if let Some(p) = load_plugin_from_package_json(&pkg, &path) {
                            plugins.push(p);
                        }
                    }
                }
            }
        }
    }

    plugins
}

fn load_plugin_manifest(path: &Path, root: &Path) -> Option<PluginManifest> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut manifest: PluginManifest = serde_json::from_str(&content).ok()?;
    manifest.root = Some(root.to_path_buf());
    Some(manifest)
}

fn load_plugin_from_package_json(path: &Path, root: &Path) -> Option<PluginManifest> {
    let content = std::fs::read_to_string(path).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;

    let name = pkg.get("name")?.as_str()?;
    let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
    let description = pkg.get("description").and_then(|v| v.as_str()).unwrap_or("");

    Some(PluginManifest {
        name: name.into(),
        version: version.into(),
        description: description.into(),
        main: pkg.get("main").and_then(|v| v.as_str()).map(String::from),
        hooks: None,
        skills_dir: None,
        commands_dir: None,
        agents_dir: None,
        root: Some(root.to_path_buf()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_plugin_from_plugin_json() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("my-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.json"),
            r#"{"name": "my-plugin", "version": "1.0.0", "description": "test plugin"}"#,
        ).unwrap();

        let plugins = discover_plugins(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "my-plugin");
    }

    #[test]
    fn discover_plugin_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("npm-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("package.json"),
            r#"{"name": "npm-plugin", "version": "2.0.0", "description": "npm plugin"}"#,
        ).unwrap();

        let plugins = discover_plugins(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "npm-plugin");
        assert_eq!(plugins[0].version, "2.0.0");
    }

    #[test]
    fn discover_claude_plugin_subdir() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("special-plugin");
        let claude_dir = plugin_dir.join(".claude-plugin");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("plugin.json"),
            r#"{"name": "special", "version": "3.0.0"}"#,
        ).unwrap();

        let plugins = discover_plugins(&[dir.path().to_path_buf()]);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "special");
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = discover_plugins(&[dir.path().to_path_buf()]);
        assert!(plugins.is_empty());
    }
}
