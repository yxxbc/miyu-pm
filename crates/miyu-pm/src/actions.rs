use crate::config;
use crate::paths::{now_epoch_secs, Paths};
use crate::registry;
use crate::state;
use crate::types::{InstalledPkg, PackageMeta};
use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub dry_run: bool,
    pub yes: bool,
}

pub fn search(paths: &Paths, query: &str) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let hits = registry::search_packages(&index, query);
    if hits.is_empty() {
        println!("no packages match '{}'", query);
        return Ok(());
    }
    println!("{} match(es) for '{}':", hits.len(), query);
    for p in hits {
        println!("  {}", p.name);
        if let Some(display) = &p.display_name {
            println!("    {}  (v{})", display, p.version);
        }
    }
    Ok(())
}

pub fn info(paths: &Paths, name: &str) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let pkg = registry::find_package(&index, name)
        .ok_or_else(|| anyhow!("package not found in registry: {}", name))?;
    print!("{}", registry::render_package(pkg));
    Ok(())
}

pub fn list(paths: &Paths, all: bool) -> Result<()> {
    let state = state::load_state(&paths.installed_state_file())?;
    if all {
        let index = registry::load_registry(&paths.registry)?;
        println!("available packages ({}):", index.packages.len());
        for p in &index.packages {
            let installed = state
                .installed
                .iter()
                .any(|i| i.name.eq_ignore_ascii_case(&p.name));
            let mark = if installed { "*" } else { " " };
            println!("  {} {} ({})", mark, p.name, p.kind);
        }
        return Ok(());
    }
    if state.installed.is_empty() {
        println!("no installed packages");
        return Ok(());
    }
    println!("installed packages ({}):", state.installed.len());
    for p in &state.installed {
        println!(
            "  {} ({}) v{}  installed_at={}",
            p.name, p.kind, p.version, p.installed_at
        );
        if let Some(dir) = &p.target_dir {
            println!("    dir: {}", dir);
        }
        if let Some(id) = &p.mcp_id {
            println!("    mcp.id: {}", id);
        }
    }
    Ok(())
}

pub fn outdated(paths: &Paths) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let state = state::load_state(&paths.installed_state_file())?;
    let mut outdated = Vec::new();
    for installed in &state.installed {
        let Some(remote) = registry::find_package(&index, &installed.name) else {
            continue;
        };
        if installed.version != remote.version {
            outdated.push((installed, remote));
        }
    }
    if outdated.is_empty() {
        println!("All packages are up to date.");
        return Ok(());
    }
    println!("Outdated packages ({})", outdated.len());
    for (installed, remote) in &outdated {
        println!(
            "  {}  {} -> {}",
            installed.name, installed.version, remote.version
        );
    }
    Ok(())
}

