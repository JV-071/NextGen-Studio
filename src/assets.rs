use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_ASSETS: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Font,
    Interface,
    Script,
    Module,
}

impl AssetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "Imagem",
            Self::Font => "Fonte",
            Self::Interface => "OTUI",
            Self::Script => "Lua",
            Self::Module => "OTMOD",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AssetEntry {
    pub path: PathBuf,
    pub relative: String,
    pub kind: AssetKind,
    pub bytes: u64,
}

#[derive(Default)]
pub struct AssetCatalog {
    pub entries: Vec<AssetEntry>,
    pub truncated: bool,
}

impl AssetCatalog {
    pub fn scan(root: &Path) -> Self {
        let mut catalog = Self::default();
        scan_dir(root, root, &mut catalog, 0);
        catalog.entries.sort_by(|a, b| a.relative.cmp(&b.relative));
        catalog
    }

    pub fn counts(&self) -> [usize; 5] {
        let mut counts = [0; 5];
        for entry in &self.entries {
            counts[entry.kind as usize] += 1;
        }
        counts
    }
}

fn scan_dir(root: &Path, directory: &Path, catalog: &mut AssetCatalog, depth: usize) {
    if depth > 24 || catalog.entries.len() >= MAX_ASSETS {
        catalog.truncated = true;
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if catalog.entries.len() >= MAX_ASSETS {
            catalog.truncated = true;
            return;
        }
        if entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
            continue;
        }
        let path = entry.path();
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            scan_dir(root, &path, catalog, depth + 1);
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = match extension.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" => AssetKind::Image,
            "ttf" | "otf" | "woff" | "font" => AssetKind::Font,
            "otui" => AssetKind::Interface,
            "lua" => AssetKind::Script,
            "otmod" => AssetKind::Module,
            _ => continue,
        };
        let bytes = entry.metadata().map_or(0, |meta| meta.len());
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        catalog.entries.push(AssetEntry {
            path,
            relative,
            kind,
            bytes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexes_supported_project_resources() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("icon.png"), b"x").unwrap();
        fs::write(temp.path().join("screen.otui"), b"Panel").unwrap();
        fs::write(temp.path().join("ignored.bin"), b"x").unwrap();
        let catalog = AssetCatalog::scan(temp.path());
        assert_eq!(catalog.entries.len(), 2);
        assert_eq!(catalog.counts()[AssetKind::Image as usize], 1);
    }
}
