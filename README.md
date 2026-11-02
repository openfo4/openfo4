# OpenFO4

A reimplementation of Creation Engine version targeting Fallout 4.

OpenFO4 currently primarily targets version 1.10.163.0 (Pre-NextGen / GOG version).
Newer version should be compatible, but are currently untested.

## Status

OpenFO4 is still in a very early stage of development.
You can load into the world and traverse it, including interior cells.
Most nif scenes render correctly (with the exception of animation sequences) and render closely to how they render ingame.
Animations load and render correctly, however some used behaviors are not implemented.
The framework for papyrus scripts exists and all scripts used in the Fallout 4 basegame compile.
Most native papyrus functions are unimplemented.
AI, audio and UI are not yet implemented.

## Building from source

Install the build prerequisites:
1. A latest stable Rust toolchain
2. Python 3

In the project root run:
```
./x.py
```

The output will be placed in the `build` directory.

## Usage

For the most straight forward usage place the `openfo4_client[.exe]` binary into your `Fallout 4` installation directory,
i.e. in the same directory as `Fallout4.exe`.
Note that OpenFO4 does not use `plugins.txt` or an equivalent file and will pick up all `esp` and `ba2` files in the current directory.

For more advanced usage you may use the `--data <DATA>` arg to specify a list of files the game will load.
`<DATA>` may refer to a `esp` or `ba2` file or a directory of loose files.
To load multiple sources specify `--data` multiple times. The sources will be loaded in the order specified.

## AI/LLM Policy

We do not accept AI generated content.
Please do not create issues or PRs containing AI generated comments, code or documentation.