fn install_mcp(
    paths: &Paths,
    name: &str,
    repo_override: Option<&str>,
    no_setup: bool,
    env_overrides: &[String],
    opts: Options,
) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let pkg = registry::find_package(&index, name)
        .ok_or_else(|| anyhow!("package not found in registry: {}", name))?;
    ensure_m1_kind(pkg)?;

    let repo = repo_override
        .map(|s| s.to_string())
        .or_else(|| pkg.repo.clone())
        .ok_or_else(|| anyhow!("package {} has no repo URL in registry", pkg.name))?;
    let mcp_id = pkg
        .mcp
        .as_ref()
        .and_then(|m| m.id.clone())
        .unwrap_or_else(|| pkg.name.clone());
    let target = paths.mcp_servers_dir().join(&mcp_id);

    let state_path = paths.installed_state_file();
    let mut st = state::load_state(&state_path)?;
    if state::find_installed(&st, &pkg.name).is_some() {
        bail!("{} is already installed (use 'upgrade')", pkg.name);
    }
    if target.exists() {
        bail!(
            "target directory already exists: {} (remove it first)",
            target.display()
        );
    }

    let setup_plan = pkg
        .install
        .as_ref()
        .map(|i| i.setup.clone())
        .unwrap_or_default();
    println!("Install plan:");
    println!("  package : {} ({}) v{}", pkg.name, pkg.kind, pkg.version);
    println!("  repo    : {}", repo);
    println!("  mcp.id  : {}", mcp_id);
    println!("  target  : {}", target.display());
    if !setup_plan.is_empty() {
        println!("  setup   :");
        for cmd in &setup_plan {
            println!("    - {}", cmd);
        }
    } else {
        println!("  setup   : (none)");
    }
    if opts.dry_run {
        println!("[dry-run] would clone and configure the package above");
        return Ok(());
    }
    confirm_or_abort("Proceed with install?", opts.yes)?;

    fs::create_dir_all(paths.mcp_servers_dir())?;
    clone_repo(&repo, &target, pkg.commit.as_deref())
        .with_context(|| format!("failed to clone {}", repo))?;

    if let Err(err) = run_setup(&pkg.name, &setup_plan, &target, no_setup) {
        let _ = remove_dir_safe(paths, &target);
        return Err(err);
    }

    let mut warnings = Vec::new();
    let root = target.to_string_lossy().to_string();
    let overrides = parse_env_overrides(env_overrides);
    let (resolved_command, mut w) =
        resolve_text(&pkg.mcp.as_ref().unwrap().command, &root, &overrides);
    warnings.append(&mut w);
    let mut args = Vec::new();
    if let Some(mcp) = &pkg.mcp {
        for a in &mcp.args {
            let (r, mut w) = resolve_text(a, &root, &overrides);
            warnings.append(&mut w);
            args.push(r);
        }
    }
    let mut env_map = Map::new();
    if let Some(mcp) = &pkg.mcp {
        for (k, v) in &mcp.env {
            let (r, mut w) = resolve_text(v, &root, &overrides);
            warnings.append(&mut w);
            env_map.insert(k.clone(), Value::String(r));
        }
    }
    for w in &warnings {
        eprintln!("warning: {}", w);
    }

    let entry = json!({
        "id": mcp_id,
        "display_name": pkg.display_name.as_deref().unwrap_or(&pkg.name),
        "command": resolved_command,
        "args": args,
        "env": Value::Object(env_map),
        "timeout_seconds": pkg.mcp.as_ref().and_then(|m| m.timeout_seconds).unwrap_or(60),
        "enabled": pkg.mcp.as_ref().and_then(|m| m.enabled).unwrap_or(true),
    });

    let backup = config::backup_config(paths)?;
    if let Some(b) = &backup {
        println!("config backed up to {}", b.display());
    }
    let mut cfg = config::load_config(paths)?;
    config::upsert_mcp_server(&mut cfg, entry)?;
    config::save_config(paths, &cfg)?;

    let installed = InstalledPkg {
        name: pkg.name.clone(),
        kind: pkg.kind.clone(),
        version: pkg.version.clone(),
        source: Some(paths.registry.to_string_lossy().to_string()),
        repo: Some(repo),
        commit: pkg.commit.clone(),
        installed_at: now_epoch_secs(),
        target_dir: Some(target.to_string_lossy().to_string()),
        mcp_id: Some(mcp_id),
        files: Vec::new(),
    };
    state::upsert(&mut st, installed);
    state::save_state(&state_path, &st)?;

    println!("installed {}", pkg.name);
    println!("remember to reload miyu (or restart it) so the MCP server is picked up");
    Ok(())
}

pub fn install(
    paths: &Paths,
    name: &str,
    repo_override: Option<&str>,
    no_setup: bool,
    env_overrides: &[String],
    opts: Options,
) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let pkg = registry::find_package(&index, name)
        .ok_or_else(|| anyhow!("package not found in registry: {}", name))?;
    ensure_supported_kind(pkg)?;
    match pkg.kind.as_str() {
        "mcp" => install_mcp(
            paths,
            &pkg.name,
            repo_override,
            no_setup,
            env_overrides,
            opts,
        ),
        "skill" => install_skill(paths, pkg, repo_override, opts),
        "script" => install_script(paths, pkg, repo_override, opts),
        _ => unreachable!("kind already checked"),
    }
}

fn install_skill(
    paths: &Paths,
    pkg: &PackageMeta,
    repo_override: Option<&str>,
    opts: Options,
) -> Result<()> {
    let repo = resolve_repo(pkg, repo_override)?;
    let target = paths.data_skills_dir().join(&pkg.name);
    let state_path = paths.installed_state_file();
    let mut st = state::load_state(&state_path)?;
    if state::find_installed(&st, &pkg.name).is_some() {
        bail!("{} is already installed (use 'upgrade')", pkg.name);
    }
    if target.exists() {
        bail!(
            "target directory already exists: {} (remove it first)",
            target.display()
        );
    }
    let entry = pkg
        .skill
        .as_ref()
        .and_then(|s| s.entry.clone())
        .unwrap_or_else(|| "SKILL.md".to_string());

    println!("Install plan:");
    println!("  package : {} ({}) v{}", pkg.name, pkg.kind, pkg.version);
    println!("  repo    : {}", repo);
    println!("  target  : {}", target.display());
    println!("  entry   : {}", entry);
    if opts.dry_run {
        println!("[dry-run] would clone and install the skill above");
        return Ok(());
    }
    confirm_or_abort("Proceed with skill install?", opts.yes)?;

    let cache = clone_cache(paths, &repo, &pkg.name, pkg.commit.as_deref())?;
    let result = (|| -> Result<()> {
        let source = package_source_root(&cache, pkg)?;
        let entry_path = safe_join(&source, &entry)?;
        if !entry_path.exists() {
            bail!("skill entry not found: {}", entry_path.display());
        }
        copy_tree_contents(&source, &target)?;
        Ok(())
    })();
    let _ = fs::remove_dir_all(&cache);
    result?;

    let installed = InstalledPkg {
        name: pkg.name.clone(),
        kind: pkg.kind.clone(),
        version: pkg.version.clone(),
        source: Some(paths.registry.to_string_lossy().to_string()),
        repo: Some(repo),
        commit: pkg.commit.clone(),
        installed_at: now_epoch_secs(),
        target_dir: Some(target.to_string_lossy().to_string()),
        mcp_id: None,
        files: Vec::new(),
    };
    state::upsert(&mut st, installed);
    state::save_state(&state_path, &st)?;
    println!("installed skill {}", pkg.name);
    println!("skill directory: {}", target.display());
    Ok(())
}

