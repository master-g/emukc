#[cfg(target_os = "android")]
#[global_allocator]
pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "freebsd")]
#[global_allocator]
pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "ios")]
#[global_allocator]
pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "macos")]
#[global_allocator]
pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "netbsd")]
#[global_allocator]
pub static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(target_os = "openbsd")]
#[global_allocator]
pub static ALLOC: tivk_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
