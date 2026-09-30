//! Comment removal for the languages this tool understands.
//!
//! Every pass is a lexer rather than a regex. That is the whole point: a `//`
//! inside a Go raw string, a `'a'` lifetime in Rust, a `/` in a JavaScript regex,
//! a `//` in JSX text and a `--` inside a SQL string are all content, and a
//! line-based or pattern-based strip would eat the rest of the file along with
//! the comment.
//!
//! Comments that *instruct a tool* are kept by default, because deleting one
//! breaks a build rather than a reader: `//go:build`, `//go:embed`,
//! `//go:generate`, `//nolint` in Go; `// rustfmt::skip` and `/// <reference>`
//! plus the `@ts-ignore` / `eslint-disable` family in JavaScript and TypeScript.
//! Everything else, including Rust doc comments and `// SAFETY:` prose, goes.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommentStyle {
    Go,
    Rust,
    JavaScript,
    Sql,
    GraphQl,
}

impl CommentStyle {
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            CommentStyle::Go => &["go"],
            CommentStyle::Rust => &["rs"],
            CommentStyle::JavaScript => &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"],
            CommentStyle::Sql => &["sql"],
            CommentStyle::GraphQl => &["graphqls", "graphql", "gql"],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CommentStyle::Go => "go",
            CommentStyle::Rust => "rust",
            CommentStyle::JavaScript => "js/ts",
            CommentStyle::Sql => "sql",
            CommentStyle::GraphQl => "graphql",
        }
    }

    /// The formatter that puts the file back the way its own toolchain would
    /// write it, once trailing comments have thrown off its alignment.
    pub fn formatters(&self) -> &'static [&'static str] {
        match self {
            CommentStyle::Go => &["gofmt", "go"],
            CommentStyle::Rust => &["rustfmt"],
            CommentStyle::JavaScript => &["prettier", "biome"],
            CommentStyle::Sql | CommentStyle::GraphQl => &[],
        }
    }
}

pub const STYLES: &[CommentStyle] = &[
    CommentStyle::Go,
    CommentStyle::JavaScript,
    CommentStyle::Rust,
    CommentStyle::Sql,
    CommentStyle::GraphQl,
];

/// A resolved `--lang` selection: which lexers to run, and which file extensions
/// to hand them. Keeping the two apart is what lets `--lang ts` touch only
/// TypeScript rather than every extension the JavaScript lexer claims.
#[derive(Debug, Clone)]
pub struct Selection {
    pub styles: Vec<CommentStyle>,
    pub extensions: Vec<String>,
}

impl Selection {
    fn with(styles: &[CommentStyle], extensions: &[&str]) -> Selection {
        let mut out_styles: Vec<CommentStyle> = Vec::new();
        for style in styles {
            if !out_styles.contains(style) {
                out_styles.push(*style);
            }
        }
        out_styles.sort_by_key(|s| s.label());
        out_styles.dedup();

        let mut out_exts: Vec<String> = extensions.iter().map(|e| e.to_string()).collect();
        out_exts.sort();
        out_exts.dedup();

        Selection {
            styles: out_styles,
            extensions: out_exts,
        }
    }

    fn merge(self, other: Selection) -> Selection {
        let mut styles = self.styles;
        for style in other.styles {
            if !styles.contains(&style) {
                styles.push(style);
            }
        }
        styles.sort_by_key(|s| s.label());
        styles.dedup();

        let mut extensions = self.extensions;
        extensions.extend(other.extensions);
        extensions.sort();
        extensions.dedup();

        Selection { styles, extensions }
    }
}

/// resolve turns a comma-separated or repeated list of names into a selection.
/// `all` selects every supported language.
pub fn resolve(names: &[String]) -> Result<Selection, String> {
    let mut out: Option<Selection> = None;

    for name in names {
        for part in name.split(',') {
            let part = part.trim().to_lowercase();
            if part.is_empty() {
                continue;
            }
            let picked: Selection = match part.as_str() {
                "all" => {
                    let all: Vec<&str> = STYLES
                        .iter()
                        .flat_map(|s| s.extensions().iter().copied())
                        .collect();
                    Selection::with(STYLES, &all)
                }
                "go" | "golang" => Selection::with(&[CommentStyle::Go], &["go"]),
                "rust" | "rs" => Selection::with(&[CommentStyle::Rust], &["rs"]),
                "js" | "javascript" | "mjs" | "cjs" | "node" => {
                    Selection::with(&[CommentStyle::JavaScript], &["js", "jsx", "mjs", "cjs"])
                }
                "jsx" => Selection::with(&[CommentStyle::JavaScript], &["jsx"]),
                "ts" | "typescript" => {
                    Selection::with(&[CommentStyle::JavaScript], &["ts", "tsx", "mts", "cts"])
                }
                "tsx" => Selection::with(&[CommentStyle::JavaScript], &["tsx"]),
                "react" | "frontend" | "web" => Selection::with(
                    &[CommentStyle::JavaScript],
                    &["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"],
                ),
                "sql" | "postgres" | "postgresql" => {
                    Selection::with(&[CommentStyle::Sql], &["sql"])
                }
                "graphql" | "gql" => {
                    Selection::with(&[CommentStyle::GraphQl], &["graphqls", "graphql", "gql"])
                }
                other => return Err(format!("unsupported language: {other}")),
            };
            out = Some(match out {
                Some(acc) => acc.merge(picked),
                None => picked,
            });
        }
    }

    match out {
        Some(selection) if !selection.styles.is_empty() => Ok(selection),
        _ => Err("no language selected".to_string()),
    }
}

pub fn style_for(path: &Path, selection: &Selection) -> Option<CommentStyle> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    if !selection.extensions.contains(&ext) {
        return None;
    }
    selection
        .styles
        .iter()
        .copied()
        .find(|s| s.extensions().iter().any(|e| *e == ext))
}

