use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result
        .map_err(|error| format!("Não foi possível atualizar a inicialização do Windows: {error}"))
}
