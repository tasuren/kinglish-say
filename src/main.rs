#![cfg_attr(not(test), windows_subsystem="windows")]

use std::{
    thread::{spawn, JoinHandle}, sync::{
        Arc, Mutex, mpsc::{SyncSender, Receiver, sync_channel}
    },
    process::{Command, Child}, cell::OnceCell
};
#[cfg(target_os="windows")]
use std::os::windows::process::CommandExt;

use tao::{
    system_tray::SystemTrayBuilder, window::Icon,
    menu::{ContextMenu, MenuItem, MenuItemAttributes},
    event_loop::{EventLoop, ControlFlow},
    event::Event
};
#[cfg(target_os="macos")]
use {
    tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS},
    objc2::{msg_send, class, runtime::Object}
};
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


struct Core {
    config: Arc<Config>,
    hotkey_manager: OnceCell<GlobalHotKeyManager>,
    clipboard: Arc<Mutex<Clipboard>>,
    child_sender: SyncSender<Child>,
    _waiter: JoinHandle<()>
}


#[cfg(target_os="windows")]
const DETACHED_PROCESS: u32 = 0x00000008;


#[inline]
fn say(config: Arc<Config>, sender: SyncSender<Child>, text: String) {
    sender.send({
        let mut cmd = Command::new(&config.command.program);
        cmd.args(config.command.args.iter().map(
            |arg| if arg.contains("{text}") {
                arg.replace("{text}", &text)
            } else { arg.clone() }
        ));
        #[cfg(target_os="windows")]
        cmd.creation_flags(DETACHED_PROCESS);
        cmd
    }.spawn().unwrap()).unwrap();
}


#[inline]
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


#[inline]
fn load_icon() -> Icon {
    let (width, height) = {
        let raw = include_bytes!("../dist/system_tray_icon/dimensions")
            .split_at(std::mem::size_of::<u32>());
        (
            u32::from_be_bytes(raw.0.try_into().unwrap()),
            u32::from_be_bytes(raw.1.try_into().unwrap())
        )
    };
    tao::system_tray::Icon::from_rgba(
        (*include_bytes!("../dist/system_tray_icon/body")).to_vec(),
        width, height
    ).expect("アイコンを開くのに失敗しました。")
}


fn main() {
    let core = Arc::new(Core::new());
    let event_loop = EventLoop::new();

    #[cfg(target_os="macos")]
    {
        // NOTE: macOSではなぜかこうしないと起動しない。
        // 詳細はここで説明：https://github.com/tauri-apps/tao/issues/774
        unsafe { let _: *const Object = msg_send![
            class!(NSApplication), sharedApplication
        ]; };
    };

    let mut menu = ContextMenu::new();

    let setting_file_item_id = menu.add_item(
        MenuItemAttributes::new(&t!("ui.system_tray.setting_file"))
    ).id();
    menu.add_native_item(MenuItem::Separator);
    let information_item_id = menu.add_item(
        MenuItemAttributes::new(&t!("ui.system_tray.info"))
    ).id();
    #[cfg(target_os="macos")]
    menu.add_native_item(MenuItem::Quit).unwrap()
        .set_title(&t!("ui.system_tray.quit"));
    #[cfg(target_os="windows")]
    let quit_id = menu.add_item(
        MenuItemAttributes::new(&t!("ui.system_tray.quit"))
    ).id();


    let _tray = SystemTrayBuilder::new(load_icon(), Some(menu))
        .build(&event_loop);


    #[cfg(target_os="macos")]
    let mut set = false;

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        #[cfg(target_os="macos")]
        if !set {
            // なぜかランタイムにActivationPolicyを変えなければクラッシュする。
            // これは恐らくイベントループの`run_return`が実行されるまで内部のデリゲートがnilとなってしまうからだろう。
            // 詳細はこちらで問い合わせ中：https://github.com/tauri-apps/tao/issues/774
            // それが解決次第、ランタイムではなくアプリケーション起動前にこれを設定するように変える。
            _event_loop.set_activation_policy_at_runtime(ActivationPolicy::Prohibited);
            set = true;
        };

        match event {
            Event::MenuEvent { menu_id, .. } => match menu_id {
                _ if menu_id == setting_file_item_id => {
                    let config = Arc::clone(&core.config);
                    opener::open(config.path.as_os_str()).unwrap();
                },
                _ if menu_id == information_item_id => {
                    let _ = AsyncMessageDialog::new()
                        .set_title(&format!(
                            "{} v{}", t!("title"),
                            env!("CARGO_PKG_VERSION")
                        ))
                        .set_description(&t!("description"))
                        .show();
                },
                _id => {
                    #[cfg(target_os="windows")]
                    if _id == quit_id { *control_flow = ControlFlow::Exit };
                }
            },
            _ => ()
        };
    });
}