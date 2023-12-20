use std::error::Error;

use lsp_server::{Connection, Message};
use lsp_types::{
    DidOpenTextDocumentParams, InitializeParams, ServerCapabilities, TextDocumentSyncKind,
};
use reqwest::{header, Method};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct GptRequest {
    model: String,
    messages: Vec<GptMessage>,
    temperature: f32,
    n: u32,
}

#[derive(Serialize, Deserialize)]
struct GptMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct GptResponse {
    id: String,
    choices: Vec<GptResponseChoice>,
    usage: GptResponseUsage,
}

#[derive(Serialize, Deserialize)]
struct GptResponseChoice {
    index: u64,
    message: GptMessage,
    finish_reason: String,
}

#[derive(Serialize, Deserialize)]
struct GptResponseUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

// the initial version is very much taken from the lsp-server example:
// https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/examples/goto_def.rs
fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    log("language server starting");

    let api_key = std::env::var("OPENAI_API_KEY")?;
    let api_company_id = std::env::var("OPENAI_ORG_ID")?;

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        completion_provider: Some(lsp_types::CompletionOptions {
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params, api_key, api_company_id)?;
    io_threads.join()?;

    log("language server stopping");

    Ok(())
}

fn log(log: &str) {
    eprintln!("{:?}: {log}", std::time::SystemTime::now());
    /*
    let mut file = std::fs::File::options()
        .append(true)
        .create(true)
        .open("/tmp/lsp-gpt.log")
        .unwrap();
    writeln!(&mut file, "{:?}: {log}", std::time::SystemTime::now()).unwrap();
    */
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
    api_key: String,
    api_company_id: String,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    log("connection established, waiting for messages");

    let client = reqwest::blocking::Client::builder().build()?;
    let auth = format!("Bearer {}", api_key);

    let initial_prompt = "You act as a language server that conforms to the LSP specification, and handles the plantuml file format. You are call 'lsp-gpt-plantuml'. You will be queried via API, meaning all responses should be in the JSON format.

Role and Goal: lsp-gpt-plantuml is a dedicated language server for PlantUML, conforming to the Language Server Protocol (LSP). It assists with PlantUML diagrams, providing expertise in syntax, structure, and best practices.

Constraints: This GPT focuses solely on PlantUML and adheres to LSP standards. It does not support other diagram types or programming languages.

Guidelines: Responses should be clear, concise, and technically accurate. They should offer specific guidance on PlantUML syntax, diagram optimization, and best practices, all encoded in JSON format to align with LSP specifications.

Clarification: When queries are ambiguous, lsp-gpt-plantuml should request specific details about the PlantUML diagram or issue to provide the most accurate, JSON-encoded assistance.

Personalization: The GPT maintains a professional, informative tone, targeting users who seek technical assistance with PlantUML diagrams, with all responses formatted according to LSP specifications in JSON.
";

    let mut latest_text_document_item = None;

    for msg in &connection.receiver {
        let raw_msg = serde_json::to_string(&msg)?;
        log(&format!("got msg: {}", raw_msg));
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                log(&format!("got request: {req:?}"));

                let mut messages = Vec::with_capacity(3);
                // prompting
                messages.push(GptMessage {
                    role: "system".to_string(),
                    content: initial_prompt.to_string(),
                });
                if let Some(item) = &latest_text_document_item {
                    messages.push(GptMessage {
                        role: "system".to_string(),
                        content: serde_json::to_string(item)?,
                    });
                }
                // actual request
                messages.push(GptMessage {
                    role: "user".to_string(),
                    content: raw_msg,
                });

                let api_request = client
                    .request(Method::POST, "https://api.openai.com/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, &auth)
                    .header("OpenAI-Organization", &api_company_id)
                    .body(serde_json::to_string(&GptRequest {
                        model: "gpt-4".to_string(),
                        messages,
                        temperature: 0.2,
                        n: 1,
                    })?)
                    .build()?;

                log(&format!(
                    "sending request to GPT API: {:?}",
                    &api_request.body().unwrap()
                ));

                let api_result = client.execute(api_request)?;

                match api_result.text() {
                    Ok(response) => {
                        match serde_json::from_str::<GptResponse>(&response) {
                            Ok(response_json) => {
                                // response is valid json, so we can use the first answer
                                log(&format!(
                                    "got GPT response: {}",
                                    serde_json::to_string(&response_json)?
                                ));
                                let response_message_raw = response_json
                                    .choices
                                    .get(0)
                                    .expect("no choices in response")
                                    .message
                                    .content
                                    .as_str();
                                match serde_json::from_str(response_message_raw) {
                                    Ok(response_message) => {
                                        // valid language server response
                                        log(&format!("all good, sending response to client: {response_message_raw}"));
                                        connection
                                            .sender
                                            .send(Message::Response(response_message))?
                                    }
                                    Err(err) => log(&format!(
                                        "error parsing response, err: {err}, response: {response_message_raw}"
                                    )),
                                }
                            }
                            Err(err) => log(&format!(
                                "error parsing response, err: {err}, response: {response}"
                            )),
                        }
                    }
                    Err(err) => log(&format!("can not parse GPT API response, err: {err}")),
                }
            }
            Message::Response(resp) => {
                log(&format!("got response: {resp:?}"));
            }
            Message::Notification(not) => {
                log(&format!("got notification: {not:?}"));
                match not.method.as_str() {
                    "textDocument/didOpen" => {
                        match serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                            Ok(text_document) => {
                                let x = text_document.text_document;
                                latest_text_document_item = Some(x);
                                ()
                            }
                            Err(err) => {
                                log(&format!("can not parse didOpen notification {err}"));
                                ()
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(())
}
