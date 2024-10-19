
# esp32-landscape-weather

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> [!NOTE]
> This is a personal project that scratched an itch of mine and is designed for
> my particular use case and hardware only. I don't accept contributions unless
> they improve the documentation or fix a bug.
>
> If you're using different hardware or are missing a feature, please fork it
> and get creative too :).

This is a rewrite of the awesome [`weather_landscape`][weather_landscape]
project. If you're confused what all this here is about, I highly recommend to
check out the original project. It has a lot of great imagery to offer, I
promise!

## Why?

I really fell in love with the idea behind
[`weather_landscape`][weather_landscape] when I first discovered it, but
couldn't make it work on my hardware. So I was thinking. I wanted to get my
hands dirty with esp32, but couldn't come up with a nice project to make use of
it. So this was a perfect opportunity!

## Setup

### `server` Setup

> [!NOTE]
> All commands are relative to the `server/` directory.
>
> Adjust the `config.toml` to your needs.

```sh
cp config.example.toml config.toml
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
cp cfg.example.toml cfg.toml
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

[weather_landscape]: https://github.com/lds133/weather_landscape/
