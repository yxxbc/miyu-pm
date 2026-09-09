use crate::actions::Options;
use crate::paths::Paths;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesFile {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub sources: Vec<SourceEntry>,
}

impl Default for SourcesFile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            sources: Vec::new(),
        }
    }
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub kind: String,
}

pub fn list(paths: &Paths) -> Result<()> {
    println!(
        "active registry (current --registry): {}",
        paths.registry.display()
    );
    let file = load(paths)?;
    if file.sources.is_empty() {
        println!("no stored sources yet (try 'miyu-pm source add <url-or-path>')");
        return Ok(());
    }
    println!("stored sources:");
    for source in &file.sources {
        println!("  {}  [{}]  {}", source.name, source.kind, source.url);
    }
    Ok(())
}

pub fn add(paths: &Paths, name: Option<String>, url: String, opts: Options) -> Result<()> {
    if url.trim().is_empty() {
        bail!("source url/path cannot be empty");
    }
    let name = name.unwrap_or_else(|| derive_name(&url));
    let kind = if url.starts_with("http://") || url.starts_with("https://") {
        "remote".to_string()
    } else {
        "local".to_string()
    };
    if opts.dry_run {
        println!(
            "[dry-run] would add source name='{}' kind='{}' url='{}'",
            name, kind, url
        );
        return Ok(());
    }
    let mut file = load(paths)?;
    if let Some(existing) = file.sources.iter_mut().find(|s| s.name == name) {
        existing.url = url.clone();
        existing.kind = kind.clone();
        println!("updated source '{}'", name);
    } else {
        file.sources.push(SourceEntry {
            name: name.clone(),
            url: url.clone(),
            kind: kind.clone(),
        });
        println!("added source '{}' [{}]", name, kind);
    }
    save(paths, &file)?;
    Ok(())
}

pub fn remove(paths: &Paths, name_or_url: &str) -> Result<()> {
    let mut file = load(paths)?;
    let before = file.sources.len();
    file.sources
        .retain(|s| s.name != name_or_url && s.url != name_or_url);
    if file.sources.len() == before {
        bail!("source not found: {}", name_or_url);
    }
    save(paths, &file)?;
    println!("removed source '{}'", name_or_url);
    Ok(())
}

/// Refresh stored sources into ~/.miyu-pm/cache/sources/. Local paths are copied;
/// remote http(s) URLs are fetched with curl.
pub fn refresh_all(paths: &Paths) -> Result<()> {
    let file = load(paths)?;
    if file.sources.is_empty() {
        return Ok(());
    }
    let cache = paths.cache_dir().join("sources");
    fs::create_dir_all(&cache)?;
    for source in &file.sources {
        let dest = cache.join(format!("{}.json", sanitize_name(&source.name)));
        if source.kind == "remote"
            || source.url.starts_with("http://")
            || source.url.starts_with("https://")
        {
            let status = Command::new("curl")
                .args(["-fsSL", "--max-time", "30", "-o"])
                .arg(&dest)
                .arg(&source.url)
                .status()
                .with_context(|| format!("curl failed for source {}", source.name))?;
            if !status.success() {
                bail!("failed to fetch source {} from {}", source.name, source.url);
            }
            println!("refreshed source '{}' -> {}", source.name, dest.display());
        } else {
            let src = Path::new(&source.url);
            if !src.exists() {
                bail!("local source path not found: {}", src.display());
            }
            fs::copy(src, &dest)?;
            println!(
                "refreshed local source '{}' -> {}",
                source.name,
                dest.display()
            );
        }
    }
    Ok(())
}

fn load(paths: &Paths) -> Result<SourcesFile> {
    let path = paths.sources_file();
    if !path.exists() {
        return Ok(SourcesFile::default());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let file: SourcesFile = serde_json::from_str(&raw)
        .with_context(|| format!("invalid sources JSON {}", path.display()))?;
    Ok(file)
}

fn save(paths: &Paths, file: &SourcesFile) -> Result<()> {
    let path = paths.sources_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(file)?;
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    let mut handle = fs::File::create(&tmp)?;
    writeln!(handle, "{}", raw)?;
    handle.sync_all()?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn derive_name(url: &str) -> String {
    let trimmed = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("source");
    let trimmed = trimmed.trim_end_matches(".json");
    if trimmed.is_empty() {
        "source".to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