fn install_script(
    paths: &Paths,
    pkg: &PackageMeta,
    repo_override: Option<&str>,
    opts: Options,
) -> Result<()> {
    let repo = resolve_repo(pkg, repo_override)?;
    let target_dir = paths.data_scripts_dir();
    let state_path = paths.installed_state_file();
    let mut st = state::load_state(&state_path)?;
    if state::find_installed(&st, &pkg.name).is_some() {
        bail!("{} is already installed (use 'upgrade')", pkg.name);
    }
    let rel_files = pkg
        .script
        .as_ref()
        .map(|s| s.files.clone())
        .unwrap_or_default();
    if rel_files.is_empty() {
        bail!("script package {} declares no files", pkg.name);
    }

    println!("Install plan:");
    println!("  package : {} ({}) v{}", pkg.name, pkg.kind, pkg.version);
    println!("  repo    : {}", repo);
    println!("  target  : {}", target_dir.display());
    println!("  files   :");
    for file in &rel_files {
        println!("    - {}", file);
    }
    if opts.dry_run {
        println!("[dry-run] would clone and install the script(s) above");
        return Ok(());
    }
    confirm_or_abort("Proceed with script install?", opts.yes)?;

    fs::create_dir_all(&target_dir)?;
    let cache = clone_cache(paths, &repo, &pkg.name, pkg.commit.as_deref())?;
    let result = (|| -> Result<Vec<PathBuf>> {
        let source = package_source_root(&cache, pkg)?;
        let mut copied = Vec::new();
        for rel in &rel_files {
            let src = safe_join(&source, rel)?;
            if !src.is_file() {
                bail!("script file not found: {}", src.display());
            }
            let file_name = src
                .file_name()
                .ok_or_else(|| anyhow!("invalid script path {}", src.display()))?
                .to_string_lossy()
                .to_string();
            let dest = target_dir.join(&file_name);
            if dest.exists() {
                bail!(
                    "destination already exists: {} (remove it first)",
                    dest.display()
                );
            }
            fs::copy(&src, &dest)?;
            set_executable(&dest)?;
            copied.push(dest);
        }
        Ok(copied)
    })();
    let _ = fs::remove_dir_all(&cache);
    let copied = result?;

    let installed = InstalledPkg {
        name: pkg.name.clone(),
        kind: pkg.kind.clone(),
        version: pkg.version.clone(),
        source: Some(paths.registry.to_string_lossy().to_string()),
        repo: Some(repo),
        commit: pkg.commit.clone(),
        installed_at: now_epoch_secs(),
        target_dir: None,
        mcp_id: None,
        files: copied
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    };
    let installed_files = installed.files.clone();
    state::upsert(&mut st, installed);
    state::save_state(&state_path, &st)?;
    println!("installed script(s) {}", pkg.name);
    for file in &installed_files {
        println!("  -> {}", file);
    }
    Ok(())
}

fn resolve_repo(pkg: &PackageMeta, repo_override: Option<&str>) -> Result<String> {
    repo_override
        .map(|s| s.to_string())
        .or_else(|| pkg.repo.clone())
        .ok_or_else(|| anyhow!("package {} has no repo URL in registry", pkg.name))
}

fn ensure_supported_kind(pkg: &PackageMeta) -> Result<()> {
    match pkg.kind.as_str() {
        "mcp" => {
            if pkg.mcp.is_none() {
                bail!("package {} has no mcp section", pkg.name);
            }
            Ok(())
        }
        "skill" => {
            if pkg.skill.is_none() {
                bail!("package {} has no skill section", pkg.name);
            }
            Ok(())
        }
        "script" => {
            if pkg.script.is_none() {
                bail!("package {} has no script section", pkg.name);
            }
            Ok(())
        }
        other => bail!(
            "M3 supports mcp/skill/script packages; '{}' is type '{}'",
            pkg.name,
            other
        ),
    }
}

