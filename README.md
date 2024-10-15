
# esp32-landscape-weather

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

This project is a rewrite of the awesome [weather_landscape](https://github.com/lds133/weather_landscape/).

## Why?

I really love the original project but didn't manage to make it work on my
hardware. Since I was eager to do a rust project on a esp for a long time, this
rewrite seemed like an enjoyable project.

## Setup

### `server` Setup

> [!NOTE]
> All commands are relative to the `server/` directory.
>
> Adjust the `config.toml` to your needs.

```sh
cp config.toml.example config.toml
cargo run
```

### `esp32` Setup

#### Tooling installation

1. Install espup:

   ```sh
   cargo install espup
   ```

   Follow the guide to set espup up:
   [espup-guide](https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html)

2. Install `espflash` and `cargo-espflash`:

   ```sh
   cargo install espflash cargo-espflash
   ```

#### Prepare config and test

> [!NOTE]
> All commands are relative to the `esp32/` directory.
>
> Adjust the `cfg.toml` to your needs.

```sh
cp cfg.toml.example cfg.toml
cargo run
```

#### Flash release build

```sh
cargo espflash --release --monitor
```

## License

The source code inside this repository is licensed under either of
[Apache License, Version 2.0](https://github.com/martinohmann/esp32-landscape-weather/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/martinohmann/esp32-landscape-weather/blob/main/LICENSE-MIT)
at your option.
