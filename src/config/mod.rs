use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub assets_root: PathBuf,
}

impl AppConfig {
    pub fn from_override(path: Option<PathBuf>) -> Result<Self> {
        let root = match path {
            Some(custom) => canonicalize_dir(&custom)?,
            None => default_assets_root()?,
        };
        Ok(Self { assets_root: root })
    }
}

fn canonicalize_dir(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("failed to resolve assets directory at {:?}", path))?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(anyhow!("assets path {:?} is not a directory", canonical))
    }
}

fn default_assets_root() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("unable to resolve current executable path")?;
    let assets = exe
        .ancestors()
        .find_map(|dir| {
            let candidate = dir.join("assets");
            candidate.is_dir().then_some(candidate)
        })
        .ok_or_else(|| anyhow!("could not locate default assets directory alongside binary"))?;
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::{default_assets_root, AppConfig};

    #[test]
    fn discovers_assets_root() {
        let root = default_assets_root().expect("assets directory should exist");
        assert!(root.ends_with("assets"));
    }

    #[test]
    fn accepts_override() {
        let config =
            AppConfig::from_override(Some(std::env::current_dir().unwrap().join("assets")))
                .unwrap();
        assert!(config.assets_root.ends_with("assets"));
    }
}
