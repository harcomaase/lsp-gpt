use std::{
    error::Error,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpt_adapter::GptResponseUsage;
use lsp_server::{Connection, Message};
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, TextDocumentItem,
};
use reqwest::{header, Method, Url};

use crate::{
    gpt_adapter::{GptMessage, GptRequest, GptResponse},
    prompt_config::PromptConfig,
};

mod gpt_adapter;
mod prompt_config;
mod server_capabilities;

const BASE_FOLDER: &str = "/home/marco/dev/projects/lsp-gpt";

// the initial version is very much taken from the lsp-server example:
// https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/examples/goto_def.rs
fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    env_logger::builder()
        .target(env_logger::Target::Stderr)
        .filter_level(log::LevelFilter::Debug)
        .init();
    log::info!("language server starting");

    // read keys
    let api_key = from_env("OPENAI_API_KEY");
    let api_company_id = from_env("OPENAI_ORG_ID");

    // create connection
    let (connection, io_threads) = Connection::stdio();

    // define language server capabilities
    let server_capabilities = server_capabilities::get_server_capabilities();
    // initialise connection and negotiate capabilities
    let initialization_params = connection.initialize(server_capabilities)?;

    // read prompt config
    let prompt_config = PromptConfig::from(create_path("/assets/prompts/").as_str());

    // enter the main event loop
    handle_messages(
        connection,
        initialization_params,
        prompt_config,
        api_key,
        api_company_id,
    )?;
    io_threads.join()?;

    log::info!("language server stopping");

    Ok(())
}

fn from_env(key: &str) -> String {
    // read from secrets file if it exists
    if let Ok(content) = std::fs::read_to_string(create_path("/secrets.txt")) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.eq(key) {
                    return v.to_string();
                }
            }
        }
    }
    // else look into the environment
    std::env::var(key).expect(&format!("{key} not set as environment variable"))
}

fn log_invocation(
    model: &str,
    duration: Duration,
    method: &str,
    prompt_quantity: usize,
    usage: &GptResponseUsage,
) -> std::io::Result<()> {
    let filename = create_path("/invocations.csv");

    log::info!("logging to {filename}");

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)?;

    let line = format!(
        "{},{},{},{},{},{},{},{}\n",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model,
        duration.as_millis(),
        method,
        prompt_quantity,
        usage.prompt_tokens,
        usage.completion_tokens,
        usage.total_tokens
    );

    file.write_all(line.as_bytes())?;

    Ok(())
}

