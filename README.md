armake2
=======

[![](https://img.shields.io/travis/KoffeinFlummi/armake2.svg?logo=travis&style=flat)](https://travis-ci.org/KoffeinFlummi/armake2)
[![](https://img.shields.io/appveyor/ci/KoffeinFlummi/armake2.svg?logo=appveyor&style=flat)](https://ci.appveyor.com/project/KoffeinFlummi/armake2)
[![](https://img.shields.io/crates/v/armake2.svg?logo=rust&style=flat)](https://crates.io/crates/armake2)

Successor to [armake](https://github.com/KoffeinFlummi/armake) written in Rust for maintainability and memory safety, aiming to provide the same features except for the custom P3D binarization, which was never finished.

**Status:** PAA commands not implemented, some options not implemented, testing.

## Changes since armake

- New v3 signatures
- Signature verification
- Seperate `preprocess` command
- Seperate `pack` command for non-binarized PBOs instead of `build -p`
- Configs are now rapified via the `rapify` command
- Improved config parser errors
- Automatic warning truncation to prevent spam

### Performance

Performance should be equal or better than `armake` depending on modification makeup and environment. More is done in-memory, reducing disk I/O at the expense of memory usage. Especially during binarization, less copies are performed, resulting in much faster builds for asset-heavy modifications or users without SSDs.

#### BWMod build benchmarks

**armake1:**

```
Time (mean ± σ):     676.463 s ± 17.609 s    [User: 1.5 ms, System: 3.9 ms]
Range (min … max):   653.793 s … 706.619 s
```

**armake2:**

```
Time (mean ± σ):     434.666 s ±  1.109 s    [User: 0.0 ms, System: 4.1 ms]
Range (min … max):   433.415 s … 435.526 s
```

**Speedup:** 1.56

#### ACE3 build benchmarks

[`da7bb856f`](https://github.com/acemod/ACE3/commit/da7bb856fb6e699d66b0ff2d0da92e65726a9305)

**armake1:**

```
Time (mean ± σ):     110.083 s ±  2.772 s    [User: 4.9 ms, System: 16.8 ms]
Range (min … max):   108.270 s … 113.274 s
```

**armake2:**

```
Time (mean ± σ):     98.190 s ±  0.452 s    [User: 0.0 ms, System: 13.6 ms]
Range (min … max):   97.767 s … 98.666 s
```

**Speedup:** 1.12

(all benchmarks performed with 4 threads on a 4 core VM on an i5-8600K)

## Building

The build requires `cargo`, Rust's package manager and the OpenSSL development libraries.
To compile and run, use:

```
cargo run
```

To build a release, use:

```
cargo build --release
```

In order to build, you'll need to have OpenSSL installed on your system.

On **Linux**, the easiest way is to install OpenSSL via your system's package manager (if it is not installed already). Make sure you also have the development packages of OpenSSL installed. For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

On **Windows**, the easiest way to get compilation and static linking of OpenSSL to work is to download [pre-compiled OpenSSL binaries](http://slproweb.com/products/Win32OpenSSL.html) (non-light, 64-bit) and set the following environment variables:

- `OPENSSL_DIR=C:\OpenSSL-WIN64`
- `OPENSSL_STATIC=1`
- `OPENSSL_LIBS=libssl_static:libcrypto_static`

## Testing

Unit tests:

```
cargo test
```

The end-to-end DayZ test harness builds a small test mod, binarizes it, packs
it into a PBO, signs it, and boots a local DayZ server (Steam app 223350, plus
app 1042420 with `--both`) to prove the mod loads and runs:

```
testharness/run.sh --both
```

See [testharness/README.md](testharness/README.md) for details.

## Usage

```
armake2

Usage:
    armake2 rapify [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 preprocess [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-v] [-f] [-d <indentation>] [<source> [<target>]]
    armake2 binarize [-v] [-f] [-w <wname>]... <source> <target>
    armake2 build [-v] [-f] [--proton-binarize] [-w <wname>]... [-i <includefolder>]... [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 pack [-v] [-f] <sourcefolder> [<target>]
    armake2 inspect [-v] [<source>]
    armake2 unpack [-v] [-f] <source> <targetfolder>
    armake2 cat [-v] <source> <filename> [<target>]
    armake2 keygen [-v] [-f] <keyname>
    armake2 sign [-v] [-f] [--v2] <privatekey> <pbo> [<signature>]
    armake2 verify [-v] <publickey> <pbo> [<signature>]
    armake2 paa2img [-v] [-f] <source> <target>
    armake2 img2paa [-v] [-f] [-c] [-t <paatype>] <source> <target>
    armake2 (-h | --help)
    armake2 --version
```

See `armake2 --help` for more.

## Binarizing models on Linux

BI's `binarize.exe` only runs on Windows, so on Linux `build` copies `.p3d`/`.rtm` files as-is (with the `non-windows-binarization` warning). With `--proton-binarize` (and for `armake2 binarize`) armake2 runs the DayZ Tools `binarize.exe` under Proton instead. Config and rvmat rapification is unchanged.

Requirements: DayZ Tools (Steam app 830640) and any Proton installed through Steam.

- `STEAM_ROOT`: Steam directory. Default: first of `~/.steam/debian-installation`, `~/.steam/steam`, `~/.local/share/Steam` that has `steamapps/common`.
- `DAYZ_TOOLS`: DayZ Tools directory. Default: `<steam>/steamapps/common/DayZ Tools` (uses `Bin/Binarize/binarize.exe`).
- `PROTON`: Proton directory (containing the `proton` script). Default: `Proton Hotfix`, `Proton - Experimental`, `Proton Experimental`, then any `Proton*` in `steamapps/common`.
- `BIOUTPUT=1`: show the output of Proton/binarize.exe.

armake2 builds a throw-away sandbox in the temp directory (a copy of `Binarize`, a fresh Wine prefix and an output folder) once per run and removes it at the end. Wine's default `Z:` drive maps to `/`, and `binarize.exe` walks directory trees from the drive root, so it would run for minutes on a real home directory; the sandbox therefore points `Z:` at itself. The first model of a run is slow because the Wine prefix has to be created (up to a few minutes on a cold start). Nothing in the Steam installation is modified.

`binarize.exe` resolves texture and material paths (`my\mod\data\x_ca.paa`) relative to the project root, and only sets a model's alpha flags if it can read the texture. armake2 therefore gives the sandbox a `P:` drive with the addon linked at its PBO prefix (`P:\my\mod` → the source folder) and runs `binarize.exe` from `P:\`. Paths outside the addon (vanilla `dz\...`) stay unresolved and are reported as `binarize-missing-texture`. The source folder is linked, not copied, so `binarize.exe` can read and in principle write it; its output goes to the sandbox, and the source is not modified.

`armake2 binarize <source> <target>` has no PBO to take the prefix from: it treats the model's folder as the addon and uses the folder name as the prefix. That only works for a model in the addon root of a single-component prefix; for anything else (a model in `data\`, or a prefix like `my\mod`) use `build --proton-binarize`, which takes the prefix from `$PBOPREFIX$`.
