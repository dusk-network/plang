// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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
