use lsp_types::{
    ServerCapabilities, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions,
};

pub(crate) fn get_server_capabilities() -> serde_json::Value {
    serde_json::to_value(&ServerCapabilities {
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(
                    lsp_types::SaveOptions {
                        include_text: Some(true),
                    },
                )),
                ..Default::default()
            },
        )),
        completion_provider: Some(lsp_types::CompletionOptions {
            ..Default::default()
        }),
        //diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
        //    lsp_types::DiagnosticOptions {
        //        inter_file_dependencies: false,
        //        workspace_diagnostics: false,
        //        ..Default::default()
        //    },
        //)),
        references_provider: Some(lsp_types::OneOf::Left(true)),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        //code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
        //rename_provider: Some(lsp_types::OneOf::Left(true)),

        //document_highlight_provider: Some(lsp_types::OneOf::Left(true)),
        //document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
        //hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        //document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        //document_range_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        /* semantic tokens disabled for now
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    legend: lsp_types::SemanticTokensLegend {
                        token_types: vec![
                            SemanticTokenType::NAMESPACE,
                            SemanticTokenType::TYPE,
                            SemanticTokenType::CLASS,
                            SemanticTokenType::ENUM,
                            SemanticTokenType::INTERFACE,
                            SemanticTokenType::STRUCT,
                            SemanticTokenType::TYPE_PARAMETER,
                            SemanticTokenType::PARAMETER,
                            SemanticTokenType::VARIABLE,
                            SemanticTokenType::PROPERTY,
                            SemanticTokenType::ENUM_MEMBER,
                            SemanticTokenType::EVENT,
                            SemanticTokenType::FUNCTION,
                            SemanticTokenType::METHOD,
                            SemanticTokenType::MACRO,
                            SemanticTokenType::KEYWORD,
                            SemanticTokenType::MODIFIER,
                            SemanticTokenType::COMMENT,
                            SemanticTokenType::STRING,
                            SemanticTokenType::NUMBER,
                            SemanticTokenType::REGEXP,
                            SemanticTokenType::OPERATOR,
                            SemanticTokenType::DECORATOR,
                        ],
                        token_modifiers: vec![
                            SemanticTokenModifier::DECLARATION,
                            SemanticTokenModifier::DEFINITION,
                            SemanticTokenModifier::READONLY,
                            SemanticTokenModifier::STATIC,
                            SemanticTokenModifier::DEPRECATED,
                            SemanticTokenModifier::ABSTRACT,
                            SemanticTokenModifier::ASYNC,
                            SemanticTokenModifier::MODIFICATION,
                            SemanticTokenModifier::DOCUMENTATION,
                            SemanticTokenModifier::DEFAULT_LIBRARY,
                        ],
                    },
                    range: Some(true),
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                },
            ),
        ),
        */
        /*
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(lsp_types::WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(lsp_types::OneOf::Left(false)),
            }),
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                did_create: Some(lsp_types::FileOperationRegistrationOptions {
                    filters: vec![FileOperationFilter {
                        scheme: None,
                        pattern: lsp_types::FileOperationPattern {
                            glob: "**".to_string(),
                            matches: None,
                            options: None,
                        },
                    }],
                }),
                ..Default::default()
            }),
        }),
        */
        ..Default::default()
    })
    .unwrap()
}
