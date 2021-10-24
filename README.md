# Plang

A language representing PLONK circuits.

## Compiler

This repository contains a compiler for a language representing PLONK circuits.
It allows circuits represented in the language to be compiled into their prover
key and verifier data.

## Usage
To compile the included circuit into its keys using random public parameters,
run the following command:

```sh
cargo run --release -- test.plang
```

