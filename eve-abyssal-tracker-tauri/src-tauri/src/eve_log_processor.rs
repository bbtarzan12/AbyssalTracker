use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use encoding_rs::UTF_16LE;
use regex::Regex;
use walkdir::WalkDir;

// Python의 constants.py에서 가져온 패턴
lazy_static::lazy_static! {
    static ref SYSTEM_CHANGE_PATTERNS: HashMap<&'static str, HashMap<&'static str, &'static str>> = {
        let mut outer_map = HashMap::new();
        let mut ko_patterns = HashMap::new();
        ko_patterns.insert("system_change", "이브 시스템 > 지역 : ");
        ko_patterns.insert("channel_changed", "채널로 변경");
        ko_patterns.insert("unknown_system", "알 수 없음");
        outer_map.insert("ko", ko_patterns);

        let mut en_patterns = HashMap::new();
        en_patterns.insert("system_change", "EVE System > Channel changed to Local : ");
        en_patterns.insert("channel_changed", "");
        en_patterns.insert("unknown_system", "Unknown");
        outer_map.insert("en", en_patterns);
        outer_map
    };

    static ref LOG_FILENAME_PATTERNS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("ko", "지역_*.txt");
        map.insert("en", "Local_*.txt");
        map
    };

    static ref TIMESTAMP_REGEX: Regex = Regex::new(r"\[ *(\d{4}\.\d{2}\.\d{2} \d{2}:\d{2}:\d{2}) *\]").unwrap();
    static ref CHARACTER_NAME_REGEX: Regex = Regex::new(r"Listener:\s*(.+)").unwrap();
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EveLogProcessor {
    pub logs_path: PathBuf,
    pub current_log_file: Option<PathBuf>,
    pub language: Option<String>,
    pub patterns: HashMap<String, String>,
}

impl EveLogProcessor {
    pub fn new(logs_path: PathBuf, language: Option<String>) -> Self {
        let default_patterns: HashMap<String, String> = SYSTEM_CHANGE_PATTERNS.get("ko").unwrap()
            .iter()
            .map(|(&k, &v)| (k.to_string(), v.to_string()))
            .collect();
        EveLogProcessor {
            logs_path,
            current_log_file: None,
            language,
            patterns: default_patterns,
        }
    }

    pub fn find_all_log_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for pattern_str in LOG_FILENAME_PATTERNS.values() {
            let full_pattern = self.logs_path.join(pattern_str);
            if let Some(parent) = full_pattern.parent() {
                if let Some(filename_glob) = full_pattern.file_name().and_then(|s| s.to_str()) {
                    for entry in WalkDir::new(parent)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if glob_match::glob_match(filename_glob, file_name) {
                                files.push(entry.path().to_path_buf());
                            }
                        }
                    }
                }
            }
        }
        files
    }

    pub fn detect_log_language(&self, file_path: &Path) -> String {
        if let Ok(file) = File::open(file_path) {
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).ok();

            let (cow, _, _) = UTF_16LE.decode(&buffer);
            let content = cow.into_owned();

            for line in content.lines().take(20) {
                for (lang, patterns) in SYSTEM_CHANGE_PATTERNS.iter() {
                    if let Some(system_change_pattern) = patterns.get("system_change") {
                        if line.contains(system_change_pattern) {
                            return lang.to_string();
                        }
                    }
                }
            }
        }
        "ko".to_string() // Default to Korean if detection fails
    }

    pub fn set_log_file(&mut self, file_path: PathBuf) {
        self.current_log_file = Some(file_path.clone());
        if self.language.is_none() {
            self.language = Some(self.detect_log_language(&file_path));
        }
        self.patterns = SYSTEM_CHANGE_PATTERNS
            .get(self.language.as_deref().unwrap_or("ko"))
            .unwrap_or_else(|| SYSTEM_CHANGE_PATTERNS.get("ko").unwrap())
            .iter()
            .map(|(&k, &v)| (k.to_string(), v.to_string()))
            .collect();
    }

    pub fn iter_lines(&self, file_path: Option<&Path>) -> Box<dyn Iterator<Item = String>> {
        let target_file = file_path.unwrap_or_else(|| self.current_log_file.as_deref().expect("No log file specified for iteration."));

        if let Ok(file) = File::open(target_file) {
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).ok();

            let (cow, _, _) = UTF_16LE.decode(&buffer);
            let content = cow.into_owned();

            Box::new(content.lines().map(|s| s.trim().trim_start_matches('\u{feff}').to_string()).collect::<Vec<_>>().into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    pub fn is_system_change_line(&self, line: &str) -> bool {
        let system_change_pattern = self.patterns.get("system_change").map(|s| s.as_str()).unwrap_or("");
        let channel_changed_pattern = self.patterns.get("channel_changed").map(|s| s.as_str()).unwrap_or("");
        
        line.contains(system_change_pattern) && line.contains(channel_changed_pattern)
    }

    pub fn iter_system_changes(&self, file_path: Option<&Path>) -> Vec<(String, String, String)> {
        let lines_iter = self.iter_lines(file_path);
        lines_iter.filter_map(|line| {
            if self.is_system_change_line(&line) {
                let (system_name, ts) = self.parse_system_change(&line);
                if let (Some(sys), Some(t)) = (system_name, ts) {
                    Some((sys, t, line.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    }

    pub fn parse_system_change(&self, line: &str) -> (Option<String>, Option<String>) {
        let system_change_prefix = self.patterns.get("system_change").cloned().unwrap_or_default();
        let channel_changed_suffix = self.patterns.get("channel_changed").cloned().unwrap_or_default();

        if system_change_prefix.is_empty() || channel_changed_suffix.is_empty() {
            return (None, None);
        }

        let system_name = if let Some(parts) = line.split(&system_change_prefix).nth(1) {
            if let Some(name_part) = parts.split(&channel_changed_suffix).next() {
                Some(name_part.trim().to_string())
            } else {
                None
            }
        } else {
            None
        };

        let ts = TIMESTAMP_REGEX.captures(line)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()));

        (system_name, ts)
    }

    pub fn is_unknown_system(&self, system_name: &str) -> bool {
        system_name == self.patterns.get("unknown_system").cloned().unwrap_or_default().as_str()
    }

    pub fn detect_character_name(&self, file_path: Option<&Path>) -> Option<String> {
        let target_file = file_path.unwrap_or_else(|| self.current_log_file.as_deref().expect("No log file specified for character name detection."));

        if let Ok(file) = File::open(target_file) {
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            if let Err(_) = reader.read_to_end(&mut buffer) {
                return None;
            }

            let (cow, _, _) = UTF_16LE.decode(&buffer);
            let content = cow.into_owned();

            for line in content.lines().take(20) {
                let processed_line = line.trim_start_matches('\u{feff}').trim();
                if let Some(captures) = CHARACTER_NAME_REGEX.captures(processed_line) {
                    if let Some(name_match) = captures.get(1) {
                        let detected_name = name_match.as_str().trim().to_string();
                        return Some(detected_name);
                    }
                }
            }
        }
        None
    }
}