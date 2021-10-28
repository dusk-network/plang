use crate::error::Result;

use pest::iterators::Pairs;
use pest::Parser;
use pest_derive::Parser;

/// This is the output of the pest parser for plang.
#[derive(Debug, Parser)]
#[grammar = "../plang.pest"]
pub struct PlangGrammar<'a> {
    pairs: Pairs<'a, Rule>,
}

impl<'a> PlangGrammar<'a> {
    pub fn new(text: &'a str) -> Result<Self> {
        let pairs = Self::parse(Rule::main, text)?;
        Ok(Self { pairs })
    }

    pub fn pairs(&self) -> Pairs<'a, Rule> {
        self.pairs.clone()
    }
}
