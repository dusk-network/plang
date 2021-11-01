use crate::error::{Error as PlangError, Result};
use crate::grammar::{PlangGrammar, Rule};

use std::collections::{hash_map::Entry, HashMap};
use std::str::FromStr;

use dusk_plonk::prelude::*;

/// A plonk circuit parsed from plang.
#[derive(Debug)]
pub struct PlangCircuit {
    exprs: Vec<PlangExpr>,
    vars: HashMap<String, WitnessOrPublic>,
}

/// Something that is either a witness or a public input.
#[derive(Debug)]
enum WitnessOrPublic {
    Witness(BlsScalar),
    PublicInput(BlsScalar),
}

impl Default for WitnessOrPublic {
    fn default() -> Self {
        Self::Witness(BlsScalar::zero())
    }
}

impl PlangCircuit {
    /// Parses a circuit from text.
    pub fn parse<S: AsRef<str>>(text: S) -> Result<Self> {
        let grammar = PlangGrammar::new(text.as_ref())?;
        Self::from_grammar(grammar)
    }

    /// Sets the witness and public input values. Any value not set will remain
    /// the default - 0. It returns an error if a value is not in the circuit.
    pub fn set_vals<B: Into<BlsScalar>, I: IntoIterator<Item = (String, B)>>(
        &mut self,
        vals: I,
    ) -> Result<()> {
        for (name, val) in vals {
            match self.vars.entry(name.clone()) {
                Entry::Vacant(_) => return Err(PlangError::NoSuchValue(name)),
                Entry::Occupied(mut entry) => match entry.get() {
                    WitnessOrPublic::PublicInput(_) => {
                        entry.insert(WitnessOrPublic::PublicInput(val.into()));
                    }
                    WitnessOrPublic::Witness(_) => {
                        entry.insert(WitnessOrPublic::Witness(val.into()));
                    }
                },
            }
        }

        Ok(())
    }

    /// Parses a circuit from a grammar.
    ///
    /// It goes through each equation, arranging them all into a vector of
    /// `PlangExpr`s, while inserting all variables into a map with with an
    /// initial default value.
    fn from_grammar(grammar: PlangGrammar<'_>) -> Result<Self> {
        let mut exprs = vec![];

        for pair in grammar.pairs() {
            let rule = pair.as_rule();
            if rule == Rule::expr {
                let mut minus = false;
                let mut public = None;

                let mut tris = vec![];
                let mut bis = vec![];

                for expr_inner in pair.into_inner() {
                    let expr_rule = expr_inner.as_rule();
                    match expr_rule {
                        Rule::sign => {
                            if expr_inner.as_span().as_str() == "-" {
                                minus = true;
                            } else {
                                minus = false;
                            }
                        }
                        Rule::tri_term => {
                            let mut coeff = 1;
                            let mut vars = vec![];

                            for term_inner in expr_inner.into_inner() {
                                let term_rule = term_inner.as_rule();
                                match term_rule {
                                    Rule::coeff => {
                                        coeff = u64::from_str(term_inner.as_span().as_str())?
                                    }
                                    Rule::var => {
                                        vars.push(term_inner.as_span().as_str().to_owned())
                                    }
                                    _ => unreachable!(),
                                }
                            }

                            tris.push(TriTerm {
                                minus,
                                coeff: coeff.into(),
                                rvar: vars.pop().unwrap(),
                                lvar: vars.pop().unwrap(),
                            })
                        }
                        Rule::bi_term => {
                            let mut coeff = 1;
                            let mut var = String::default();

                            for term_inner in expr_inner.into_inner() {
                                let term_rule = term_inner.as_rule();
                                match term_rule {
                                    Rule::coeff => {
                                        coeff = u64::from_str(term_inner.as_span().as_str())?
                                    }
                                    Rule::var => var = term_inner.as_span().as_str().to_owned(),
                                    _ => unreachable!(),
                                }
                            }

                            bis.push(BiTerm {
                                minus,
                                coeff: coeff.into(),
                                var,
                            })
                        }
                        Rule::var => {
                            let var = expr_inner.as_span().as_str().to_owned();
                            public = Some(Public { minus, var });
                        }
                        _ => {}
                    }
                }

                // TODO this could be enforced in the grammar - possibly simplifying this
                //  function as well
                if tris.len() > 1 {
                    return Err(PlangError::TooManyTriTerms);
                }

                exprs.push(PlangExpr {
                    tri: tris.pop(),
                    bis,
                    public,
                })
            }
        }

        // some checks on the expression to make sure its ok.
        check_different_tri_vars(&exprs)?;
        check_less_than_5_vars(&exprs)?;
        check_no_repeat_vars_in_bis(&exprs)?;
        check_public_different_from_other_vars(&exprs)?;

        let vars = vars_from_exprs(&exprs);
        Ok(Self { exprs, vars })
    }
}

