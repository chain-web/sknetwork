#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use sk_common::events::LifeCycleEvents;
use skchain::node::start_node;
use tauri::{Manager, Size, PhysicalSize};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn start_sk(window: tauri::Window, env: &str) -> String {
    let _ = window.emit_all("sk_lifecycle_events", "start sk node");
    let mut node = start_node().unwrap();
    let emit_bridge = move |key: LifeCycleEvents, msg: String| {
        let mut str_msg = key.to_string();
        str_msg.push_str(" ");
        str_msg.push_str(&msg);
        let _ = &window.emit_all("sk_lifecycle_events", str_msg);
    };
    node.lifecycle_events.register_all(emit_bridge);
    node.init();
    format!("start sk node use env:, {}!", env)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start_sk])
        // .setup(|app| {
        //     let window = app.get_window("main").unwrap();
        //     window.open_devtools();
        //     let _ = window.set_size(Size::Physical(PhysicalSize{
        //         width: 800,
        //         height: 800
        //     }));
        //     Ok(())
        // })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
