use anyhow::Result;

use crate::{
    manifest::Manifest,
    types::{AgentProvider, FileScope},
};

pub fn install(manifest: &Manifest, provider: &AgentProvider, scope: &FileScope) -> Result<()> {
    for file in &manifest.files {
        provider.install(scope, &file.kind)?;
    }

    Ok(())
}
