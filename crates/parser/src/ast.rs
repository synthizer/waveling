#[derive(Debug)]
pub struct Program {
    pub program_decl: ProgramDecl,
    pub external: External,
}
/// A span in source text, which we use to track for errors.
///
/// We don't use the one from Pest, because the one from Pest holds a reference to the source text and that infests
/// everything everywhere with a lifetime parameter.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,

    pub start_line: usize,
    pub start_line_col: usize,
    pub end_line: usize,
    pub end_line_col: usize,
}

#[derive(Debug)]
pub struct ProgramDecl {
    pub span: Span,
    pub program_name: String,
}

#[derive(Debug)]
pub enum PrimitiveTypeLitKind {
    F32,
    F64,
    I32,
    I64,
    Bool,
}

#[derive(Debug)]
pub struct PrimitiveTypeLit {
    pub span: Span,
    pub kind: PrimitiveTypeLitKind,
}

/// Either an input or output shape.
#[derive(Debug)]
pub struct MetaPinDef {
    pub span: Span,
    pub width: u64,
    pub pin_type: PrimitiveTypeLit,
}

#[derive(Debug)]
pub struct MetaPropertyDef {
    pub span: Span,
    pub property_type: PrimitiveTypeLit,
}

#[derive(Debug)]
pub struct External {
    pub span: Span,
    pub inputs: Vec<MetaPinDef>,
    pub outputs: Vec<MetaPinDef>,
    pub properties: Vec<MetaPropertyDef>,
}

impl<'i> From<&pest::Span<'i>> for Span {
    fn from(input: &pest::Span<'i>) -> Span {
        let (start_line, start_line_col) = input.start_pos().line_col();
        let (end_line, end_line_col) = input.end_pos().line_col();

        Span {
            start: input.start(),
            end: input.end(),
            start_line,
            start_line_col,
            end_line,
            end_line_col,
        }
    }
}

impl<'i> From<pest::Span<'i>> for Span {
    fn from(input: pest::Span<'i>) -> Span {
        (&input).into()
    }
}