// Creates a map of names to witnesses or public inputs.
fn vars_from_exprs(exprs: &[PlangExpr]) -> HashMap<String, WitnessOrPublic> {
    let mut vars = HashMap::new();

    for expr in exprs {
        // if there is a PI in the expression (right equation side), also
        // insert it in the map.
        if let Some(public) = &expr.public {
            vars.insert(
                public.var.clone(),
                WitnessOrPublic::PublicInput(BlsScalar::zero()),
            );
        }

        // A term of the form `q_m · a · b` contains two witnesses.
        if let Some(tri) = &expr.tri {
            vars.insert(
                tri.lvar.clone(),
                WitnessOrPublic::Witness(BlsScalar::zero()),
            );
            vars.insert(
                tri.rvar.clone(),
                WitnessOrPublic::Witness(BlsScalar::zero()),
            );
        }

        // A term of the form `q_x · y` contains one witness.
        for bi in &expr.bis {
            vars.insert(bi.var.clone(), Default::default());
        }
    }

    vars
}

// Check that `a != b` for all expressions the form `q_m · a · b`.
fn check_different_tri_vars(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        if let Some(tri) = &expr.tri {
            if tri.lvar == tri.rvar {
                return Err(PlangError::SameTriVars);
            }
        }
    }

    Ok(())
}

// Check that each expression has less than 5 vars.
fn check_less_than_5_vars(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        let mut vars = HashMap::with_capacity(5);

        if let Some(public) = &expr.public {
            vars.insert(&public.var, ());
        }

        if let Some(tri) = &expr.tri {
            vars.insert(&tri.lvar, ());
            vars.insert(&tri.rvar, ());
        }

        for bi in &expr.bis {
            vars.insert(&bi.var, ());
        }

        if vars.len() == 5 {
            return Err(PlangError::TooManyVars);
        }
    }

    Ok(())
}

// Check that there's no terms of the form `q_x · y` where variables are have
// the same name in the same expression.
fn check_no_repeat_vars_in_bis(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        let mut nterms = 0;
        let mut vars = HashMap::with_capacity(5);

        for bi in &expr.bis {
            nterms += 1;
            vars.insert(&bi.var, ());
        }

        if vars.len() != nterms {
            return Err(PlangError::RepeatedVars);
        }
    }

    Ok(())
}

// Check the public input is different from all other variables.
fn check_public_different_from_other_vars(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        if let Some(public) = &expr.public {
            let mut vars = HashMap::with_capacity(5);

            if let Some(tri) = &expr.tri {
                vars.insert(&tri.lvar, ());
                vars.insert(&tri.rvar, ());
            }

            for bi in &expr.bis {
                vars.insert(&bi.var, ());
            }

            if vars.contains_key(&public.var) {
                return Err(PlangError::PublicVarNotSingular);
            }
        }
    }

    Ok(())
}

impl Circuit for PlangCircuit {
    const CIRCUIT_ID: [u8; 32] = [0u8; 32];