fn clone_cache(paths: &Paths, repo: &str, pkg_name: &str, commit: Option<&str>) -> Result<PathBuf> {
    let dir = paths
        .cache_clones_dir()
        .join(format!("{}-{}", pkg_name, now_epoch_secs()));
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }
    clone_repo(repo, &dir, commit)?;
    Ok(dir)
}

fn package_source_root(cache: &Path, pkg: &PackageMeta) -> Result<PathBuf> {
    let path = pkg
        .install
        .as_ref()
        .and_then(|i| i.path.clone())
        .unwrap_or_else(|| ".".to_string());
    if path == "." || path.is_empty() {
        Ok(cache.to_path_buf())
    } else {
        safe_join(cache, &path)
    }
}

fn safe_join(base: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() || rel.split('/').any(|part| part == "..") {
        bail!("unsafe relative path in package: {}", rel);
    }
    Ok(base.join(rel_path))
}

fn copy_tree_contents(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    let status = Command::new("cp")
        .arg("-R")
        .arg(source.join("."))
        .arg(target)
        .status()
        .with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.display(),
                target.display()
            )
        })?;
    if !status.success() {
        bail!("cp failed while installing {}", source.display());
    }
    let git_dir = target.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir)?;
    }
    Ok(())
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

pub fn remove(paths: &Paths, name: &str, opts: Options) -> Result<()> {
    let state_path = paths.installed_state_file();
    let mut st = state::load_state(&state_path)?;
    let pkg = state::find_installed(&st, name).ok_or_else(|| anyhow!("not installed: {}", name))?;
    let pkg = pkg.clone();
    if opts.dry_run {
        println!("[dry-run] would remove {} from config and state", pkg.name);
        if let Some(dir) = &pkg.target_dir {
            println!("[dry-run] would delete {}", dir);
        }
        for file in &pkg.files {
            println!("[dry-run] would delete {}", file);
        }
        return Ok(());
    }
    confirm_or_abort(&format!("Remove {}?", pkg.name), opts.yes)?;

    match pkg.kind.as_str() {
        "mcp" => {
            if let Some(mcp_id) = &pkg.mcp_id {
                let backup = config::backup_config(paths)?;
                if let Some(b) = &backup {
                    println!("config backed up to {}", b.display());
                }
                let mut cfg = config::load_config(paths)?;
                let changed = config::remove_mcp_server(&mut cfg, mcp_id)?;
                if changed {
                    config::save_config(paths, &cfg)?;
                    println!("removed mcp.servers entry '{}'", mcp_id);
                } else {
                    println!("mcp.servers entry '{}' not found in config", mcp_id);
                }
            }
        }
        "skill" | "script" => {
            // no central config entry to remove for these types
        }
        other => {
            eprintln!(
                "warning: unknown installed type '{}', skipping type cleanup",
                other
            );
        }
    }

    match pkg.kind.as_str() {
        "mcp" => {
            if let Some(dir) = pkg.target_dir.clone() {
                let dir = PathBuf::from(dir);
                if dir.exists() {
                    remove_dir_safe(paths, &dir)
                        .with_context(|| format!("failed to remove {}", dir.display()))?;
                    println!("deleted {}", dir.display());
                } else {
                    println!("target dir already gone: {}", dir.display());
                }
            }
        }
        "skill" => {
            if let Some(dir) = pkg.target_dir.clone() {
                let dir = PathBuf::from(dir);
                if dir.exists() {
                    remove_managed_dir(&dir, &paths.data_skills_dir())
                        .with_context(|| format!("failed to remove {}", dir.display()))?;
                    println!("deleted skill {}", dir.display());
                } else {
                    println!("skill dir already gone: {}", dir.display());
                }
            }
        }
        "script" => {
            for file in &pkg.files {
                let file = PathBuf::from(file);
                if file.exists() {
                    remove_managed_file(paths, &file)
                        .with_context(|| format!("failed to remove {}", file.display()))?;
                    println!("deleted {}", file.display());
                } else {
                    println!("script file already gone: {}", file.display());
                }
            }
        }
        _ => {}
    }

    state::remove_by_name(&mut st, &pkg.name);
    state::save_state(&state_path, &st)?;
    println!("removed {}", pkg.name);
    Ok(())
}

pub fn update(paths: &Paths) -> Result<()> {
    let is_official =
        paths.registry.file_name().and_then(|n| n.to_str()) == Some("official-index.json");
    if is_official {
        registry::refresh_default_registry(&paths.registry)?;
    }
    let index = registry::load_registry(&paths.registry)?;
    if is_official {
        println!(
            "✓ official source updated ({} package(s))",
            index.packages.len()
        );
    } else {
        println!(
            "✓ registry updated: {} ({} package(s))",
            paths.registry.display(),
            index.packages.len()
        );
    }
    crate::source::refresh_all(paths)?;
    Ok(())
}

