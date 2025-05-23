# weather-landscape

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
check out the original project. It has a lot of great imagery describing the
encoding principles, too.

<table>
<tr>
<td><img src="assets/box-closed.jpg" /></td>
<td><img src="assets/box-open.jpg" /></td>
</tr>
</table>

## Why?

I really fell in love with the idea behind
[`weather_landscape`][weather_landscape] when I first discovered it, but
couldn't make it work on my hardware. So I was thinking. I wanted to get my
hands dirty with esp32, but couldn't come up with a nice project to make use of
it. So this was a perfect opportunity!

## Additional features over the original

Apart from the features mentioned in the README of
[`weather_landscape`][weather_landscape], this implementation also includes:

- **Fog**: On a foggy day you will see wavelike lines below the clouds. The
  more lines there are, the heavier the fog.
- **Sleet**: Apart from rain and snow, sleet is also shown as a mixture of rain
  drops and snow flakes.
- **Thunderstorm**: Clouds spit lightnings if there's some probability of a
  thunderstorm.
- **Night mode**: At night time the colors are inverted (white scenery on black
  background).
- **Metrics**: The server provides Prometheus metrics for monitoring. I use
  these to get alerted when the battery of the esp32 died, for example.
- **Altitude**: In addition to latitude and longitude, the server also
  optionally accepts an altitude for even more precise weather data.
- **Randomness control**: The `/image.{format}` endpoint supports the boolean
  query parameter `wreck_havoc` to add a lot of randomness to the weather data
  to make it seem unpredictable. This turned out to be really useful for
  testing. Additionally, the image endpoint accepts an optional `seed` query
  parameter (`u64`) which allows passing a seed to the RNG to make the
  randomness more predictable. If absent, the RNG used to render the image is
  seeded from the system entropy source.

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

Visit `http://localhost:8080/image.{format}` to grab the rendered image of the
current weather.

Available `{format}` values are: `epd` (binary data, meant to be used by the
esp32), `png`, `gif` and `bmp`.

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
[Apache License, Version 2.0](https://github.com/martinohmann/weather-landscape/blob/main/LICENSE-APACHE)
or [MIT license](https://github.com/martinohmann/weather-landscape/blob/main/LICENSE-MIT)
at your option.

[weather_landscape]: https://github.com/lds133/weather_landscape/