pub fn extensions_for(selection: &Selection) -> Vec<String> {
    selection.extensions.clone()
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct StripResult {
    pub content: String,
    pub comments: usize,
}

#[derive(Debug)]
pub struct Options {
    /// Keep the blank line a removed comment leaves in its place.
    pub keep_blank_lines: bool,
    /// Delete tool directives as well, not just commentary.
    pub strip_directives: bool,
}

pub fn is_generated(source: &str) -> bool {
    let head: String = source.chars().take(4096).collect();
    head.contains("Code generated by") && head.contains("DO NOT EDIT")
}

pub fn strip(style: CommentStyle, source: &str, opts: &Options) -> StripResult {
    match style {
        CommentStyle::Go => strip_braced(source, go_comment_spans(source, opts), opts),
        CommentStyle::Rust => strip_braced(source, rust_comment_spans(source, opts), opts),
        CommentStyle::JavaScript => strip_braced(source, js_comment_spans(source, opts), opts),
        CommentStyle::Sql => strip_lines(source, sql_comment_spans(source), opts),
        CommentStyle::GraphQl => strip_lines(source, graphql_comment_spans(source), opts),
    }
}

// ---------------------------------------------------------------- shared

/// strip_braced serves the curly-brace languages. A comment is taken out with
/// the whitespace that framed it, then the leftover blank lines at the top and
/// bottom of a block are dropped, because that is where the comment used to sit
/// and no formatter in this toolchain would leave the gap behind.
fn strip_braced(source: &str, spans: Vec<Span>, opts: &Options) -> StripResult {
    if spans.is_empty() {
        return StripResult {
            content: source.to_string(),
            comments: 0,
        };
    }

    let src = source.as_bytes();
    let widened: Vec<Span> = spans.iter().map(|sp| widen(src, *sp)).collect();

    let mut out = String::with_capacity(source.len());
    let mut prev = 0usize;
    for sp in &widened {
        out.push_str(&source[prev..sp.start]);
        prev = sp.end;
    }
    out.push_str(&source[prev..]);

    let content = if opts.keep_blank_lines {
        out
    } else {
        tidy_block_blanks(&out)
    };

    StripResult {
        content,
        comments: widened.len(),
    }
}

/// widen takes a comment out together with the whitespace that framed it. A
/// comment on its own line takes the line; a trailing one leaves the line and its
/// newline so the statement it annotated keeps its own line.
fn widen(src: &[u8], sp: Span) -> Span {
    let line_start = line_start(src, sp.start);
    let tail_end = line_end(src, sp.end);
    let own_line = src[line_start..sp.start]
        .iter()
        .all(|b| b.is_ascii_whitespace());
    let ends_line = src[sp.end..tail_end]
        .iter()
        .all(|b| b.is_ascii_whitespace());

    match (own_line, ends_line) {
        (true, true) => Span {
            start: line_start,
            end: (tail_end + 1).min(src.len()),
        },
        (true, false) => Span {
            start: line_start,
            end: sp.end,
        },
        (false, true) => Span {
            start: trim_back(src, sp.start),
            end: tail_end,
        },
        (false, false) => Span {
            // Code follows on the same line, so the gap before the comment was
            // only ever separator whitespace.
            start: trim_back(src, sp.start),
            end: sp.end,
        },
    }
}

fn trim_back(src: &[u8], mut off: usize) -> usize {
    while off > 0 && (src[off - 1] == b' ' || src[off - 1] == b'\t') {
        off -= 1;
    }
    off
}

fn line_start(src: &[u8], off: usize) -> usize {
    src[..off]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |p| p + 1)
}

/// line_end returns the offset of the newline that ends this line, or the end
/// of the file when the last line has none.
fn line_end(src: &[u8], off: usize) -> usize {
    src[off..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(src.len(), |p| off + p)
}

/// tidy_block_blanks is the post-pass the brace languages need and the
/// line-oriented ones do not: it holds the file to one blank line in a row and
/// drops a run that sits directly inside a block's opening or closing brace.
/// Both are normalisations rustfmt, gofmt and prettier all perform anyway.
fn tidy_block_blanks(text: &str) -> String {
    let (lines, _) = split_lines(text);
    if lines.is_empty() {
        return text.to_string();
    }

    let blank: Vec<bool> = lines.iter().map(|l| l.body.trim().is_empty()).collect();
    let mut keep = vec![true; lines.len()];

    let mut i = 0;
    while i < blank.len() {
        if !blank[i] {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        while end < blank.len() && blank[end] {
            end += 1;
        }

        let opens = (0..start)
            .rev()
            .find(|k| !blank[*k])
            .is_some_and(|k| lines[k].body.trim_end().ends_with('{'));

        let closes = (end..blank.len())
            .find(|k| !blank[*k])
            .is_some_and(|k| lines[k].body.trim_start().starts_with('}'));

        // A run at either end of the file is never what the author meant, and a
        // run hugging a brace is a gap a comment just left behind.
        if start == 0 || end == blank.len() || (start > 0 && (opens || closes)) {
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = false;
            }
        } else if end - start > 1 {
            for slot in keep.iter_mut().take(end).skip(start + 1) {
                *slot = false;
            }
        }
        i = end;
    }

    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        if keep[i] {
            out.push_str(&line.body);
            out.push_str(&line.eol);
        }
    }
    out
}

// ---------------------------------------------------------------- Go

/// Go has interpreted strings, raw strings and rune literals, and they are the
/// only place a `//` or `/*` can appear without starting a comment.
fn go_comment_spans(source: &str, opts: &Options) -> Vec<Span> {
    let src = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;

    while i + 1 < src.len() {
        match src[i] {
            b'/' if src[i + 1] == b'/' => {
                let start = i;
                while i < src.len() && src[i] != b'\n' {
                    i += 1;
                }
                if opts.strip_directives || !is_go_directive(&source[start..i]) {
                    spans.push(Span { start, end: i });
                }
            }
            b'/' if src[i + 1] == b'*' => {
                let start = i;
                i += 2;
                while i + 1 < src.len() && !(src[i] == b'*' && src[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(src.len());
                spans.push(Span { start, end: i });
            }
            b'"' => i = skip_go_quoted(src, i, b'"'),
            b'`' => i = skip_go_quoted(src, i, b'`'),
            b'\'' => i = skip_go_quoted(src, i, b'\''),
            _ => i += 1,
        }
    }
    spans
}

/// skip_go_quoted returns the offset just past the closing delimiter. A backslash
/// escapes the next byte in an interpreted string and a rune, but is an ordinary
/// character inside a raw string.
fn skip_go_quoted(src: &[u8], open: usize, quote: u8) -> usize {
    let mut i = open + 1;
    while i < src.len() {
        if src[i] == b'\\' && quote != b'`' {
            i += 2;
            continue;
        }
        if src[i] == quote {
            return i + 1;
        }
        if src[i] == b'\n' && quote != b'`' {
            return i;
        }
        i += 1;
    }
    i
}

fn is_go_directive(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("//go:")
            || t == "//nolint"
            || t.starts_with("//nolint:")
            || t.starts_with("//go:generate")
            || is_generated_banner(t)
    })
}

fn is_generated_banner(line: &str) -> bool {
    line.starts_with("// Code generated ") && line.contains("DO NOT EDIT.")
}

// ---------------------------------------------------------------- Rust