pub fn upgrade(paths: &Paths, names: &[String], no_setup: bool, opts: Options) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let state_path = paths.installed_state_file();
    let mut st = state::load_state(&state_path)?;
    if st.installed.is_empty() {
        println!("nothing installed; nothing to upgrade");
        return Ok(());
    }
    let targets: Vec<InstalledPkg> = if names.is_empty() {
        st.installed.clone()
    } else {
        names
            .iter()
            .filter_map(|n| state::find_installed(&st, n).cloned())
            .collect()
    };
    if targets.is_empty() {
        bail!("no matching installed packages to upgrade");
    }

    println!("The following packages will be upgraded:");
    for pkg in &targets {
        if let Some(meta) = registry::find_package(&index, &pkg.name) {
            println!("  {}  {} -> {}", pkg.name, pkg.version, meta.version);
        } else {
            println!("  {}  {} -> latest", pkg.name, pkg.version);
        }
    }
    confirm_or_abort("Do you want to proceed with the upgrade?", opts.yes)?;

    for pkg in &targets {
        let meta = registry::find_package(&index, &pkg.name).cloned();
        match pkg.kind.as_str() {
            "mcp" => {
                let dir = pkg
                    .target_dir
                    .as_ref()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow!("{} has no target_dir in state", pkg.name))?;
                if !dir.join(".git").exists() {
                    eprintln!(
                        "warning: {} is not a git checkout at {}",
                        pkg.name,
                        dir.display()
                    );
                    continue;
                }
                if opts.dry_run {
                    println!("[dry-run] would pull latest for {}", pkg.name);
                    continue;
                }
                let status = Command::new("git")
                    .args(["pull", "--ff-only"])
                    .current_dir(&dir)
                    .status()
                    .with_context(|| format!("git pull failed for {}", pkg.name))?;
                if !status.success() {
                    eprintln!("warning: git pull reported failure for {}", pkg.name);
                } else {
                    println!("updated {}", pkg.name);
                }

                let setup_plan = meta
                    .as_ref()
                    .and_then(|p| p.install.as_ref())
                    .map(|i| i.setup.clone())
                    .unwrap_or_default();
                run_setup(&pkg.name, &setup_plan, &dir, no_setup)?;
            }
            "skill" => {
                let repo = pkg
                    .repo
                    .clone()
                    .ok_or_else(|| anyhow!("{} has no repo in state", pkg.name))?;
                let dir = pkg
                    .target_dir
                    .as_ref()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow!("{} has no skill dir in state", pkg.name))?;
                if !dir.exists() {
                    bail!("skill dir missing: {}", dir.display());
                }
                if opts.dry_run {
                    println!("[dry-run] would refresh skill {}", pkg.name);
                    continue;
                }
                let cache = clone_cache(paths, &repo, &pkg.name, None)?;
                let result = (|| -> Result<()> {
                    let source = match &meta {
                        Some(m) => package_source_root(&cache, m)?,
                        None => cache.clone(),
                    };
                    copy_tree_contents(&source, &dir)?;
                    Ok(())
                })();
                let _ = fs::remove_dir_all(&cache);
                result?;
                println!("updated skill {}", pkg.name);
            }
            "script" => {
                let repo = pkg
                    .repo
                    .clone()
                    .ok_or_else(|| anyhow!("{} has no repo in state", pkg.name))?;
                if pkg.files.is_empty() {
                    eprintln!("warning: {} has no recorded script files", pkg.name);
                    continue;
                }
                if opts.dry_run {
                    println!("[dry-run] would refresh script package {}", pkg.name);
                    continue;
                }
                let cache = clone_cache(paths, &repo, &pkg.name, None)?;
                let result = (|| -> Result<()> {
                    let source = match &meta {
                        Some(m) => package_source_root(&cache, m)?,
                        None => cache.clone(),
                    };
                    for file in &pkg.files {
                        let dest = PathBuf::from(file);
                        let file_name = dest
                            .file_name()
                            .ok_or_else(|| anyhow!("invalid installed file {}", dest.display()))?;
                        let src = source.join(file_name);
                        if !src.is_file() {
                            bail!("upstream file missing for {}: {}", pkg.name, src.display());
                        }
                        fs::copy(&src, &dest)?;
                        set_executable(&dest)?;
                    }
                    Ok(())
                })();
                let _ = fs::remove_dir_all(&cache);
                result?;
                println!("updated script package {}", pkg.name);
            }
            other => {
                println!(
                    "skipping {}: {} upgrade is not implemented",
                    pkg.name, other
                );
                continue;
            }
        }

        if let Some(meta) = &meta {
            if let Some(item) = st.installed.iter_mut().find(|i| i.name == pkg.name) {
                item.version = meta.version.clone();
                item.installed_at = now_epoch_secs();
            }
        }
    }
    state::save_state(&state_path, &st)?;
    Ok(())
}

