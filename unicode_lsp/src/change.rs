use tower_lsp::lsp_types;

use crate::encoding::Encoding;

pub(crate) struct Change {
    pub(crate) range: Range,
    pub(crate) new_content: String,
}

pub(crate) struct Range {
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Range {
    pub(crate) fn from_lsp_range(lsp_range: lsp_types::Range, pos_encoding: Encoding) -> Self {
        let lsp_start = lsp_range.start;
        let lsp_end = lsp_range.end;
        Self {
            start: Position {
                line: lsp_start.line as usize,
                codeunit: CodeUnitIndex::new(pos_encoding, lsp_start.character as usize),
            },
            end: Position {
                line: lsp_end.line as usize,
                codeunit: CodeUnitIndex::new(pos_encoding, lsp_end.character as usize),
            },
        }
    }
}

pub(crate) struct Position {
    pub(crate) line: usize,
    pub(crate) codeunit: CodeUnitIndex,
}

pub(crate) enum CodeUnitIndex {
    Utf8(usize),
    Utf16(usize),
    Utf32(usize),
}
impl CodeUnitIndex {
    fn new(pos_encoding: Encoding, i: usize) -> CodeUnitIndex {
        match pos_encoding {
            Encoding::Utf8 => CodeUnitIndex::Utf8(i),
            Encoding::Utf16 => CodeUnitIndex::Utf16(i),
            Encoding::Utf32 => CodeUnitIndex::Utf32(i),
        }
    }
}
