use std::error::Error;

use lsp_server::{Connection, Message, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InitializeParams,
    ServerCapabilities,
};

// the initial version is very much taken from the lsp-server example:
// https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/examples/goto_def.rs
fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("language server starting");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        completion_provider: Some(lsp_types::CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: None,
            all_commit_characters: None,
            work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    eprintln!("language server stopping");

    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    eprintln!("connection established, waiting for messages");
    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");

                match req.method.as_str() {
                    "textDocument/completion" => {
                        let id = req.id;
                        let completion: CompletionParams = serde_json::from_value(req.params)?;
                        eprintln!("completion params: {completion:?}");
                        let mut completion_items = Vec::new();
                        completion_items.push(CompletionItem {
                            label: "test".to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            ..Default::default()
                        });
                        let result = CompletionResponse::Array(completion_items);
                        let resp = Response {
                            id,
                            result: Some(serde_json::to_value(&result).unwrap()),
                            error: None,
                        };
                        connection.sender.send(Message::Response(resp))?;
                        continue;
                    }

                    _ => eprintln!(""),
                }
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
            }
        }
    }
    Ok(())
}
