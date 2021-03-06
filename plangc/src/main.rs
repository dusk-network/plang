// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use plang::{PlangCircuit, PlangError};

use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

use dusk_bytes::{DeserializableSlice, Serializable};
use rand_core::OsRng;
use structopt::StructOpt;

use plang::dusk_plonk::circuit::{Circuit, VerifierData};
use plang::dusk_plonk::commitment_scheme::PublicParameters;
use plang::dusk_plonk::prelude::{BlsScalar, ProverKey};
use plang::dusk_plonk::proof_system::Proof;

type Result<T> = std::result::Result<T, PlangError>;

#[derive(Debug, StructOpt)]
#[structopt(name = "plangc", about = "A language for plonk circuits")]
enum Plangc {
    /// Compile the given circuit into its keys.
    Compile {
        /// The circuit to compile.
        #[structopt(parse(from_os_str))]
        circuit: PathBuf,
        /// Public parameters for compilation. If not specified random parameters will be used.
        #[structopt(long, short, parse(from_os_str))]
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
    /// Attempt to generate a proof given the parameter set.
    Prove {
        /// Circuit to solve for.
        #[structopt(parse(from_os_str))]
        circuit: PathBuf,
        /// Public parameters for verification. If not specified a file with the name of the circuit
        /// plus the extension ".pp" will be tried. If this fails random parameters will be used.
        #[structopt(long, short, parse(from_os_str))]
        params: Option<PathBuf>,
        /// Prover key generated by compiling the circuit. If not specified a file with the name of
        /// the circuit plus the extension ".pk" will be tried. If this fails the circuit will be
        /// compiled.
        #[structopt(long, short, parse(from_os_str))]
        key: Option<PathBuf>,
        /// Values to use for witnesses and public inputs.
        #[structopt(long, short, parse(try_from_str = parse_key_val))]
        vals: Vec<(String, i64)>,
        /// Where to write the proof to. If not specified the proof will be writen to a file with
        /// the name of the circuit plus the extension ".proof".
        #[structopt(long, short, parse(from_os_str))]
        output: Option<PathBuf>,
        /// The transcript to use to generate a proof with. If not specified the transcript
        /// "dusk_plang" will be used.
        #[structopt(long, short)]
        transcript: Option<String>,
    },
    /// Verify the given proof for the circuit.
    Verify {
        /// Circuit to verify proof for.
        #[structopt(parse(from_os_str))]
        circuit: PathBuf,
        /// Public parameters for verification. If not specified a file with the name of the circuit
        /// plus the extension ".pp" will be tried. If this fails random parameters will be used.
        #[structopt(long, parse(from_os_str))]
        params: Option<PathBuf>,
        /// Verifier data generated by compiling the circuit. If not specified a file with the name
        /// of the circuit plus the extension ".vd" will be tried. If this fails the circuit will be
        /// compiled.
        #[structopt(long, parse(from_os_str))]
        vdata: Option<PathBuf>,
        /// Values to use for public inputs.
        #[structopt(long, parse(try_from_str = parse_key_val))]
        vals: Vec<(String, i64)>,
        /// The proof to check.
        #[structopt(long, parse(from_os_str))]
        proof: PathBuf,
        #[structopt(long, short)]
        /// The transcript to use to generate a proof with. If not specified the transcript
        /// "dusk_plang" will be used.
        #[structopt(long, short)]
        transcript: Option<String>,
    },
}

fn parse_key_val<T, U>(s: &str) -> std::result::Result<(T, U), Box<dyn Error>>
where
    T: std::str::FromStr,
    T::Err: Error + 'static,
    U: std::str::FromStr,
    U::Err: Error + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].trim().parse()?, s[pos + 1..].trim().parse()?))
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
            let mut circuit = PlangCircuit::parse(text)?;

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
            let circuit = PlangCircuit::parse(text)?;

            let pp = PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?;

            let out = output.map_or(circuit_file.with_extension("pp"), |out| out);
            fs::write(out, &pp.to_var_bytes())?;
        }
        Plangc::Prove {
            circuit: circuit_file,
            params,
            key,
            vals,
            output,
            transcript,
        } => {
            let bytes = fs::read(&circuit_file)?;

            let text = String::from_utf8(bytes)?;
            let mut circuit = PlangCircuit::parse(text)?;

            let vals: Vec<(String, BlsScalar)> = vals
                .into_iter()
                .map(|(name, val)| {
                    (
                        name,
                        match val.is_negative() {
                            true => -BlsScalar::from((-val) as u64),
                            false => BlsScalar::from(val as u64),
                        },
                    )
                })
                .collect();
            circuit.set_vals(vals)?;

            let transcript: &'static [u8] =
                transcript.map_or(b"dusk_plang", |t| Box::leak(t.into_boxed_str()).as_bytes());

            let pp = get_pp_or_generate_and_write(&circuit, circuit_file.clone(), params)?;

            let pk = {
                match key {
                    Some(key_path) => ProverKey::from_slice(&fs::read(key_path)?)?,
                    None => match fs::read(circuit_file.with_extension("pp")) {
                        Ok(bytes) => ProverKey::from_slice(&bytes)?,
                        Err(_) => {
                            let (pk, _) = circuit.compile(&pp)?;
                            fs::write(circuit_file.with_extension("pk"), pk.to_var_bytes())?;
                            pk
                        }
                    },
                }
            };

            let proof = circuit.prove(&pp, &pk, transcript)?;

            let out = output.map_or(circuit_file.with_extension("proof"), |out| out);
            fs::write(out, &proof.to_bytes())?;
        }
        Plangc::Verify {
            circuit: circuit_file,
            params,
            vdata,
            mut vals,
            proof,
            transcript,
        } => {
            let bytes = fs::read(&circuit_file)?;

            let text = String::from_utf8(bytes)?;
            let mut circuit = PlangCircuit::parse(text)?;

            let proof = Proof::from_slice(&fs::read(proof)?)
                .map_err(|_| PlangError::Io(io::Error::from(io::ErrorKind::InvalidInput)))?;

            let transcript: &'static [u8] =
                transcript.map_or(b"dusk_plang", |t| Box::leak(t.into_boxed_str()).as_bytes());

            let pp = get_pp_or_generate_and_write(&circuit, circuit_file.clone(), params)?;

            let vd = {
                match vdata {
                    Some(key_path) => VerifierData::from_slice(&fs::read(key_path)?)?,
                    None => match fs::read(circuit_file.with_extension("vd")) {
                        Ok(bytes) => VerifierData::from_slice(&bytes)?,
                        Err(_) => {
                            let (_, vd) = circuit.compile(&pp)?;
                            fs::write(circuit_file.with_extension("vd"), vd.to_var_bytes())?;
                            vd
                        }
                    },
                }
            };

            vals.sort_by(|(name1, _), (name2, _)| Ord::cmp(name1, name2));
            let mut pinputs = Vec::with_capacity(vals.len());
            pinputs.append(
                &mut vals
                    .into_iter()
                    .map(|(_, v)| match v.is_negative() {
                        true => -BlsScalar::from((-v) as u64),
                        false => BlsScalar::from(v as u64),
                    })
                    .map(Into::into)
                    .collect(),
            );

            PlangCircuit::verify(&pp, &vd, &proof, &pinputs, transcript)?;
        }
    }

    Ok(())
}

fn get_pp_or_generate_and_write(
    circuit: &PlangCircuit,
    circuit_file: PathBuf,
    params: Option<PathBuf>,
) -> Result<PublicParameters> {
    Ok(match params {
        Some(params) => PublicParameters::from_slice(&fs::read(params)?)?,
        None => match fs::read(circuit_file.with_extension("pp")) {
            Ok(bytes) => PublicParameters::from_slice(&bytes)?,
            Err(_) => {
                let pp = PublicParameters::setup(circuit.padded_gates() << 1, &mut OsRng)?;
                fs::write(circuit_file.with_extension("pp"), &pp.to_var_bytes())?;
                pp
            }
        },
    })
}