/// Rust's traps are the raw string family (`r#"..."#`, `br#"..."#`), the byte
/// prefix, and the `'` that opens a lifetime rather than a char literal. A
/// lifetime is told apart by failing to parse as a char, which is exact: `'a'`
/// is a char, `'a` and `'static` are not.
fn rust_comment_spans(source: &str, opts: &Options) -> Vec<Span> {
    let src = source.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;

    while i < src.len() {
        if let Some(end) = rust_raw_string_end(src, i) {
            i = end;
            continue;
        }

        match src[i] {
            b'/' if src.get(i + 1) == Some(&b'/') => {
                let start = i;
                while i < src.len() && src[i] != b'\n' {
                    i += 1;
                }
                if opts.strip_directives || !is_rust_directive(&source[start..i]) {
                    spans.push(Span { start, end: i });
                }
            }
            b'/' if src.get(i + 1) == Some(&b'*') => {
                let start = i;
                i = rust_block_end(src, i);
                spans.push(Span { start, end: i });
            }
            b'"' => i = rust_str_end(src, i),
            b'\'' => match rust_char_end(src, i) {
                Some(end) => i = end,
                None => i += 1,
            },
            c if is_rust_word(c) => {
                while i < src.len() && is_rust_word(src[i]) {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    spans
}

fn is_rust_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// is_rust_directive keeps `// rustfmt::skip`, which tells rustfmt to leave the
/// next item exactly as written. Doc comments are not directives and do go.
fn is_rust_directive(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim_start().starts_with("// rustfmt::"))
}

/// rust_block_end walks a block comment to its close, counting nesting because
/// Rust allows `/* /* */ */`.
fn rust_block_end(src: &[u8], open: usize) -> usize {
    let mut depth = 1usize;
    let mut i = open + 2;
    while i < src.len() {
        match (src[i], src.get(i + 1)) {
            (b'/', Some(&b'*')) => {
                depth += 1;
                i += 2;
            }
            (b'*', Some(&b'/')) => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    src.len()
}

fn rust_str_end(src: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            b'\n' => return i,
            _ => i += 1,
        }
    }
    i
}

/// rust_char_end parses `'x'`, `'\n'` and `'\u{1F600}'` and returns the offset
/// past the closing quote. Anything else is a lifetime or a loop label, and
/// returning None is what stops the lexer eating code after a lifetime.
fn rust_char_end(src: &[u8], open: usize) -> Option<usize> {
    let mut i = open + 1;
    if i >= src.len() {
        return None;
    }

    if src[i] == b'\\' {
        i += 1;
        if src.get(i) == Some(&b'u') {
            i += 1;
            if src.get(i) == Some(&b'{') {
                while i < src.len() && src[i] != b'}' {
                    i += 1;
                }
                if i >= src.len() {
                    return None;
                }
                i += 1;
            } else {
                for _ in 0..4 {
                    match src.get(i) {
                        Some(b) if b.is_ascii_hexdigit() => i += 1,
                        _ => return None,
                    }
                }
            }
        } else {
            i += 1;
        }
    } else {
        i += 1;
        while i < src.len() && (src[i] & 0xC0) == 0x80 {
            i += 1;
        }
    }

    if src.get(i) == Some(&b'\'') {
        Some(i + 1)
    } else {
        None
    }
}

/// rust_raw_string_end recognises `r"..."`, `r#"..."#`, and the `b` and `c`
/// prefixed forms. `r#ident` is a raw identifier, not a string, so a `#` run
/// only counts when a quote follows it.
fn rust_raw_string_end(src: &[u8], i: usize) -> Option<usize> {
    let mut p = i;

    if matches!(src.get(i), Some(&b'b') | Some(&b'c')) {
        match src.get(i + 1) {
            Some(&b'"') | Some(&b'\'') => return None,
            Some(&b'r') => p = i + 1,
            _ => return None,
        }
    }

    if src.get(p) != Some(&b'r') {
        return None;
    }
    p += 1;

    let mut hashes = 0usize;
    while src.get(p) == Some(&b'#') {
        hashes += 1;
        p += 1;
    }

    if src.get(p) != Some(&b'"') {
        return None;
    }
    p += 1;

    while p < src.len() {
        if src[p] == b'"' && (0..hashes).all(|h| src.get(p + 1 + h) == Some(&b'#')) {
            return Some(p + 1 + hashes);
        }
        p += 1;
    }
    Some(src.len())
}

// ---------------------------------------------------------------- JavaScript / TypeScript

#[derive(Clone, Copy, PartialEq)]
enum JsMode {
    Code,
    Template,
    JsxChildren,
    JsxTag,
}

/// What the last significant token was, which is the only reliable way to tell a
/// regex literal from a division: `a / b` divides, `= /a/` starts a regex.
#[derive(Clone, Copy, PartialEq)]
enum Prev {
    None,
    Value,
    Operand,
}

struct JsFrame {
    mode: JsMode,
    /// `{` nesting inside this frame, so the `}` that closes a `${` or a JSX
    /// expression container pops the frame instead of unbalancing the file.
    braces: u32,
}

/// js_comment_spans walks JS, TS, JSX and TSX as a small state machine. It has to
/// understand four things a regex cannot: template literals with `${}`
/// interpolations, regex literals, JSX children (where `//` is rendered text, as
/// in a URL) and the braces that hold a JSX expression.
fn js_comment_spans(source: &str, opts: &Options) -> Vec<Span> {
    let src = source.as_bytes();
    let mut spans = Vec::new();
    let mut stack: Vec<JsFrame> = vec![JsFrame {
        mode: JsMode::Code,
        braces: 0,
    }];
    let mut prev = Prev::None;
    let mut i = 0usize;

    while i < src.len() {
        let Some(mode) = stack.last().map(|f| f.mode) else {
            break;
        };

        match mode {
            JsMode::Template => match src[i] {
                b'\\' => i += 2,
                b'`' => {
                    stack.pop();
                    prev = Prev::Value;
                    i += 1;
                }
                b'$' if src.get(i + 1) == Some(&b'{') => {
                    stack.push(JsFrame {
                        mode: JsMode::Code,
                        braces: 0,
                    });
                    prev = Prev::None;
                    i += 2;
                }
                _ => i += 1,
            },

            JsMode::JsxChildren => {
                if src[i] == b'<' && src.get(i + 1) == Some(&b'/') {
                    i = js_skip_to_tag_close(src, i + 2);
                    stack.pop();
                    prev = Prev::Value;
                    i += 1;
                } else if src[i] == b'<' && starts_jsx_element(&source[i + 1..]) {
                    stack.push(JsFrame {
                        mode: JsMode::JsxTag,
                        braces: 0,
                    });
                    i += 1;
                } else if src[i] == b'{' {
                    stack.push(JsFrame {
                        mode: JsMode::Code,
                        braces: 0,
                    });
                    prev = Prev::None;
                    i += 1;
                } else {
                    i += 1;
                }
            }

            JsMode::JsxTag => match src[i] {
                b'/' if src.get(i + 1) == Some(&b'>') => {
                    stack.pop();
                    prev = Prev::Value;
                    i += 2;
                }
                b'>' => {
                    stack.pop();
                    stack.push(JsFrame {
                        mode: JsMode::JsxChildren,
                        braces: 0,
                    });
                    prev = Prev::Value;
                    i += 1;
                }
                b'/' if src.get(i + 1) == Some(&b'/') => {
                    i = take_js_comment(src, i, source, opts, &mut spans)
                }
                b'/' if src.get(i + 1) == Some(&b'*') => {
                    i = take_js_comment(src, i, source, opts, &mut spans)
                }
                b'"' | b'\'' => {
                    i = skip_js_string(src, i, src[i]);
                    prev = Prev::Value;
                }
                b'`' => {
                    stack.push(JsFrame {
                        mode: JsMode::Template,
                        braces: 0,
                    });
                    prev = Prev::None;
                    i += 1;
                }
                b'{' => {
                    if let Some(frame) = stack.last_mut() {
                        frame.braces += 1;
                    }
                    prev = Prev::Operand;
                    i += 1;
                }
                b'}' => {
                    if let Some(frame) = stack.last_mut() {
                        frame.braces = frame.braces.saturating_sub(1);
                    }
                    prev = Prev::Value;
                    i += 1;
                }
                b'=' => {
                    prev = Prev::Operand;
                    i += 1;
                }
                _ => i += 1,
            },

            JsMode::Code => match src[i] {
                b'/' if src.get(i + 1) == Some(&b'/') => {
                    i = take_js_comment(src, i, source, opts, &mut spans);
                    prev = Prev::Value;
                }
                b'/' if src.get(i + 1) == Some(&b'*') => {
                    i = take_js_comment(src, i, source, opts, &mut spans);
                }
                b'"' | b'\'' => {
                    i = skip_js_string(src, i, src[i]);
                    prev = Prev::Value;
                }
                b'`' => {
                    stack.push(JsFrame {
                        mode: JsMode::Template,
                        braces: 0,
                    });
                    prev = Prev::None;
                    i += 1;
                }
                b'/' => {
                    if prev == Prev::Value {
                        prev = Prev::Operand;
                        i += 1;
                    } else {
                        i = skip_js_regex(src, i);
                        prev = Prev::Value;
                    }
                }
                b'<' if prev != Prev::Value && starts_jsx_element(&source[i + 1..]) => {
                    stack.push(JsFrame {
                        mode: JsMode::JsxTag,
                        braces: 0,
                    });
                    i += 1;
                }
                b'{' => {
                    if let Some(frame) = stack.last_mut() {
                        frame.braces += 1;
                    }
                    prev = Prev::Operand;
                    i += 1;
                }
                b'}' => {
                    let closes_interpolation =
                        stack.len() > 1 && stack.last().is_some_and(|f| f.braces == 0);
                    if let Some(frame) = stack.last_mut() {
                        frame.braces = frame.braces.saturating_sub(1);
                    }
                    if closes_interpolation {
                        stack.pop();
                    }
                    prev = Prev::Value;
                    i += 1;
                }
                c if is_js_word(c) => {
                    let (end, word) = scan_js_word(source, i);
                    prev = if is_regex_keyword(word) {
                        Prev::Operand
                    } else {
                        Prev::Value
                    };
                    i = end;
                }
                c if c.is_ascii_digit() => {
                    i = skip_js_number(src, i);
                    prev = Prev::Value;
                }
                b';' | b',' | b'(' | b'[' => {
                    prev = Prev::Operand;
                    i += 1;
                }
                b')' | b']' => {
                    prev = Prev::Value;
                    i += 1;
                }
                _ => {
                    prev = Prev::Operand;
                    i += 1;
                }
            },
        }
    }

    spans
}

/// take_js_comment records a `//` or `/* */` comment, unless it is a directive
/// the caller asked to keep, and returns the offset just past it.
fn take_js_comment(
    src: &[u8],
    start: usize,
    source: &str,
    opts: &Options,
    spans: &mut Vec<Span>,
) -> usize {
    let end = if src.get(start + 1) == Some(&b'/') {
        js_line_comment_end(src, start)
    } else {
        js_block_comment_end(src, start)
    };

    if opts.strip_directives || !is_js_directive(&source[start..end]) {
        spans.push(Span { start, end });
    }
    end
}

fn js_line_comment_end(src: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < src.len() && src[i] != b'\n' {
        i += 1;
    }
    i
}

fn js_block_comment_end(src: &[u8], start: usize) -> usize {
    let mut i = start + 2;
    while i + 1 < src.len() && !(src[i] == b'*' && src[i + 1] == b'/') {
        i += 1;
    }
    (i + 2).min(src.len())
}

fn skip_js_string(src: &[u8], open: usize, quote: u8) -> usize {
    let mut i = open + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'\n' => return i,
            c if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// skip_js_regex consumes a regex literal including its flags. An unterminated
/// one stops at the newline rather than swallowing the rest of the file.
fn skip_js_regex(src: &[u8], open: usize) -> usize {
    let mut i = open + 1;
    let mut in_class = false;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'\n' => return i,
            b'[' => {
                in_class = true;
                i += 1;
            }
            b']' => {
                in_class = false;
                i += 1;
            }
            b'/' if !in_class => {
                i += 1;
                while i < src.len() && src[i].is_ascii_alphabetic() {
                    i += 1;
                }
                return i;
            }
            _ => i += 1,
        }
    }
    i
}

fn skip_js_number(src: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < src.len() && (src[i].is_ascii_alphanumeric() || src[i] == b'.' || src[i] == b'_') {
        i += 1;
    }
    i
}

fn js_skip_to_tag_close(src: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < src.len() && src[i] != b'>' {
        i += 1;
    }
    i
}

fn is_js_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn scan_js_word(source: &str, start: usize) -> (usize, &str) {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && is_js_word(bytes[i]) {
        i += 1;
    }
    (i, &source[start..i])
}

fn is_regex_keyword(word: &str) -> bool {
    matches!(
        word,
        "return"
            | "typeof"
            | "instanceof"
            | "in"
            | "of"
            | "new"
            | "delete"
            | "void"
            | "case"
            | "do"
            | "else"
            | "yield"
            | "await"
            | "throw"
    )
}

/// starts_jsx_element decides whether a `<` opens JSX or is a comparison, a shift
/// or a TypeScript generic. An uppercase or `_`/`$` name, a closing tag, a
/// fragment, and the known HTML element names all count.
///
/// The bias is deliberate: a false positive only means a comment nearby is left
/// behind, while a false negative would let a `//` in JSX text be read as a
/// comment and take the rest of the line with it.
fn starts_jsx_element(rest: &str) -> bool {
    let bytes = rest.as_bytes();
    let Some(&first) = bytes.first() else {
        return false;
    };
    match first {
        b'/' | b'>' | b'_' | b'$' => return true,
        b'A'..=b'Z' => return true,
        b'a'..=b'z' => {}
        _ => return false,
    }

    let mut i = 0;
    while i < bytes.len() && is_js_word(bytes[i]) {
        i += 1;
    }
    is_html_element(&rest[..i])
}

fn is_html_element(name: &str) -> bool {
    HTML_ELEMENTS.binary_search(&name).is_ok()
}

/// The HTML element names, so `<p>` in a `.js` file reads as JSX while `<T>` in a
/// `.ts` file does not collide with a generic.
const HTML_ELEMENTS: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "base",
    "bdi",
    "bdo",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "mark",
    "menu",
    "meta",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "param",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "script",
    "search",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "title",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// is_js_directive keeps the comments that a build or a linter reads. Deleting
/// `// @ts-ignore` changes what compiles, so it is a directive and not commentary.
fn is_js_directive(text: &str) -> bool {
    let Some(first) = text.lines().next() else {
        return false;
    };
    let t = first.trim_start();

    t.starts_with("///")
        || t.starts_with("/*@")
        || t.starts_with("/* @")
        || t.starts_with("/* eslint")
        || t.starts_with("//#")
        || t.starts_with("//!")
        || t.starts_with("// rustfmt::")
        || t.starts_with("//@ts-")
        || t.starts_with("// @ts-")
        || t.starts_with("// @ts-check")
        || t.starts_with("// @flow")
        || t.starts_with("// biome-ignore")
        || t.starts_with("// c8 ignore")
        || t.starts_with("// istanbul ignore")
        || t.starts_with("// next-line")
        || t.starts_with("//#region")
        || t.starts_with("//#endregion")
        || is_named_directive(t, "eslint")
        || is_named_directive(t, "prettier-ignore")
        || is_named_directive(t, "vue")
        || is_named_directive(t, "stylelint")
}

fn is_named_directive(line: &str, name: &str) -> bool {
    line == format!("// {name}")
        || line.starts_with(&format!("// {name} "))
        || line.starts_with(&format!("// {name}:"))
        || line.starts_with(&format!("//{name} "))
}

// ---------------------------------------------------------------- SQL

/// PostgreSQL comments start with `--` or a (nestable) `/* */`, and none of
/// those sequences mean anything inside `'...'` or `"..."`.
///
/// A dollar-quoted body is not a single string to this pass: it is a function
/// body whose own quotes and comments still parse, so `$$ BEGIN -- why END $$`
/// loses the comment and keeps the words. Only the closing tag is special.
fn sql_comment_spans(src: &str) -> Vec<Span> {
    let bytes = src.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    let mut dollar_tag: Option<&str> = None;

    while i < bytes.len() {
        if let Some(tag) = dollar_tag {
            if src[i..].starts_with(tag) {
                i += tag.len();
                dollar_tag = None;
            } else {
                i = sql_code_step(src, i, &mut spans, &mut dollar_tag);
            }
            continue;
        }
        i = sql_code_step(src, i, &mut spans, &mut dollar_tag);
    }
    spans
}

/// sql_code_step handles one position of ordinary SQL: a comment, a quoted
/// literal, a dollar-quote open, or a plain byte.
fn sql_code_step<'a>(
    src: &'a str,
    mut i: usize,
    spans: &mut Vec<Span>,
    dollar_tag: &mut Option<&'a str>,
) -> usize {
    let bytes = src.as_bytes();
    if i >= bytes.len() {
        return i;
    }

    match bytes[i] {
        b'-' if bytes.get(i + 1) == Some(&b'-') => {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if !is_sql_directive(&src[start..i]) {
                spans.push(Span { start, end: i });
            }
            i
        }
        b'/' if bytes.get(i + 1) == Some(&b'*') => {
            let start = i;
            i = sql_block_end(bytes, i);
            spans.push(Span { start, end: i });
            i
        }
        b'\'' | b'"' => skip_sql_quoted(bytes, i, bytes[i]),
        b'$' => match dollar_tag_at(src, i) {
            Some(tag) => {
                *dollar_tag = Some(tag);
                i + tag.len()
            }
            None => i + 1,
        },
        _ => i + 1,
    }
}

/// is_sql_directive reports whether a `--` comment is a directive that a
/// migration runner reads rather than commentary.
///
/// golang-migrate and its forks split a file at `-- +migrate down` and decide
/// which half to run by that marker alone. Removing it silently folds the
/// rollback into the forward pass, so a migration that creates a function ends
/// up dropping it again before the next file can call it. Anything in the
/// `+migrate` namespace is kept for the same reason.
fn is_sql_directive(line: &str) -> bool {
    let rest = line.strip_prefix("--").unwrap_or(line);
    let rest = rest.trim_start();
    rest.starts_with("+migrate") || rest.starts_with("+goose")
}

fn sql_block_end(bytes: &[u8], open: usize) -> usize {
    let mut depth = 1usize;
    let mut i = open + 2;
    while i + 1 < bytes.len() {
        match (bytes[i], bytes[i + 1]) {
            (b'/', b'*') => {
                depth += 1;
                i += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return i;
                }
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// skip_sql_quoted returns the offset just past the closing quote. Only a
/// doubled quote escapes: with standard_conforming_strings on, a backslash is an
/// ordinary character, so `'C:\'` is the two-character path and not a string
/// that swallows the rest of the file.
///
/// A newline does *not* end the literal. Postgres lets a quoted string run over
/// several lines, and migration helpers rely on it to hold a formatted CREATE
/// TRIGGER across two lines. Bailing at the newline would hand the second half
/// to the code scanner and corrupt the surrounding statement.
fn skip_sql_quoted(src: &[u8], open: usize, quote: u8) -> usize {
    let mut i = open + 1;
    while i < src.len() {
        if src[i] == quote {
            if src.get(i + 1) == Some(&quote) {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    i
}

fn dollar_tag_at(src: &str, i: usize) -> Option<&str> {
    let rest = src.get(i..)?;
    if rest.starts_with("$$") {
        return Some("$$");
    }
    let body = rest.strip_prefix('$')?;
    let end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|end| *end > 0)?;
    if body.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    Some(&rest[..end + 2])
}

// ---------------------------------------------------------------- GraphQL

/// A GraphQL `#` is a comment only outside a string or a `"""` block string.
/// Descriptions are strings, not comments: they are part of the schema a client
/// introspects, so they stay.
fn graphql_comment_spans(src: &str) -> Vec<Span> {
    let bytes = src.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'#' => {
                let start = i;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                spans.push(Span { start, end: i });
            }
            b'"' => i = skip_graphql_string(src, i),
            _ => i += 1,
        }
    }
    spans
}

fn skip_graphql_string(src: &str, open: usize) -> usize {
    let rest = &src[open..];
    if rest.starts_with("\"\"\"") {
        let after_open = open + 3;
        return match src[after_open..].find("\"\"\"") {
            Some(p) => after_open + p + 3,
            None => src.len(),
        };
    }

    let bytes = src.as_bytes();
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            b'\n' => return i,
            _ => i += 1,
        }
    }
    i
}

// ---------------------------------------------------------------- line-oriented files

struct SourceLine {
    body: String,
    eol: String,
    dropped: bool,
}

fn strip_lines(source: &str, spans: Vec<Span>, opts: &Options) -> StripResult {
    let (mut lines, offsets) = split_lines(source);

    let mut per_line: Vec<Vec<Span>> = vec![Vec::new(); lines.len()];
    for sp in spans {
        if let Some(idx) = line_at(&offsets, sp.start) {
            per_line[idx].push(sp);
        }
    }

    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut origin: Vec<usize> = Vec::with_capacity(lines.len());
    let mut removed = 0usize;

    for i in 0..lines.len() {
        if per_line[i].is_empty() {
            out.push(source[offsets[i]..offsets[i + 1]].to_string());
            origin.push(i);
            continue;
        }
        removed += per_line[i].len();

        let text = cut(&lines[i], &per_line[i], offsets[i]);
        if text.trim().is_empty() && !lines[i].body.trim().is_empty() {
            lines[i].dropped = true;
            continue;
        }
        out.push(text + &lines[i].eol);
        origin.push(i);
    }

    StripResult {
        content: collapse(&lines, &out, &origin, opts.keep_blank_lines),
        comments: removed,
    }
}

fn cut(line: &SourceLine, spans: &[Span], offset: usize) -> String {
    let mut sorted: Vec<Span> = spans.to_vec();
    sorted.sort_by_key(|s| s.start);

    let mut out = String::with_capacity(line.body.len());
    let mut prev = 0usize;
    for sp in sorted {
        let from = (sp.start - offset).clamp(prev, line.body.len());
        let to = (sp.end - offset).clamp(from, line.body.len());
        out.push_str(&line.body[prev..from]);
        prev = to;
    }
    out.push_str(&line.body[prev..]);
    out.trim_end().to_string()
}

fn split_lines(source: &str) -> (Vec<SourceLine>, Vec<usize>) {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut offsets = vec![0usize];
    let mut start = 0usize;

    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' {
            let raw = &source[start..=i];
            let body_len = raw.trim_end_matches(['\r', '\n']).len();
            lines.push(SourceLine {
                body: raw[..body_len].to_string(),
                eol: raw[body_len..].to_string(),
                dropped: false,
            });
            offsets.push(i + 1);
            start = i + 1;
        }
    }
    if start < bytes.len() {
        lines.push(SourceLine {
            body: source[start..].to_string(),
            eol: String::new(),
            dropped: false,
        });
        offsets.push(bytes.len());
    }

    (lines, offsets)
}

