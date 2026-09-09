use anyhow::{bail, Context, Result};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    pub miyu_home: PathBuf,
    pub pm_home: PathBuf,
    pub registry: PathBuf,
}

impl Paths {
    pub fn resolve(
        miyu_home_override: Option<PathBuf>,
        pm_home_override: Option<PathBuf>,
        registry_override: Option<PathBuf>,
    ) -> Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set")?;
        let miyu_home = miyu_home_override
            .or_else(|| env::var_os("MIYU_PM_MIYU_HOME").map(PathBuf::from))
            .unwrap_or_else(|| home.join(".miyu"));
        let pm_home = pm_home_override
            .or_else(|| env::var_os("MIYU_PM_HOME").map(PathBuf::from))
            .unwrap_or_else(|| home.join(".miyu-pm"));
        let registry = match registry_override
            .or_else(|| env::var_os("MIYU_PM_REGISTRY").map(PathBuf::from))
        {
            Some(path) => path,
            // No explicit registry: always use the official online index cache.
            None => pm_home.join("cache").join("official-index.json"),
        };
        Ok(Self {
            miyu_home,
            pm_home,
            registry,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.miyu_home.join("config").join("config.jsonc")
    }

    pub fn mcp_servers_dir(&self) -> PathBuf {
        self.miyu_home.join("mcp-servers")
    }

    pub fn data_skills_dir(&self) -> PathBuf {
        self.miyu_home.join("data").join("skills")
    }

    pub fn data_scripts_dir(&self) -> PathBuf {
        self.miyu_home.join("data").join("scripts")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.pm_home.join("state")
    }

    pub fn installed_state_file(&self) -> PathBuf {
        self.state_dir().join("installed.json")
    }

    pub fn sources_file(&self) -> PathBuf {
        self.state_dir().join("sources.json")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.pm_home.join("cache")
    }

    pub fn cache_clones_dir(&self) -> PathBuf {
        self.cache_dir().join("clones")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.miyu_home.join("config").join(".miyu-pm-backups")
    }

    /// Ensure the target dir is a child of one of miyu's managed roots.
    pub fn is_managed_mcp_dir(&self, target: &Path) -> bool {
        let root = self.mcp_servers_dir();
        target.starts_with(&root)
    }

    pub fn is_managed_scripts_file(&self, target: &Path) -> bool {
        target.starts_with(self.data_scripts_dir())
    }
}

pub fn now_epoch_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn validate_registry_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!(
            "registry index not found: {} (pass --registry or set MIYU_PM_REGISTRY)",
            path.display()
        );
    }
    Ok(())
}