pub fn doctor(paths: &Paths) -> Result<()> {
    println!("miyu home : {}", paths.miyu_home.display());
    println!("pm home   : {}", paths.pm_home.display());
    println!("registry  : {}", paths.registry.display());
    let mut issues = 0usize;

    let cfg = match config::load_config(paths) {
        Ok(c) => c,
        Err(e) => {
            println!("  [issue] config: {}", e);
            return Ok(());
        }
    };
    println!(
        "config     : {} mcp.servers entries",
        config::count_mcp_servers(&cfg)
    );

    let st = state::load_state(&paths.installed_state_file())?;
    println!("installed  : {} packages", st.installed.len());
    for p in &st.installed {
        let mut ok = true;
        if let Some(dir) = &p.target_dir {
            if !Path::new(dir).exists() {
                println!("  [issue] {} target dir missing: {}", p.name, dir);
                ok = false;
            }
        }
        if p.kind == "script" {
            if p.files.is_empty() {
                println!("  [issue] {} script package has no recorded files", p.name);
                ok = false;
            }
            for file in &p.files {
                if !Path::new(file).exists() {
                    println!("  [issue] {} script file missing: {}", p.name, file);
                    ok = false;
                }
            }
        }
        if let (Some(id), Some(dir)) = (&p.mcp_id, &p.target_dir) {
            if !config::has_mcp_server(&cfg, id) {
                println!("  [issue] {} mcp entry missing in config: {}", p.name, id);
                ok = false;
            } else if !Path::new(dir).exists() {
                println!("  [issue] {} dir missing: {}", p.name, dir);
                ok = false;
            }
        }
        if ok {
            println!("  ok       : {}", p.name);
        } else {
            issues += 1;
        }
    }
    println!(
        "doctor complete: {} issue(s){}",
        issues,
        if issues == 0 { " (all good)" } else { "" }
    );
    Ok(())
}

pub fn status(paths: &Paths) -> Result<()> {
    println!("miyu-pm v{}", env!("CARGO_PKG_VERSION"));
    println!("miyu home : {}", paths.miyu_home.display());
    println!("pm home   : {}", paths.pm_home.display());
    println!("registry  : {}", paths.registry.display());
    println!(
        "config    : {}",
        if paths.config_file().exists() {
            "exists"
        } else {
            "missing (defaults will be used)"
        }
    );
    let cfg = config::load_config(paths).unwrap_or_else(|_| config::default_config());
    println!("mcp.servers count: {}", config::count_mcp_servers(&cfg));
    let st = state::load_state(&paths.installed_state_file())?;
    println!("installed package count: {}", st.installed.len());
    Ok(())
}