fn line_at(offsets: &[usize], off: usize) -> Option<usize> {
    if offsets.is_empty() || off < offsets[0] {
        return None;
    }
    let idx = match offsets.binary_search(&off) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    if idx + 1 < offsets.len() {
        Some(idx)
    } else {
        None
    }
}

/// collapse holds the file to at most one blank line in a row and drops the blanks
/// a comment manufactured.
///
/// A run goes when the comment sat directly on both sides of it, because then
/// the gap was the comment's own separator rather than something the author
/// wrote. A blank the author put between two statements survives, because the
/// comment beside it describes the statement below and not the gap. A run with
/// only comments around it goes too, which is what a file's header and footer
/// blocks leave behind.
fn collapse(
    lines: &[SourceLine],
    out: &[String],
    origin: &[usize],
    keep_blank_lines: bool,
) -> String {
    let mut res: Vec<&str> = Vec::with_capacity(out.len());

    let mut i = 0;
    while i < out.len() {
        if !out[i].trim().is_empty() {
            res.push(&out[i]);
            i += 1;
            continue;
        }

        let mut j = i;
        while j < out.len() && out[j].trim().is_empty() {
            j += 1;
        }

        if keep_blank_lines || !blank_is_manufactured(lines, origin[i], origin[j - 1]) {
            // Keep the first line of the run verbatim: a blank line is its
            // newline, and pushing "" here would delete it altogether.
            res.push(&out[i]);
        }
        i = j;
    }

    res.concat()
}

