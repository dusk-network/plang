# Plang

A language representing PLONK circuits.

## Compiler

This repository contains a compiler for a language representing PLONK circuits.
It allows circuits represented in the language to be compiled into their prover
key and verifier data.

## Usage

To compile one of the included circuits into its keys using the provided public
parameters, run the following command:

```sh
cargo run --release compile -p plang/test.pp plang/test.plang
```

Circuits are declared using a language defined in the `pest`
[grammar file](./plang/plang.pest). The contents for of a test circuit:

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

## Licensing

This code is licensed under Mozilla Public License Version 2.0 (MPL-2.0). Please
see [LICENSE](./LICENSE) for further info.

## About

Designed by the [dusk](https://dusk.network) team.

## Contributing

- If you want to contribute to this repository/project please,
  check [CONTRIBUTING.md](./CONTRIBUTING.md)
- If you want to report a bug or request a new feature addition, please open an
  issue on this repository.
