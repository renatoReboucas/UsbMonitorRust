mod config;
mod cpu;
mod gpu;
mod ram;
mod serial;
mod startup;
mod telemetry;
mod tray;
mod weather;

use std::sync::Mutex;

use config::{AppConfig, ConfigStore};
use serial::{DeviceStatus, SerialState};
use tauri::{Manager, State};
use telemetry::{SystemMetrics, TelemetryState};
use tray::TrayState;
use weather::{WeatherSnapshot, WeatherState};

#[tauri::command]
fn get_metrics(state: State<'_, TelemetryState>) -> Result<SystemMetrics, String> {
    state.latest()
}

#[tauri::command]
fn get_history(state: State<'_, TelemetryState>) -> Result<Vec<SystemMetrics>, String> {
    state.history()
}

#[tauri::command]
async fn get_weather(
    city: String,
    state: State<'_, WeatherState>,
) -> Result<WeatherSnapshot, String> {
    weather::current(&state, &city).await
}

#[tauri::command]
fn get_device_status(state: State<'_, SerialState>) -> Result<DeviceStatus, String> {
    serial::status(&state)
}

#[tauri::command]
fn connect_serial(
    port: String,
    baud_rate: u32,
    state: State<'_, SerialState>,
) -> Result<DeviceStatus, String> {
    serial::connect(&state, &port, baud_rate)?;
    serial::status(&state)
}

#[tauri::command]
async fn send_display_frame(
    image_png: Vec<u8>,
    state: State<'_, SerialState>,
) -> Result<serial::FrameSendResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || serial::send_png_frame(&state, &image_png))
        .await
        .map_err(|error| format!("A tarefa de transmissão do display falhou: {error}"))?
}

#[tauri::command]
fn disconnect_serial(state: State<'_, SerialState>) -> Result<DeviceStatus, String> {
    serial::disconnect(&state)?;
    serial::status(&state)
}

#[tauri::command]
fn load_config(state: State<'_, ConfigStore>) -> Result<AppConfig, String> {
    state
        .0
        .lock()
        .map(|config| config.clone())
        .map_err(|_| "Não foi possível acessar as configurações.".to_owned())
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    config: AppConfig,
    state: State<'_, ConfigStore>,
) -> Result<AppConfig, String> {
    let config = config.validate()?;
    let mut current = state
        .0
        .lock()
        .map_err(|_| "Não foi possível acessar as configurações.".to_owned())?;
    let previous = current.clone();

    if previous.start_with_windows != config.start_with_windows {
        startup::set_enabled(&app, config.start_with_windows)?;
    }
    if let Err(error) = config::save(&app, &config) {
        if previous.start_with_windows != config.start_with_windows {
            if let Err(rollback_error) = startup::set_enabled(&app, previous.start_with_windows) {
                eprintln!("Falha ao restaurar a inicialização automática: {rollback_error}");
            }
        }
        return Err(error);
    }
    *current = config.clone();
    Ok(config)
}

#[tauri::command]
fn export_logs(app: tauri::AppHandle, state: State<'_, TelemetryState>) -> Result<String, String> {
    telemetry::export_logs(&app, &state)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .manage(TelemetryState::default())
        .manage(SerialState::default())
        .manage(WeatherState::default())
        .manage(TrayState::default())
        .manage(ConfigStore(Mutex::new(AppConfig::default())))
        .setup(|app| {
            let loaded = config::load(app.handle())?;
            {
                let store = app.state::<ConfigStore>();
                let mut current = store
                    .0
                    .lock()
                    .map_err(|_| std::io::Error::other("Configuração em estado inválido."))?;
                *current = loaded.clone();
            }
            if loaded.start_with_windows {
                startup::set_enabled(app.handle(), true).map_err(std::io::Error::other)?;
            }
            let tray_state = app.state::<TrayState>().inner().clone();
            tray::install(app, &tray_state)?;
            app.state::<TelemetryState>()
                .start(app.handle().clone(), tray_state)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            get_history,
            get_weather,
            get_device_status,
            connect_serial,
            disconnect_serial,
            send_display_frame,
            load_config,
            save_config,
            export_logs
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("Não foi possível ocultar a janela: {error}");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Falha ao iniciar UsbMonitorRust");
}