pub fn audit(paths: &Paths, name: &str) -> Result<()> {
    let index = registry::load_registry(&paths.registry)?;
    let pkg = registry::find_package(&index, name)
        .ok_or_else(|| anyhow!("package not found in registry: {}", name))?;
    println!(
        "audit {} (static checks only; not a security guarantee)",
        pkg.name
    );
    println!("  type    : {}", pkg.kind);
    println!("  trust   : community (unreviewed)");
    if pkg.archived == Some(true) {
        println!("  warning : archived upstream");
    }
    if let Some(mcp) = &pkg.mcp {
        println!("  command : {}", mcp.command);
        if !mcp.env.is_empty() {
            println!(
                "  env keys: {}",
                mcp.env.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }
    let setup = pkg
        .install
        .as_ref()
        .map(|i| i.setup.as_slice())
        .unwrap_or(&[]);
    if setup.is_empty() {
        println!("  setup   : none");
    } else {
        let risky = [
            "curl", "wget", "sudo", "chmod", "eval", "base64", "| sh", "bash -c", "rm ",
        ];
        let mut risk = false;
        for cmd in setup {
            println!("  setup   : {}", cmd);
            for kw in risky {
                if cmd.contains(kw) {
                    println!("    ^ risk keyword: {}", kw);
                    risk = true;
                }
            }
        }
        if risk {
            println!("  result  : review setup commands carefully");
        } else {
            println!("  result  : no obvious setup risk keywords");
        }
    }
    Ok(())
}

pub fn self_update(paths: &Paths, opts: Options) -> Result<()> {
    println!("current miyu-pm v{}", env!("CARGO_PKG_VERSION"));
    let index = registry::load_registry(&paths.registry)?;
    let pkg = match registry::find_package(&index, "miyu-pm") {
        Some(p) => p,
        None => {
            println!("miyu-pm package not found in active registry; nothing to update");
            println!("hint: publish miyu-pm as an app package or add the official source");
            return Ok(());
        }
    };
    if pkg.kind != "app" {
        bail!(
            "registry package miyu-pm is type '{}', expected 'app'",
            pkg.kind
        );
    }
    let release = pkg
        .release
        .as_ref()
        .ok_or_else(|| anyhow!("miyu-pm app package has no release section"))?;
    let repo = release
        .repo
        .as_deref()
        .ok_or_else(|| anyhow!("release.repo is missing"))?;
    let pattern = release
        .asset_pattern
        .as_deref()
        .ok_or_else(|| anyhow!("release.asset_pattern is missing"))?;

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "30",
            "-H",
            "Accept: application/vnd.github+json",
        ])
        .arg(&api_url)
        .output()
        .with_context(|| format!("failed to query GitHub release {}", api_url))?;
    if !output.status.success() {
        bail!(
            "GitHub release query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let release_json: Value =
        serde_json::from_slice(&output.stdout).context("invalid GitHub release JSON")?;
    let tag = release_json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let version = tag.trim_start_matches('v');
    let os = match std::env::consts::OS {
        "macos" => "macos",
        "linux" => "linux",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => other,
    };
    let asset_name = pattern
        .replace("{version}", version)
        .replace("{os}", os)
        .replace("{arch}", arch);
    let asset = release_json
        .get("assets")
        .and_then(|a| a.as_array())
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| a.get("name").and_then(|v| v.as_str()) == Some(asset_name.as_str()))
        })
        .ok_or_else(|| {
            let names: Vec<String> = release_json
                .get("assets")
                .and_then(|a| a.as_array())
                .map(|assets| {
                    assets
                        .iter()
                        .filter_map(|a| a.get("name").and_then(|v| v.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            anyhow!(
                "asset {} not found in release {}; available: {}",
                asset_name,
                tag,
                names.join(", ")
            )
        })?;
    let download_url = asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("asset has no browser_download_url"))?;

    println!("latest release : {}", tag);
    println!("asset          : {}", asset_name);
    if opts.dry_run {
        println!("[dry-run] would download {}", download_url);
        return Ok(());
    }
    confirm_or_abort(
        &format!("Download and replace current miyu-pm binary with {}?", tag),
        opts.yes,
    )?;

    let current = std::env::current_exe().context("cannot locate current executable")?;
    let cache_dir = paths.cache_dir();
    fs::create_dir_all(&cache_dir)?;
    let ts = now_epoch_secs();
    let archive = cache_dir.join(format!("miyu-pm-self-update-{}.tar.gz", ts));
    let extract = cache_dir.join(format!("miyu-pm-self-update-extract-{}", ts));
    fs::create_dir_all(&extract)?;

    let download = Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o"])
        .arg(&archive)
        .arg(download_url)
        .status()
        .with_context(|| format!("failed to download {}", download_url))?;
    if !download.success() {
        bail!("download failed for {}", download_url);
    }
    let extract_status = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&extract)
        .status()
        .with_context(|| format!("failed to extract {}", archive.display()))?;
    if !extract_status.success() {
        bail!("extract failed for {}", archive.display());
    }
    let candidate = extract.join("miyu-pm");
    if !candidate.is_file() {
        bail!(
            "extracted archive does not contain miyu-pm binary (found at {})",
            extract.display()
        );
    }
    set_executable(&candidate)?;

    let backup = cache_dir.join(format!("miyu-pm.bak.{}", ts));
    if current.exists() {
        fs::copy(&current, &backup)?;
        println!("backed up current binary to {}", backup.display());
    }
    match fs::rename(&candidate, &current) {
        Ok(()) => {}
        Err(_) => {
            fs::copy(&candidate, &current)?;
            set_executable(&current)?;
        }
    }
    println!("updated miyu-pm to {}", tag);
    println!("restart miyu-pm to use the new version");
    Ok(())
}

pub fn tui(_paths: &Paths, args: &[String]) -> Result<()> {
    let script = find_tui_script()?;
    let status = Command::new(&script)
        .args(args)
        .status()
        .with_context(|| format!("failed to run TUI script {}", script.display()))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn find_tui_script() -> Result<PathBuf> {
    if let Some(script) = env::var_os("MIYU_PM_TUI_SCRIPT") {
        let path = PathBuf::from(script);
        if path.is_file() {
            return Ok(path);
        }
    }
    let exe = std::env::current_exe().context("cannot locate current executable")?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| anyhow!("cannot determine executable directory"))?;
    let candidates = [
        // development layout: <repo>/target/debug/miyu-pm -> <repo>/tui/bin
        exe_dir.join("../../tui/bin/miyu-pm-tui"),
        // installed layout: ~/.local/bin/miyu-pm -> ~/.local/share/miyu-pm/tui/bin
        exe_dir.join("../share/miyu-pm/tui/bin/miyu-pm-tui"),
        // current directory layout
        PathBuf::from("tui/bin/miyu-pm-tui"),
    ];
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "TUI script not found.\n\
         Run miyu-pm from the source tree, set MIYU_PM_TUI_SCRIPT=/path/to/miyu-pm-tui, \
         or reinstall with a release that bundles tui/"
    );
}

