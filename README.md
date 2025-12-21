# Emul8rs

A chip-8 emulator (or interpreter if you want to be pedantic) written in rust.
See
[Guide to making a CHIP-8 emulator](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/)
or [Wikipedia CHIP-8](https://en.wikipedia.org/wiki/CHIP-8) for more information
about CHIP-8 and its history.

This emulator is tested against
[Timendus's chip8-test-suite](https://github.com/Timendus/chip8-test-suite), and
passes all the testing ROMs with the exception of 2 of the quirks in the quirks
test.

## Usage

This crate includes both a library and an executable, the library is essentially
an emulator backend that handles all the instruction opcodes, and updating
timers, drawing sprites to an internal display variable, etc. which can be
combined with a front-end that actually handles the drawing, interacting with
the keyboard, and playing sounds.

The backend can be used as a library by adding it as a dependency. To create a
new front-end, just create a struct implementing the Frontend trait, and pass
that to the emulator's `new` function.

The executable adds a front-end made using Raylib, and can be installed using
cargo (see
[cargo installation](https://doc.rust-lang.org/cargo/getting-started/installation.html)
for details on this). The Raylib front-end (shockingly) depends on Raylib, so
you'll need to install some additional dependencies that Raylib has prior to
installing the executable.

The build dependencies for the raylib-rs crate are:

- glfw
- cmake
- curl

When installing it will pull in the raylib c library, so you'll also need
build/runtime dependencies for that, see the
[raylib wiki](https://github.com/raysan5/raylib/wiki). (Honestly, you can mostly
just try to install it and it will tell you what you are missing, but ymmv).

Once the dependencies are installed, install the executable by calling:

```{bash}
cargo install --git https://github.com/Braden-Griebel/emul8rs.git
```

and then run (assuming the cargo install directory is on your path):

```{bash}
# Run a ROM
emul8rs path-to-chip8-rom
# See CLI help
emul8rs --help
```

The executable has a variety of configuration options, with a TOML configuration
file located at XDG_CONFIG_HOME/emul8rs/emul8rs.toml (which will be
automatically created and populated with default values if it doesn't exist).

The default config is:

```{toml}
instructions_per_second = 700 # Number of instructions to try and execute per second
foreground = "000000" # Color to use for cells/pixels that are on
background = "FFFFFF" # Color to use for cells/pixels that are off
# Configuration of some quirks of different Chip8 implementations
shift_use_vy = true
jump_offset_use_v0 = true
store_memory_update_index = false
```

and all of the options can also be over-ridden by passing them as command line
arguments (run `emul8rs --help` to see the various command line arguments).

## Configuration

In addition to configuration of the speed of the interpreter and the background
and foreground colors, you are also able to configure some quirks of the
emulation to hopefully be able to run most Chip-8 ROMs that you find (not
necessarily all, but if you find that doesn't work as expected you can report an
issue with a link to the ROM in question).

Since the opcodes are 2-bytes in width, broken into 4 half-byte parts, the
notation followed below is that each instruction is made up of IXYN, where each
letter represents a half-byte, I is describing which instruction the opcode
performs, the X and Y are two register addresses, and N is an immediate number.
Additionally, NN refers to the second byte, and NNN references to the second,
third and fourth half-bytes (the last 12 bits of the 2-byte instruction).

These options are:

- shift_use_vy: For the bit-shift opcodes, should the value of the X register be
  first set to the value in the Y register, or should the value of the X
  register be shifted in place
- jump_offset_use_v0: When performing a jump with offset instruction, should the
  offset be the value in the 0 register, or should it be the value in the X
  register.
- store_memory_update_index: Whether the Index register should be updated during
  storing or loading registers into/from memory.

The defaults for all of these should reflect more modern behavior, and should
work for most ROMs, but may need to be tweaked depending on the behavior of the
emulator the ROM is assuming.

## Licensing

All code written for the interpreter is licensed under the MIT license. The test
ROM in the resources directory is from
[corax89/chip8-test-rom](https://github.com/corax89/chip8-test-rom) and is also
licensed under the
[MIT license](https://github.com/corax89/chip8-test-rom/blob/master/LICENSE).
This crate uses the following dependencies:

- anyhow: Licensed under the
  [Apache-2.0 License](https://github.com/dtolnay/anyhow?tab=Apache-2.0-1-ov-file)
- cfg-if: Licensed under the
  [MIT license](https://github.com/rust-lang/cfg-if/blob/main/LICENSE-MIT)
- clap: Licensed under the
  [MIT license](https://github.com/clap-rs/clap?tab=MIT-2-ov-file)
- colog: Licensed under the
  [LGPL-v3](https://github.com/chrivers/rust-colog?tab=License-1-ov-file)
- confy: Licensed under the
  [MIT license](https://github.com/rust-cli/confy?tab=MIT-3-ov-file)
- log: Licensed under the
  [MIT license](https://github.com/rust-lang/log?tab=MIT-2-ov-file)
- rand: Licenced under the
  [MIT license](https://github.com/rust-random/rand?tab=MIT-2-ov-file)
- raylib: Licensed under the
  [MIT license](https://github.com/deltaphc/raylib-rs?tab=License-1-ov-file)
- serde: Licensed under the
  [MIT license](https://github.com/serde-rs/serde?tab=MIT-2-ov-file)

## References for Chip-8 Opcodes

- [Wikipedia](https://en.wikipedia.org/wiki/CHIP-8#Opcode_table)
- [Guide to making a CHIP-8 emulator](https://tobiasvl.github.io/blog/write-a-chip-8-emulator/)
