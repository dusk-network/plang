use std::fs;

use plang::{PlangCircuit, PlangGrammar};
use plang::error::Result;
use plang::dusk_plonk::prelude::*;

use rand_core::OsRng;

#[derive(Default)]
struct TestCircuit {
    a: BlsScalar,
    b: BlsScalar,
    c: BlsScalar,
    d: BlsScalar,
}

impl Circuit for TestCircuit {
    const CIRCUIT_ID: [u8; 32] = [0u8; 32];

    fn gadget(&mut self, composer: &mut TurboComposer) -> std::result::Result<(), Error> {
        let a = composer.append_witness(self.a);
        let b = composer.append_witness(self.b);

        let constraint = Constraint::new()
            .left(1)
            .right(1)
            .public(-self.c)
            .a(a)
            .b(b);

        composer.append_gate(constraint);

        let constraint = Constraint::new()
            .mult(1)
            .public(-self.d)
            .a(a)
            .b(b);

        composer.append_gate(constraint);

        Ok(())
    }

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        vec![self.c.into()]
    }

    fn padded_gates(&self) -> usize {
        1 << 3
    }
}

#[test]
fn produces_same_as_test() -> Result<()> {
    let bytes = fs::read("test.plang")?;

    let text = String::from_utf8(bytes)?;
    let grammar = PlangGrammar::new(&text)?;

    let mut circuit = PlangCircuit::from_grammar(grammar)?;

    let pp = PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?;
    let (pk, vd) = circuit.compile(&pp)?;

    let mut circuit = TestCircuit::default();
    let (tpk, tvd) = circuit.compile(&pp)?;

    assert_eq!(pk.to_var_bytes(), tpk.to_var_bytes());
    assert_eq!(vd.to_var_bytes(), tvd.to_var_bytes());

    Ok(())
}