# Xander: A Reinforcement Learning Environment for Dungeons & Dragons Combat 

Xander is:
- A D&D Rules Engine written in Rust :crab:, with Python bindings
- A `gymnasium`-compatible Reinforcement Learning environment

## Features

Xander supports:
- Custom creatures
- Monster Stat Blocks as JSON (hint: install xander.py and use `xander schema creature > creature.schema.json`)
- Actions (Attack, Dash, Disengage, Dodge)
- Damage: (Resistance, Immunity, Vulnerability)
- Situational Advantage and Disadvantage 
- and More!

## Structure

The project is mostly written in Rust, with extra Python modules for the AI part:
- `crates/xander` &mdash; The pluggable, modular D&D rules engine.
- `crates/d20` &mdash; A dice expression parser and roller based on [Avrae's `d20`](https://github.com/avrae/d20), written in Rust.
- `crates/xander_runtime`&mdash; Base data structures and traits used throughout Xander.
- `crates/dynx`, `crates/dynx_macros` &mdash; Type-safe registries, which Xander uses for in-game content.

## Building Xander.py

You will need:
- Python >= 3.11
- A Rust nightly toolchain (see `1.`)

1. To build `xander.py` you need to have the [`rustup`](https://rustup.rs) installed.
2. Make a new Python environment with `venv`, `conda`, etc. and `pip install maturin`
2. In the `crates/xander_py` directory, run `maturin build`
3. Maturin will output the output path of the wheel file
4. You may now distribute the file, or install it with `pip install /path/to/file/xander-X.X.X-cp311-cp311-XXXX.whl`


## Generative AI Statement
No Generative AI was used within this codebase.

This code is here just as evidence for now; it will be merged in an org repo soon.