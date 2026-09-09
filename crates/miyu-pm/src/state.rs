use crate::types::{InstalledPkg, InstalledState};
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn load_state(path: &Path) -> Result<InstalledState> {
    if !path.exists() {
        return Ok(InstalledState::default());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read state {}", path.display()))?;
    let mut state: InstalledState = serde_json::from_str(&raw)
        .with_context(|| format!("invalid state JSON {}", path.display()))?;
    if state.schema_version == 0 {
        state.schema_version = 1;
    }
    Ok(state)
}

pub fn save_state(path: &Path, state: &InstalledState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(state)?;
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "{}", raw)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn find_installed<'a>(state: &'a InstalledState, name: &str) -> Option<&'a InstalledPkg> {
    state
        .installed
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn upsert(state: &mut InstalledState, pkg: InstalledPkg) {
    if let Some(existing) = state
        .installed
        .iter_mut()
        .find(|p| p.name.eq_ignore_ascii_case(&pkg.name))
    {
        *existing = pkg;
    } else {
        state.installed.push(pkg);
    }
}

pub fn remove_by_name(state: &mut InstalledState, name: &str) -> bool {
    let before = state.installed.len();
    state
        .installed
        .retain(|p| !p.name.eq_ignore_ascii_case(name));
    state.installed.len() != before
}
