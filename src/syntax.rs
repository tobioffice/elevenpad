//! Lightweight syntax highlighting for the editor.
//!
//! A small hand-written tokenizer: comments, strings, numbers, identifiers,
//! keywords and types. No parser, no dependencies, bounded work per keystroke.

use egui::{Color32, FontId, TextFormat, text::LayoutJob};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    None,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    C,
    Cpp,
    Go,
    Java,
    Json,
    Toml,
    Yaml,
    Shell,
    Sql,
}

pub fn detect(path: Option<&Path>) -> Lang {
    let Some(path) = path else {
        return Lang::None;
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "rs" => Lang::Rust,
        "py" | "pyi" | "pyw" => Lang::Python,
        "js" | "mjs" | "cjs" => Lang::JavaScript,
        "ts" | "tsx" | "jsx" => Lang::TypeScript,
        "c" | "h" => Lang::C,
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Lang::Cpp,
        "go" => Lang::Go,
        "java" => Lang::Java,
        "json" => Lang::Json,
        "toml" => Lang::Toml,
        "yaml" | "yml" => Lang::Yaml,
        "sh" | "bash" | "zsh" | "fish" => Lang::Shell,
        "sql" => Lang::Sql,
        _ => Lang::None,
    }
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub keyword: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub number: Color32,
    pub function: Color32,
    pub ty: Color32,
}

impl Colors {
    pub const fn light() -> Self {
        Self {
            keyword: Color32::from_rgb(0x00, 0x00, 0xC0),
            string: Color32::from_rgb(0xA3, 0x15, 0x15),
            comment: Color32::from_rgb(0x00, 0x80, 0x00),
            number: Color32::from_rgb(0x09, 0x86, 0x58),
            function: Color32::from_rgb(0x79, 0x5E, 0x26),
            ty: Color32::from_rgb(0x26, 0x7F, 0x99),
        }
    }

    pub const fn dark() -> Self {
        Self {
            keyword: Color32::from_rgb(0x56, 0x9C, 0xD6),
            string: Color32::from_rgb(0xCE, 0x91, 0x78),
            comment: Color32::from_rgb(0x6A, 0x99, 0x55),
            number: Color32::from_rgb(0xB5, 0xCE, 0xA8),
            function: Color32::from_rgb(0xDC, 0xDC, 0xAA),
            ty: Color32::from_rgb(0x4E, 0xC9, 0xB0),
        }
    }
}

const MAX_HIGHLIGHT_BYTES: usize = 300_000;

pub fn highlight(
    text: &str,
    lang: Lang,
    font: FontId,
    base: Color32,
    colors: Colors,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.break_on_newline = true;

    let plain = |color: Color32| TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };

    if lang == Lang::None || text.len() > MAX_HIGHLIGHT_BYTES {
        job.append(text, 0.0, plain(base));
        return job;
    }

    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut flush = 0usize;

    macro_rules! emit {
        ($start:expr, $end:expr, $color:expr) => {{
            if flush < $start {
                job.append(&text[flush..$start], 0.0, plain(base));
            }
            job.append(&text[$start..$end], 0.0, plain($color));
            flush = $end;
        }};
    }

    while i < bytes.len() {
        // comments first
        if let Some(end) = comment_end(bytes, i, lang) {
            emit!(i, end, colors.comment);
            i = end;
            continue;
        }

        let c = bytes[i];

        // strings
        if c == b'"' || c == b'`' || (c == b'\'' && lang != Lang::Rust) {
            if let Some(end) = string_end(bytes, i, c) {
                let mut color = colors.string;
                if lang == Lang::Json && next_non_space(bytes, end) == Some(b':') {
                    color = colors.function;
                }
                emit!(i, end, color);
                i = end;
                continue;
            }
        } else if c == b'\'' && lang == Lang::Rust {
            // char literal, not a lifetime
            if let Some(end) = char_literal_end(bytes, i) {
                emit!(i, end, colors.string);
                i = end;
                continue;
            }
        }

        // numbers
        if c.is_ascii_digit() {
            let end = number_end(bytes, i);
            emit!(i, end, colors.number);
            i = end;
            continue;
        }

        // identifiers
        if is_ident_start(c) {
            let start = i;
            let end = ident_end(bytes, i);
            let word = &text[start..end];
            let after = next_non_space(bytes, end);
            let color = if keywords(lang).contains(&word) {
                Some(colors.keyword)
            } else if after == Some(b'(') {
                Some(colors.function)
            } else if is_type_name(word, lang) {
                Some(colors.ty)
            } else {
                None
            };
            if let Some(color) = color {
                emit!(start, end, color);
            }
            i = end;
            continue;
        }

        // advance one UTF-8 character
        i += utf8_len(c);
    }

    if flush < text.len() {
        job.append(&text[flush..], 0.0, plain(base));
    }
    job
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

