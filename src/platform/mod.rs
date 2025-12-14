use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::keep_awake;
    } else if #[cfg(target_os = "macos")] {
        mod macos;
        pub use macos::keep_awake;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        pub use linux::keep_awake;
    } else {
        pub fn keep_awake() -> Result<(), String> {
            Err("unsupported platform".to_string())
        }
    }
}
