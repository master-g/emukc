//! Forces dynamic linking of `EmuKC`.

#[expect(
    unused_imports,
    reason = "the crate exists only to link `emukc_internal` as a dylib; nothing names the import"
)]
#[expect(
    clippy::single_component_path_imports,
    reason = "a single-segment `use` is the way to force the dependency to link"
)]
use emukc_internal;
