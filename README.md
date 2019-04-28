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

## Usage

```
armake2

Usage:
    armake2 rapify [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 preprocess [-v] [-f] [-w <wname>]... [-i <includefolder>]... [<source> [<target>]]
    armake2 derapify [-v] [-f] [-d <indentation>] [<source> [<target>]]
    armake2 binarize [-v] [-f] [-w <wname>]... <source> <target>
    armake2 build [-v] [-f] [-w <wname>]... [-i <includefolder>]... [-x <excludepattern>]... [-e <headerext>]... [-k <privatekey>] [-s <signature>] <sourcefolder> [<target>]
    armake2 pack [-v] [-f] <sourcefolder> [<target>]
    armake2 inspect [-v] [<source>]
    armake2 unpack [-v] [-f] <source> <targetfolder>
    armake2 cat [-v] <source> <filename> [<target>]
    armake2 keygen [-v] [-f] <keyname>
    armake2 sign [-v] [-f] [-s <signature>] [--v2] <privatekey> <pbo> [<signature>]
    armake2 verify [-v] <publickey> <pbo> [<signature>]
    armake2 paa2img [-v] [-f] [<source> [<target>]]
    armake2 img2paa [-v] [-f] [-z] [-t <paatype>] [<source> [<target>]]
    armake2 (-h | --help)
    armake2 --version
```

See `armake2 --help` for more.
