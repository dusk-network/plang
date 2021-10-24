use std::collections::HashMap;
use crate::error::{Result, Error as PlangError};
use crate::grammar::{PlangGrammar, Rule};

use std::str::FromStr;

use dusk_plonk::prelude::*;

/// A plonk circuit parsed from plang.
#[derive(Debug)]
pub struct PlangCircuit {
    exprs: Vec<PlangExpr>,
    vars: HashMap<String, WitnessOrPublic>,
}

#[derive(Debug)]
enum WitnessOrPublic {
    Witness(BlsScalar),
    Public(BlsScalar),
}

impl Default for WitnessOrPublic {
    fn default() -> Self {
        Self::Witness(BlsScalar::zero())
    }
}

impl PlangCircuit {
    pub fn from_grammar(grammar: PlangGrammar<'_>) -> Result<Self> {
        let mut exprs = vec![];

        for pair in grammar.pairs() {
            let rule = pair.as_rule();
            match rule {
                Rule::expr => {
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
                                        Rule::coeff =>
                                            coeff = u64::from_str(term_inner.as_span().as_str())?,
                                        Rule::var =>
                                            vars.push(term_inner.as_span().as_str().to_owned()),
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
                                        Rule::coeff =>
                                            coeff = u64::from_str(term_inner.as_span().as_str())?,
                                        Rule::var =>
                                            var = term_inner.as_span().as_str().to_owned(),
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
                                public = Some(Public {
                                    minus,
                                    var,
                                });
                            }
                            _ => {}
                        }
                    }

                    if tris.len() > 1 {
                        return Err(PlangError::TooManyTriTerms);
                    }

                    exprs.push(PlangExpr {
                        tri: tris.pop(),
                        bis,
                        public,
                    })
                }
                _ => {}
            }
        }

        check_different_tri_vars(&exprs)?;
        check_less_than_5_vars(&exprs)?;
        check_no_repeat_vars_in_bis(&exprs)?;
        check_public_different_from_other_vars(&exprs)?;

        let vars = vars_from_exprs(&exprs);
        Ok(Self { exprs, vars })
    }
}

fn vars_from_exprs(exprs: &[PlangExpr]) -> HashMap<String, WitnessOrPublic> {
    let mut vars = HashMap::new();

    for expr in exprs {
        if let Some(public) = &expr.public {
            vars.insert(public.var.clone(), WitnessOrPublic::Public(BlsScalar::zero()));
        }

        if let Some(tri) = &expr.tri {
            vars.insert(tri.lvar.clone(), Default::default());
            vars.insert(tri.rvar.clone(), Default::default());
        }

        for bi in &expr.bis {
            vars.insert(bi.var.clone(), Default::default());
        }
    }

    vars
}

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

struct Void;

fn check_less_than_5_vars(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        let mut vars = HashMap::with_capacity(5);

        if let Some(public) = &expr.public {
            vars.insert(&public.var, Void);
        }

        if let Some(tri) = &expr.tri {
            vars.insert(&tri.lvar, Void);
            vars.insert(&tri.rvar, Void);
        }

        for bi in &expr.bis {
            vars.insert(&bi.var, Void);
        }

        if vars.len() == 5 {
            return Err(PlangError::TooManyVars);
        }
    }

    Ok(())
}

fn check_no_repeat_vars_in_bis(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        let mut nterms = 0;
        let mut vars = HashMap::with_capacity(5);

        for bi in &expr.bis {
            nterms += 1;
            vars.insert(&bi.var, Void);
        }

        if vars.len() != nterms {
            return Err(PlangError::RepeatedVars);
        }
    }

    Ok(())
}

fn check_public_different_from_other_vars(exprs: &[PlangExpr]) -> Result<()> {
    for expr in exprs {
        if let Some(public) = &expr.public {
            let mut vars = HashMap::with_capacity(5);

            if let Some(tri) = &expr.tri {
                vars.insert(&tri.lvar, Void);
                vars.insert(&tri.rvar, Void);
            }

            for bi in &expr.bis {
                vars.insert(&bi.var, Void);
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

    fn gadget(&mut self, composer: &mut TurboComposer) -> std::result::Result<(), Error> {
        let witnesses = {
            let mut ws = HashMap::new();

            for (vname, wop) in &self.vars {
                if let WitnessOrPublic::Witness(wval) = wop {
                    ws.insert(vname, composer.append_witness(*wval));
                }
            }

            ws
        };

        for expr in &self.exprs {
            let mut constraint = Constraint::new();

            if let Some(public) = &expr.public {
                let val = match self.vars.get(&public.var).expect("public input isn't in map") {
                    WitnessOrPublic::Public(scalar) => scalar,
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

            // `q_m · a · b  + q_l · a + q_r · b + q_o · o = -PI`.
            if let Some(tri) = &expr.tri {
                let lwit = witnesses.get(&tri.lvar).expect("tri term witness not in witness map");
                let rwit = witnesses.get(&tri.rvar).expect("tri term witness not in witness map");

                tri_wits = Some((lwit, rwit));

                match tri.minus {
                    true => {
                        constraint = constraint.mult(-tri.coeff)
                    }
                    false => {
                        constraint = constraint.mult(tri.coeff)
                    }
                }

                constraint = constraint.a(*lwit);
                constraint = constraint.b(*rwit);
            }

            let mut bi_num = 0;
            for bi in &expr.bis {
                let wit = witnesses.get(&bi.var).expect("bi term witness not in witness map");

                match tri_wits {
                    Some((lwit, rwit)) => {
                        match (wit == lwit, wit == rwit) {
                            (false, false) => {
                                constraint = constraint.o(*wit);
                                match bi.minus {
                                    true => {
                                        constraint = constraint.output(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.output(-bi.coeff)
                                    }
                                }
                            }
                            (true, false) => {
                                match bi.minus {
                                    true => {
                                        constraint = constraint.left(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.left(-bi.coeff)
                                    }
                                }
                            }
                            (false, true) => {
                                match bi.minus {
                                    true => {
                                        constraint = constraint.right(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.right(-bi.coeff)
                                    }
                                }
                            },
                            _ => panic!("witness is both lwit and rwit"),
                        }
                    }
                    None => {
                        match bi_num {
                            0 => {
                                constraint = constraint.a(*wit);
                                match bi.minus {
                                    true => {
                                        constraint = constraint.left(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.left(-bi.coeff)
                                    }
                                }
                            }
                            1 => {
                                constraint = constraint.b(*wit);
                                match bi.minus {
                                    true => {
                                        constraint = constraint.right(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.right(-bi.coeff)
                                    }
                                }
                            }
                            2 => {
                                constraint = constraint.o(*wit);
                                match bi.minus {
                                    true => {
                                        constraint = constraint.output(bi.coeff)
                                    }
                                    false => {
                                        constraint = constraint.output(-bi.coeff)
                                    }
                                }
                            }
                            _ => panic!("there should be max 3 bi terms")
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
        let mut values = vec![];

        for (_, wop) in &self.vars {
            if let WitnessOrPublic::Public(pval) = wop {
                values.push((*pval).into());
            }
        }

        values
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

impl Into<Constraint> for PlangExpr {
    fn into(self) -> Constraint {
        Constraint::new()
    }
}

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