use agentfiles::{cli, git, installer, manifest, scanner, types};
use anyhow::{Context, Result};
use clap::Parser;
use types::{AgentProvider, FileScope, FileStrategy};

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    match args.command {
        cli::Command::Install {
            source,
            scope,
            providers,
            strategy,
            root,
        } => cmd_install(source, scope, providers, strategy, root),
        cli::Command::Init { path, name } => cmd_init(path, name),
        cli::Command::Scan { path, write } => cmd_scan(path, write),
        cli::Command::Matrix => cmd_matrix(),
    }
}

fn cmd_install(
    source: String,
    scope_str: String,
    providers_str: Option<Vec<String>>,
    strategy_str: Option<String>,
    root: std::path::PathBuf,
) -> Result<()> {
    let scope: FileScope = scope_str.parse()?;

    // Parse providers or default to all
    let providers = match providers_str {
        Some(names) => names
            .iter()
            .map(|n| n.parse::<AgentProvider>())
            .collect::<Result<Vec<_>>>()?,
        None => AgentProvider::all(),
    };

    // Parse global strategy override
    let strategy_override: Option<FileStrategy> = strategy_str
        .map(|s| s.parse::<FileStrategy>())
        .transpose()?;

    // Resolve the source: either a remote git URL or a local path
    let (manifest_dir, mut loaded) = if git::is_git_url(&source) {
        resolve_remote_source(&source)?
    } else {
        resolve_local_source(&source)?
    };

    // Apply strategy override to all files that use the default
    if let Some(ref strategy) = strategy_override {
        for file in &mut loaded.files {
            if file.strategy == FileStrategy::Copy {
                file.strategy = strategy.clone();
            }
        }
    }

    let project_root = root
        .canonicalize()
        .context("could not resolve project root")?;

    let results = installer::install(&loaded, &providers, &scope, &project_root, &manifest_dir)?;

    if results.is_empty() {
        println!("No files installed (no compatible provider/kind combinations found).");
    } else {
        println!(
            "Installed {} file(s) from '{}' (v{}):\n",
            results.len(),
            loaded.name,
            loaded.version
        );
        for r in &results {
            println!(
                "  [{:>11}] {} -> {} ({})",
                r.provider.to_string(),
                r.source,
                r.target,
                r.strategy
            );
        }
    }

    Ok(())
}

/// Resolve a remote git URL to a local directory and manifest.
///
/// Clones (or updates the cache of) the remote repository, then either
/// loads `agentfiles.json` from it or auto-discovers agent files via scanning.
fn resolve_remote_source(source: &str) -> Result<(std::path::PathBuf, manifest::Manifest)> {
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

    // Try to load a manifest; fall back to scanning
    let manifest_path = local_path.join("agentfiles.json");
    let loaded = if manifest_path.is_file() {
        println!("Found agentfiles.json in remote repository.");
        manifest::load_manifest(&local_path)?
    } else {
        println!("No agentfiles.json found — scanning for agent files...");
        let files = scanner::scan_agent_files(&local_path)?;
        if files.is_empty() {
            anyhow::bail!(
                "no agentfiles.json and no agent files found in {}",
                git_source.url
            );
        }
        println!("Discovered {} agent file(s) via scan.\n", files.len());

        // Build a synthetic manifest from scanned files
        let name = remote
            .url
            .rsplit('/')
            .next()
            .unwrap_or("remote")
            .trim_end_matches(".git")
            .to_string();

        manifest::Manifest::default()
            .with_name(name)
            .with_repository(remote.url.clone())
            .with_files(files)
    };

    Ok((local_path, loaded))
}

/// Resolve a local path to a directory and manifest.
fn resolve_local_source(source: &str) -> Result<(std::path::PathBuf, manifest::Manifest)> {
    let path = std::path::PathBuf::from(source);

    let manifest_dir = if path.is_dir() {
        path.clone()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };

    let loaded = manifest::load_manifest(&path)?;
    Ok((manifest_dir, loaded))
}

fn cmd_init(path: std::path::PathBuf, name: Option<String>) -> Result<()> {
    let dir = if path.is_dir() {
        path.clone()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };

    let manifest_path = dir.join("agentfiles.json");
    if manifest_path.exists() {
        anyhow::bail!(
            "agentfiles.json already exists at {}",
            manifest_path.display()
        );
    }

    // Try to scan for existing files
    let files = scanner::scan_agent_files(&dir).unwrap_or_default();

    let pkg_name = name.unwrap_or_else(|| scanner::infer_name(&dir));

    let m = manifest::Manifest::default()
        .with_name(pkg_name)
        .with_files(files);

    let output_path = manifest::save_manifest(&m, &dir)?;
    println!("Created {}", output_path.display());

    if !m.files.is_empty() {
        println!("Discovered {} agent file(s):", m.files.len());
        for f in &m.files {
            println!("  {} ({})", f.path.display(), f.kind);
        }
    } else {
        println!(
            "No agent files found. Add files to skills/, commands/, or agents/ and run 'agentfiles scan'."
        );
    }

    Ok(())
}

fn cmd_scan(path: std::path::PathBuf, write: bool) -> Result<()> {
    let files = scanner::scan_agent_files(&path)?;

    if files.is_empty() {
        println!("No agent files found in {}", path.display());
        return Ok(());
    }

    println!("Found {} agent file(s):\n", files.len());
    for f in &files {
        println!("  [{}] {}", f.kind, f.path.display());
    }

    if write {
        let name = scanner::infer_name(&path);
        let m = manifest::Manifest::default()
            .with_name(name)
            .with_files(files);
        let output = manifest::save_manifest(&m, &path)?;
        println!("\nWrote manifest to {}", output.display());
    }

    Ok(())
}

fn cmd_matrix() -> Result<()> {
    use types::FileKind;

    let kinds = [FileKind::Skill, FileKind::Command, FileKind::Agent];
    let providers = AgentProvider::all();

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
    for provider in &providers {
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
