//! Text editing, cursor models, undo/redo, bracket matching, and syntax tokenization.

pub mod buffer;

pub use buffer::{
    tokenize_json, EditAction, SyntaxToken, SyntaxTokenType, TextBuffer, TextPosition,
    TextSelection,
};
