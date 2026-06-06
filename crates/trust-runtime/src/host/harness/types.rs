//! Harness shared types.

#![allow(missing_docs)]

use annotate_snippets::{Level, Renderer, Snippet};
use trust_syntax::lexer::Lexer;

use crate::error::RuntimeError;
use crate::value::Duration;

/// A source file and optional path metadata.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: Option<String>,
    pub text: String,
}

impl SourceFile {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            path: None,
            text: text.into(),
        }
    }

    pub fn with_path(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            text: text.into(),
        }
    }
}

/// A single compile diagnostic anchored at a byte range in a source file.
///
/// Carried alongside [`CompileError`] so the CLI can render rustc-style
/// annotated source snippets, while the plain `Display` text stays available
/// for programmatic consumers and tests.
#[derive(Debug, Clone)]
pub struct DiagnosticSnippet {
    /// Human-readable message (the diagnostic title and span label).
    pub message: String,
    /// Optional diagnostic code, rendered as `error[CODE]`.
    pub code: Option<String>,
    /// Optional source path, used as the snippet origin.
    pub path: Option<String>,
    /// Full source text the range refers to.
    pub source: String,
    /// Byte offset where the offending span starts.
    pub start: usize,
    /// Byte offset where the offending span ends (may equal `start`).
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    message: String,
    snippets: Vec<DiagnosticSnippet>,
}

impl CompileError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            snippets: Vec::new(),
        }
    }

    /// Creates an error that also carries structured diagnostics for rich
    /// console rendering. `message` remains the plain, single-line summary.
    pub fn with_snippets(message: impl Into<String>, snippets: Vec<DiagnosticSnippet>) -> Self {
        Self {
            message: message.into(),
            snippets,
        }
    }

    /// Structured diagnostics for rich rendering, empty when unavailable.
    #[must_use]
    pub fn snippets(&self) -> &[DiagnosticSnippet] {
        &self.snippets
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}

/// Renders compile diagnostics as rustc-style annotated source snippets.
///
/// `color` selects ANSI-styled vs plain output (callers typically pass the
/// result of a `stderr().is_terminal()` check). Returns an empty string when
/// `snippets` is empty.
#[must_use]
pub fn render_diagnostics(snippets: &[DiagnosticSnippet], color: bool) -> String {
    let renderer = if color {
        Renderer::styled()
    } else {
        Renderer::plain()
    };

    let mut out = String::new();
    for snippet in snippets {
        // Clamp the span to the buffer, then tighten it to the offending text so
        // the carets don't underline surrounding trivia (diagnostic ranges often
        // include trailing whitespace, blank lines, or comments).
        let end = snippet.end.min(snippet.source.len());
        let start = snippet.start.min(end);
        let (start, end) = tighten_span(&snippet.source, start, end);

        let mut source = Snippet::source(&snippet.source).line_start(1).fold(true);
        if let Some(path) = &snippet.path {
            source = source.origin(path);
        }
        source = source.annotation(Level::Error.span(start..end).label(&snippet.message));

        let mut message = Level::Error.title(&snippet.message);
        if let Some(code) = &snippet.code {
            message = message.id(code);
        }
        let message = message.snippet(source);

        out.push_str(&renderer.render(message).to_string());
        out.push('\n');
    }
    out
}

/// Shrinks a byte span to its first and last non-trivia tokens, dropping
/// surrounding whitespace, comments, and pragmas. Returns the span unchanged
/// when it is empty or contains only trivia.
fn tighten_span(source: &str, start: usize, end: usize) -> (usize, usize) {
    if start >= end {
        return (start, end);
    }

    let mut first = None;
    let mut last = None;
    for token in Lexer::new(&source[start..end]) {
        if token.kind.is_trivia() {
            continue;
        }
        let token_start = usize::from(token.range.start());
        let token_end = usize::from(token.range.end());
        first.get_or_insert(token_start);
        last = Some(token_end);
    }

    match (first, last) {
        (Some(first), Some(last)) => (start + first, start + last),
        _ => (start, end),
    }
}

/// Result of a single execution cycle.
#[derive(Debug, Clone)]
pub struct CycleResult {
    pub cycle_number: u64,
    pub elapsed_time: Duration,
    pub errors: Vec<RuntimeError>,
}
