use std::fmt::Display;
use std::path::PathBuf;

use color_eyre::eyre::{OptionExt, Result};
use ini::Ini;
use itertools::Itertools;

static DEFAULT_XDG_DATA_DIRS: &str = "/usr/local/share:/usr/share";

static SESSION_SUBDIRS: [(&str, SessionType); 2] =
    [("xsessions", SessionType::X11), ("wayland-sessions", SessionType::Wayland)];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SessionType {
    X11,
    Wayland,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    slug: String,
    name: String,
    exec: String,
    r#type: SessionType,
    desktop_names: Option<String>,
}

impl Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

pub fn get_sessions() -> Vec<Session> {
    let xdg_data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| DEFAULT_XDG_DATA_DIRS.to_owned());

    let session_dirs = xdg_data_dirs.split(":").flat_map(|dir| {
        let dir = PathBuf::from(dir);
        SESSION_SUBDIRS.map(|(subdir, r#type)| (dir.join(subdir), r#type))
    });

    let desktop_files = session_dirs.flat_map(|(dir, r#type)| match std::fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(Result::ok).map(|entry| (entry.path(), r#type)).collect(),
        Err(_) => Vec::new(),
    });

    desktop_files
        .map(|(path, r#type)| read_desktop_file(path, r#type))
        .filter_map(Result::ok)
        .collect()
}

pub fn read_desktop_file(path: PathBuf, r#type: SessionType) -> Result<Session> {
    let ini = Ini::load_from_file(&path)?;

    let section = ini
        .section(Some("Desktop Entry"))
        .ok_or_eyre("missing [Desktop Entry] section in .desktop file")?;

    let name = section.get("Name").ok_or_eyre("missing Name= property in .desktop file")?;
    let exec = section.get("Exec").ok_or_eyre("missing Exec= property in .desktop file")?;
    let desktop_names = section.get("DesktopNames");

    Ok(Session {
        slug: path.file_stem().unwrap().to_string_lossy().to_string(),
        name: name.to_owned(),
        exec: exec.to_owned(),
        r#type: r#type.to_owned(),
        desktop_names: desktop_names.map(str::to_owned),
    })
}
