use std::{
    error::Error,
    io::Write,
    path::PathBuf,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use application_config::{ApplicationConfig, PromptConfigEntry};
use gpt_adapter::GptResponseUsage;
use lsp_server::{Connection, Message};
use lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeParams, TextDocumentItem, Uri,
};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Method,
};

use crate::gpt_adapter::{GptMessage, GptRequest, GptResponse};

mod application_config;
mod gpt_adapter;
mod server_capabilities;

const BASE_FOLDER: &str = "/home/marco/dev/projects/lsp-gpt";
const USE_WORKSPACE_FOLDERS: bool = true;
const USE_ADDITIONAL_PARAMETERS: bool = false;

// the initial version is very much taken from the lsp-server example:
// https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/examples/goto_def.rs
fn main() -> Result<(), Box<dyn Error>> {
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
    let application_config = ApplicationConfig::from(create_path("/assets/prompts/").as_str());

    // enter the main event loop
    handle_messages(
        connection,
        initialization_params,
        application_config,
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
) -> Result<(), Box<dyn Error>> {
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
    mut application_config: ApplicationConfig,
    api_key: String,
    api_company_id: String,
) -> Result<(), Box<dyn Error>> {
    let params: InitializeParams = serde_json::from_value(params).unwrap();

    log::info!("connection established, waiting for messages");

    // create http client
    let http_client = reqwest::blocking::Client::builder()
        .timeout(Some(Duration::new(60, 0)))
        .build()?;
    let auth = format!("Bearer {}", api_key);
    let headers = create_headers(&auth, &api_company_id);

    // buffer for document updates
    let mut latest_text_document_item = None;

    // wait for messages
    for msg in &connection.receiver {
        let raw_msg = serde_json::to_string(&msg)?;
        log::info!("got msg: {}", raw_msg);
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                log::info!("got request: {req:?}");

                let prompt_config_entry =
                    application_config.prompt_config.get_or_default(&req.method);
                let messages = create_messages(
                    raw_msg,
                    &application_config.language,
                    prompt_config_entry,
                    &params,
                    &latest_text_document_item,
                )?;

                // query the GPT API
                let model = &prompt_config_entry.model;
                let temperature = prompt_config_entry.model_temperature;
                let gpt_request = GptRequest {
                    model: model.clone(),
                    messages,
                    temperature,
                    n: 1,
                };
                let response_text =
                    send_request(gpt_request, &http_client, &headers, &req.method, model)?;
                map_and_return_response(&response_text, &connection)
            }
            Message::Response(resp) => {
                log::info!("got response (why though?): {resp:?}");
            }
            Message::Notification(not) => {
                // notification handling, for updates about textDocuments
                log::info!("got notification: {not:?}");
                match not.method.as_str() {
                    "textDocument/didOpen" => {
                        // text document opened, cache its contents
                        match serde_json::from_value::<DidOpenTextDocumentParams>(not.params) {
                            Ok(text_document) => {
                                log::info!("document opened: {:?}", &text_document);
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
                        // text document changed, update cached contents
                        match serde_json::from_value::<DidChangeTextDocumentParams>(not.params) {
                            Ok(text_document) => {
                                let url = text_document.text_document.uri.as_str();
                                log::info!("document changed: {}", url);
                                update_latest_document(&mut latest_text_document_item, url)?;
                            }
                            Err(err) => log::info!("can not parse didChange notification {err}"),
                        }
                    }
                    "textDocument/didSave" => {
                        // the user saved the document, this triggers a diagnostics request
                        match serde_json::from_value::<DidSaveTextDocumentParams>(not.params) {
                            Ok(did_save_params) => {
                                log::info!("received save event: {:?}", &did_save_params);
                                let url = did_save_params.text_document.uri.as_str();
                                update_latest_document(&mut latest_text_document_item, url)?;

                                let method = "textDocument/diagnostic";
                                //HACK: very ugly and manual way of constructing that request
                                //TODO: refactor to use struct or at least something serde-ish
                                let diagnostic_request_msg =
                                    format!("{{\"method\":\"{method}\",\"params\":{{\"textDocument\":{{\"uri\":\"{url}\"}}, \"response_type\":\"notification\"}}}}");

                                let prompt_config_entry =
                                    application_config.prompt_config.get_or_default(&not.method);
                                let messages = create_messages(
                                    diagnostic_request_msg,
                                    &application_config.language,
                                    prompt_config_entry,
                                    &params,
                                    &latest_text_document_item,
                                )?;

                                // query the GPT API
                                let model = &prompt_config_entry.model;
                                let temperature = prompt_config_entry.model_temperature;
                                let gpt_request = GptRequest {
                                    model: model.clone(),
                                    messages,
                                    temperature,
                                    n: 1,
                                };
                                let response_text = send_request(
                                    gpt_request,
                                    &http_client,
                                    &headers,
                                    &format!("{}#{}", method, not.method),
                                    model,
                                )?;
                                map_and_return_notification(&response_text, &connection);
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

fn map_and_return_response(response_text: &str, connection: &Connection) {
    match serde_json::from_str(response_text) {
        Ok(response_message) => {
            // valid language server response
            log::info!("all good, sending response to client: {response_text}");
            if let Err(err) = connection.sender.send(Message::Response(response_message)) {
                log::info!("could not send response to client: {err}")
            }
        }
        Err(err) => {
            log::info!("error parsing response, err: {err}, response: {response_text}")
        }
    }
}

fn map_and_return_notification(response_text: &str, connection: &Connection) {
    match serde_json::from_str(response_text) {
        Ok(response_message) => {
            // valid language server response
            log::info!("all good, sending notification to client: {response_text}");
            if let Err(err) = connection
                .sender
                .send(Message::Notification(response_message))
            {
                log::info!("could not send notification to client: {err}")
            }
        }
        Err(err) => {
            log::info!("error parsing server notification, err: {err}, response: {response_text}")
        }
    }
}

fn update_latest_document(
    latest_text_document_item: &mut Option<TextDocumentItem>,
    url: &str,
) -> std::io::Result<()> {
    if let Some(doc) = latest_text_document_item {
        // update only if the URLs match
        if doc.uri.as_str().eq(url) {
            doc.text = read_file(url)?;
        }
    }
    Ok(())
}

fn create_headers(auth: &str, api_company_id: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(header::AUTHORIZATION, auth.parse().unwrap());
    headers.insert("OpenAI-Organization", api_company_id.parse().unwrap());
    headers
}

fn send_request(
    gpt_request: GptRequest,
    http_client: &reqwest::blocking::Client,
    headers: &HeaderMap,
    method: &str,
    model: &str,
) -> Result<String, Box<dyn Error>> {
    let request = serde_json::to_string(&gpt_request)?;
    log::info!("sending request to GPT API: {request}");

    // build request
    let api_request = http_client
        .request(Method::POST, "https://api.openai.com/v1/chat/completions")
        .headers(headers.clone())
        .body(request)
        .build()?;

    // execute request and measure time
    let start = SystemTime::now();
    let api_result = http_client.execute(api_request)?;
    let duration = start.elapsed()?;

    // handle result
    let response = api_result.text()?;
    log::info!("got GPT response: {}", &response);
    let response_json = serde_json::from_str::<GptResponse>(&response)?;
    log_invocation(
        model,
        duration,
        method,
        gpt_request.messages.len(),
        &response_json.usage,
    )?;

    // response is valid json, so we can use the first answer
    let response_message_raw = response_json
        .choices
        .first()
        .expect("no choices in response")
        .message
        .content
        .as_str();

    let extracted_response_message = extract_json(response_message_raw);
    Ok(extracted_response_message.to_string())
}

fn read_file(url: &str) -> std::io::Result<String> {
    let file = url.strip_prefix("file://").unwrap_or(url);
    std::fs::read_to_string(&file)
}

fn create_messages(
    mut raw_msg: String,
    language_id: &str,
    prompt_config_entry: &PromptConfigEntry,
    params: &InitializeParams,
    latest_text_document_item: &Option<TextDocumentItem>,
) -> std::io::Result<Vec<GptMessage>> {
    let mut messages = Vec::with_capacity(3);
    // prompting
    for prompt in &prompt_config_entry.prompt_messages {
        messages.push(GptMessage {
            role: "system".to_string(),
            content: prompt.to_string(),
        });
    }
    // send all documents in workspace
    //TODO: define a limit?
    let workspace_documents = gather_workspace_documents(&params, language_id)?;
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
    if USE_ADDITIONAL_PARAMETERS {
        let mut req: lsp_server::Request = serde_json::from_str(&raw_msg)?;
        let params = req.params.as_object_mut().unwrap();
        params.insert("min_results".to_string(), serde_json::Value::from(3));
        params.insert("max_results".to_string(), serde_json::Value::from(3));
        raw_msg = serde_json::to_string(&req).unwrap();
    }
    // actual request from client
    messages.push(GptMessage {
        role: "user".to_string(),
        content: raw_msg,
    });

    Ok(messages)
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

fn gather_workspace_documents(
    params: &InitializeParams,
    language_id: &str,
) -> std::io::Result<Vec<TextDocumentItem>> {
    let mut workspace_documents = Vec::new();
    if !USE_WORKSPACE_FOLDERS {
        return Ok(workspace_documents);
    }
    if let Some(workspace_folders) = &params.workspace_folders {
        for (worksspace_folder_index, workspace_folder) in workspace_folders.into_iter().enumerate()
        {
            let path = workspace_folder.uri.as_str();
            let path = path.strip_prefix("file://").unwrap_or(path);
            let directory = &PathBuf::from(path);
            for (file_index, file) in collect_puml_files(directory)?.into_iter().enumerate() {
                let content = std::fs::read_to_string(&file)?;
                let uri = Uri::from_str(&format!(
                    "file://{}",
                    file.to_str().unwrap_or(
                        format!("{}-{}.puml", worksspace_folder_index, file_index).as_str()
                    )
                ))
                .unwrap();
                workspace_documents.push(TextDocumentItem {
                    language_id: language_id.to_string(),
                    uri,
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
    //TODO: other file extensions could be needed to; make configurable?
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
