use crate::paths::validate_registry_path;
use crate::types::{PackageMeta, RegistryIndex};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn load_registry(path: &Path) -> Result<RegistryIndex> {
    validate_registry_path(path)?;
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read registry {}", path.display()))?;
    let index: RegistryIndex = serde_json::from_str(&raw)
        .with_context(|| format!("invalid registry JSON {}", path.display()))?;
    Ok(index)
}

pub fn find_package<'a>(index: &'a RegistryIndex, name: &str) -> Option<&'a PackageMeta> {
    index
        .packages
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn search_packages<'a>(index: &'a RegistryIndex, query: &str) -> Vec<&'a PackageMeta> {
    let q = query.to_lowercase();
    index
        .packages
        .iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q)
                || p.display_name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q)
                || p.description.to_lowercase().contains(&q)
        })
        .collect()
}

pub fn render_package(p: &PackageMeta) -> String {
    let mut out = String::new();
    let display = p.display_name.as_deref().unwrap_or(&p.name);
    out.push_str(&format!(
        "{} ({})  {}  v{}\n",
        p.name, p.kind, display, p.version
    ));
    out.push_str(&format!("  {}\n", p.description));
    if let Some(license) = &p.license {
        out.push_str(&format!("  license: {}\n", license));
    }
    if let Some(repo) = &p.repo {
        out.push_str(&format!("  repo: {}\n", repo));
    }
    if let Some(mcp) = &p.mcp {
        let mcp_id = mcp.id.as_deref().unwrap_or(&p.name);
        out.push_str(&format!("  mcp.id: {}\n", mcp_id));
        out.push_str(&format!("  command: {}\n", mcp.command));
        if !mcp.args.is_empty() {
            out.push_str(&format!("  args: {}\n", mcp.args.join(" ")));
        }
        if let Some(timeout) = mcp.timeout_seconds {
            out.push_str(&format!("  timeout_seconds: {}\n", timeout));
        }
    }
    if let Some(install) = &p.install {
        if !install.setup.is_empty() {
            out.push_str("  setup:\n");
            for cmd in &install.setup {
                out.push_str(&format!("    - {}\n", cmd));
            }
        }
    }
    if let Some(homepage) = &p.homepage {
        out.push_str(&format!("  homepage: {}\n", homepage));
    }
    out
}
