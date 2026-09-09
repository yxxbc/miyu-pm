use crate::paths::{now_epoch_secs, Paths};
use anyhow::{anyhow, Context, Result};
use json_comments::StripComments;
use serde_json::{json, Map, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn load_config(paths: &Paths) -> Result<Value> {
    let path = paths.config_file();
    if !path.exists() {
        return Ok(default_config());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let stripped = StripComments::new(raw.as_bytes());
    let value: Value = serde_json::from_reader(stripped)
        .with_context(|| format!("invalid JSONC in {}", path.display()))?;
    Ok(value)
}

pub fn default_config() -> Value {
    json!({
        "config_version": 2,
        "mcp": {
            "enabled": true,
            "servers": []
        }
    })
}

pub fn backup_config(paths: &Paths) -> Result<Option<PathBuf>> {
    let source = paths.config_file();
    if !source.exists() {
        return Ok(None);
    }
    let dir = paths.backup_dir();
    fs::create_dir_all(&dir)?;
    let ts = now_epoch_secs();
    let dest = dir.join(format!(
        "config.jsonc.bak.{}.{}",
        ts,
        source.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::copy(&source, &dest)
        .with_context(|| format!("failed to backup config to {}", dest.display()))?;
    Ok(Some(dest))
}

pub fn save_config(paths: &Paths, value: &Value) -> Result<()> {
    let path = paths.config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(value)?;
    let dir = path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent"))?;
    let tmp = dir.join(format!(".config.jsonc.tmp.{}", std::process::id()));
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "{}", raw)?;
    file.sync_all()?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn mcp_enabled_mut(value: &mut Value) -> Result<&mut Map<String, Value>> {
    if !value.get("mcp").is_some() {
        value["mcp"] = json!({"enabled": true, "servers": []});
    }
    let mcp = value
        .get_mut("mcp")
        .and_then(|m| m.as_object_mut())
        .ok_or_else(|| anyhow!("config mcp section is not an object"))?;
    if !mcp.contains_key("enabled") {
        mcp.insert("enabled".into(), Value::Bool(true));
    }
    if !mcp.contains_key("servers") {
        mcp.insert("servers".into(), Value::Array(vec![]));
    }
    Ok(mcp)
}

/// Insert or replace an mcp.servers entry. Returns true when a new entry was added.
pub fn upsert_mcp_server(value: &mut Value, entry: Value) -> Result<bool> {
    let id = entry
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("mcp entry requires an id string"))?
        .to_string();
    let mcp = mcp_enabled_mut(value)?;
    let servers = mcp
        .get_mut("servers")
        .and_then(|s| s.as_array_mut())
        .ok_or_else(|| anyhow!("mcp.servers is not an array"))?;
    if let Some(existing) = servers
        .iter_mut()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
    {
        *existing = entry;
        Ok(false)
    } else {
        servers.push(entry);
        Ok(true)
    }
}

pub fn remove_mcp_server(value: &mut Value, id: &str) -> Result<bool> {
    if !value.get("mcp").is_some() {
        return Ok(false);
    }
    let mcp = value
        .get_mut("mcp")
        .and_then(|m| m.as_object_mut())
        .ok_or_else(|| anyhow!("config mcp section is not an object"))?;
    let servers = mcp
        .get_mut("servers")
        .and_then(|s| s.as_array_mut())
        .ok_or_else(|| anyhow!("mcp.servers is not an array"))?;
    let before = servers.len();
    servers.retain(|e| e.get("id").and_then(|v| v.as_str()) != Some(id));
    Ok(servers.len() != before)
}

pub fn has_mcp_server(value: &Value, id: &str) -> bool {
    value
        .get("mcp")
        .and_then(|m| m.get("servers"))
        .and_then(|s| s.as_array())
        .map(|servers| {
            servers
                .iter()
                .any(|e| e.get("id").and_then(|v| v.as_str()) == Some(id))
        })
        .unwrap_or(false)
}

pub fn count_mcp_servers(value: &Value) -> usize {
    value
        .get("mcp")
        .and_then(|m| m.get("servers"))
        .and_then(|s| s.as_array())
        .map(|s| s.len())
        .unwrap_or(0)
}
