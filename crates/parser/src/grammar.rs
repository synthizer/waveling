#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub(crate) struct WavelingParser;