fn ensure_m1_kind(pkg: &PackageMeta) -> Result<()> {
    if pkg.kind != "mcp" {
        bail!(
            "M1 prototype only installs mcp packages; '{}' is type '{}'",
            pkg.name,
            pkg.kind
        );
    }
    if pkg.mcp.is_none() {
        bail!("package {} has no mcp section", pkg.name);
    }
    Ok(())
}

fn confirm_or_abort(prompt: &str, yes: bool) -> Result<()> {
    if yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        bail!("{} use --yes to proceed non-interactively", prompt);
    }
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if !line.trim().eq_ignore_ascii_case("y") {
        bail!("aborted");
    }
    Ok(())
}

fn clone_repo(repo: &str, target: &Path, commit: Option<&str>) -> Result<()> {
    let status = Command::new("git")
        .args(["clone", "--quiet", repo])
        .arg(target)
        .status()
        .with_context(|| format!("git clone {} failed", repo))?;
    if !status.success() {
        bail!("git clone {} failed", repo);
    }
    if let Some(commit) = commit {
        if !commit.is_empty() {
            let status = Command::new("git")
                .args(["-C"])
                .arg(target)
                .args([
                    "-c",
                    "advice.detachedHead=false",
                    "checkout",
                    "--quiet",
                    commit,
                ])
                .status()?;
            if !status.success() {
                eprintln!(
                    "warning: could not checkout commit {}; staying on default branch",
                    commit
                );
            }
        }
    }
    Ok(())
}

fn run_setup(pkg_name: &str, setup: &[String], target: &Path, no_setup: bool) -> Result<()> {
    if setup.is_empty() {
        return Ok(());
    }
    if no_setup {
        println!("skipping setup for {} (--no-setup)", pkg_name);
        return Ok(());
    }
    for cmd in setup {
        println!("running setup: {}", cmd);
        let status = Command::new("/bin/sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(target)
            .status()
            .with_context(|| format!("setup command failed: {}", cmd))?;
        if !status.success() {
            bail!("setup command failed with status {}: {}", status, cmd);
        }
    }
    Ok(())
}

fn parse_env_overrides(items: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for item in items {
        if let Some((k, v)) = item.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

fn resolve_text(
    input: &str,
    root: &str,
    overrides: &BTreeMap<String, String>,
) -> (String, Vec<String>) {
    let mut out = input.to_string();
    let mut warnings = Vec::new();
    while let Some(pos) = out.find("{root}") {
        out.replace_range(pos..pos + "{root}".len(), root);
    }
    for kind in ["{env:", "{user:"] {
        let label = kind;
        loop {
            let Some(start) = out.find(label) else { break };
            let body_start = start + label.len();
            let Some(rel_end) = out[body_start..].find('}') else {
                warnings.push(format!("unterminated placeholder at {}", start));
                break;
            };
            let end = body_start + rel_end;
            let var = out[body_start..end].to_string();
            let value = overrides
                .get(&var)
                .cloned()
                .or_else(|| env::var(&var).ok())
                .unwrap_or_default();
            if value.is_empty() {
                warnings.push(format!(
                    "{} is empty; please provide it via --env {}={}... or your environment",
                    label.trim_end_matches(':').trim_end_matches('{'),
                    var,
                    var
                ));
            }
            out.replace_range(start..=end, &value);
        }
    }
    (out, warnings)
}

fn remove_dir_safe(paths: &Paths, dir: &Path) -> Result<()> {
    if !paths.is_managed_mcp_dir(dir) {
        bail!("refusing to remove non-managed path {}", dir.display());
    }
    if dir == paths.mcp_servers_dir() {
        bail!("refusing to remove the whole mcp-servers directory");
    }
    fs::remove_dir_all(dir).with_context(|| format!("failed to remove {}", dir.display()))?;
    Ok(())
}

fn remove_managed_dir(dir: &Path, root: &Path) -> Result<()> {
    if !dir.starts_with(root) {
        bail!("refusing to remove non-managed path {}", dir.display());
    }
    if dir == root {
        bail!("refusing to remove whole data directory {}", root.display());
    }
    fs::remove_dir_all(dir).with_context(|| format!("failed to remove {}", dir.display()))?;
    Ok(())
}

fn remove_managed_file(paths: &Paths, file: &Path) -> Result<()> {
    if !paths.is_managed_scripts_file(file) {
        bail!("refusing to remove non-managed file {}", file.display());
    }
    fs::remove_file(file).with_context(|| format!("failed to remove {}", file.display()))?;
    Ok(())
}
