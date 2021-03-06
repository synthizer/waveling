#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
WHITESPACE = _{
    " " | "\t" | NEWLINE
}

COMMENT = _{
    "//" ~ (!NEWLINE ~ ANY) + ~ NEWLINE
}

identifier = @{
    (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")*
}

number = @{
    ("-"){,1} ~ (
        "0x" ~ ASCII_HEX_DIGIT+
        // The repetition here makes . optional.
        | ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+){,1}
    )
}

// Careful here: we don't want to allow `programname` only `program name`
program_decl = ${
    "program" ~ (WHITESPACE | COMMENT)+ ~ identifier ~ (WHITESPACE | COMMENT)* ~ ";"
}

meta_obj = {
    "{" ~ (identifier ~ ":" ~ meta_val ~ ("," | &"}"))* ~ "}"
}

meta_array = {
    "[" ~ (meta_val ~ ("," | &"]")) * ~ "]"
}

meta_literal = {
    identifier | number
}

meta_val = {
    meta_obj | meta_array | meta_literal
}

external_block = {
    "external" ~ meta_obj
}

path = {
    identifier ~ ("." ~ identifier)*
}

// To use precedence climbing, we need rules that match exactly all of our operators.
plus = { "+" }
minus = { "-" }
star = { "*" }
dash ={ "-" }
slash = { "/" }
percent = { "%" }
leftarrow = { "<-" }
rightarrow = { "->" }

bundle_kv = {
    identifier ~ ":" ~ expr
}

bundle_index = {
    expr
}

bundle = {
    "{"
    ~ (bundle_index ~ ("," | &"}"))*
    ~ (bundle_kv ~ ("," | &"}"))*
    ~ "}"
}

expr_unary = {
    number
    | path
    | bundle
    | "(" ~ expr ~ ")"
    // Negated expression, e.g. `-(2 + 3)`.
    | minus ~ "(" ~ expr ~ ")"
}

expr = {
    expr_unary ~ ((leftarrow | rightarrow | plus | minus | star | slash | percent) ~ expr_unary)*
}

binding_let = ${
    "let" ~ (WHITESPACE | COMMENT)+ ~ identifier
}

binding = {
    binding_let ~ "=" ~ expr
}

statement = {
    (binding | expr) ~ ";"
}

stage_output_decl = {
    identifier ~ "(" ~ number ~ ")"
}

stage_header = {
    identifier ~ "(" ~ (stage_output_decl ~ ("," | &")"))* ~ ")"
}

stage_body = {
    "{" ~ statement* ~ "}"
}

// Also have to be careful about doing whitespace ourselves here again.
stage_start = @{
    "stage" ~ (WHITESPACE|COMMENT)
}

stage = {
    stage_start ~ stage_header ~ stage_body
}

program = {
    SOI ~ program_decl ~ external_block ~ stage+ ~ EOI
}
"#]
pub(crate) struct WavelingParser;
