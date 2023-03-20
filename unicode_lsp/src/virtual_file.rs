use std::cmp::min;

use ropey::Rope;

use crate::change::{Change, Position};

#[derive(Clone, Debug)]
pub(crate) struct VirtualFile {
    pub(crate) content: Rope,
    // TODO: This makes rust-analyzer panic without the brace
}

impl VirtualFile {
    pub(crate) fn new(content: &str) -> Self {
        Self {
            content: Rope::from_str(content),
        }
    }
    pub(crate) fn apply_changes(&mut self, changes: Vec<Change>) {
        for Change { range, new_content } in changes {
            let Some(start_id) = self.get_index_of(range.start) else {
                return;
            };
            let Some(end_id) = self.get_index_of(range.end) else {
                return;
            };

            self.content.remove(start_id..end_id);
            self.content.insert(start_id, &new_content);
            // TODO: If we get multiple changes, are they affected by the moves??
        }
    }

    /// Converts a [Position] to a `char` index
    fn get_index_of(&self, Position { line, codeunit }: Position) -> Option<usize> {
        Some(match codeunit {
            crate::change::CodeUnitIndex::Utf8(i) => {
                // utf-8 code unit index is a byte index
                let start_of_line = self.content.try_line_to_byte(line).ok()?;
                let start_of_next_line = self.content.try_line_to_byte(line + 1).ok()?;

                let position = self
                    .content
                    .try_byte_to_char(start_of_line + i)
                    .unwrap_or(start_of_next_line);

                // If the index is after the end of this line (so equal or greater than the start of the
                // next) we just return the start of the next, since a byte index can't leave the line and
                // so probably refers to a not yet existing character
                min(position, start_of_next_line)
            }
            crate::change::CodeUnitIndex::Utf16(i) => {
                let start_of_line = self.content.try_line_to_char(line).ok()?;
                let start_of_line = self.content.try_char_to_utf16_cu(start_of_line).ok()?;

                let start_of_next_line = self.content.try_line_to_char(line + 1).ok()?;

                let position = self
                    .content
                    .try_utf16_cu_to_char(start_of_line + i)
                    .unwrap_or(start_of_next_line);

                min(position, start_of_next_line)
            }
            crate::change::CodeUnitIndex::Utf32(i) => {
                let start_of_line = self.content.try_line_to_char(line).ok()?;
                let start_of_next_line = self.content.try_line_to_char(line + 1).ok()?;

                min(start_of_line + i, start_of_next_line)
            }
        })
    }
}
