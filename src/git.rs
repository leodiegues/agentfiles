use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};

/// Result of resolving a remote git source.
///
/// Contains the local path to the cloned/cached repository and the
/// original URL used for display purposes.
pub struct GitSource {
    /// Local path to the cloned repository content.
    pub local_path: PathBuf,
    /// The original URL (without ref) for display.
    pub url: String,
    /// The git ref that was checked out, if any.
    pub git_ref: Option<String>,
}

/// Parsed git remote input — a URL and an optional ref.
#[derive(Debug, PartialEq)]
pub struct ParsedRemote {
    /// The git-cloneable URL (with scheme).
    pub url: String,
    /// Optional ref (branch, tag, commit) to check out.
    pub git_ref: Option<String>,
}

/// Normalize a source string for deduplication.
///
/// Strips `@ref` suffixes, normalizes URL schemes, and removes trailing `.git`
/// so that `github.com/org/repo`, `https://github.com/org/repo`, and
/// `https://github.com/org/repo.git` all compare as equal.
pub fn normalize_source(input: &str) -> String {
    let (base, _ref) = strip_ref(input);
    let url = normalize_url(base);
    url.trim_end_matches(".git").to_string()
}

/// Check whether a string looks like a git remote URL rather than a local path.
///
/// Recognizes:
/// - Explicit schemes: `https://`, `http://`, `git://`, `ssh://`
/// - SCP-style: `git@host:org/repo`
/// - Shorthand: `github.com/org/repo`, `gitlab.com/org/repo`, etc.
pub fn is_git_url(input: &str) -> bool {
    // Strip any @ref suffix before checking
    let base = strip_ref(input).0;

    // Explicit schemes
    if base.starts_with("https://")
        || base.starts_with("http://")
        || base.starts_with("git://")
        || base.starts_with("ssh://")
    {
        return true;
    }

    // SCP-style: git@github.com:org/repo
    if base.starts_with("git@") && base.contains(':') {
        return true;
    }

    // Shorthand: github.com/org/repo, gitlab.com/org/repo, etc.
    let known_hosts = [
        "github.com/",
        "gitlab.com/",
        "bitbucket.org/",
        "codeberg.org/",
        "sr.ht/",
    ];
    for host in &known_hosts {
        if base.starts_with(host) {
            return true;
        }
    }

    false
}

/// Parse a remote input string into a URL and optional ref.
///
/// The ref is specified with `@ref` syntax at the end of the URL.
/// We're careful not to confuse `git@github.com` (part of SCP syntax)
/// with a ref delimiter.
///
/// # Examples
///
/// ```
/// use agentfiles::git::parse_remote;
///
/// let p = parse_remote("github.com/org/repo@v1.0");
/// assert_eq!(p.url, "https://github.com/org/repo");
/// assert_eq!(p.git_ref, Some("v1.0".to_string()));
///
/// let p = parse_remote("git@github.com:org/repo.git");
/// assert_eq!(p.url, "git@github.com:org/repo.git");
/// assert_eq!(p.git_ref, None);
/// ```
pub fn parse_remote(input: &str) -> ParsedRemote {
    let (base, git_ref) = strip_ref(input);
    let url = normalize_url(base);
    ParsedRemote { url, git_ref }
}

/// Clone or update a remote git repository and return the local path.
///
/// Uses a cache directory at `~/.cache/agentfiles/<hash>/` (or the platform
/// equivalent via `dirs::cache_dir()`). If the cache already exists, it fetches
/// updates instead of re-cloning.
///
/// If a `git_ref` is provided, checks out that ref after clone/fetch.
pub fn resolve_remote(remote: &ParsedRemote) -> Result<GitSource> {
    ensure_git_available()?;

    let cache_dir = get_cache_dir(&remote.url)?;

    if cache_dir.exists() {
        // Update existing clone
        fetch_repo(&cache_dir)?;
    } else {
        // Fresh clone
        clone_repo(&remote.url, &cache_dir)?;
    }

    // Check out the requested ref, or reset to the default branch HEAD
    if let Some(ref git_ref) = remote.git_ref {
        checkout_ref(&cache_dir, git_ref)?;
    } else {
        reset_to_default_branch(&cache_dir)?;
    }

    Ok(GitSource {
        local_path: cache_dir,
        url: remote.url.clone(),
        git_ref: remote.git_ref.clone(),
    })
}

