use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Source {
    pub path: String,
    pub title: String,
    pub duration: f64,
    pub origin: String,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub id: u64,
    pub title: String,
    pub start: f64,
    pub end: f64,
    pub score: u8,
    pub quote: String,
    pub caption: String,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Portrait,
    Square,
    Wide,
}

impl Aspect {
    pub fn size(self) -> (u32, u32) {
        match self {
            Aspect::Portrait => (1080, 1920),
            Aspect::Square => (1080, 1080),
            Aspect::Wide => (1920, 1080),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Aspect::Portrait => "9:16",
            Aspect::Square => "1:1",
            Aspect::Wide => "16:9",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub api_key: String,
    pub output_dir: String,
    pub duration_secs: u32,
    pub clip_count: u32,
    pub focus: String,
    pub language: String,
    pub burn_title: bool,
    pub aspect: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            output_dir: default_output_dir().to_string_lossy().to_string(),
            duration_secs: 30,
            clip_count: 6,
            focus: String::new(),
            language: "id".into(),
            burn_title: true,
            aspect: "9:16".into(),
        }
    }
}

impl Settings {
    pub fn aspect(&self) -> Aspect {
        match self.aspect.as_str() {
            "1:1" => Aspect::Square,
            "16:9" => Aspect::Wide,
            _ => Aspect::Portrait,
        }
    }

    pub fn load() -> Self {
        let path = settings_path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(raw) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, raw);
        }
    }
}

fn config_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("cheat-clip");
    }
    PathBuf::from(".cheat-clip")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn cache_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("cheat-clip");
    }
    PathBuf::from(".cheat-clip-cache")
}

fn default_output_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join("Videos").join("Cheat Clip");
    }
    PathBuf::from("exports")
}

pub fn fmt_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u32;
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}
