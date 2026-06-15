# NES Emulator in Rust

🚧 UNDER CONSTRUCTION 🚧

We're building this out in the open. The CPU emulator works but we haven't implemented graphics or audio yet.

[![Build Status](https://travis-ci.org/fdb/emunes.svg?branch=master)](https://travis-ci.org/fdb/emunes)

## Installation (Ubuntu)

```
# Install dependencies
sudo apt install curl libsdl2-dev libsdl2-ttf-dev clang git

# Install Rust, if needed
curl https://sh.rustup.rs -sSf | sh

# Clone the repo
git clone https://github.com/fdb/emunes.git

# Cd into the repo to run the application
cd emunes
```

## Installation (macOS)

```
# Install dependencies (Homebrew)
brew install sdl2 sdl2_ttf

# Install Rust, if needed
curl https://sh.rustup.rs -sSf | sh

# Clone the repo
git clone https://github.com/fdb/emunes.git
cd emunes
```

On Apple Silicon, Homebrew installs into `/opt/homebrew`, which is not on the
default linker search path. This repo's `.cargo/config.toml` already adds it,
so no extra steps are needed.

## Running

    cargo run <romfile.nes>

## Testing

To test the "golden master" [nestest rom](http://www.qmtpro.com/~nes/misc/nestest.txt):

    cargo test

## Credits

- [Frederik De Bleser](https://github.com/fdb)
- [Michael Smith](https://github.com/michaelshmitty)

