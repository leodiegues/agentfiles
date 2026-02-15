use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::manifest::{Dependency, FileMapping};
use crate::types::{AgentProvider, FileKind, FileScope, FileStrategy};
use crate::{git, installer, manifest, scanner};

// ---------------------------------------------------------------------------
// Install command
// ---------------------------------------------------------------------------

/// Install agent files. Two flows:
///
/// - **No source**: reads `agentfiles.json` from the project root and installs
///   all dependencies listed there.
/// - **With source**: resolves the source, scans it for agent files, installs
///   them, and (unless `no_save` is set) adds the source to `agentfiles.json`.
pub fn cmd_install(
    source: Option<String>,
    scope: FileScope,
    providers: Option<Vec<AgentProvider>>,
    strategy_override: Option<FileStrategy>,
    pick: Option<Vec<String>>,
    no_save: bool,
    root: PathBuf,
) -> Result<()> {
    let providers = providers.unwrap_or_else(|| AgentProvider::ALL.to_vec());

    let project_root = root
        .canonicalize()
        .context("could not resolve project root")?;

    match source {
        None => install_from_manifest(&project_root, &providers, &scope, strategy_override),
        Some(src) => install_from_source(
            &src,
            &project_root,
            &providers,
            &scope,
            strategy_override,
            pick.as_deref(),
            no_save,
        ),
    }
}

/// Install all dependencies listed in the project's `agentfiles.json`.
fn install_from_manifest(
    project_root: &std::path::Path,
    providers: &[AgentProvider],
    scope: &FileScope,
    strategy_override: Option<FileStrategy>,
) -> Result<()> {
    let manifest_path = project_root.join("agentfiles.json");
    if !manifest_path.is_file() {
        anyhow::bail!(
            "no agentfiles.json found in {}. Run 'agentfiles init' first or specify a source.",
            project_root.display()
        );
    }

    let loaded = manifest::load_manifest(project_root)?;
    if loaded.dependencies.is_empty() {
        println!("No dependencies in agentfiles.json. Add one with 'agentfiles install <source>'.");
        return Ok(());
    }

    println!(
        "Installing {} dependency(ies) from '{}' (v{})...\n",
        loaded.dependencies.len(),
        loaded.name,
        loaded.version,
    );

    let mut total_results = Vec::new();

    for dep in &loaded.dependencies {
        let dep_results =
            install_dependency(dep, project_root, providers, scope, strategy_override)?;
        total_results.extend(dep_results);
    }

    print_results(&total_results);
    Ok(())
}

/// Install from a specific source, optionally saving it to agentfiles.json.
fn install_from_source(
    source: &str,
    project_root: &std::path::Path,
    providers: &[AgentProvider],
    scope: &FileScope,
    strategy_override: Option<FileStrategy>,
    pick: Option<&[String]>,
    no_save: bool,
) -> Result<()> {
    let (source_dir, mut files) = resolve_source(source)?;

    // Apply pick filter
    if let Some(pick_list) = pick {
        files = scanner::filter_by_pick(files, pick_list);
        if files.is_empty() {
            anyhow::bail!("no files matched the pick filter");
        }
    }

    // Apply strategy override
    if let Some(strategy) = strategy_override {
        for file in &mut files {
            if file.strategy == FileStrategy::Copy {
                file.strategy = strategy;
            }
        }
    }

    let results = installer::install(&files, providers, scope, project_root, &source_dir)?;

    if !no_save {
        save_dependency(source, pick, project_root)?;
    }

    print_results(&results);
    Ok(())
}

/// Install a single dependency from the manifest.
fn install_dependency(
    dep: &Dependency,
    project_root: &std::path::Path,
    providers: &[AgentProvider],
    scope: &FileScope,
    strategy_override: Option<FileStrategy>,
) -> Result<Vec<installer::InstallResult>> {
    let source = dep.source();
    println!("  -> {source}");

    let (source_dir, mut files) = resolve_source(source)?;

    // Apply custom path mappings if specified
    if let Some(custom_paths) = dep.paths() {
        // Re-scan with custom paths (the initial resolve_source used defaults)
        files = scanner::scan_agent_files(&source_dir, Some(custom_paths))?;
    }

    // Apply pick filter
    if let Some(pick_list) = dep.pick() {
        files = scanner::filter_by_pick(files, pick_list);
    }

    // Apply per-dependency strategy, then global override
    let dep_strategy = dep.strategy();
    for file in &mut files {
        if let Some(strategy) = dep_strategy
            && file.strategy == FileStrategy::Copy
        {
            file.strategy = strategy;
        }
        if let Some(strategy) = strategy_override
            && file.strategy == FileStrategy::Copy
        {
            file.strategy = strategy;
        }
    }

    if files.is_empty() {
        println!("    (no matching files found)");
        return Ok(vec![]);
    }

    installer::install(&files, providers, scope, project_root, &source_dir)
}