fn handle_messages(
    connection: Connection,
    params: serde_json::Value,
    prompt_config: PromptConfig,
    api_key: String,
    api_company_id: String,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params: InitializeParams = serde_json::from_value(params).unwrap();

    log::info!("connection established, waiting for messages");

    // create http client
    let http_client = reqwest::blocking::Client::builder()
        .timeout(Some(Duration::new(60, 0)))
        .build()?;
    let auth = format!("Bearer {}", api_key);

    // buffer for document updates
    let mut latest_text_document_item = None;

    // wait for messages
    for msg in &connection.receiver {
        let raw_msg = serde_json::to_string(&msg)?;
        //log(&format!("got msg: {}", raw_msg));
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                log::info!("got request: {req:?}");

                let prompt_config_entry = prompt_config.get_or_default(&req.method);

                let mut messages = Vec::with_capacity(3);
                // prompting
                for prompt in &prompt_config_entry.prompt_messages {
                    messages.push(GptMessage {
                        role: "system".to_string(),
                        content: prompt.to_string(),
                    });
                }
                // send all documents in workspace
                //TODO: define a limit
                let workspace_documents = gather_workspace_documents(&params)?;
                for workspace_document in &workspace_documents {
                    messages.push(GptMessage {
                        role: "system".to_string(),
                        content: serde_json::to_string(&workspace_document)?,
                    });
                }
                // if workspace is not available or empty, use latest opened/changed document
                if workspace_documents.is_empty() {
                    if let Some(item) = &latest_text_document_item {
                        messages.push(GptMessage {
                            role: "system".to_string(),
                            content: serde_json::to_string(item)?,
                        });
                    }
                }
                // actual request from client
                messages.push(GptMessage {
                    role: "user".to_string(),
                    content: raw_msg,
                });

                // query the GPT API
                let model = &prompt_config_entry.model;
                let temperature = prompt_config_entry.model_temperature;
                let prompt_quantity = messages.len();
                let api_request = http_client
                    .request(Method::POST, "https://api.openai.com/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, &auth)
                    .header("OpenAI-Organization", &api_company_id)
                    .body(serde_json::to_string(&GptRequest {
                        model: model.clone(),
                        messages,
                        temperature,
                        n: 1,
                    })?)
                    .build()?;

                log::info!(
                    "sending request to GPT API",
                    //&api_request.body().unwrap()
                );

                let start = SystemTime::now();
                let api_result = http_client.execute(api_request)?;
                let duration = start.elapsed()?;

                // handle result
                match api_result.text() {
                    Ok(response) => {
                        match serde_json::from_str::<GptResponse>(&response) {
                            Ok(response_json) => {
                                let _ = log_invocation(
                                    &model,
                                    duration,
                                    &req.method,
                                    prompt_quantity,
                                    &response_json.usage,
                                );
                                // response is valid json, so we can use the first answer
                                log::info!(
                                    "got GPT response: {}",
                                    serde_json::to_string(&response_json)?
                                );
                                let response_message_raw = response_json
                                    .choices
                                    .get(0)
                                    .expect("no choices in response")
                                    .message
                                    .content
                                    .as_str();

                                let extracted_response_message = extract_json(response_message_raw);

                                match serde_json::from_str(extracted_response_message) {
                                    Ok(response_message) => {
                                        // valid language server response
                                        log::info!("all good, sending response to client: {response_message_raw}");
                                        connection
                                            .sender
                                            .send(Message::Response(response_message))?
                                    }
                                    Err(err) => log::info!(
                                        "error parsing response, err: {err}, response: {response_message_raw}"
                                    ),
                                }
                            }
                            Err(err) => log::info!(
                                "error parsing response, err: {err}, response: {response}"
                            ),
                        }
                    }
                    Err(err) => log::info!("can not parse GPT API response, err: {err}"),
                }
            }
            Message::Response(resp) => {
                log::info!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                // notification handling, for updates about textDocuments
                log::info!("got notification: {not:?}");
                match not.method.as_str() {
                    "textDocument/didOpen" => {
                        match serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                            Ok(text_document) => {
                                let x = text_document.text_document;
                                latest_text_document_item = Some(x);
                                ()
                            }
                            Err(err) => {
                                log::info!("can not parse didOpen notification {err}");
                                ()
                            }
                        }
                    }
                    "textDocument/didChange" => {
                        match serde_json::from_value::<DidChangeTextDocumentParams>(not.params) {
                            Ok(text_document) => {
                                //let x = text_document.content_changes;
                                log::info!("received changes: {:?}", &text_document);
                                //latest_text_document_item = Some(x);
                                ()
                            }
                            Err(err) => {
                                log::info!("can not parse didChange notification {err}");
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

/// extracts the contents of markdown JSON code blocks
fn extract_json(response_message_raw: &str) -> &str {
    let start_element = "```json\n";
    let end_element = "\n```";
    match response_message_raw.find(start_element) {
        Some(start_index) => {
            let json_content = &response_message_raw[start_index + start_element.len()..];
            match json_content.find(end_element) {
                Some(end_index) => &json_content[..end_index],
                None => json_content,
            }
        }
        None => response_message_raw,
    }
}

fn create_path(subpath: &str) -> String {
    format!("{BASE_FOLDER}{subpath}")
}

fn gather_workspace_documents(params: &InitializeParams) -> std::io::Result<Vec<TextDocumentItem>> {
    let mut workspace_documents = Vec::new();
    if let Some(workspace_folders) = &params.workspace_folders {
        for (worksspace_folder_index, workspace_folder) in workspace_folders.into_iter().enumerate()
        {
            //log(&format!(
            //    "in workspace folder: {}",
            //    &workspace_folder.uri.as_str()
            //));
            let path = workspace_folder.uri.as_str();
            let path = path.strip_prefix("file://").unwrap_or(path);
            let directory = &PathBuf::from(path);
            for (file_index, file) in collect_puml_files(directory)?.into_iter().enumerate() {
                let content = std::fs::read_to_string(&file)?;
                let url = Url::parse(&format!(
                    "file://{}",
                    file.to_str().unwrap_or(
                        format!("{}-{}.puml", worksspace_folder_index, file_index).as_str()
                    )
                ))
                .unwrap();
                workspace_documents.push(TextDocumentItem {
                    language_id: "plantuml".to_string(),
                    uri: url,
                    version: 1,
                    text: content,
                });
            }
        }
    }
    Ok(workspace_documents)
}

fn collect_puml_files(path: &PathBuf) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() && path.extension().map_or(false, |e| e.eq("puml")) {
        files.push(path.clone());
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let mut entry_files = collect_puml_files(&entry.path())?;
            files.append(&mut entry_files);
        }
    }
    Ok(files)
}
