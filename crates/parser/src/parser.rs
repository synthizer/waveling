use std::collections::HashMap;

use pest::iterators::Pair;

use waveling_diagnostics::{CompilationError, Span};

use crate::ast;
use crate::grammar::Rule;
use crate::*;

impl ast::PrimitiveTypeLit {
    pub(crate) fn parse_from_str(
        span: &Span,
        input: &str,
    ) -> Result<ast::PrimitiveTypeLit, CompilationError> {
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
            Err(CompilationError::new(
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

fn parse_path(pair: Pair<Rule>) -> ast::Path {
    let span = pair.as_span().into();
    let mut segs = vec![];

    for seg in pair.into_inner() {
        assert_eq!(seg.as_rule(), Rule::identifier);
        segs.push(seg.as_str().to_string());
    }

    ast::Path {
        span,
        segments: segs,
    }
}

fn parse_bundle(pair: Pair<Rule>) -> Result<ast::Bundle, CompilationError> {
    let span = pair.as_span().into();

    let inner = pair.into_inner();

    let mut array = vec![];
    let mut kv: HashMap<String, ast::Expr> = Default::default();

    // A bundle consists of 0 or more index/array components and 0 or more key/value pairs in that order.  Since the
    // grammar enforces this we just go through them without duplicating that here for simplicity.
    for entry in inner {
        let rule = entry.as_rule();
        match rule {
            Rule::bundle_index => {
                // Bundle indices are just expressions.
                array.push(parse_expr(entry.into_inner().next().unwrap())?);
            }
            Rule::bundle_kv => {
                let mut children = entry.into_inner();
                let key = children.next().unwrap().as_str().to_string();
                let value = parse_expr(children.next().unwrap())?;
                kv.insert(key, value);
            }
            r => panic!("Got non-bundle rule {:?} at {:?}", r, span),
        }
    }

    Ok(ast::Bundle { span, array, kv })
}

fn parse_number(pair: Pair<Rule>) -> Result<rust_decimal::Decimal, CompilationError> {
    let span = pair.as_span().into();
    let num = pair.as_str();

    // Maybe negative.
    let neg = num.starts_with('-');

    // Grammar ensures that 0x is at the beginning.
    let hex = num.contains("0x");

    let skipped_chars = (neg as usize) + 2 * (hex as usize);

    let digits = &num[skipped_chars..];

    // rust_decimal isn't good at parsing, but it was far too late to back out that decision by the time I found this
    // out.
    let mut ret = if hex {
        rust_decimal::Decimal::from_str_radix(digits, 16)
            .map_err(|_| CompilationError::new(span, "Unable to parse decimal"))?
    } else {
        rust_decimal::Decimal::from_str_radix(digits, 10)
            .map_err(|_| CompilationError::new(span, "Unable to parse decimal"))?
    };
    ret.set_sign_positive(!neg);

    Ok(ret)
}

fn parse_expr_unary(pair: Pair<Rule>) -> Result<ast::Expr, CompilationError> {
    let span = pair.as_span().into();

    // This is a unary expr, so the first pair tells us what kind of expr it is.
    //
    // In the specific case of negation, the first token is minus, and the data is in the second pair.
    let mut inner = pair.into_inner();
    // Always at least one.
    let first = inner.next().unwrap();
    // And if negated, an optional second, which we will unwrap below.
    let negated = inner.next();

    let kind = match first.as_rule() {
        Rule::number => ast::ExprKind::Number(parse_number(first)?),
        Rule::path => ast::ExprKind::Path(parse_path(first)),
        Rule::bundle => ast::ExprKind::Bundle(parse_bundle(first)?),
        Rule::minus => ast::ExprKind::Negate(Box::new(parse_expr(negated.unwrap())?)),
        r => panic!("Unexpected rule {:?} at {:?}", r, span),
    };

    Ok(ast::Expr { span, kind })
}

fn parse_expr(pair: Pair<Rule>) -> Result<ast::Expr, CompilationError> {
    use pest::prec_climber::{Assoc::Left, Operator, PrecClimber};

    let mul_div_rem = Operator::new(Rule::star, Left)
        | Operator::new(Rule::slash, Left)
        | Operator::new(Rule::percent, Left);
    let add_sub = Operator::new(Rule::plus, Left) | Operator::new(Rule::minus, Left);

    let climber = PrecClimber::new(vec![add_sub, mul_div_rem]);

    climber.climb(
        pair.into_inner(),
        parse_expr_unary,
        |left, op, right| -> Result<ast::Expr, CompilationError> {
            // We might want to consider being smarter here and trying to merge the spans of left and right, but this is
            // good enough for now.
            let span = op.as_span().into();
            let left = left?;
            let right = right?;

            let op = match op.as_rule() {
                Rule::plus => ast::BinOp::Add,
                Rule::minus => ast::BinOp::Sub,
                Rule::star => ast::BinOp::Mul,
                Rule::slash => ast::BinOp::Div,
                Rule::percent => ast::BinOp::Mod,
                r => panic!("Unexpected operator rule {:?} at {:?}", r, span),
            };

            Ok(ast::Expr {
                span,
                kind: ast::ExprKind::Binary(op, Box::new(left), Box::new(right)),
            })
        },
    )
}

fn parse_binding(pair: Pair<Rule>) -> Result<ast::Binding, CompilationError> {
    let span = pair.as_span().into();

    let mut inner = pair.into_inner();
    let mut let_ident = inner.next().unwrap().into_inner();
    let name = let_ident.next().unwrap().as_str().to_string();
    let expr = parse_expr(inner.next().unwrap())?;
    Ok(ast::Binding { span, name, expr })
}

fn parse_statement(pair: Pair<Rule>) -> Result<ast::Statement, CompilationError> {
    let span = pair.as_span().into();
    let payload = pair.into_inner().next().unwrap();
    let kind = match payload.as_rule() {
        Rule::binding => ast::StatementKind::Binding(parse_binding(payload)?),
        Rule::expr => ast::StatementKind::Expr(parse_expr(payload)?),
        r => panic!(
            "Got {:?}, which is not a valid statement rule at {:?}",
            r, span
        ),
    };

    Ok(ast::Statement { span, kind })
}

/// Parse a list of statements, returning either a vec of parsed statements or a vec of errors for reporting.
pub fn parse_stage_body(pair: Pair<Rule>) -> Result<Vec<ast::Statement>, Vec<CompilationError>> {
    let mut statements = vec![];
    let mut errors = vec![];

    for statement in pair.into_inner() {
        match parse_statement(statement) {
            Ok(s) => statements.push(s),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(statements)
    }
}

fn parse_stage_output_decl(pair: Pair<Rule>) -> Result<ast::StageOutput, CompilationError> {
    let span = pair.as_span().into();

    let mut inner = pair.into_inner();
    let unparsed_ty = inner.next().unwrap();
    let unparsed_width = inner.next().unwrap();

    let output_type = ast::PrimitiveTypeLit::parse_from_str(&span, unparsed_ty.as_str())?;
    let width = unparsed_width
        .as_str()
        .parse()
        .map_err(|_| CompilationError::new(span, "Expected a number"))?;

    Ok(ast::StageOutput {
        span,
        width,
        output_type,
    })
}

// Returns `(name, outputs)`.
fn parse_stage_header_outputs(
    pair: Pair<Rule>,
) -> Result<(String, Vec<ast::StageOutput>), Vec<CompilationError>> {
    let mut outputs = vec![];
    let mut errors = vec![];
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    for entry in inner {
        match parse_stage_output_decl(entry) {
            Ok(x) => outputs.push(x),
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok((name, outputs))
    }
}

fn parse_stage(pair: Pair<Rule>) -> Result<ast::Stage, Vec<CompilationError>> {
    let span = pair.as_span().into();
    let mut inner = pair.into_inner();
    let mut errors = vec![];

    // Skip the dummy rule at the beginning, which is introduced to handle whitespace.
    inner.next();

    let maybe_outputs = parse_stage_header_outputs(inner.next().unwrap());
    let maybe_statements = parse_stage_body(inner.next().unwrap());

    let good = maybe_outputs.is_ok() && maybe_statements.is_ok();
    if !good {
        if let Err(e) = maybe_outputs {
            errors.extend(e);
        }
        if let Err(e) = maybe_statements {
            errors.extend(e);
        }

        return Err(errors);
    }

    let statements = maybe_statements.unwrap();
    let (name, outputs) = maybe_outputs.unwrap();

    Ok(ast::Stage {
        span,
        name,
        outputs,
        statements,
    })
}

pub fn parse(input: &str) -> Result<ast::Program, Vec<CompilationError>> {
    use pest::Parser;

    let mut program_parts = crate::grammar::WavelingParser::parse(Rule::program, input)
        .map_err(|x| vec![pest_to_diagnostic(&x)])?
        .next()
        .unwrap()
        .into_inner();

    let program_decl = parse_program_decl(program_parts.next().unwrap());

    let maybe_external = crate::external_parser::parse_external(program_parts.next().unwrap());
    let stages = program_parts
        .take_while(|x| x.as_rule() == Rule::stage)
        .map(parse_stage)
        .collect::<Vec<_>>();

    let is_good = maybe_external.is_ok() && stages.iter().all(|x| x.is_ok());
    if !is_good {
        let mut errors = vec![];
        if let Err(e) = maybe_external {
            errors.extend(e);
        }

        for s in stages {
            if let Err(e) = s {
                errors.extend(e);
            }
        }

        return Err(errors);
    }

    let external = maybe_external.unwrap();
    let stages = stages.into_iter().map(|x| x.unwrap()).collect();

    Ok(ast::Program {
        program_decl,
        external,
        stages,
    })
}