// ---------------------------------------------------------------------------
// Source resolution
// ---------------------------------------------------------------------------

/// Resolve a source (remote or local) to a local directory and scanned files.
fn resolve_source(source: &str) -> Result<(PathBuf, Vec<FileMapping>)> {
    if git::is_git_url(source) {
        resolve_remote_source(source)
    } else {
        resolve_local_source(source)
    }
}

/// Clone/fetch a remote git repo and scan for agent files.
fn resolve_remote_source(source: &str) -> Result<(PathBuf, Vec<FileMapping>)> {
    let remote = git::parse_remote(source);

    let ref_display = remote
        .git_ref
        .as_deref()
        .map(|r| format!(" @ {r}"))
        .unwrap_or_default();
    println!("Resolving remote: {}{ref_display}", remote.url);

    let git_source = git::resolve_remote(&remote)?;
    let local_path = git_source.local_path;

    println!("Cached at: {}\n", local_path.display());

    let files = scanner::scan_agent_files(&local_path, None)?;
    if files.is_empty() {
        anyhow::bail!("no agent files found in {}", git_source.url);
    }
    println!("Discovered {} agent file(s).\n", files.len());

    Ok((local_path, files))
}

/// Resolve a local path and scan for agent files.
fn resolve_local_source(source: &str) -> Result<(PathBuf, Vec<FileMapping>)> {
    let path = PathBuf::from(source);
    if !path.exists() {
        anyhow::bail!("source path not found: {}", path.display());
    }

    let dir = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let files = scanner::scan_agent_files(&dir, None)?;
    if files.is_empty() {
        anyhow::bail!("no agent files found in {}", dir.display());
    }

    let canonical = dir
        .canonicalize()
        .context("could not resolve source path")?;
    Ok((canonical, files))
}

// ---------------------------------------------------------------------------
// Manifest auto-save
// ---------------------------------------------------------------------------

