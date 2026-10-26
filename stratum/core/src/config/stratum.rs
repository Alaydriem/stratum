use super::{CacheConfig, DebugConfig, GhostConfig, ImportSettings, LibraryConfig, MarkerConfig};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct StratumConfig {
    pub library: LibraryConfig,
    pub markers: MarkerConfig,
    pub import: ImportSettings,
    pub cache: CacheConfig,
    pub ghost: GhostConfig,
    pub debug: DebugConfig,
}
