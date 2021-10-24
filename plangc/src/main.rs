use plang::error::Result;
use plang::{PlangCircuit, PlangGrammar};

use std::fs;
use std::path::PathBuf;

use structopt::StructOpt;

use rand_core::OsRng;

use plang::dusk_plonk::circuit::Circuit;
use plang::dusk_plonk::commitment_scheme::PublicParameters;

#[derive(Debug, StructOpt)]
#[structopt(name = "plangc", about = "A language for plonk circuits")]
enum Plangc {
    /// Compile the given circuit.
    Compile {
        /// The circuit to compile.
        #[structopt(parse(from_os_str))]
        circuit: PathBuf,
        /// Public parameters for compilation. If not specified random parameters will be used.
        #[structopt(short, parse(from_os_str))]
        params: Option<PathBuf>,
        /// The file name of the generated keys, excluding the extensions ".vd" and "pk".
        #[structopt(long, short, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    /// Generate random public parameters to use with compilation of a circuit.
    GenerateParams {
        /// Circuit to generate public parameters for.
        #[structopt(parse(from_os_str))]
        circuit: PathBuf,
        /// Where to write the public parameters. If not specified the public parameters will be
        /// written to a file with the name of circuit plus the extension ".pp".
        #[structopt(long, short, parse(from_os_str))]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let opt = Plangc::from_args();

    match opt {
        Plangc::Compile {
            circuit: circuit_file,
            params,
            output,
        } => {
            let bytes = fs::read(&circuit_file)?;

            let text = String::from_utf8(bytes)?;
            let grammar = PlangGrammar::new(&text)?;

            let mut circuit = PlangCircuit::from_grammar(grammar)?;

            let pp = match params {
                Some(params) => PublicParameters::from_slice(&fs::read(params)?)?,
                None => PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?,
            };
            let (pk, vd) = circuit.compile(&pp)?;

            let out = output.map_or(circuit_file, |out| out);
            fs::write(out.with_extension("pk"), &pk.to_var_bytes())?;
            fs::write(out.with_extension("vd"), &vd.to_var_bytes())?;
        }
        Plangc::GenerateParams {
            circuit: circuit_file,
            output,
        } => {
            let bytes = fs::read(&circuit_file)?;

            let text = String::from_utf8(bytes)?;
            let grammar = PlangGrammar::new(&text)?;

            let circuit = PlangCircuit::from_grammar(grammar)?;
            let pp = PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?;

            let out = output.map_or(circuit_file.with_extension("pp"), |out| out);
            fs::write(out, &pp.to_var_bytes())?;
        }
    }

    Ok(())
}
