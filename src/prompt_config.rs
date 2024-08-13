use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct PromptConfig {
    folder: String,
    file: PromptConfigFile,
}

#[derive(Serialize, Deserialize)]
struct PromptConfigFile {
    prompts: Vec<PromptConfigEntry>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PromptConfigEntry {
    pub(crate) method: String,
    pub(crate) model: String,
    pub(crate) model_temperature: f32,
    pub(crate) file: String,
    #[serde(skip)]
    pub(crate) prompt_messages: Vec<String>,
}

const FALLBACK: &str = "fallback";
const ALWAYS_RELOAD: bool = true;

impl From<&str> for PromptConfig {
    /// read prompt config and associated prompt files
    fn from(folder: &str) -> Self {
        PromptConfig {
            file: Self::read_config_files(folder),
            folder: folder.to_string(),
        }
    }
}

impl PromptConfigFile {
    fn get(&self, method: &str) -> Option<&PromptConfigEntry> {
        self.prompts.iter().find(|p| p.method.eq(method))
    }
}

impl PromptConfig {
    pub(crate) fn get_or_default(&mut self, method: &str) -> &PromptConfigEntry {
        if ALWAYS_RELOAD {
            self.file = Self::read_config_files(&self.folder);
        }
        self.get(method).or(self.get(FALLBACK)).unwrap()
    }

    fn get(&self, method: &str) -> Option<&PromptConfigEntry> {
        self.file.get(method)
    }

    fn read_config_files(folder: &str) -> PromptConfigFile {
        let mut config_file_path = PathBuf::new();
        config_file_path.push(folder);
        config_file_path.push("config.json");
        let mut config_file: PromptConfigFile = match std::fs::read_to_string(&config_file_path) {
            Ok(file_content) => {
                serde_json::from_str(&file_content).expect("can not read prompt config")
            }
            Err(err) => panic!("can not find prompt config: {err}"),
        };

        for entry in &mut config_file.prompts {
            let mut file = PathBuf::new();
            file.push(folder);
            file.push(&entry.file);
            match std::fs::read_to_string(file) {
                Ok(file_content) => {
                    entry.prompt_messages = file_content
                        .split("\n\n")
                        .map(|paragraph| paragraph.to_string())
                        .collect()
                }
                Err(err) => panic!("can not find prompt file for '{}': {}", entry.file, err),
            }
        }

        if config_file.get(FALLBACK).is_none() {
            panic!("no default/fallback prompt in config. Please add an entry with `method` name '{FALLBACK}'");
        }

        config_file
    }
}
