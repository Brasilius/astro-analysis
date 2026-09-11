# NOCTILUCA

A Ratatui observatory for the stars above you. Warm instrument markings, dark violet panels, drifting lavender hues, and arcane geometric glyphs give it a retro science-fiction feel. No special icon font required; use a Unicode terminal with true color, ideally 120 × 40 or larger.

## Install and run

On Linux or macOS, install [Rust and Cargo](https://rustup.rs) first (Rust 1.85
or newer is required for the 2024 edition). You also need `curl`, `tar`, and a
native linker/toolchain for Rust builds.

```sh
curl -fsSL https://raw.githubusercontent.com/Brasilius/astro-analysis/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
astro-analysis
```

The installer builds the latest `main` source with the committed dependency lockfile
and installs the executable in `~/.local/bin`, without sudo. Add the PATH line to
your shell configuration to keep the command available in new terminals. To choose
another directory, set `ASTRO_ANALYSIS_INSTALL_DIR` to an absolute path on the `sh`
side of the pipeline. Run the installer again to update; remove the installed
`astro-analysis` executable to uninstall.

Use `astro-analysis --demo` for a deterministic demo or `astro-analysis --offline`
to launch without network requests. The installer itself needs internet access.

## Run from source

```sh
cargo run --release -- --demo
cargo run --release
cargo run --release -- --lat 51.5074 --lon -0.1278
cargo run --release -- --offline --lat 51.5074 --lon -0.1278
```

The default launch requests approximate IP geolocation from **ipapi.co**, then sends coordinates to **Open-Meteo** for weather. IP location can be inaccurate, especially with a VPN. Supply `--lat` and `--lon` to bypass IP lookup, or press **L** to enter coordinates. `--offline` makes no network requests. Without coordinates, offline mode uses a clearly labeled San Francisco demo; `--demo` fixes both location and time to September 11, 2026, 06:00 UTC.

The list and map share selection. Click a star, click a list row, or scroll with the mouse wheel. The bottom controls are clickable. Press **?** for the full guide.

| Key | Action |
| --- | --- |
| ↑ / ↓, J / K | Select star |
| / | Search name, HYG identifier, or constellation abbreviation |
| L | Set latitude, longitude |
| S | Sort by brightness or altitude |
| H | Toggle minimum altitude between 0° and 20° |
| + / - | Change limiting magnitude in 0.5 steps, up to 6 |
| R | Refresh online catalog and weather |
| Esc | Clear search / dismiss input or help |
| Q / Ctrl+C | Quit |

The map is an azimuthal equidistant projection: zenith at the center, horizon at the outer ring, intermediate rings at 30° and 60° altitude. North is up and east is left, as when looking up at the sky. Only stars matching the current filters appear. The selected star is amber. No invented constellation connections are drawn.

## Data and calculations

- [HYG v4.1](https://github.com/astronexus/HYG-Database/tree/main/hyg/CURRENT): 5,070 catalog stars with apparent magnitude ≤ 6, embedded for immediate offline use. Online startup, daily refresh, and **R** fetch the archived v4.1 CSV (~34 MB); successful data replaces the in-memory catalog. The archive is deliberately versioned; newer HYG development moved to [Codeberg](https://codeberg.org/astronexus/hyg). Downloads are not persisted across sessions.
- [Open-Meteo](https://open-meteo.com/en/docs): current cloud cover and day/night status, requested every ten minutes. Its observation timestamp is shown in UTC. These conditions are model estimates, not a measurement at your telescope. Data provided under [CC BY 4.0](https://open-meteo.com/en/terms).
- [ipapi](https://ipapi.co/api/): initial approximate location when no coordinates are supplied. If lookup fails, no location is silently assumed; manual entry remains available.

Positions are calculated locally from J2000 right ascension and declination, with precession to date, approximate mean sidereal time, and spherical horizontal-coordinate conversion. Longitude is positive east. The clock and sky update while the interface runs; network work runs on a background thread with request timeouts. Failed refreshes retain the last good catalog and mark weather unavailable/stale. See the [US Naval Observatory sidereal-time reference](https://aa.usno.navy.mil/faq/GAST).

“Above horizon” means geometric altitude > 0°, not guaranteed visibility. Daylight, clouds, light pollution, terrain, atmospheric refraction, proper motion, nutation, and aberration are not incorporated into the star positions. This is a finding aid rather than telescope-pointing software. Distance is catalog parsecs converted to light-years; unknown catalog distances are labeled unavailable.

The bundled catalog is adapted from David Nash / Astronomy Nexus's HYG database, licensed **CC BY-SA 4.0**. Changes: exclude the Sun, filter magnitude ≤ 6, and retain eight fields. The derived CSV retains that license; see [data/LICENSE-HYG.md](data/LICENSE-HYG.md).

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- --snapshot
```

`--snapshot` renders a deterministic 120 × 40 text frame without entering the alternate screen or accessing the network. The tests cover coordinate reference cases, catalog integrity, filtering, location validation, mouse selection, and rendering at multiple terminal sizes. `src/astro.rs` owns astronomy; `src/sources.rs` owns background HTTP; `src/ui.rs` owns rendering; `src/main.rs` owns state and interaction.
