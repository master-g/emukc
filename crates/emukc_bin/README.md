# EmuKC_Bin

This crate contains the binary for the `EmuKC` project.

## Usage

### Bootstrap

```shell
cargo run -- bootstrap --overwrite --config ../../config.emukc.toml --log trace
```

### Cache

#### Import

```shell
cargo run -- cache import --json_path ../../z/cache/cached.json --config ../../config.emukc.toml --log trace
```

#### Check

```shell
cargo run -- cache check --dry --config ../../config.emukc.toml --log trace
```

### Serve

```shell
cargo run -- serve --config ../../config.emukc.toml --log trace
```
