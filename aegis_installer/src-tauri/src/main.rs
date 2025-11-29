// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod usb_commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            usb_commands::cmd_list_drives,
            usb_commands::cmd_create_repo_structure,
            usb_commands::cmd_write_object,
            usb_commands::cmd_write_manifest,
            usb_commands::cmd_sign_manifest,
            usb_commands::cmd_write_signature
        ])
        .run(tauri::generate_context!())
        .expect("error running Aegis Installer");
}
