// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;

use plang::dusk_plonk::prelude::*;
use plang::{PlangCircuit, PlangError};

type Result<T> = std::result::Result<T, PlangError>;

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

        let constraint = Constraint::new().left(1).right(1).public(-self.c).a(a).b(b);

        composer.append_gate(constraint);

        let constraint = Constraint::new().mult(1).public(-self.d).a(a).b(b);

        composer.append_gate(constraint);

        Ok(())
    }

    fn public_inputs(&self) -> Vec<PublicInputValue> {
        vec![self.c.into(), self.d.into()]
    }

    fn padded_gates(&self) -> usize {
        1 << 3
    }
}

#[test]
fn produces_same_keys() -> Result<()> {
    let bytes = fs::read("./test.plang")?;

    let text = String::from_utf8(bytes)?;
    let mut circuit = PlangCircuit::parse(text)?;

    let pp = PublicParameters::from_slice(&fs::read("./test.pp")?)?;
    let (pk, vd) = circuit.compile(&pp)?;

    let mut circuit = TestCircuit::default();
    let (tpk, tvd) = circuit.compile(&pp)?;

    assert_eq!(pk.to_var_bytes(), tpk.to_var_bytes());
    assert_eq!(vd.to_var_bytes(), tvd.to_var_bytes());

    Ok(())
}

#[test]
fn produces_same_valid_proof() -> Result<()> {
    let bytes = fs::read("./test.plang")?;

    let text = String::from_utf8(bytes)?;
    let mut circuit = PlangCircuit::parse(text)?;

    let pp = PublicParameters::from_slice(&fs::read("./test.pp")?)?;
    let (pk, vd) = circuit.compile(&pp)?;

    // Solution to `test.plang`
    let vals = vec![
        ("a".to_owned(), 1),
        ("b".to_owned(), 1),
        ("c".to_owned(), 2),
        ("d".to_owned(), 1),
    ];

    circuit.set_vals(vals)?;

    let proof = circuit.prove(&pp, &pk, b"test")?;

    let mut circuit = TestCircuit {
        a: 1.into(),
        b: 1.into(),
        c: 2.into(),
        d: 1.into(),
    };

    let prooft = circuit.prove(&pp, &pk, b"test")?;
    assert_eq!(proof, prooft);

    TestCircuit::verify(
        &pp,
        &vd,
        &proof,
        &[BlsScalar::from(2).into(), BlsScalar::from(1).into()],
        b"test",
    )?;
    TestCircuit::verify(
        &pp,
        &vd,
        &prooft,
        &[BlsScalar::from(2).into(), BlsScalar::from(1).into()],
        b"test",
    )?;

    Ok(())
}