    // Gadget implementation for a plang circuit.
    fn gadget(&mut self, composer: &mut TurboComposer) -> std::result::Result<(), Error> {
        // Append all witnesses in the map to the composer.
        let witnesses = {
            let mut ws = HashMap::new();

            for (vname, wop) in &self.vars {
                if let WitnessOrPublic::Witness(wval) = wop {
                    ws.insert(vname, composer.append_witness(*wval));
                }
            }

            ws
        };

        // For every expression build the constraint according to the existing terms.
        for expr in &self.exprs {
            let mut constraint = Constraint::new();

            // If there is a public input add it as a `.public()` selector.
            if let Some(public) = &expr.public {
                let val = match self
                    .vars
                    .get(&public.var)
                    .expect("public input isn't in map")
                {
                    WitnessOrPublic::PublicInput(scalar) => scalar,
                    _ => panic!("public is not as public in map"),
                };

                match public.minus {
                    true => {
                        constraint = constraint.public(*val);
                    }
                    false => {
                        constraint = constraint.public(-*val);
                    }
                }
            }

            let mut tri_wits = None;

            // If there is a term of the form `q_m · a · b` add it as a
            // `mult()` selector.
            if let Some(tri) = &expr.tri {
                let lwit = witnesses
                    .get(&tri.lvar)
                    .expect("tri term witness not in witness map");
                let rwit = witnesses
                    .get(&tri.rvar)
                    .expect("tri term witness not in witness map");

                tri_wits = Some((lwit, rwit));

                match tri.minus {
                    true => constraint = constraint.mult(-tri.coeff),
                    false => constraint = constraint.mult(tri.coeff),
                }

                constraint = constraint.a(*lwit);
                constraint = constraint.b(*rwit);
            }

            let mut bi_num = 0;
            for bi in &expr.bis {
                let wit = witnesses
                    .get(&bi.var)
                    .expect("bi term witness not in witness map");

                // If there is a term of the form `q_m · a · b` then if there
                // is a term of the form `q_l · a` or `q_r · b` add a left
                // wire, or a right wire selector respectively. If there is
                // not, then one just adds the selectors sequentially, as it
                // produces the same mathematical constraint.
                match tri_wits {
                    Some((lwit, rwit)) => match (wit == lwit, wit == rwit) {
                        (false, false) => {
                            constraint = constraint.o(*wit);
                            match bi.minus {
                                true => constraint = constraint.output(bi.coeff),
                                false => constraint = constraint.output(-bi.coeff),
                            }
                        }
                        (true, false) => match bi.minus {
                            true => constraint = constraint.left(bi.coeff),
                            false => constraint = constraint.left(-bi.coeff),
                        },
                        (false, true) => match bi.minus {
                            true => constraint = constraint.right(bi.coeff),
                            false => constraint = constraint.right(-bi.coeff),
                        },
                        _ => panic!("witness is both lwit and rwit"),
                    },
                    None => {
                        match bi_num {
                            0 => {
                                constraint = constraint.a(*wit);
                                match bi.minus {
                                    true => constraint = constraint.left(-bi.coeff),
                                    false => constraint = constraint.left(bi.coeff),
                                }
                            }
                            1 => {
                                constraint = constraint.b(*wit);
                                match bi.minus {
                                    true => constraint = constraint.right(-bi.coeff),
                                    false => constraint = constraint.right(bi.coeff),
                                }
                            }
                            2 => {
                                constraint = constraint.o(*wit);
                                match bi.minus {
                                    true => constraint = constraint.output(-bi.coeff),
                                    false => constraint = constraint.output(bi.coeff),
                                }
                            }
                            _ => panic!("there should be max 3 bi terms"),
                        }

                        bi_num += 1;
                    }
                }
            }

            composer.append_gate(constraint);
        }

        Ok(())
    }

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        let mut named_pinputs: Vec<(&String, PublicInputValue)> = self
            .vars
            .iter()
            .filter_map(|(name, wop)| {
                if let WitnessOrPublic::PublicInput(pval) = wop {
                    return Some((name, (*pval).into()));
                }
                None
            })
            .collect();

        named_pinputs.sort_by(|(name1, _), (name2, _)| Ord::cmp(name1, name2));

        let mut pinputs = Vec::with_capacity(named_pinputs.len());
        pinputs.append(&mut named_pinputs.into_iter().map(|(_, v)| v).collect());

        pinputs
    }

    fn padded_gates(&self) -> usize {
        1 << (self.exprs.len() + 1)
    }
}

#[derive(Debug, Default)]
struct PlangExpr {
    tri: Option<TriTerm>,
    bis: Vec<BiTerm>,
    public: Option<Public>,
}

// TODO find a better way of dealing with negative coefficients

#[derive(Debug)]
struct TriTerm {
    minus: bool,
    coeff: BlsScalar,
    lvar: String,
    rvar: String,
}

#[derive(Debug)]
struct BiTerm {
    minus: bool,
    coeff: BlsScalar,
    var: String,
}

#[derive(Debug)]
struct Public {
    minus: bool,
    var: String,
}
