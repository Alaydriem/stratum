use std::path::{Path, PathBuf};

// Where the preview reads from: the game's resource packs folder, None when it was not found, and the folder of
// the stored structure files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSource {
    pub packs_dir: Option<PathBuf>,
    pub structures_dir: PathBuf,
}

impl PreviewSource {
    pub fn new(packs_dir: Option<PathBuf>, structures_dir: PathBuf) -> Self {
        Self { packs_dir, structures_dir }
    }

    // Looks for `data/resource_packs` holding `vanilla/blocks.json`, first under the game's working folder, then
    // under each folder above `data_dir` (the mod's data folder sits inside the game folder).
    pub fn locate(data_dir: &Path) -> Self {
        let mut roots: Vec<PathBuf> = std::env::current_dir().into_iter().collect();
        roots.extend(data_dir.ancestors().map(Path::to_path_buf));
        let packs_dir = roots
            .into_iter()
            .map(|root| root.join("data").join("resource_packs"))
            .find(|dir| dir.join("vanilla").join("blocks.json").is_file());
        Self { packs_dir, structures_dir: data_dir.join("structures") }
    }
}
