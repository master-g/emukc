use std::sync::LazyLock;

pub const LOGO: &str = r#"
▓█████  ███▄ ▄███▓ █    ██  ██ ▄█▀ ▄████▄
▓█   ▀ ▓██▒▀█▀ ██▒ ██  ▓██▒ ██▄█▒ ▒██▀ ▀█
▒███   ▓██    ▓██░▓██  ▒██░▓███▄░ ▒▓█    ▄
▒▓█  ▄ ▒██    ▒██ ▓▓█  ░██░▓██ █▄ ▒▓▓▄ ▄██▒
░▒████▒▒██▒   ░██▒▒▒█████▓ ▒██▒ █▄▒ ▓███▀ ░
░░ ▒░ ░░ ▒░   ░  ░░▒▓▒ ▒ ▒ ▒ ▒▒ ▓▒░ ░▒ ▒  ░
 ░ ░  ░░  ░      ░░░▒░ ░ ░ ░ ░▒ ▒░  ░  ▒
   ░   ░      ░    ░░░ ░ ░ ░ ░░ ░ ░
   ░  ░       ░      ░     ░  ░   ░ ░
"#;

/// What is the runtime thread memory stack size (defaults to 10MiB)
pub static RUNTIME_STACK_SIZE: LazyLock<usize> = LazyLock::new(|| {
    // Stack frames are generally larger in debug mode.
    let default = if cfg!(debug_assertions) {
        20 * 1024 * 1024 // 20MiB in debug mode
    } else {
        10 * 1024 * 1024 // 10MiB in release mode
    };
    option_env!("EMUKC_RUNTIME_STACK_SIZE").and_then(|s| s.parse::<usize>().ok()).unwrap_or(default)
});

/// How many threads which can be started for blocking operations (defaults to 512)
pub static RUNTIME_MAX_BLOCKING_THREADS: LazyLock<usize> = LazyLock::new(|| {
    option_env!("EMUKC_RUNTIME_MAX_BLOCKING_THREADS")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(512)
});

/// The version identifier of this build
pub static PKG_VERSION: LazyLock<String> =
    LazyLock::new(|| match option_env!("EMUKC_BUILD_METADATA") {
        Some(metadata) if !metadata.trim().is_empty() => {
            let version = env!("CARGO_PKG_VERSION");
            format!("{version}+{metadata}")
        }
        _ => env!("CARGO_PKG_VERSION").to_owned(),
    });
