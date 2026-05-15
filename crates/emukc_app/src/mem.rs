cfg_select! {
    target_os = "netbsd" => {
        #[global_allocator]
        pub static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
    target_os = "openbsd" => {
        #[global_allocator]
        pub static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
    _ => {
        #[global_allocator]
        pub static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
    }
}
