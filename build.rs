#[cfg(target_os="windows")]
use {
    std::{fs::File, path::Path},
    tauri_winres::WindowsResource,
    reqwest::blocking::get
};


fn main() {
    #[cfg(target_os="windows")]
    {
        if !Path::new("./wsay.exe").exists() {
            println!("Downloading `wsay.exe`...");
            let mut r = get("https://github.com/p-groarke/wsay/releases/latest/download/wsay.exe").unwrap();
            if !r.status().is_success() {
                panic!(
                    "Failed to download wsay application with status code {}. \
                    If you are temporarily unable to download the file, place your own `wsay.exe` in the current directory.",
                    r.status().as_str()
                );
            };
            r.copy_to(&mut File::create("wsay.exe").unwrap()).expect(
                "If you are temporarily unable to download the file, place your own `wsay.exe` in the current directory."
            );
        };

        let mut res = WindowsResource::new();
        res.set_icon("icon/other/main.ico");
        res.append_rc_content("main-icon ICON \"icon/other/main.ico\"");
        res.compile().unwrap();
    }
}