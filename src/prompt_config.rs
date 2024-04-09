use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct PromptConfig {
    pub(crate) prompts: Vec<PromptConfigEntry>,
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

impl From<&str> for PromptConfig {
    /// read prompt config and associated prompt files
    fn from(folder: &str) -> Self {
        let mut config_file = PathBuf::new();
        config_file.push(folder);
        config_file.push("config.json");
        let mut config: PromptConfig = match std::fs::read_to_string(&config_file) {
            Ok(file_content) => {
                serde_json::from_str(&file_content).expect("can not read prompt config")
            }
            Err(err) => panic!("can not find prompt config: {err}"),
        };

        for entry in &mut config.prompts {
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

        if config.get(FALLBACK).is_none() {
            panic!("no default/fallback prompt in config. Please add an entry with `method` name '{FALLBACK}'");
        }

        config
    }
}

impl PromptConfig {
    pub(crate) fn get_or_default(&self, method: &str) -> &PromptConfigEntry {
        self.get(method).or(self.get(FALLBACK)).unwrap()
    }

    fn get(&self, method: &str) -> Option<&PromptConfigEntry> {
        self.prompts.iter().find(|p| p.method.eq(method))
    }
}