/// Return the cache directory path for a given URL without performing any git operations.
///
/// Useful for checking cache status or cleaning up.
pub fn get_cache_dir(url: &str) -> Result<PathBuf> {
    let base = dirs::cache_dir().context("could not determine cache directory")?;
    let hash = hash_url(url);
    Ok(base.join("agentfiles").join(hash))
}

/// Split a ref suffix from the input.
///
/// The ref delimiter is `@` but only when it appears after a `/` character,
/// so `git@github.com:org/repo@v1.0` yields ("git@github.com:org/repo", Some("v1.0"))
/// and `git@github.com:org/repo` yields ("git@github.com:org/repo", None).
fn strip_ref(input: &str) -> (&str, Option<String>) {
    // Find the last '@' that comes after a '/'
    if let Some(last_slash) = input.rfind('/') {
        let after_slash = &input[last_slash..];
        if let Some(at_pos) = after_slash.rfind('@') {
            let absolute_pos = last_slash + at_pos;
            let base = &input[..absolute_pos];
            let git_ref = &input[absolute_pos + 1..];
            if !git_ref.is_empty() {
                return (base, Some(git_ref.to_string()));
            }
        }
    }
    (input, None)
}

/// Normalize a URL so it's git-cloneable.
///
/// - Already has a scheme -> use as-is
/// - SCP-style (`git@...`) -> use as-is
/// - Shorthand (`github.com/org/repo`) -> prepend `https://`
fn normalize_url(url: &str) -> String {
    if url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("git://")
        || url.starts_with("ssh://")
        || url.starts_with("git@")
    {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

/// Hash a URL string to a 16-character hex string using FNV-1a.
///
/// Uses a deterministic hash algorithm (FNV-1a 64-bit) so that cache
/// directory names remain stable across Rust toolchain upgrades.
fn hash_url(url: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let mut hash = FNV_OFFSET;
    for byte in url.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// Verify that `git` is available on the system.
///
/// Only runs the actual check once per process; subsequent calls return
/// the cached result immediately.
fn ensure_git_available() -> Result<()> {
    static GIT_CHECKED: OnceLock<Result<(), String>> = OnceLock::new();

    let result = GIT_CHECKED.get_or_init(|| {
        let output = Command::new("git")
            .arg("--version")
            .output()
            .map_err(|_| "'git' is not installed or not in PATH".to_string())?;

        if !output.status.success() {
            return Err("'git --version' failed — is git installed?".to_string());
        }
        Ok(())
    });

    result
        .as_ref()
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Clone a repository into the cache directory.
fn clone_repo(url: &str, target: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache directory: {}", parent.display()))?;
    }

    let output = Command::new("git")
        .args(["clone", url])
        .arg(target)
        .output()
        .with_context(|| format!("failed to run 'git clone {url}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed:\n{stderr}");
    }

    Ok(())
}

/// Fetch updates in an existing clone.
fn fetch_repo(repo_dir: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["fetch", "--all", "--prune"])
        .current_dir(repo_dir)
        .output()
        .context("failed to run 'git fetch'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git fetch failed:\n{stderr}");
    }

    Ok(())
}

/// Validate a git ref to prevent flag injection.
///
/// Rejects refs that could be interpreted as git command-line flags
/// or that contain path traversal sequences.
fn validate_git_ref(git_ref: &str) -> Result<()> {
    if git_ref.is_empty() {
        bail!("git ref cannot be empty");
    }
    if git_ref.starts_with('-') {
        bail!("git ref '{git_ref}' looks like a command-line flag — refusing for safety");
    }
    if git_ref.contains("..") {
        bail!("git ref '{git_ref}' contains '..' — refusing for safety");
    }
    if git_ref.bytes().any(|b| b.is_ascii_control() || b == b' ') {
        bail!("git ref '{git_ref}' contains whitespace or control characters");
    }
    Ok(())
}

/// Check out a specific ref (branch, tag, or commit hash).
fn checkout_ref(repo_dir: &Path, git_ref: &str) -> Result<()> {
    validate_git_ref(git_ref)?;

    // First, try a detached checkout (works for tags and commit hashes)
    let output = Command::new("git")
        .args(["checkout", git_ref])
        .current_dir(repo_dir)
        .output()
        .with_context(|| format!("failed to run 'git checkout {git_ref}'"))?;

    if output.status.success() {
        return Ok(());
    }

    // If that failed, try as a remote tracking branch
    // First clean up any partial state
    let output2 = Command::new("git")
        .args(["checkout", "-B", git_ref, &format!("origin/{git_ref}")])
        .current_dir(repo_dir)
        .output()
        .with_context(|| format!("failed to checkout remote branch '{git_ref}'"))?;

    if !output2.status.success() {
        let stderr = String::from_utf8_lossy(&output2.stderr);
        bail!("git checkout '{git_ref}' failed:\n{stderr}");
    }

    Ok(())
}

/// Reset the working tree to the latest commit on the default branch.
fn reset_to_default_branch(repo_dir: &Path) -> Result<()> {
    // Get the default branch name from the remote HEAD
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(repo_dir)
        .output()
        .context("failed to determine default branch")?;

    let mut branch = if output.status.success() {
        let full = String::from_utf8_lossy(&output.stdout).trim().to_string();
        full.strip_prefix("origin/").unwrap_or(&full).to_string()
    } else {
        "main".to_string()
    };

    let output = Command::new("git")
        .args(["checkout", &branch])
        .current_dir(repo_dir)
        .output()
        .with_context(|| format!("failed to checkout '{branch}'"))?;

    if !output.status.success() {
        // Try "master" as a fallback
        let output2 = Command::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_dir)
            .output()
            .context("failed to checkout 'master'")?;

        if !output2.status.success() {
            bail!("could not determine or checkout the default branch");
        }
        branch = "master".to_string();
    }

    // Reset to match remote and pull latest
    let reset_output = Command::new("git")
        .args(["reset", "--hard", &format!("origin/{branch}")])
        .current_dir(repo_dir)
        .output()
        .context("failed to run 'git reset'")?;

    if !reset_output.status.success() {
        let stderr = String::from_utf8_lossy(&reset_output.stderr);
        eprintln!("warning: git reset --hard failed: {stderr}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod is_git_url_tests {
        use super::*;

        #[test]
        fn https_url() {
            assert!(is_git_url("https://github.com/org/repo.git"));
            assert!(is_git_url("https://github.com/org/repo"));
        }

        #[test]
        fn http_url() {
            assert!(is_git_url("http://github.com/org/repo"));
        }

        #[test]
        fn git_protocol() {
            assert!(is_git_url("git://github.com/org/repo.git"));
        }

        #[test]
        fn ssh_protocol() {
            assert!(is_git_url("ssh://git@github.com/org/repo.git"));
        }

        #[test]
        fn scp_style() {
            assert!(is_git_url("git@github.com:org/repo.git"));
            assert!(is_git_url("git@gitlab.com:org/repo"));
        }

        #[test]
        fn shorthand() {
            assert!(is_git_url("github.com/org/repo"));
            assert!(is_git_url("gitlab.com/org/repo"));
            assert!(is_git_url("bitbucket.org/team/repo"));
            assert!(is_git_url("codeberg.org/user/repo"));
        }

        #[test]
        fn shorthand_with_ref() {
            assert!(is_git_url("github.com/org/repo@v1.0"));
            assert!(is_git_url("github.com/org/repo@main"));
        }

        #[test]
        fn local_paths_are_not_urls() {
            assert!(!is_git_url("."));
            assert!(!is_git_url("./my-dir"));
            assert!(!is_git_url("/absolute/path"));
            assert!(!is_git_url("relative/path"));
            assert!(!is_git_url("agentfiles.json"));
        }
    }

    mod parse_remote_tests {
        use super::*;

        #[test]
        fn https_without_ref() {
            let p = parse_remote("https://github.com/org/repo.git");
            assert_eq!(p.url, "https://github.com/org/repo.git");
            assert_eq!(p.git_ref, None);
        }

        #[test]
        fn https_with_ref() {
            let p = parse_remote("https://github.com/org/repo@v1.0");
            assert_eq!(p.url, "https://github.com/org/repo");
            assert_eq!(p.git_ref, Some("v1.0".to_string()));
        }

        #[test]
        fn shorthand_without_ref() {
            let p = parse_remote("github.com/org/repo");
            assert_eq!(p.url, "https://github.com/org/repo");
            assert_eq!(p.git_ref, None);
        }

        #[test]
        fn shorthand_with_ref() {
            let p = parse_remote("github.com/org/repo@main");
            assert_eq!(p.url, "https://github.com/org/repo");
            assert_eq!(p.git_ref, Some("main".to_string()));
        }

        #[test]
        fn scp_style_without_ref() {
            let p = parse_remote("git@github.com:org/repo.git");
            assert_eq!(p.url, "git@github.com:org/repo.git");
            assert_eq!(p.git_ref, None);
        }

        #[test]
        fn scp_style_with_ref() {
            let p = parse_remote("git@github.com:org/repo@v2.0");
            assert_eq!(p.url, "git@github.com:org/repo");
            assert_eq!(p.git_ref, Some("v2.0".to_string()));
        }

        #[test]
        fn ssh_protocol_with_ref() {
            let p = parse_remote("ssh://git@github.com/org/repo@abc123");
            assert_eq!(p.url, "ssh://git@github.com/org/repo");
            assert_eq!(p.git_ref, Some("abc123".to_string()));
        }
    }

    mod cache_dir_tests {
        use super::*;

        #[test]
        fn deterministic_hash() {
            let dir1 = get_cache_dir("https://github.com/org/repo").unwrap();
            let dir2 = get_cache_dir("https://github.com/org/repo").unwrap();
            assert_eq!(dir1, dir2);
        }

        #[test]
        fn different_urls_different_dirs() {
            let dir1 = get_cache_dir("https://github.com/org/repo-a").unwrap();
            let dir2 = get_cache_dir("https://github.com/org/repo-b").unwrap();
            assert_ne!(dir1, dir2);
        }

        #[test]
        fn cache_dir_under_agentfiles() {
            let dir = get_cache_dir("https://github.com/org/repo").unwrap();
            assert!(dir.to_string_lossy().contains("agentfiles"));
        }
    }

    mod validate_git_ref_tests {
        use super::*;

        #[test]
        fn accepts_valid_refs() {
            assert!(validate_git_ref("main").is_ok());
            assert!(validate_git_ref("v1.0").is_ok());
            assert!(validate_git_ref("feature/my-branch").is_ok());
            assert!(validate_git_ref("abc123def").is_ok());
            assert!(validate_git_ref("release/2.0.0").is_ok());
        }

        #[test]
        fn rejects_flag_injection() {
            assert!(validate_git_ref("--help").is_err());
            assert!(validate_git_ref("-c").is_err());
            assert!(validate_git_ref("--upload-pack=evil").is_err());
        }

        #[test]
        fn rejects_path_traversal() {
            assert!(validate_git_ref("../etc/passwd").is_err());
            assert!(validate_git_ref("foo/../bar").is_err());
        }

        #[test]
        fn rejects_empty() {
            assert!(validate_git_ref("").is_err());
        }

        #[test]
        fn rejects_whitespace_and_control_chars() {
            assert!(validate_git_ref("main branch").is_err());
            assert!(validate_git_ref("main\tbranch").is_err());
            assert!(validate_git_ref("main\0branch").is_err());
        }
    }

    mod normalize_url_tests {
        use super::*;

        #[test]
        fn already_https() {
            assert_eq!(
                normalize_url("https://github.com/org/repo"),
                "https://github.com/org/repo"
            );
        }

        #[test]
        fn scp_style_unchanged() {
            assert_eq!(
                normalize_url("git@github.com:org/repo"),
                "git@github.com:org/repo"
            );
        }

        #[test]
        fn shorthand_gets_https() {
            assert_eq!(
                normalize_url("github.com/org/repo"),
                "https://github.com/org/repo"
            );
        }
    }
}
