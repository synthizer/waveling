use std::collections::HashMap;

use rust_decimal::Decimal;

#[derive(Debug)]
pub struct Program {
    pub program_decl: ProgramDecl,
    pub external: External,
    pub stages: Vec<Stage>,
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

#[derive(Debug)]
pub struct Path {
    pub span: Span,
    pub segments: Vec<String>,
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug)]
pub struct Bundle {
    pub span: Span,
    pub array: Vec<Expr>,
    pub kv: HashMap<String, Expr>,
}

#[derive(Debug)]
pub enum ExprKind {
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Number(Decimal),
    Path(Path),
    Bundle(Bundle),
}

#[derive(Debug)]
pub struct Binding {
    pub span: Span,
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug)]
pub enum StatementKind {
    Binding(Binding),
    Expr(Expr),
}

#[derive(Debug)]
pub struct Statement {
    pub span: Span,
    pub kind: StatementKind,
}

#[derive(Debug)]
pub struct StageOutput {
    pub span: Span,
    pub output_type: PrimitiveTypeLit,
    pub width: u64,
}

#[derive(Debug)]
pub struct Stage {
    pub span: Span,
    pub name: String,
    pub outputs: Vec<StageOutput>,
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

impl<'i> From<pest::Span<'i>> for Span {
    fn from(input: pest::Span<'i>) -> Span {
        (&input).into()
    }
}
