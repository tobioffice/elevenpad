//! Document model: text, file IO, line endings, undo history.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Crlf,
    Lf,
}

impl Eol {
    pub fn label(self) -> &'static str {
        match self {
            Eol::Crlf => "Windows (CRLF)",
            Eol::Lf => "Unix (LF)",
        }
    }
}

pub struct Doc {
    pub path: Option<PathBuf>,
    pub text: String,
    pub modified: bool,
    pub eol: Eol,
    pub bom: bool,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
    pub scroll_to_line: Option<usize>,
    pub scroll_offset: Option<f32>,
}

impl Doc {
    pub fn new() -> Self {
        Self {
            path: None,
            text: String::new(),
            modified: false,
            eol: Eol::Crlf,
            bom: false,
            undo: Vec::new(),
            redo: Vec::new(),
            scroll_to_line: None,
            scroll_offset: None,
        }
    }

    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let bytes = std::fs::read(&path)?;
        let bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
        let body = if bom { &bytes[3..] } else { &bytes[..] };
        let text = String::from_utf8_lossy(body).replace("\r\n", "\n");
        let eol = if bytes.windows(2).any(|w| w == b"\r\n") {
            Eol::Crlf
        } else {
            Eol::Lf
        };
        Ok(Self {
            path: Some(path),
            text,
            modified: false,
            eol,
            bom,
            undo: Vec::new(),
            redo: Vec::new(),
            scroll_to_line: None,
            scroll_offset: None,
        })
    }

    pub fn save_to(&mut self, path: PathBuf) -> std::io::Result<()> {
        let body = match self.eol {
            Eol::Crlf => self.text.replace('\n', "\r\n"),
            Eol::Lf => self.text.clone(),
        };
        let mut out = Vec::with_capacity(body.len() + 3);
        if self.bom {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
        }
        out.extend_from_slice(body.as_bytes());
        std::fs::write(&path, out)?;
        self.path = Some(path);
        self.modified = false;
        Ok(())
    }

    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".into()),
            None => "Untitled".into(),
        }
    }

    /// Push a snapshot before an edit; called only when the text actually changed.
    pub fn snapshot(&mut self, previous: String) {
        if self.undo.last().map(|s| s == &previous).unwrap_or(false) {
            return;
        }
        self.undo.push(previous);
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.text, previous));
            self.modified = true;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.text, next));
            self.modified = true;
            true
        } else {
            false
        }
    }

    pub fn line_col(&self, char_index: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.text.chars().enumerate() {
            if i >= char_index {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    pub fn char_index_of_line(&self, line: usize) -> usize {
        let mut current = 1;
        for (i, ch) in self.text.chars().enumerate() {
            if current >= line {
                return i;
            }
            if ch == '\n' {
                current += 1;
            }
        }
        self.text.chars().count()
    }

    pub fn line_count(&self) -> usize {
        self.text.chars().filter(|c| *c == '\n').count() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_roundtrip() {
        let dir = std::env::temp_dir().join("notepad-test-crlf.txt");
        std::fs::write(&dir, b"one\r\ntwo\r\n").unwrap();
        let mut doc = Doc::from_file(dir.clone()).unwrap();
        assert_eq!(doc.text, "one\ntwo\n");
        assert_eq!(doc.eol, Eol::Crlf);
        doc.text.push_str("three\n");
        doc.save_to(dir.clone()).unwrap();
        let bytes = std::fs::read(&dir).unwrap();
        assert_eq!(bytes, b"one\r\ntwo\r\nthree\r\n");
        std::fs::remove_file(&dir).ok();
    }

    #[test]
    fn line_col_and_goto() {
        let mut doc = Doc::new();
        doc.text = "abc\ndef\nghi".into();
        assert_eq!(doc.line_col(0), (1, 1));
        assert_eq!(doc.line_col(4), (2, 1));
        assert_eq!(doc.line_col(9), (3, 2));
        assert_eq!(doc.char_index_of_line(2), 4);
        assert_eq!(doc.line_count(), 3);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut doc = Doc::new();
        doc.text = "hello".into();
        doc.snapshot("hell".into());
        assert!(doc.undo());
        assert_eq!(doc.text, "hell");
        assert!(doc.redo());
        assert_eq!(doc.text, "hello");
    }
}
