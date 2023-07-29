use std::fs::{read_to_string, write};
#[cfg(target_os="windows")]
use std::fs::create_dir_all;
#[cfg(not(target_os="windows"))]
use std::fs::create_dir;

use rfd::MessageDialog;
use rust_i18n::t;

use directories::ProjectDirs;
use serde::{Serialize, Deserialize};

use smallvec::{SmallVec, smallvec};
use toml::to_string;


#[derive(Serialize, Deserialize)]
pub struct Command {
    pub program: String,
    pub args: SmallVec<[String;5]>
}


#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: std::path::PathBuf,
    pub language: String,
    pub command: Command
}


impl Config {
    pub fn new() -> Self {
        let base = ProjectDirs::from("jp", "tasuren", "kinglish").unwrap();

        if !base.config_local_dir().exists() {
            #[cfg(target_os="windows")]
            create_dir_all(base.config_local_dir()).unwrap();
            #[cfg(not(target_os="windows"))]
            create_dir(base.config_local_dir()).unwrap();
        };

        let path = base.config_local_dir().join("main.toml");

        if path.exists() {
            let mut c: Self = toml::from_str(
                &read_to_string(&path).unwrap()
            ).unwrap_or_else(|e| {
                MessageDialog::new()
                    .set_title("Parsing error")
                    .set_description(&t!(
                        "misc.parse_error",
                        path=path.display(),
                        error_code=e
                    ))
                    .show();
                let _ = opener::open(&path);
                panic!("{e:?}");
            });

            c.path = path;
            c
        } else {
            let c = Self {
                path: path, language: "ja".to_string(),
                command: if cfg!(target_os="macos") {
                    Command {
                        program: "say".to_string(),
                        args: smallvec![
                            "-v".to_string(),
                            "Samantha".to_string(),
                            "{text}".to_string()
                        ]
                    }
                } else if cfg!(target_os="windows") {
                    Command {
                        program: "wsay".to_string(),
                        args: smallvec![
                            "-v".to_string(),
                            "1".to_string(),
                            "{text}".to_string()
                        ]
                    }
                } else { unimplemented!("現在、macOSとWindows以外は非対応です。") }
            };
            write(&c.path, to_string(&c).unwrap()).unwrap();
            c
        }
    }
}