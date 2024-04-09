use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct GptRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<GptMessage>,
    pub(crate) temperature: f32,
    pub(crate) n: u32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GptMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GptResponse {
    pub(crate) id: String,
    pub(crate) choices: Vec<GptResponseChoice>,
    pub(crate) usage: GptResponseUsage,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GptResponseChoice {
    pub(crate) index: u64,
    pub(crate) message: GptMessage,
    pub(crate) finish_reason: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GptResponseUsage {
    pub(crate) prompt_tokens: u64,
    pub(crate) completion_tokens: u64,
    pub(crate) total_tokens: u64,
}
