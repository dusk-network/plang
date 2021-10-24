use plang::error::Result;
use plang::{PlangCircuit, PlangGrammar};

use std::fs;
use std::path::PathBuf;

use structopt::StructOpt;

use rand_core::OsRng;

use plang::dusk_plonk::circuit::Circuit;
use plang::dusk_plonk::commitment_scheme::PublicParameters;

#[derive(Debug, StructOpt)]
#[structopt(name = "plang", about = "A language for plonk circuits")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let bytes = fs::read(&opt.input)?;

    let text = String::from_utf8(bytes)?;
    let grammar = PlangGrammar::new(&text)?;

    let mut circuit = PlangCircuit::from_grammar(grammar)?;

    let pp = PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?;
    let (pk, vd) = circuit.compile(&pp)?;

    fs::write(opt.input.with_extension("pk"), &pk.to_var_bytes())?;
    fs::write(opt.input.with_extension("vd"), &vd.to_var_bytes())?;

    Ok(())
}
