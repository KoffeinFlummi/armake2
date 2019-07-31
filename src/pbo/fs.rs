use std::path::PathBuf;
use std::fs::read_dir;

use crate::ArmakeError;

pub fn matches_glob(s: &str, pattern: &str) -> bool {
    if let Some(index) = pattern.find('*') {
        if s[..index] != pattern[..index] { return false; }

        for i in (index+1)..(s.len()-1) {
            if matches_glob(&s[i..].to_string(), &pattern[(index+1)..].to_string()) { return true; }
        }

        false
    } else {
        s == pattern
    }
}

pub fn file_allowed(name: &str, exclude_patterns: &[&str]) -> bool {
    for pattern in exclude_patterns {
        if matches_glob(&name, &pattern) { return false; }
    }

    true
}

pub fn list_files(directory: &PathBuf) -> Result<Vec<PathBuf>, ArmakeError> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            for f in list_files(&path)? {
                files.push(f);
            }
        } else {
            files.push(path);
        }
    }

    Ok(files)
}
