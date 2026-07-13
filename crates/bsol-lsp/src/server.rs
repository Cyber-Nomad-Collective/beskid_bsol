//! BSOL LSP backend.

use std::collections::HashMap;

use bsol::{analyze_with_profile, parse_bsol_document, BsolError, ValidatedDocument};
use bsol_syntax::BsolError as ParseError;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub struct BsolLanguageServer {
    client: Client,
    documents: HashMap<Url, String>,
}

impl BsolLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: HashMap::new(),
        }
    }

    fn profile_for_uri(&self, uri: &Url) -> &str {
        match uri.path().rsplit('.').next() {
            Some("bws") => "workspace.v1",
            Some("bsol") => "schema.v1",
            _ => "project.v1",
        }
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let Some(source) = self.documents.get(&uri) else {
            return;
        };
        let profile = self.profile_for_uri(&uri);
        let diagnostics = collect_diagnostics(source, profile);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BsolLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut this = self.clone_inner();
        this.documents
            .insert(uri.clone(), params.text_document.text);
        this.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut this = self.clone_inner();
        if let Some(change) = params.content_changes.into_iter().last() {
            this.documents.insert(uri.clone(), change.text);
        }
        this.publish_diagnostics(uri).await;
    }
}

impl BsolLanguageServer {
    fn clone_inner(&self) -> Self {
        Self {
            client: self.client.clone(),
            documents: self.documents.clone(),
        }
    }
}

fn collect_diagnostics(source: &str, profile: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if let Err(err) = parse_bsol_document(source) {
        out.push(diagnostic_from_parse_error(&err));
        return out;
    }
    if let Err(err) = analyze_with_profile(source, profile) {
        out.push(diagnostic_from_error(&err));
    }
    out
}

fn diagnostic_from_parse_error(err: &ParseError) -> Diagnostic {
    let line = err.source_line().unwrap_or(1).saturating_sub(1);
    let (start, end) = err
        .source_span()
        .map(|(s, e)| (s as u32, e as u32))
        .unwrap_or((0, 1));
    Diagnostic {
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: (end - start).max(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: err.to_string(),
        ..Default::default()
    }
}

fn diagnostic_from_error(err: &BsolError) -> Diagnostic {
    let line = err.manifest_source_line().unwrap_or(1).saturating_sub(1);
    let (start, end) = err
        .manifest_source_span()
        .map(|(s, e)| (s as u32, e as u32))
        .unwrap_or((0, 1));
    Diagnostic {
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: (end - start).max(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: err.to_string(),
        ..Default::default()
    }
}

#[allow(dead_code)]
fn _validated_hint(_doc: &ValidatedDocument) {}
