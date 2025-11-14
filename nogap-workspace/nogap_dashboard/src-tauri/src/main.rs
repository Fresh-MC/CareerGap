use nogap_core;

#[tauri::command]
fn run_audit() -> String {
    nogap_core::audit_system()
}

#[tauri::command]
fn get_version() -> String {
    nogap_core::get_version()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_audit, get_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}