fn blank_is_manufactured(lines: &[SourceLine], first: usize, last: usize) -> bool {
    let head = lines[..first].iter().filter(|l| l.dropped).count();
    let tail = lines[last + 1..].iter().filter(|l| l.dropped).count();
    if head == first && tail == lines.len() - 1 - last {
        return true;
    }

    first > 0 && last + 1 < lines.len() && lines[first - 1].dropped && lines[last + 1].dropped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options {
            keep_blank_lines: false,
            strip_directives: false,
        }
    }

    fn run(style: CommentStyle, src: &str) -> StripResult {
        strip(style, src, &opts())
    }

    #[test]
    fn go_keeps_a_slash_inside_a_raw_string() {
        let src = "var a = `SELECT 1 -- not a comment`\nvar b = 1\n";
        let r = run(CommentStyle::Go, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn go_keeps_a_slash_inside_an_interpreted_string() {
        let src = "var url = \"https://example.test/a\" // real comment\n";
        let r = run(CommentStyle::Go, src);
        assert_eq!(r.comments, 1);
        assert_eq!(r.content, "var url = \"https://example.test/a\"\n");
    }

    #[test]
    fn go_keeps_toolchain_directives() {
        let src = "//go:build tools\n\n//go:generate go run x\nvar a = 1 // drop me\n//nolint:unused\nvar b = 2\n";
        let r = run(CommentStyle::Go, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("//go:build tools"));
        assert!(r.content.contains("//go:generate go run x"));
        assert!(r.content.contains("//nolint:unused"));
        assert!(!r.content.contains("drop me"));
    }

    #[test]
    fn go_drops_directives_when_asked() {
        let src = "//go:build tools\n//nolint:unused\nvar a = 1\n";
        let r = strip(
            CommentStyle::Go,
            src,
            &Options {
                strip_directives: true,
                ..opts()
            },
        );
        assert_eq!(r.comments, 2);
        assert_eq!(r.content, "var a = 1\n");
    }

    #[test]
    fn go_keeps_a_trailing_comment_on_its_own_line() {
        let src = "func f() {\n\ta() // note\n\tb()\n}\n";
        let r = run(CommentStyle::Go, src);
        assert_eq!(r.content, "func f() {\n\ta()\n\tb()\n}\n");
    }

    #[test]
    fn braced_languages_drop_the_gap_a_comment_left_in_a_block() {
        let src = "function f() {\n    // step one\n    return 1;\n}\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.content, "function f() {\n    return 1;\n}\n");
    }

    #[test]
    fn braced_languages_drop_the_gap_before_a_closing_brace() {
        let src = "function f() {\n    return 1;\n    // trailing note\n}\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.content, "function f() {\n    return 1;\n}\n");
    }

    #[test]
    fn braced_languages_keep_a_blank_between_two_statements() {
        let src = "const a = 1;\n\nconst b = 2;\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.content, src);
    }

    // ------------------------------------------------------------ Rust

    #[test]
    fn rust_tells_a_lifetime_from_a_char_literal() {
        let src = "struct S<'a> {\n    f: &'a str,\n}\n\nlet c = 'x';\nlet s = 'static;\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn rust_strips_doc_and_line_comments() {
        let src = "//! Crate docs.\n\n/// Item docs.\npub fn f() {}\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 2);
        assert_eq!(r.content, "pub fn f() {}\n");
    }

    #[test]
    fn rust_keeps_attributes_because_they_are_not_comments() {
        let src = "#[derive(Debug)] // note\n#[doc = \"kept\"]\npub struct S;\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("#[derive(Debug)]"));
        assert!(r.content.contains("#[doc = \"kept\"]"));
    }

    #[test]
    fn rust_keeps_comment_markers_inside_a_raw_string() {
        let src = "const Q: &str = r#\"SELECT 1 -- not a comment\"#;\n// drop me\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("r#\"SELECT 1 -- not a comment\"#"));
        assert!(!r.content.contains("drop me"));
    }

    #[test]
    fn rust_handles_nested_block_comments() {
        let src = "/* outer /* inner */ still a comment */\nlet a = 1;\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 1);
        assert_eq!(r.content, "let a = 1;\n");
    }

    #[test]
    fn rust_keeps_a_raw_identifier_out_of_the_string_lexer() {
        let src = "fn r#match() {}\nlet a = 1; // drop me\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("fn r#match() {}"));
    }

    #[test]
    fn rust_keeps_rustfmt_skip() {
        let src = "// rustfmt::skip\nlet a   =   1;\n";
        let r = run(CommentStyle::Rust, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    // ------------------------------------------------------------ JS / TS / React

    #[test]
    fn js_keeps_a_url_inside_a_template_literal() {
        let src = "const u = `https://example.test/a`;\n// drop me\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("https://example.test/a"));
        assert!(!r.content.contains("drop me"));
    }

    #[test]
    fn js_keeps_a_template_interpolation_string_and_strips_inside_it() {
        let src = "const s = `${a /* drop me */}`;\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert_eq!(r.content, "const s = `${a}`;\n");
    }

    #[test]
    fn js_reads_a_regex_literal_as_a_literal() {
        let src = "const re = /a\\/\\/b/g; // drop me\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("/a\\/\\/b/g"));
        assert!(!r.content.contains("drop me"));
    }

    #[test]
    fn js_reads_a_slash_between_values_as_division() {
        let src = "const half = (total / 2) / 1; // drop me\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("total / 2"));
    }

    #[test]
    fn jsx_treats_a_double_slash_in_children_as_rendered_text() {
        let src = "export function P() {\n  return (\n    <p>\n      Visit https://example.test // real text\n    </p>\n  );\n}\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn jsx_strips_comments_inside_an_expression_container() {
        let src = "const a = <div className=\"x\" /* drop me */>{/* drop me too */1}</div>;\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 2);
        assert!(!r.content.contains("drop me"));
        assert!(r.content.contains("<div className=\"x\">{1}</div>"));
    }

    #[test]
    fn ts_generics_do_not_open_a_jsx_element() {
        let src =
            "const m = new Map<string, number>();\nconst n: Array<string> = [];\n// drop me\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("new Map<string, number>()"));
        assert!(r.content.contains("Array<string>"));
    }

    #[test]
    fn ts_keeps_a_comparison_operator() {
        let src = "if (a < b && c > d) {\n  run();\n}\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn js_keeps_linter_directives() {
        let src = "// @ts-expect-error legacy\nexport const a = 1; // drop me\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("@ts-expect-error"));
        assert!(!r.content.contains("drop me"));
    }

    #[test]
    fn js_keeps_the_jsx_import_source_pragma() {
        let src = "/* @jsxImportSource x */\nexport const a = 1;\n";
        let r = run(CommentStyle::JavaScript, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn react_component_survives_a_full_strip() {
        let src = concat!(
            "// Page header.\n",
            "import { useState } from 'react';\n",
            "\n",
            "/** The doc block. */\n",
            "export function Counter({ start = 0 }) {\n",
            "  // the value lives in state\n",
            "  const [n, setN] = useState(start); // trailing note\n",
            "  return (\n",
            "    <button onClick={() => setN(n + 1)}>\n",
            "      {n} // shown to the user\n",
            "    </button>\n",
            "  );\n",
            "}\n",
        );
        let r = run(CommentStyle::JavaScript, src);
        // Page header, the doc block, the state note and the trailing note. The
        // fifth `//` is JSX text, so it is not a comment at all.
        assert_eq!(r.comments, 4);
        assert!(!r.content.contains("Page header"));
        assert!(!r.content.contains("doc block"));
        assert!(!r.content.contains("lives in state"));
        assert!(!r.content.contains("trailing note"));
        // JSX children are text, so the label survives.
        assert!(r.content.contains("{n} // shown to the user"));
        assert!(r
            .content
            .contains("export function Counter({ start = 0 }) {"));
    }

    // ------------------------------------------------------------ SQL / GraphQL

    #[test]
    fn sql_keeps_a_dash_inside_a_string_and_strips_one_in_a_dollar_body() {
        let src = "SELECT '-- not a comment';\nCREATE FUNCTION f() RETURNS void AS $$\n-- real comment\nBEGIN\nEND;\n$$;\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("'-- not a comment'"));
        assert!(!r.content.contains("real comment"));
    }

    #[test]
    fn sql_keeps_a_quoted_tag_inside_a_dollar_body() {
        let src =
            "CREATE FUNCTION f() RETURNS void AS $body$\n-- drop me\nRETURN '$body$';\n$body$;\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 1);
        assert!(r.content.contains("RETURN '$body$'"));
        assert!(r.content.contains("$body$;"));
    }

    #[test]
    fn sql_keeps_a_string_that_runs_across_lines_inside_a_dollar_body() {
        // A quoted string may span lines in Postgres, and migration helpers use
        // that to hold a formatted CREATE TRIGGER over two lines. Treating the
        // newline as the end of the literal would corrupt the function.
        let src =
            "CREATE OR REPLACE FUNCTION attach_updated_at(target_table TEXT) RETURNS VOID AS $$\n\
             BEGIN\n\
             \x20   EXECUTE format(\n\
             \x20       'CREATE TRIGGER %I_set_updated_at BEFORE UPDATE ON %I\n\
             \x20        FOR EACH ROW EXECUTE FUNCTION set_updated_at()',\n\
             \x20       target_table, target_table\n\
             \x20   );\n\
             END;\n\
             $$ LANGUAGE plpgsql;\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 0);
        assert_eq!(r.content, src);
    }

    #[test]
    fn sql_strips_a_comment_after_a_string_that_ran_across_lines() {
        // Same shape, but a real comment follows. Ending the literal at the
        // newline would leave the comment looking like code and drop the
        // closing quote of the function.
        let src = "SELECT 'one\ntwo';\n-- drop me\nSELECT 2;\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 1);
        assert_eq!(r.content, "SELECT 'one\ntwo';\nSELECT 2;\n");
    }

    #[test]
    fn sql_keeps_the_migrate_down_marker() {
        // The runner splits the file on this marker, so stripping it would fold
        // the rollback into the forward pass.
        let src = "CREATE FUNCTION f() RETURNS void AS $$ BEGIN END; $$;\n-- +migrate down\nDROP FUNCTION IF EXISTS f();\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 0);
        assert!(r.content.contains("-- +migrate down"));
    }

    #[test]
    fn sql_strips_a_comment_next_to_a_kept_migrate_marker() {
        // The kept marker must not drag its neighbouring comment along with it.
        let src = "SELECT 1;\n-- drop me\n-- +migrate down\nDROP FUNCTION f();\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 1);
        assert_eq!(
            r.content,
            "SELECT 1;\n-- +migrate down\nDROP FUNCTION f();\n"
        );
    }

    #[test]
    fn sql_keeps_the_blank_between_two_statements() {
        let src = "SELECT 1;\n\n-- describes the next statement\nCREATE TABLE t (id INT);\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.content, "SELECT 1;\n\nCREATE TABLE t (id INT);\n");
    }

    #[test]
    fn sql_drops_the_blank_that_sat_between_two_comments() {
        // The gap separated the comments, so removing them removes the gap.
        let src = "CREATE TABLE t (\n    -- the id\n\n    -- the name\n    name TEXT\n);\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 2);
        assert_eq!(r.content, "CREATE TABLE t (\n    name TEXT\n);\n");
    }

    #[test]
    fn sql_keeps_the_blank_an_author_used_to_group_columns() {
        // Here the blank is the author's own grouping, so it stays even though
        // comments sat above and below it.
        let src =
            "CREATE TABLE t (\n    -- the id\n    id INT,\n\n    -- the name\n    name TEXT\n);\n";
        let r = run(CommentStyle::Sql, src);
        assert_eq!(r.comments, 2);
        assert_eq!(
            r.content,
            "CREATE TABLE t (\n    id INT,\n\n    name TEXT\n);\n"
        );
    }

    #[test]
    fn graphql_keeps_descriptions_and_drops_hashes() {
        let src = "# header\ntype T {\n    \"A field. # not a comment\"\n    id: ID!\n}\n";
        let r = run(CommentStyle::GraphQl, src);
        assert_eq!(r.comments, 1);
        assert_eq!(
            r.content,
            "type T {\n    \"A field. # not a comment\"\n    id: ID!\n}\n"
        );
    }

    // ------------------------------------------------------------ selection

    #[test]
    fn the_html_table_is_sorted_so_the_binary_search_is_valid() {
        let mut sorted = HTML_ELEMENTS.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, HTML_ELEMENTS, "HTML_ELEMENTS must stay sorted");
    }

    #[test]
    fn jsx_detection_accepts_real_tags_and_rejects_operators() {
        assert!(starts_jsx_element("div className=\"x\">"));
        assert!(starts_jsx_element("/div>"));
        assert!(starts_jsx_element(">"));
        assert!(starts_jsx_element("Counter />"));
        assert!(!starts_jsx_element("= b"));
        assert!(!starts_jsx_element("divisor)"));
        assert!(!starts_jsx_element("2"));
    }

    #[test]
    fn resolve_keeps_a_lang_narrow_to_its_own_extensions() {
        let s = resolve(&["ts".to_string()]).expect("ts resolves");
        assert_eq!(s.styles, vec![CommentStyle::JavaScript]);
        assert!(s.extensions.contains(&"tsx".to_string()));
        assert!(!s.extensions.contains(&"js".to_string()));
        assert!(style_for(Path::new("a/b.tsx"), &s).is_some());
        assert!(style_for(Path::new("a/b.js"), &s).is_none());
    }

    #[test]
    fn resolve_maps_react_to_the_whole_js_family() {
        let s = resolve(&["react".to_string()]).expect("react resolves");
        for name in ["a.jsx", "a.tsx", "a.js", "a.ts"] {
            assert_eq!(
                style_for(Path::new(name), &s),
                Some(CommentStyle::JavaScript),
                "{name} should be selected"
            );
        }
    }

    #[test]
    fn resolve_combines_names_and_rejects_the_unknown() {
        let s = resolve(&["go,rust".to_string(), "sql".to_string()]).expect("combined");
        assert_eq!(s.styles.len(), 3);
        assert!(resolve(&["cobol".to_string()]).is_err());
        assert!(resolve(&[]).is_err());
    }
}