fn is_ident_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn ident_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
        i += 1;
    }
    i
}

fn next_non_space(bytes: &[u8], mut i: usize) -> Option<u8> {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    bytes.get(i).copied()
}

fn comment_end(bytes: &[u8], i: usize, lang: Lang) -> Option<usize> {
    let line_comment = match lang {
        Lang::Python | Lang::Shell | Lang::Toml | Lang::Yaml => bytes[i] == b'#',
        Lang::Sql => bytes.len() > i + 1 && &bytes[i..i + 2] == b"--",
        Lang::Rust
        | Lang::JavaScript
        | Lang::TypeScript
        | Lang::C
        | Lang::Cpp
        | Lang::Go
        | Lang::Java => bytes.len() > i + 1 && &bytes[i..i + 2] == b"//",
        _ => false,
    };
    if line_comment {
        let mut j = i;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        return Some(j);
    }
    if matches!(
        lang,
        Lang::Rust | Lang::JavaScript | Lang::TypeScript | Lang::C | Lang::Cpp | Lang::Go | Lang::Java | Lang::Sql
    ) && bytes.len() > i + 1
        && &bytes[i..i + 2] == b"/*"
    {
        let mut j = i + 2;
        while j + 1 < bytes.len() {
            if &bytes[j..j + 2] == b"*/" {
                return Some(j + 2);
            }
            j += 1;
        }
        return Some(bytes.len());
    }
    None
}

fn string_end(bytes: &[u8], start: usize, quote: u8) -> Option<usize> {
    let multiline = quote == b'`';
    let mut i = start + 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            i += 2;
            continue;
        }
        if b == quote {
            return Some(i + 1);
        }
        if b == b'\n' && !multiline {
            return Some(i);
        }
        i += 1;
    }
    Some(bytes.len())
}

fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let limit = (start + 6).min(bytes.len());
    while i < limit {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'\'' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn is_type_name(word: &str, lang: Lang) -> bool {
    if !matches!(
        lang,
        Lang::Rust | Lang::C | Lang::Cpp | Lang::Java | Lang::Go | Lang::TypeScript
    ) {
        return false;
    }
    word.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

fn keywords(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::Rust => &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while", "None", "Some", "Ok",
            "Err",
        ],
        Lang::Python => &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
            "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return",
            "self", "True", "try", "while", "with", "yield",
        ],
        Lang::JavaScript | Lang::TypeScript => &[
            "async", "await", "break", "case", "catch", "class", "const", "continue", "default",
            "delete", "do", "else", "export", "extends", "false", "finally", "for", "from",
            "function", "if", "import", "in", "instanceof", "interface", "let", "new", "null",
            "of", "private", "protected", "public", "readonly", "return", "static", "super",
            "switch", "this", "throw", "true", "try", "type", "typeof", "undefined", "var",
            "void", "while", "yield",
        ],
        Lang::C | Lang::Cpp => &[
            "auto", "break", "case", "char", "class", "const", "continue", "default", "delete",
            "do", "double", "else", "enum", "extern", "false", "float", "for", "if", "inline",
            "int", "long", "namespace", "new", "nullptr", "private", "protected", "public",
            "return", "short", "signed", "sizeof", "static", "struct", "switch", "template",
            "this", "throw", "true", "try", "typedef", "typename", "union", "unsigned", "using",
            "virtual", "void", "volatile", "while",
        ],
        Lang::Go => &[
            "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
            "false", "for", "func", "go", "goto", "if", "import", "interface", "map", "nil",
            "package", "range", "return", "select", "struct", "switch", "true", "type", "var",
        ],
        Lang::Java => &[
            "abstract", "boolean", "break", "case", "catch", "char", "class", "continue",
            "default", "do", "double", "else", "enum", "extends", "false", "final", "finally",
            "float", "for", "if", "implements", "import", "int", "interface", "long", "new",
            "null", "package", "private", "protected", "public", "return", "short", "static",
            "super", "switch", "this", "throw", "throws", "true", "try", "void", "while",
        ],
        Lang::Json => &["true", "false", "null"],
        Lang::Toml | Lang::Yaml => &["true", "false", "null"],
        Lang::Shell => &[
            "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if",
            "in", "local", "readonly", "return", "set", "then", "unset", "while",
        ],
        Lang::Sql => &[
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
            "CREATE", "TABLE", "ALTER", "DROP", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON",
            "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "AS", "AND", "OR", "NOT",
            "NULL", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
        ],
        Lang::None => &[],
    }
}
