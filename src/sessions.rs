use std::fmt::Display;
use std::path::PathBuf;

use color_eyre::eyre::{OptionExt, Result};
use ini::Ini;
use itertools::Itertools;

static DEFAULT_XDG_DATA_DIRS: &str = "/usr/local/share:/usr/share";

static SESSION_SUBDIRS: &[(&str, SessionType)] = &[
    // FIXME: X sessions don't launch correctly
    // Need to launch X server before launching the session
    // ("xsessions", SessionType::X11),
    ("wayland-sessions", SessionType::Wayland),
];

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SessionType {
    #[allow(dead_code)]
    X11,
    Wayland,
}

impl Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X11 => write!(f, "x11"),
            Self::Wayland => write!(f, "wayland"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub slug: String,
    pub name: String,
    pub exec: Vec<String>,
    pub r#type: SessionType,
    pub desktop_names: Vec<String>,
}

impl Display for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

impl Session {
    pub fn to_environment(&self) -> Vec<String> {
        vec![
            format!("XDG_SESSION_TYPE={}", self.r#type),
            format!("XDG_SESSION_DESKTOP={}", self.slug),
            format!("XDG_CURRENT_DESKTOP={}", self.desktop_names.join(":")),
        ]
    }
}

pub fn get_sessions_mock() -> Vec<Session> {
    vec![
        Session {
            slug: "test-wayland".to_owned(),
            name: "Test (Wayland)".to_owned(),
            exec: vec![],
            r#type: SessionType::Wayland,
            desktop_names: vec![],
        },
        Session {
            slug: "test-xorg".to_owned(),
            name: "Test (Xorg)".to_owned(),
            exec: vec![],
            r#type: SessionType::X11,
            desktop_names: vec![],
        },
    ]
}

pub fn get_sessions() -> Vec<Session> {
    let xdg_data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| DEFAULT_XDG_DATA_DIRS.to_owned());

    let session_dirs = xdg_data_dirs.split(":").flat_map(|dir| {
        let dir = PathBuf::from(dir);
        SESSION_SUBDIRS.iter().map(move |(subdir, r#type)| (dir.join(subdir), *r#type))
    });

    let desktop_files = session_dirs.flat_map(|(dir, r#type)| match std::fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(Result::ok).map(|entry| (entry.path(), r#type)).collect(),
        Err(_) => Vec::new(),
    });

    desktop_files
        .map(|(path, r#type)| read_desktop_file(path, r#type))
        .filter_map(Result::ok)
        .unique_by(|session| session.slug.clone())
        .collect()
}

pub fn read_desktop_file(path: PathBuf, r#type: SessionType) -> Result<Session> {
    let ini = Ini::load_from_file(&path)?;

    let section = ini
        .section(Some("Desktop Entry"))
        .ok_or_eyre("missing [Desktop Entry] section in .desktop file")?;

    let name = section.get("Name").ok_or_eyre("missing Name= property in .desktop file")?;
    let exec = section.get("Exec").ok_or_eyre("missing Exec= property in .desktop file")?;
    let desktop_names = section.get("DesktopNames").unwrap_or("");

    Ok(Session {
        slug: path.file_stem().unwrap().to_string_lossy().to_string(),
        name: name.to_owned(),
        exec: shlex::split(exec).ok_or_eyre("failed to parse Exec= in .desktop file")?,
        r#type: r#type.to_owned(),
        desktop_names: desktop_names.split(";").map(str::to_owned).collect(),
    })
}