/// Add a dependency to agentfiles.json, creating the file if it doesn't exist.
fn save_dependency(
    source: &str,
    pick: Option<&[String]>,
    project_root: &std::path::Path,
) -> Result<()> {
    let manifest_path = project_root.join("agentfiles.json");

    let mut loaded = if manifest_path.is_file() {
        manifest::load_manifest(project_root)?
    } else {
        let name = scanner::infer_name(project_root);
        manifest::Manifest::default().with_name(name)
    };

    let dep = if let Some(pick_list) = pick {
        Dependency::Detailed(manifest::DependencySpec {
            source: source.to_string(),
            git_ref: None,
            pick: Some(pick_list.to_vec()),
            strategy: None,
            paths: None,
        })
    } else {
        Dependency::Simple(source.to_string())
    };

    if loaded.add_dependency(dep) {
        manifest::save_manifest(&loaded, project_root)?;
        println!("Saved to agentfiles.json");
    } else {
        println!("Dependency already in agentfiles.json");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn print_results(results: &[installer::InstallResult]) {
    if results.is_empty() {
        println!("No files installed (no compatible provider/kind combinations found).");
    } else {
        println!("\nInstalled {} file(s):\n", results.len(),);
        for r in results {
            println!(
                "  [{:>11}] {} -> {} ({})",
                r.provider.to_string(),
                r.source,
                r.target,
                r.strategy
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Init command
// ---------------------------------------------------------------------------

pub fn cmd_init(path: PathBuf, name: Option<String>) -> Result<()> {
    let dir = if path.is_dir() {
        path.clone()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let manifest_path = dir.join("agentfiles.json");
    if manifest_path.exists() {
        anyhow::bail!(
            "agentfiles.json already exists at {}",
            manifest_path.display()
        );
    }

    let pkg_name = name.unwrap_or_else(|| scanner::infer_name(&dir));

    let m = manifest::Manifest::default().with_name(pkg_name);

    let output_path = manifest::save_manifest(&m, &dir)?;
    println!("Created {}", output_path.display());
    println!(
        "Add dependencies with 'agentfiles install <source>' or edit agentfiles.json directly."
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scan command
// ---------------------------------------------------------------------------

pub fn cmd_scan(source: String) -> Result<()> {
    let files = if git::is_git_url(&source) {
        let remote = git::parse_remote(&source);

        let ref_display = remote
            .git_ref
            .as_deref()
            .map(|r| format!(" @ {r}"))
            .unwrap_or_default();
        println!("Resolving remote: {}{ref_display}", remote.url);

        let git_source = git::resolve_remote(&remote)?;
        println!("Cached at: {}\n", git_source.local_path.display());

        scanner::scan_agent_files(&git_source.local_path, None)?
    } else {
        scanner::scan_agent_files(&PathBuf::from(&source), None)?
    };

    if files.is_empty() {
        println!("No agent files found in {source}");
        return Ok(());
    }

    println!("Found {} agent file(s):\n", files.len());
    for f in &files {
        println!("  [{}] {}", f.kind, f.path.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix command
// ---------------------------------------------------------------------------

pub fn cmd_matrix() -> Result<()> {
    let kinds = [FileKind::Skill, FileKind::Command, FileKind::Agent];
    let providers = AgentProvider::ALL;

    // Header
    print!("{:<14}", "Provider");
    for kind in &kinds {
        print!("{:<12}", kind.to_string());
    }
    println!();

    // Separator
    print!("{}", "-".repeat(14));
    for _ in &kinds {
        print!("{}", "-".repeat(12));
    }
    println!();

    // Rows
    for provider in providers {
        print!("{:<14}", provider.to_string());
        for kind in &kinds {
            let supported = if provider.supports_kind(kind) {
                "Yes"
            } else {
                "-"
            };
            print!("{:<12}", supported);
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Scan command tests
    // -----------------------------------------------------------------------

    #[test]
    fn scan_local_with_files() -> Result<()> {
        let dir = TempDir::new()?;
        // Create bare skills directory
        let skill_dir = dir.path().join("skills").join("review");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "test skill")?;

        let source = dir.path().to_string_lossy().into_owned();
        let result = cmd_scan(source);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn scan_local_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let source = dir.path().to_string_lossy().into_owned();
        let result = cmd_scan(source);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn scan_local_nonexistent_path() {
        let result = cmd_scan("/tmp/this-path-definitely-does-not-exist-agentfiles".into());
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Init command tests
    // -----------------------------------------------------------------------

    #[test]
    fn init_creates_empty_manifest() -> Result<()> {
        let dir = TempDir::new()?;
        cmd_init(dir.path().to_path_buf(), Some("my-project".to_string()))?;

        let manifest_path = dir.path().join("agentfiles.json");
        assert!(manifest_path.exists());

        let loaded = manifest::load_manifest(dir.path())?;
        assert_eq!(loaded.name, "my-project");
        assert!(loaded.dependencies.is_empty());
        Ok(())
    }

    #[test]
    fn init_errors_if_manifest_exists() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("agentfiles.json"), "{}")?;

        let result = cmd_init(dir.path().to_path_buf(), None);
        assert!(result.is_err());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Install from manifest tests
    // -----------------------------------------------------------------------

    #[test]
    fn install_no_source_without_manifest_errors() {
        let dir = TempDir::new().unwrap();
        let result = cmd_install(
            None,
            FileScope::Project,
            None,
            None,
            None,
            false,
            dir.path().to_path_buf(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn install_no_source_with_empty_manifest() -> Result<()> {
        let dir = TempDir::new()?;
        let manifest = manifest::Manifest::default().with_name("test".to_string());
        manifest::save_manifest(&manifest, dir.path())?;

        // Should succeed but print "no dependencies" message
        let result = cmd_install(
            None,
            FileScope::Project,
            None,
            None,
            None,
            false,
            dir.path().to_path_buf(),
        );
        assert!(result.is_ok());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Auto-save tests
    // -----------------------------------------------------------------------

    #[test]
    fn install_source_auto_saves_to_manifest() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        // Create agent files in source
        let skill_dir = src_dir.path().join("skills").join("review");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Review")?;

        let source = src_dir.path().to_string_lossy().into_owned();

        cmd_install(
            Some(source.clone()),
            FileScope::Project,
            None,
            None,
            None,
            false, // auto-save on
            dst_dir.path().to_path_buf(),
        )?;

        // agentfiles.json should be created in the project root
        let manifest_path = dst_dir.path().join("agentfiles.json");
        assert!(manifest_path.exists());

        let loaded = manifest::load_manifest(dst_dir.path())?;
        assert_eq!(loaded.dependencies.len(), 1);
        assert_eq!(loaded.dependencies[0].source(), source);
        Ok(())
    }

    #[test]
    fn install_source_no_save_skips_manifest() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        let skill_dir = src_dir.path().join("skills").join("review");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Review")?;

        let source = src_dir.path().to_string_lossy().into_owned();

        cmd_install(
            Some(source),
            FileScope::Project,
            None,
            None,
            None,
            true, // no-save
            dst_dir.path().to_path_buf(),
        )?;

        // agentfiles.json should NOT be created
        assert!(!dst_dir.path().join("agentfiles.json").exists());
        Ok(())
    }
}
