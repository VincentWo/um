use tower_lsp::lsp_types::PositionEncodingKind;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl From<PositionEncodingKind> for Encoding {
    fn from(value: PositionEncodingKind) -> Self {
        if value == PositionEncodingKind::UTF8 {
            Self::Utf8
        } else if value == PositionEncodingKind::UTF16 {
            Self::Utf16
        } else if value == PositionEncodingKind::UTF32 {
            Self::Utf32
        } else {
            todo!()
        }
    }
}
