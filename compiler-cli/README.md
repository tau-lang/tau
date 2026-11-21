
# tau-cli

[<img alt="crates.io" src="https://img.shields.io/crates/v/tau_lang_cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tau-lang-cli)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tau_lang_cli-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/tau-lang-cli)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/quote/ci.yml?branch=master&style=for-the-badge" height="20">](https://github.com/tau-lang/tau/actions?query=branch%3Amain)

This is the default CLI for the Tau programming language.

## Usage

**Usage**: **`Usage`** \[**`--out-dir`**=_`DIR`_\] \[**`-o`**=_`FILENAME`_\] \[**`--crate-name`**=_`NAME`_\] \[**`--module-name`**=_`NAME`_\] \[**`-C`**=_`GEN`_\] \[**`--emit`**=_`ARG`_\] _`PATH`_

**Available options:**
- **`    --out-dir`**=_`DIR`_ &mdash; 
  Write output to compiler-chosen filename in DIR
- **`-o`**, **`--output`**=_`FILENAME`_ &mdash; 
  Write output to FILENAME
- **`    --crate-name`**=_`NAME`_ &mdash; 
  Specify the NAME of the crate being built
- **`    --module-name`**=_`NAME`_ &mdash; 
  Specify the NAME of the module being built
- **`-C`**, **`--codegen`**=_`GEN`_ &mdash; 
  Set the code generator
- **`    --emit`**=_`ARG`_ &mdash; 
  Comma separeted list of types of output for the compiler to emit.
- **`-h`**, **`--help`** &mdash; 
  Prints help information



## Optional features

- `c` _(enabled by default)_ – Transpiler for Tau to C code
- `qbe` _(enabled by default)_ – Compiles the Tau ir to [QBE](https://c9x.me/compile/)
- `bright-color` – Enables bright coloring
- `dull-color` – Enables dull coloring

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md).

## License

Tau is licensed under the EUPLv1.2.

See [LICENSE](../LICENSE) for details.
