use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub packages: Vec<PackageMeta>,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub manifest_path: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
    #[serde(default)]
    pub install: Option<InstallSpec>,
    #[serde(default)]
    pub mcp: Option<McpSpec>,
    #[serde(default)]
    pub skill: Option<SkillSpec>,
    #[serde(default)]
    pub script: Option<ScriptSpec>,
    #[serde(default)]
    pub plugin: Option<PluginSpec>,
    #[serde(default)]
    pub release: Option<ReleaseSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallSpec {
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub setup: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpSpec {
    #[serde(default)]
    pub id: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillSpec {
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptSpec {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub argv: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSpec {
    #[serde(default)]
    pub api_version: Option<serde_json::Value>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub min_miyu_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseSpec {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub asset_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledState {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub installed: Vec<InstalledPkg>,
}

impl Default for InstalledState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            installed: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPkg {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    pub installed_at: u64,
    #[serde(default)]
    pub target_dir: Option<String>,
    #[serde(default)]
    pub mcp_id: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
}
