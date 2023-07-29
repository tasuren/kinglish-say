use std::{fs::write, path::Path};

fn main() {
    let dist = Path::new("dist/system_tray_icon");

    if cfg!(not(debug_assertions)) || {
        !dist.join("body").exists()
        || !dist.join("dimensions").exists()
    } {
        let image = image::load_from_memory(
            if cfg!(target_os="macos") {
                include_bytes!("icon/mac/status_item.png")
            } else {
                include_bytes!("icon/other/main.ico")
            }
        )
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();

        write("dist/system_tray_icon/body", image.into_raw()).unwrap();
        write(
            "dist/system_tray_icon/dimensions",
            [width.to_be_bytes(), height.to_be_bytes()].concat()
        ).unwrap();
    }
}