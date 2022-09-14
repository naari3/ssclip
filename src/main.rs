#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod watcher;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{borrow::Cow, path::Path};

use arboard::{Clipboard, ImageData};
use crossbeam_channel::{unbounded, Sender};
use notify::{RecommendedWatcher, Watcher};
use trayicon::*;
use windows::core::HSTRING;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONSTOP};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage, MB_OK, MSG},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::config::Config;
use crate::watcher::WatchKind;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum TrayEvents {
    ClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    OpenSettings,
    ToggleAutorun,
}

fn main() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = Path::new("Software")
        .join("Microsoft")
        .join("Windows")
        .join("CurrentVersion")
        .join("Run");
    let (key, disp) = hkcu.create_subkey(&path).unwrap();
    dbg!(&disp);
    let is_exists = match key.get_value::<String, &str>(APP_NAME) {
        Ok(_) => true,
        Err(_) => false,
    };
    dbg!(is_exists);

    let watcher_map: HashMap<PathBuf, RecommendedWatcher> = HashMap::new();
    let watcher_map = Arc::new(Mutex::new(watcher_map));

    let (s, r) = std::sync::mpsc::channel::<TrayEvents>();
    let icon = include_bytes!("../icon/icon.ico");

    // Needlessly complicated tray icon with all the whistles and bells
    let mut tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip(APP_NAME)
        .on_click(TrayEvents::ClickTrayIcon)
        .on_double_click(TrayEvents::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("O&pen Settings", TrayEvents::OpenSettings)
                .separator()
                .checkable(
                    "Start ssclip on system startup",
                    is_exists,
                    TrayEvents::ToggleAutorun,
                )
                .separator()
                .item("E&xit", TrayEvents::Exit),
        )
        .build()
        .unwrap();

    let (reload_tx, reload_rx) = unbounded();

    let reload_tx2 = reload_tx.clone();
    std::thread::spawn(move || {
        r.iter().for_each(|m| match m {
            TrayEvents::DoubleClickTrayIcon => {
                println!("Double click");
            }
            TrayEvents::ClickTrayIcon => {
                reload_tx2.send(()).unwrap();
                println!("Single click");
            }
            TrayEvents::Exit => {
                std::process::exit(0);
            }
            TrayEvents::OpenSettings => {
                // TODO: Open Settings GUI
                open::that(Config::get_config_path()).unwrap();
            }
            TrayEvents::ToggleAutorun => {
                let state = tray_icon
                    .get_menu_item_checkable(TrayEvents::ToggleAutorun)
                    .unwrap();
                let new_state = !state;
                tray_icon
                    .set_menu_item_checkable(TrayEvents::ToggleAutorun, new_state)
                    .unwrap();
                if new_state {
                    let current_exe = std::env::current_exe()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace("\\\\?\\", "");
                    key.set_value(APP_NAME, &current_exe).unwrap();
                } else {
                    key.delete_value(APP_NAME).unwrap();
                }
            }
        })
    });

    let (notify_tx, notify_rx) = unbounded();
    let (watch_tx, watch_rx) = unbounded();

    fn send_notify_handler(
        tx: Sender<(PathBuf, WatchKind)>,
        kind: WatchKind,
    ) -> impl FnMut(notify::Result<notify::Event>) + Send + 'static {
        move |res| {
            if let Ok(e) = res {
                let e = match e.kind {
                    notify::EventKind::Create(_) => e,
                    notify::EventKind::Modify(_) => e,
                    _ => return,
                };
                for path in e.paths {
                    // check paths file sizes
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        if metadata.len() > 0 {
                            tx.send((path, kind.clone())).unwrap();
                            break;
                        }
                    }
                }
            }
        }
    }

    let notify_tx2_1 = notify_tx.clone();
    let notify_tx3 = notify_tx.clone();
    let watcher_map2 = watcher_map.clone();
    std::thread::spawn(move || loop {
        let config = match Config::load() {
            Ok(config) => config,
            Err(err) => {
                let msg = format!("Failed to load config: {:?}", err);
                message_box(&msg);
                std::process::exit(1);
            }
        };
        let paths: Vec<_> = config.path_iter().collect();
        let keys = (*watcher_map2.lock().unwrap())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for k in keys {
            if !paths.contains(&k) {
                (*watcher_map2.lock().unwrap()).remove(&k);
            }
        }
        let notify_tx2_2 = notify_tx2_1.clone();
        for path in paths {
            let mut watcher: notify::RecommendedWatcher = Watcher::new(
                send_notify_handler(notify_tx2_2.clone(), WatchKind::Watch),
                notify::Config::default(),
            )
            .unwrap();

            watcher
                .watch(&path.clone(), notify::RecursiveMode::Recursive)
                .unwrap();

            (*watcher_map2.lock().unwrap()).insert(path.clone(), watcher);
        }

        let config_path = Config::get_config_path();
        let mut watcher: notify::RecommendedWatcher = Watcher::new(
            send_notify_handler(notify_tx3.clone(), WatchKind::Reload),
            notify::Config::default(),
        )
        .unwrap();
        watcher
            .watch(&config_path.clone(), notify::RecursiveMode::Recursive)
            .unwrap();
        (*(watcher_map2.lock().unwrap())).insert(config_path.clone(), watcher);

        reload_rx.recv().unwrap();
        println!("reload");
    });

    std::thread::spawn(move || loop {
        let (path, kind) = notify_rx.clone().recv().unwrap();

        match kind {
            WatchKind::Watch => {
                watch_tx.send(path).unwrap();
            }
            WatchKind::Reload => {
                reload_tx.send(()).unwrap();
            }
        };
    });

    std::thread::spawn(move || loop {
        let path = watch_rx.recv().unwrap();
        match image::open(path) {
            Ok(image) => {
                let width = image.width() as usize;
                let height = image.height() as usize;
                let bytes = image.into_rgba8().into_vec();
                let image_data = ImageData {
                    width,
                    height,
                    bytes: Cow::from(&bytes[..]),
                };
                Clipboard::new().unwrap().set_image(image_data).unwrap();
                println!("copied");
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    });

    // Your applications message loop. Because all applications require an
    // application loop, you are best served using an `winit` crate.
    unsafe {
        let mut message = MSG::default();

        while GetMessageA(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }
}

fn message_box(text: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(APP_NAME),
            MB_ICONSTOP | MB_OK,
        );
    }
}
