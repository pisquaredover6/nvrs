use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Package {
    pub github: String,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub packages: HashMap<String, Package>,
}

pub fn load() -> Config {
    let config_path = Path::new("nvrs.toml");
    let config_home = format!(
        "{}/nvrs.toml",
        env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            format!(
                "{}/.config",
                env::var("HOME").unwrap_or_else(|_| ".".to_string())
            )
        })
    );
    let config_home_path = Path::new(&config_home);

    let content = if config_path.exists() && config_path.is_file() {
        fs::read_to_string(config_path).unwrap_or_default()
    } else if config_home_path.exists() && config_home_path.is_file() {
        fs::read_to_string(config_home_path).unwrap_or_default()
    } else {
        String::new()
    };

    if content.is_empty() {
        crate::custom_error(
            "no config found",
            "config file locations:\n ~/.config/nvrs.toml\n ./nvrs.toml\nmake sure the file is not empty".to_string(),
        );
    }

    toml::from_str(&content).expect("error reading the config file")
}
