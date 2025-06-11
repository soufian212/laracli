use std::{collections::HashSet, fs, path::PathBuf};

use colored::Colorize;
use serde::{Deserialize, Serialize};


pub fn create_config_file() {
    println!("{}", "Creating config file".yellow());
    let config_path = PathBuf::from(r"C:\laracli\config.json");
    let config_dir = config_path.parent().unwrap();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).expect("Failed to create config directory");
    }

    if !config_path.exists() {
        let default_config = Config {
            watched_paths: HashSet::new(),
            linked_paths: HashSet::new(),
        };

        let config_json = serde_json::to_string_pretty(&default_config).unwrap();
        fs::write(&config_path, config_json).expect("Failed to create config file");
    }
        println!("{}", "✅ Config file created".green());

}

pub fn get_config_path() -> PathBuf {    
    
    // Try system locations
    let system_locations = vec![
        PathBuf::from(r"C:\laracli\config.json"),
        PathBuf::from(r"C:\ProgramData\laracli\config.json"),
    ];
    
    for location in &system_locations {
        if location.exists() {
            return location.clone();
        }
    }
    
    // Default to user location for CLI (we can create it)
    dirs::home_dir()
        .expect("Could not get home dir")
        .join(".laracli")
        .join("config.json")
}

// data in config.json
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub watched_paths: HashSet<String>,
    #[serde(default)]
    pub linked_paths: HashSet<String>,
}


// Normalize path to remove Windows extended-length prefix
fn normalize_path_string(path_str: &str) -> String {
    let path = PathBuf::from(path_str);

    // Try to canonicalize and remove \\?\ prefix
    if let Ok(canonical) = path.canonicalize() {
        let canonical_str = canonical.to_string_lossy();
        if canonical_str.starts_with("\\\\?\\") {
            canonical_str[4..].to_string()
        } else {
            canonical_str.to_string()
        }
    } else {
        path_str.to_string()
    }
}


// Load config.json
pub fn load_config() -> Config {
    let path = get_config_path();
    
    // Ensure the directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create config directory");
        }
    }
    
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<Config>(&contents) {
                    Ok(mut config) => {
                        // Normalize all paths in the config
                        let normalized_paths: HashSet<String> = config
                            .linked_paths
                            .iter()
                            .map(|p| normalize_path_string(p))
                            .collect();
                        
                        // If paths were normalized, save the updated config
                        if normalized_paths != config.linked_paths {
                            config.linked_paths = normalized_paths;
                            fs::write(&path, serde_json::to_string_pretty(&config).unwrap())
                                .expect("Failed to save config");
                            println!("✅ Normalized paths in config file");
                        }
                        
                        config
                    }
                    Err(e) => {
                        // Try to recover partial config
                        let mut default_config = Config {
                            watched_paths: HashSet::new(),
                            linked_paths: HashSet::new(),
                        };
                        
                        // Attempt to preserve watched_paths if they exist in the file
                        if let Ok(partial) = serde_json::from_str::<serde_json::Value>(&contents) {
                            if let Some(paths) = partial.get("watched_paths") {
                                if let Ok(paths) = serde_json::from_value(paths.clone()) {
                                    default_config.watched_paths = paths;
                                }
                            }
                        }
                        
                        eprintln!("Failed to parse config file: {}. Creating new config with existing watched_paths.", e);
                        fs::write(&path, serde_json::to_string_pretty(&default_config).unwrap())
                            .expect("Failed to save config");
                        default_config
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read config file: {}. Creating new config.", e);
                let default_config = Config {
                    watched_paths: HashSet::new(),
                    linked_paths: HashSet::new(),
                };
                fs::write(&path, serde_json::to_string_pretty(&default_config).unwrap())
                    .expect("Failed to save config");
                default_config
            }
        }
    } else {
        println!("Config file doesn't exist. Creating new config at: {:?}", path);
        let default_config = Config {
            watched_paths: HashSet::new(),
            linked_paths: HashSet::new(),
        };
        fs::write(&path, serde_json::to_string_pretty(&default_config).unwrap())
            .expect("Failed to save config");
        default_config
    }
}

// add a path to linked object in config.json
pub fn add_to_linked_paths(path: &str) {
    let mut config = load_config();

    let normalized_path = normalize_path_string(path);

    if config.linked_paths.insert(normalized_path.clone()) {
        println!("✅ Added path to linked_paths: {}", normalized_path);

        let config_path = get_config_path();
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("Failed to serialize config"),
        )
        .expect("Failed to save config");
    } else {
        println!("⚠️ Path already exists in linked_paths: {}", normalized_path);
    }
}

// Add a path to watched_paths in config.json
pub fn add_to_watched_paths(path: &str) -> Result<(), String> {
    let mut config = load_config();

    let normalized_path = normalize_path_string(path);

    if config.watched_paths.insert(normalized_path.clone()) {
        let config_path = get_config_path();
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("Failed to serialize config"),
        )
        .expect("Failed to save config");
        Ok(())
    } else {
        Err(format!("Path already exists in watched_paths: {}", normalized_path))
    }
}

// Remove a path from watched_paths in config.json
pub fn remove_from_watched_paths(path: &str) -> Result<(), String> {
    let mut config = load_config();

    let normalized_path = normalize_path_string(path);
    if config.watched_paths.remove(&normalized_path) {
        let config_path = get_config_path();
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("Failed to serialize config"),
        )
        .expect("Failed to save config");
        Ok(())
    } else {
        Err(format!("Path not found in watched_paths: {}", normalized_path))
    }
}

pub fn remove_from_linked_paths(path: &str) -> Result<(), String> {
    let mut config = load_config();
    let normalized_path = normalize_path_string(path);
    if config.linked_paths.remove(&normalized_path) {
        let config_path = get_config_path();
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).expect("Failed to serialize config"),
        )
        .expect("Failed to save config");
        Ok(())
    } else {
        Err(format!("Path not found in linked_paths: {}", normalized_path))
    }
}