
# esp32-landscape-weather

![image](media/logo_small.jpg)

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

>_This project is currently WIP._
![image](media/weather_goat.bmp)

It gets local weather data to prepesent on a epaper (eInk) display. Inspired by
the awesome idea of [weather_landscape](https://github.com/lds133/weather_landscape/)
this project aims to deliver an innovative way of presenting the weather. It
aims to be intuitive and understandable by a glance.


## Setup

### Quick Server Setup

Adjust the config.toml to your needs.

```bash
cp server/config.toml.example server/config.toml
cd server
cargo run
```

### ESP Setup

#### Install

[rustup](https://rustup.rs/)

Then install rust (in this case stable version)

```sh
rustup default stable
```

Next install espup

```sh
cargo install espup
```

Follow the guide to set espup up:
[espup-guide](https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html)

#### Set Up Toolchain and Install ESPFlash

```
rustup target add xtensa-esp32-espidf

cargo install espflash
```

#### Build Image

##### Prepare Image Config

```bash
cp esp32/cfg.toml.example esp32/cfg.toml
```
> Change the values according to your setup

##### Build Release (For Production)

```bash
cargo build --release --target xtensa-esp32-espidf
```
Image will be stored at:
`target/xtensa-esp32-espidf/release/esp32-landscape-weather`

##### Build Debug (For Development)

```bash
cargo build --target xtensa-esp32-espidf
```

Image will be stored at:
`target/xtensa-esp32-espidf/release/esp32-landscape-weather`

##### Build Clean

It is good practice to get rid of the data from old builds:

```bash
cargo clean
```

#### Flash Image to ESP

```bash
espflash flash target/xtensa-esp32-espidf/release/esp32-landscape-weather
```

You can connect to your ESP using `picocom`

```bash
picocom -b 115200 /dev/ttyUSB0 
```

> To disconnect press Control + A + X

## License

The source code inside this repository is licensed under either of
[Apache License, Version 2.0](https://github.com/martinohmann/esp32-landscape-weather/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/martinohmann/esp32-landscape-weather/blob/main/LICENSE-MIT)
at your option.
