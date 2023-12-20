use std::error::Error;

use lsp_server::{Connection, Message, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InitializeParams,
    ServerCapabilities,
};

// the initial version is very much taken from the lsp-server example:
// https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/examples/goto_def.rs
fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    log("language server starting");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        completion_provider: Some(lsp_types::CompletionOptions {
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
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
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    log("connection established, waiting for messages");
    for msg in &connection.receiver {
        log(&format!("got msg: {msg:?}"));
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                log(&format!("got request: {req:?}"));

                match req.method.as_str() {
                    "textDocument/completion" => {
                        let id = req.id;
                        let completion: CompletionParams = serde_json::from_value(req.params)?;
                        log(&format!("completion params: {completion:?}"));
                        let mut completion_items = Vec::new();
                        completion_items.push(CompletionItem {
                            label: "hello from lsp-gpt".to_string(),
                            kind: Some(CompletionItemKind::KEYWORD),
                            ..Default::default()
                        });
                        completion_items.push(CompletionItem {
                            label: "Hallo Charlotte ^^".to_string(),
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

                    _ => log(&format!("unimplemented method: {}", req.method.as_str())),
                }
            }
            Message::Response(resp) => {
                log(&format!("got response: {resp:?}"));
            }
            Message::Notification(not) => {
                log(&format!("got notification: {not:?}"));
            }
        }
    }
    Ok(())
}
