use std::{
    thread::{spawn, JoinHandle}, sync::{
        Arc, Mutex, mpsc::{SyncSender, Receiver, sync_channel}
    },
    process::{Command, Child}, cell::OnceCell
};

use tray_item::{TrayItem, IconSource};
use rfd::AsyncMessageDialog;

use global_hotkey::{
    GlobalHotKeyManager, GlobalHotKeyEvent,
    hotkey::{HotKey, Modifiers, Code}
};
use arboard::Clipboard;

use rust_i18n::{i18n, t, set_locale};

pub mod config;

use config::Config;


i18n!("locales", fallback="ja");


#[cfg(target_os="windows")]
enum Message {
    Quit
}


struct Core {
    config: Arc<Config>,
    hotkey_manager: OnceCell<GlobalHotKeyManager>,
    clipboard: Arc<Mutex<Clipboard>>,
    child_sender: SyncSender<Child>,
    _waiter: JoinHandle<()>
}


#[inline(always)]
fn say(config: Arc<Config>, sender: SyncSender<Child>, text: String) {
    sender.send(
        Command::new(&config.command.program)
            .args(config.command.args.iter().map(
                |arg| if arg.contains("{text}") {
                    arg.replace("{text}", &text)
                } else { arg.clone() }
            )).spawn().unwrap()
    ).unwrap();
}


#[inline(always)]
fn spawn_waiter(rx: Receiver<Child>) -> JoinHandle<()> {
    spawn(move || loop {
        if let Ok(child) = rx.recv() {
            if let Ok(output) = child.wait_with_output() {
                if !output.status.success() {
                    let _ = AsyncMessageDialog::new()
                        .set_title(&t!("ui.error.command_failed.title"))
                        .set_description(&t!("ui.error.command_failed.description"))
                        .show();
                };
            } else {
                let _ = AsyncMessageDialog::new()
                    .set_title(&t!("ui.error.unexpected.title"))
                    .set_description(&t!("ui.error.unexpected.description"))
                    .show();
            };
        } else { break; }
    })
}


impl Core {
    fn new() -> Self {
        let config = Config::new();
        set_locale(&config.language);

        let (tx, rx) = sync_channel(2);

        let c = Self {
            config: Arc::new(config),
            hotkey_manager: OnceCell::new(),
            clipboard: Arc::new(Mutex::new(Clipboard::new().unwrap())),
            child_sender: tx, _waiter: spawn_waiter(rx)
        };

        c.hotkey_manager.set(c.setup_hotkey()).ok().unwrap();

        c
    }

    /// ホットキーの設定をします。
    #[inline(always)]
    fn setup_hotkey(&self) -> GlobalHotKeyManager {
        let hotkey_manager = GlobalHotKeyManager::new().unwrap();

        hotkey_manager.register(HotKey::new(
            Some(Modifiers::CONTROL), Code::KeyS
        )).unwrap();

        GlobalHotKeyEvent::set_event_handler({
            let config = Arc::clone(&self.config);
            let clipboard = Arc::clone(&self.clipboard);
            let sender = self.child_sender.clone();

            Some(move |_| {
                // もし二個目のホットキーが三秒以内に押されたのなら
                say(
                    Arc::clone(&config), sender.clone(),
                    clipboard.lock().unwrap()
                        .get_text().unwrap()
                );
            })
        });

        hotkey_manager
    }
}


fn main() {
    let core = Arc::new(Core::new());

    let mut tray = TrayItem::new(&t!("title"), IconSource::Resource(""))
        .expect("常駐アイコンの初期化に失敗しました。");

    tray.add_menu_item(&t!("ui.tasktray.info"), || {
        let _ = AsyncMessageDialog::new()
            .set_title(&format!(
                "{} v{}", t!("title"),
                env!("CARGO_PKG_VERSION")
            ))
            .set_description(&t!("description"))
            .show();
    }).unwrap();
    tray.add_menu_item(
        &t!("ui.tasktray.setting_file"),
        {
            let config = Arc::clone(&core.config);
            move || opener::open(config.path.as_os_str()).unwrap()
        }
    ).unwrap();

    let inner = tray.inner_mut();

    #[cfg(target_os="macos")]
    {
        inner.add_quit_item(&t!("ui.tasktray.quit"));
        inner.display();
    }
    #[cfg(target_os="windows")]
    {
        let (tx, rx) = channel();
        tray.add_menu_item(
            &t!("ui.tasktray.quit"),
            move || tx.send(Message::Quit).unwrap()
        );

        loop {
            match rx.recv().unwrap() {
                Message::Quit => break
            }
        }
    }
    #[cfg(target_os="linux")]
    compile_error!("Linuxはまだ対応していません。");
}