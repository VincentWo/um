#![feature(core_intrinsics)]
use std::collections::HashMap;

use change::{Change, Range};

use encoding::Encoding;

use fuzzy_matcher::clangd::fuzzy_match;
use serde::Deserialize;
use tokio::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionList, CompletionOptions, CompletionOptionsCompletionItem,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, Position as LspPosition,
    PositionEncodingKind, Range as LspRange, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};
use virtual_file::VirtualFile;

mod change;
mod encoding;
mod virtual_file;

#[derive(Debug)]
struct Backend {
    client: Client,
    opened_documents: RwLock<HashMap<Url, VirtualFile>>,
    position_encoding: RwLock<Encoding>,
    unicode_data: Vec<UnicodeData>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Pick whatever the client prefers or UTF-16 if they don't give any
        let lsp_position_encoding = params
            .capabilities
            .general
            .unwrap()
            .position_encodings
            .unwrap_or_default()
            .first()
            .cloned()
            .unwrap_or(PositionEncodingKind::UTF16);

        *self.position_encoding.write().await = lsp_position_encoding.clone().into();

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..Default::default()
                    },
                )),
                position_encoding: Some(lsp_position_encoding),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![r"\".into()]),
                    completion_item: Some(CompletionOptionsCompletionItem {
                        label_details_support: Some(true),
                    }),
                    ..CompletionOptions::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let new_doc = params.text_document;
        // TODO: This doesn't split on '\r', only on '\n\r'
        let file = VirtualFile::new(&new_doc.text);
        {
            let mut documents = self.opened_documents.write().await;
            documents.insert(new_doc.uri, file);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut opened_docs = self.opened_documents.write().await;
        let file = opened_docs.get_mut(&uri).unwrap();
        let pos_encoding = *self.position_encoding.read().await;
        file.apply_changes(
            params
                .content_changes
                .into_iter()
                .map(|lsp_change| Change {
                    range: Range::from_lsp_range(lsp_change.range.unwrap(), pos_encoding),
                    new_content: lsp_change.text,
                })
                .collect(),
        );
    }
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let file = self.opened_documents.read().await[&uri].clone();
        let raw_line = file.content.line(position.line as usize);

        let byte_position = raw_line.char_to_byte((position.character) as usize);
        let line = &raw_line.to_string()[..byte_position];

        let Some(completion_start) = line.rfind('\\') else {
            return Ok(None);
        };

        // let Some(completion_start) =  else {
        //     return Ok(None);
        // };
        // This is fine since completion_start is the valid index of '\' which
        // is a one byte character.
        let search_text = &line[completion_start + 1..];
        self.client
            .log_message(MessageType::INFO, format!("Searching with '{search_text}'"))
            .await;
        Ok(Some(CompletionResponse::List(
            CompletionList {
                is_incomplete: true,
                items: self
                    .unicode_data
                    .iter()
                    .flat_map(|d| fuzzy_match(&d.name, search_text).map(|score| (score, d)))
                    .map(|(_score, d)| CompletionItem {
                        label: format!(
                            "{} - {} U+{}",
                            char::from_u32(d.codepoint).unwrap_or('�'),
                            d.name,
                            d.codepoint
                        ),
                        // sort_text: Some(format!("{:0>25}", d.name.len())),
                        text_edit: Some(tower_lsp::lsp_types::CompletionTextEdit::Edit(TextEdit {
                            range: LspRange {
                                start: LspPosition {
                                    line: position.line,
                                    //TODO: This is only valid if utf-32 is used
                                    character: raw_line.byte_to_char(completion_start) as u32,
                                },
                                end: position,
                            },
                            new_text: char::from_u32(d.codepoint).unwrap_or('�').to_string(),
                        })),
                        ..Default::default()
                    })
                    // .take(100)
                    .collect(),
            }, //     CompletionItem {
               // label: "ß - SMALL LETTER SHARP S U+00DF".into(),
               //     label: "".into(),
               //     kind: Some(CompletionItemKind::TEXT),
               //     deprecated: Some(false),
               //     text_edit: Some(tower_lsp::lsp_types::CompletionTextEdit::Edit(TextEdit { range: LspRange { start: LspPosition { line: position.line, character: completion_start as u32 }, end: position }, new_text: "∫".into() } )),
               //     commit_characters: Some(vec![" ".into()]),
               //     // TODO: hmmmm
               //     filter_text: None,
               //     // TODO: Put "matchiness here"
               //     sort_text: None,
               //     ..Default::default()
               // }, CompletionItem {
        )))
    }
}

#[derive(Debug, Deserialize)]
struct RawUnicodeData {
    codepoint: String,
    name: String,
}

#[derive(Debug)]
struct UnicodeData {
    codepoint: u32,
    name: String,
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut unicode_data = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .flexible(false)
        .from_path("../UnicodeData.txt")
        .unwrap();

    unicode_data.set_headers(vec!["codepoint", "name"].into());

    let mut unicode_data = unicode_data
        .into_deserialize::<RawUnicodeData>()
        .map(|res| {
            res.map(|d| {
                let codepoint = u32::from_str_radix(&d.codepoint, 16).unwrap();
                UnicodeData {
                    codepoint,
                    name: d.name,
                }
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();

    unicode_data.sort_unstable_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let (service, socket) = LspService::new(|client| Backend {
        client,
        opened_documents: Default::default(),
        position_encoding: RwLock::new(Encoding::Utf16),
        unicode_data,
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
