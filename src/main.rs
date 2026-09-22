#[cfg(any(feature = "desktop", feature = "mobile"))]
fn main() {
    #[cfg(all(
        feature = "desktop",
        not(feature = "mobile"),
        not(target_os = "android")
    ))]
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("WHAT / TO / EAT")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1060.0, 840.0))
                    .with_min_inner_size(dioxus::desktop::LogicalSize::new(360.0, 600.0)),
            ),
        )
        .launch(whatoeat::ui::App);
    #[cfg(any(feature = "mobile", target_os = "android"))]
    dioxus::launch(whatoeat::ui::App);
}
#[cfg(not(any(feature = "desktop", feature = "mobile")))]
fn main() {
    eprintln!("运行桌面应用：cargo run --features desktop --bin whatoeat");
}
