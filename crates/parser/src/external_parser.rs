//! This module parses the external block, which is basically an entirely separate domain language.
use std::collections::HashMap;

use pest::iterators::Pair;

use waveling_diagnostics::{CompilationError, Span};

use crate::ast;
use crate::grammar::*;

#[derive(Debug)]
enum Value {
    Object(Span, HashMap<String, Value>),
    Array(Span, Vec<Value>),
    Literal(Span, String),
}

fn parse_object(pair: Pair<Rule>) -> Value {
    let mut obj = HashMap::new();
    let obj_span = pair.as_span().into();

    let mut pairs = pair.into_inner();
    while let Some(ident) = pairs.next() {
        let val = pairs.next().unwrap();
        let val = parse_inner(val);
        obj.insert(ident.as_str().to_string(), val);
    }
    Value::Object(obj_span, obj)
}

fn parse_array(pair: Pair<Rule>) -> Value {
    let mut elems = vec![];
    let span = pair.as_span().into();

    let pairs = pair.into_inner();
    for child in pairs {
        let val = parse_inner(child);
        elems.push(val);
    }

    Value::Array(span, elems)
}

fn parse_literal(pair: Pair<Rule>) -> Value {
    let span = pair.as_span().into();
    let lit = pair.as_str().to_string();

    Value::Literal(span, lit)
}

fn parse_inner(pair: Pair<Rule>) -> Value {
    assert_eq!(pair.as_rule(), Rule::meta_val);
    let pair = pair.into_inner().next().unwrap();

    match pair.as_rule() {
        Rule::meta_obj => parse_object(pair),
        Rule::meta_array => parse_array(pair),
        Rule::meta_literal => parse_literal(pair),
        v => panic!("Mismatch between grammar and Rust code: {:?}", v),
    }
}

impl Value {
    fn get_key(&self, key: &str) -> Result<&Value, CompilationError> {
        let val = match self {
            Value::Object(s, c) => c.get(key).ok_or_else(|| {
                CompilationError::new(Some(*s), format!("Expected to find key {}", key))
            })?,
            Value::Array(s, _) | Value::Literal(s, _) => {
                return Err(CompilationError::new(Some(*s), "Expected an object"))
            }
        };
        Ok(val)
    }

    fn iter_array(&self) -> Result<impl Iterator<Item = &Value>, CompilationError> {
        let iter = match self {
            Value::Array(_, c) => c.iter(),
            Value::Object(s, _) | Value::Literal(s, _) => {
                return Err(CompilationError::new(Some(*s), "Expected an array"))
            }
        };

        Ok(iter)
    }

    fn get_literal_str(&self) -> Result<&str, CompilationError> {
        match self {
            Value::Literal(_, v) => Ok(v),
            Value::Array(s, _) | Value::Object(s, _) => {
                Err(CompilationError::new(Some(*s), "Expected a literal"))
            }
        }
    }

    fn get_literal_u64(&self) -> Result<u64, CompilationError> {
        let lit = self.get_literal_str()?;
        lit.parse().map_err(|_| {
            CompilationError::new(
                Some(self.get_span()),
                format!("Expected an integer constant but found {}", lit),
            )
        })
    }

    fn get_span(&self) -> Span {
        match self {
            Value::Array(s, _) | Value::Object(s, _) | Value::Literal(s, _) => *s,
        }
    }
}

fn parse_pin(val: &Value) -> Result<ast::MetaPinDef, CompilationError> {
    let width = val.get_key("width")?.get_literal_u64()?;
    let pin_type = ast::PrimitiveTypeLit::parse_from_str(
        &val.get_span(),
        val.get_key("type")?.get_literal_str()?,
    )?;
    Ok(ast::MetaPinDef {
        span: val.get_span(),
        width,
        pin_type,
    })
}

/// Parse an array of pins under the given key.
fn parse_pin_array(key: &str, val: &Value) -> Result<Vec<ast::MetaPinDef>, Vec<CompilationError>> {
    let pins = val.get_key(key).map_err(|x| vec![x])?;

    let mut ret = vec![];
    let mut errors = vec![];

    for x in pins.iter_array().map_err(|x| vec![x])? {
        match parse_pin(x) {
            Ok(pin) => ret.push(pin),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(ret)
    } else {
        Err(errors)
    }
}

/// Parse a single property definition.
fn parse_prop(val: &Value) -> Result<ast::MetaPropertyDef, CompilationError> {
    let property_type = ast::PrimitiveTypeLit::parse_from_str(
        &val.get_span(),
        val.get_key("type")?.get_literal_str()?,
    )?;

    Ok(ast::MetaPropertyDef {
        span: val.get_span(),
        property_type,
    })
}

fn parse_props(val: &Value) -> Result<Vec<ast::MetaPropertyDef>, Vec<CompilationError>> {
    let props = val.get_key("properties").map_err(|x| vec![x])?;

    let mut ret = vec![];
    let mut errors = vec![];

    for p in props.iter_array().map_err(|x| vec![x])? {
        match parse_prop(p) {
            Ok(x) => ret.push(x),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(ret)
    } else {
        Err(errors)
    }
}

pub(crate) fn parse_external(obj: Pair<Rule>) -> Result<ast::External, Vec<CompilationError>> {
    let val = parse_object(obj.into_inner().next().unwrap());

    let mut all_errors = vec![];

    let maybe_inputs = parse_pin_array("inputs", &val).map_err(|x| all_errors.extend(x));
    let maybe_outputs = parse_pin_array("outputs", &val).map_err(|x| all_errors.extend(x));
    let maybe_properties = parse_props(&val).map_err(|x| all_errors.extend(x));

    if maybe_inputs.is_err() || maybe_outputs.is_err() || maybe_properties.is_err() {
        return Err(all_errors);
    }

    let inputs = maybe_inputs.unwrap();
    let outputs = maybe_outputs.unwrap();
    let properties = maybe_properties.unwrap();

    let span = val.get_span();

    Ok(ast::External {
        inputs,
        outputs,
        properties,
        span,
    })
}
