//! Text buffer, cursor & selection models, undo/redo history, and syntax tokenization.

use std::cmp::{max, min};

/// 0-indexed position within a text buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

impl TextPosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Selection spanning an anchor position to a head (active cursor) position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: TextPosition,
    pub head: TextPosition,
}

impl TextSelection {
    pub fn new(anchor: TextPosition, head: TextPosition) -> Self {
        Self { anchor, head }
    }

    pub fn start(&self) -> TextPosition {
        min(self.anchor, self.head)
    }

    pub fn end(&self) -> TextPosition {
        max(self.anchor, self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// Atomic edit action stored in the undo/redo history.
#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    Insert {
        pos: TextPosition,
        text: String,
    },
    Delete {
        start: TextPosition,
        end: TextPosition,
        deleted_text: String,
    },
}

/// Line-indexed text buffer supporting editing, selection, undo/redo, and search.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    lines: Vec<String>,
    cursor: TextPosition,
    selection: Option<TextSelection>,
    undo_stack: Vec<EditAction>,
    redo_stack: Vec<EditAction>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for TextBuffer {
    fn from(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        Self {
            lines,
            cursor: TextPosition::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

impl std::str::FromStr for TextBuffer {
    type Err = std::convert::Infallible;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(text))
    }
}

impl std::fmt::Display for TextBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: TextPosition::default(),
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_text(text: &str) -> Self {
        Self::from(text)
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn cursor(&self) -> TextPosition {
        self.cursor
    }

    pub fn selection(&self) -> Option<TextSelection> {
        self.selection
    }

    pub fn set_cursor(&mut self, pos: TextPosition) {
        self.cursor = self.clamp_position(pos);
        self.selection = None;
    }

    fn clamp_position(&self, pos: TextPosition) -> TextPosition {
        let line = min(pos.line, self.lines.len().saturating_sub(1));
        let max_col = self.lines.get(line).map(|l| l.len()).unwrap_or(0);
        let column = min(pos.column, max_col);
        TextPosition::new(line, column)
    }

    // -----------------------------------------------------------------------
    // Editing operations
    // -----------------------------------------------------------------------

    pub fn insert_char(&mut self, c: char) {
        if c == '\n' {
            self.insert_newline();
        } else {
            self.insert_text(&c.to_string());
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.delete_selection();
        let pos = self.cursor;

        if let Some(line) = self.lines.get_mut(pos.line) {
            line.insert_str(pos.column, text);
            self.cursor.column += text.len();
        }

        self.undo_stack.push(EditAction::Insert {
            pos,
            text: text.to_string(),
        });
        self.redo_stack.clear();
    }

    pub fn insert_newline(&mut self) {
        self.delete_selection();
        let pos = self.cursor;
        let current_line = &self.lines[pos.line];
        let rest = current_line[pos.column..].to_string();

        self.lines[pos.line].truncate(pos.column);
        self.lines.insert(pos.line + 1, rest);
        self.cursor = TextPosition::new(pos.line + 1, 0);

        self.undo_stack.push(EditAction::Insert {
            pos,
            text: "\n".to_string(),
        });
        self.redo_stack.clear();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor.column > 0 {
            let pos = TextPosition::new(self.cursor.line, self.cursor.column - 1);
            let deleted_char = self.lines[pos.line].remove(pos.column);
            self.cursor.column -= 1;
            self.undo_stack.push(EditAction::Delete {
                start: pos,
                end: TextPosition::new(pos.line, pos.column + 1),
                deleted_text: deleted_char.to_string(),
            });
            self.redo_stack.clear();
        } else if self.cursor.line > 0 {
            let prev_line_idx = self.cursor.line - 1;
            let prev_len = self.lines[prev_line_idx].len();
            let curr_line = self.lines.remove(self.cursor.line);
            self.lines[prev_line_idx].push_str(&curr_line);
            self.cursor = TextPosition::new(prev_line_idx, prev_len);

            self.undo_stack.push(EditAction::Delete {
                start: self.cursor,
                end: TextPosition::new(self.cursor.line + 1, 0),
                deleted_text: "\n".to_string(),
            });
            self.redo_stack.clear();
        }
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some(sel) = self.selection.take() {
            if !sel.is_empty() {
                let start = sel.start();
                let end = sel.end();
                let deleted = self.extract_range(start, end);

                if start.line == end.line {
                    self.lines[start.line].drain(start.column..end.column);
                } else {
                    let tail = self.lines[end.line][end.column..].to_string();
                    self.lines[start.line].truncate(start.column);
                    self.lines[start.line].push_str(&tail);
                    self.lines.drain((start.line + 1)..=end.line);
                }

                self.cursor = start;
                self.undo_stack.push(EditAction::Delete {
                    start,
                    end,
                    deleted_text: deleted,
                });
                self.redo_stack.clear();
                return true;
            }
        }
        false
    }

    fn extract_range(&self, start: TextPosition, end: TextPosition) -> String {
        if start.line == end.line {
            self.lines[start.line][start.column..end.column].to_string()
        } else {
            let mut parts = Vec::new();
            parts.push(self.lines[start.line][start.column..].to_string());
            for l in (start.line + 1)..end.line {
                parts.push(self.lines[l].clone());
            }
            parts.push(self.lines[end.line][..end.column].to_string());
            parts.join("\n")
        }
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action {
                EditAction::Insert { pos, text } => {
                    let end_pos = if text == "\n" {
                        TextPosition::new(pos.line + 1, 0)
                    } else {
                        TextPosition::new(pos.line, pos.column + text.len())
                    };
                    self.selection = Some(TextSelection::new(pos, end_pos));
                    self.delete_selection();
                    self.cursor = pos;
                    self.redo_stack.push(EditAction::Insert { pos, text });
                }
                EditAction::Delete { start, deleted_text, .. } => {
                    self.cursor = start;
                    self.insert_text(&deleted_text);
                    self.redo_stack.push(EditAction::Delete {
                        start,
                        end: self.cursor,
                        deleted_text,
                    });
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Bracket Matching
    // -----------------------------------------------------------------------

    pub fn find_matching_bracket(&self) -> Option<TextPosition> {
        let line = self.lines.get(self.cursor.line)?;
        let col = self.cursor.column;
        let c = line.chars().nth(col)?;

        let (open, close, forward) = match c {
            '(' => ('(', ')', true),
            '{' => ('{', '}', true),
            '[' => ('[', ']', true),
            ')' => ('(', ')', false),
            '}' => ('{', '}', false),
            ']' => ('[', ']', false),
            _ => return None,
        };

        if forward {
            let mut depth = 0;
            for l in self.cursor.line..self.lines.len() {
                let start_c = if l == self.cursor.line { col } else { 0 };
                for (c_idx, ch) in self.lines[l].chars().enumerate().skip(start_c) {
                    if ch == open {
                        depth += 1;
                    } else if ch == close {
                        depth -= 1;
                        if depth == 0 {
                            return Some(TextPosition::new(l, c_idx));
                        }
                    }
                }
            }
        } else {
            let mut depth = 0;
            for l in (0..=self.cursor.line).rev() {
                let end_c = if l == self.cursor.line { col + 1 } else { usize::MAX };
                let chars: Vec<(usize, char)> = self.lines[l]
                    .chars()
                    .enumerate()
                    .take(end_c)
                    .collect();
                for (c_idx, ch) in chars.into_iter().rev() {
                    if ch == close {
                        depth += 1;
                    } else if ch == open {
                        depth -= 1;
                        if depth == 0 {
                            return Some(TextPosition::new(l, c_idx));
                        }
                    }
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // In-buffer Search & Replace
    // -----------------------------------------------------------------------

    pub fn search(&self, query: &str) -> Vec<TextPosition> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        for (l_idx, line) in self.lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(query) {
                results.push(TextPosition::new(l_idx, start + pos));
                start += pos + query.len();
            }
        }
        results
    }
}

// ---------------------------------------------------------------------------
// Syntax Tokenization Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTokenType {
    Keyword,
    String,
    Number,
    Punctuation,
    Comment,
    Identifier,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxToken {
    pub token_type: SyntaxTokenType,
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

pub fn tokenize_json(text: &str) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let mut in_string = false;
        let mut string_start = 0;

        for (col, c) in line.chars().enumerate() {
            if c == '"' {
                if in_string {
                    tokens.push(SyntaxToken {
                        token_type: SyntaxTokenType::String,
                        line: line_idx,
                        start_col: string_start,
                        end_col: col + 1,
                    });
                    in_string = false;
                } else {
                    in_string = true;
                    string_start = col;
                }
            } else if !in_string
                && (c == '{' || c == '}' || c == '[' || c == ']' || c == ':' || c == ',')
            {
                tokens.push(SyntaxToken {
                    token_type: SyntaxTokenType::Punctuation,
                    line: line_idx,
                    start_col: col,
                    end_col: col + 1,
                });
            }
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_buffer_insert_and_backspace() {
        let mut buffer = TextBuffer::new();
        buffer.insert_text("hello");
        assert_eq!(buffer.to_string(), "hello");
        assert_eq!(buffer.cursor(), TextPosition::new(0, 5));

        buffer.backspace();
        assert_eq!(buffer.to_string(), "hell");
    }

    #[test]
    fn test_bracket_matching() {
        let buffer = TextBuffer::from("{\"key\": [1, 2]}");
        let mut b2 = buffer.clone();
        b2.set_cursor(TextPosition::new(0, 0)); // cursor at '{'
        assert_eq!(b2.find_matching_bracket(), Some(TextPosition::new(0, 14)));

        b2.set_cursor(TextPosition::new(0, 8)); // cursor at '['
        assert_eq!(b2.find_matching_bracket(), Some(TextPosition::new(0, 13)));

        b2.set_cursor(TextPosition::new(0, 14)); // cursor at '}'
        assert_eq!(b2.find_matching_bracket(), Some(TextPosition::new(0, 0)));

        b2.set_cursor(TextPosition::new(0, 13)); // cursor at ']'
        assert_eq!(b2.find_matching_bracket(), Some(TextPosition::new(0, 8)));
    }

    #[test]
    fn test_search() {
        let buffer = TextBuffer::from("apple banana apple");
        let matches = buffer.search("apple");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], TextPosition::new(0, 0));
        assert_eq!(matches[1], TextPosition::new(0, 13));
    }
}
