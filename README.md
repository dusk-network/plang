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
cargo run --release compile -p test.pp test.plang
```

Circuits are declared using a custom language defined in the `pest`
[grammar file](./plang/plang.pest). Its contents for reference:

```text
# Equations of the form:
# 
# q_m⋅a⋅b + q_l⋅a + q_r⋅b + q_o⋅o = PI
#
# Can be processed and compiled into prover and verifier keys.
a + b = c
a * b = d
```

## Disclaimer

This is a prototype and as such not ready for production use. Use with caution.
