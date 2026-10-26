use std::path::PathBuf;

use stratum_core::config::ImportConfig;

use super::StStr;
use crate::services::StBoundary;

// The files of one picker answer, each checked as UTF-8. Bound: at most `MAX` paths, the deepest import queue;
// the entries past it are not read, because the worker would refuse them anyway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathList {
    paths: Vec<PathBuf>,
}

impl PathList {
    pub const MAX: usize = ImportConfig::MAX_QUEUE;

    // None when an entry is null with a length or is not UTF-8.
    // SAFETY: the caller passes views whose bytes live for the call
    pub unsafe fn read(entries: &[StStr]) -> Option<Self> {
        if entries.len() > Self::MAX {
            curia::warn!("picked files past the import queue were left out", { picked: entries.len(), kept: Self::MAX });
        }
        let mut paths = Vec::with_capacity(entries.len().min(Self::MAX));
        for entry in entries.iter().take(Self::MAX) {
            // SAFETY: see the function contract
            let text = unsafe { StBoundary::text(entry) }?;
            paths.push(PathBuf::from(text));
        }
        Some(Self { paths })
    }

    pub fn into_paths(self) -> Vec<PathBuf> {
        self.paths
    }
}
