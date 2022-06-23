use pest::iterators::Pair;

use crate::ast;
use crate::grammar::Rule;
use crate::*;

impl ast::PrimitiveTypeLit {
    pub(crate) fn parse_from_str(
        span: &ast::Span,
        input: &str,
    ) -> Result<ast::PrimitiveTypeLit, Error> {
        if input == "f32" {
            Ok(ast::PrimitiveTypeLit {
                span: *span,
                kind: ast::PrimitiveTypeLitKind::F32,
            })
        } else if input == "f64" {
            Ok(ast::PrimitiveTypeLit {
                span: *span,
                kind: ast::PrimitiveTypeLitKind::F64,
            })
        } else if input == "i32" {
            Ok(ast::PrimitiveTypeLit {
                span: *span,
                kind: ast::PrimitiveTypeLitKind::I32,
            })
        } else if input == "i64" {
            Ok(ast::PrimitiveTypeLit {
                span: *span,
                kind: ast::PrimitiveTypeLitKind::I64,
            })
        } else {
            Err(Error::new(
                *span,
                format!("{} is not a valid primitive", input),
            ))
        }
    }
}

fn parse_program_decl(pair: Pair<Rule>) -> ast::ProgramDecl {
    let span = pair.as_span().into();
    let program_name = pair.into_inner().next().unwrap().as_str().to_string();
    ast::ProgramDecl { span, program_name }
}

pub fn parse(input: &str) -> Result<ast::Program, Vec<Error>> {
    use pest::Parser;

    let mut program_parts = crate::grammar::WavelingParser::parse(Rule::program, input)
        .map_err(|x| vec![x.into()])?
        .next()
        .unwrap()
        .into_inner();

    let program_decl = parse_program_decl(program_parts.next().unwrap());
    let external = crate::external_parser::parse_external(program_parts.next().unwrap())?;

    Ok(ast::Program {
        program_decl,
        external,
    })
}
