#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;

use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use crossbeam_channel::unbounded;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use trayicon::*;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage, MSG},
};

fn main() {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Events {
        ClickTrayIcon,
        DoubleClickTrayIcon,
        Exit,
        Item1,
        Item2,
        Item3,
        Item4,
        CheckItem1,
        SubItem1,
        SubItem2,
        SubItem3,
    }

    let (s, r) = std::sync::mpsc::channel::<Events>();
    let icon = include_bytes!("./icon1.ico");
    let icon2 = include_bytes!("./icon1.ico");

    let second_icon = Icon::from_buffer(icon2, None, None).unwrap();
    let first_icon = Icon::from_buffer(icon, None, None).unwrap();

    // Needlessly complicated tray icon with all the whistles and bells
    let mut tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip("Cool Tray 👀 Icon")
        .on_click(Events::ClickTrayIcon)
        .on_double_click(Events::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("Item 3 Replace Menu 👍", Events::Item3)
                .item("Item 2 Change Icon Green", Events::Item2)
                .item("Item 1 Change Icon Red", Events::Item1)
                .separator()
                .checkable("This is checkable", true, Events::CheckItem1)
                .submenu(
                    "Sub Menu",
                    MenuBuilder::new()
                        .item("Sub item 1", Events::SubItem1)
                        .item("Sub Item 2", Events::SubItem2)
                        .item("Sub Item 3", Events::SubItem3),
                )
                .with(MenuItem::Item {
                    name: "Item Disabled".into(),
                    disabled: true, // Disabled entry example
                    id: Events::Item4,
                    icon: None,
                })
                .separator()
                .item("E&xit", Events::Exit),
        )
        .build()
        .unwrap();

    std::thread::spawn(move || {
        r.iter().for_each(|m| match m {
            Events::DoubleClickTrayIcon => {
                println!("Double click");
            }
            Events::ClickTrayIcon => {
                println!("Single click");
            }
            Events::Exit => {
                std::process::exit(0);
            }
            Events::Item1 => {
                tray_icon.set_icon(&second_icon).unwrap();
            }
            Events::Item2 => {
                tray_icon.set_icon(&first_icon).unwrap();
            }
            Events::Item3 => {
                tray_icon
                    .set_menu(
                        &MenuBuilder::new()
                            .item("New menu item", Events::Item1)
                            .item("Exit", Events::Exit),
                    )
                    .unwrap();
            }
            e => {
                println!("{:?}", e);
            }
        })
    });

    let config = config::Config::load().unwrap();

    println!("{:?}", config);
    // for watcher lives
    let mut watchers = Vec::new();

    let (tx, rx) = unbounded();
    for path in config.paths.iter() {
        let tx = tx.clone();
        let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).unwrap();
        watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .unwrap();
        watchers.push(watcher);
    }

    std::thread::spawn(move || loop {
        match rx.recv() {
            Ok(event) => match event {
                Ok(event) => match event.kind {
                    notify::EventKind::Remove(_) => {}
                    _ => {
                        for path in event.paths {
                            println!("{:?}", path);
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
                        }
                    }
                },
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
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
