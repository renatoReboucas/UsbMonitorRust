use std::sync::{Arc, Mutex};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Wry,
};

use crate::serial::{self, SerialState};
use crate::telemetry::SystemMetrics;

#[derive(Clone, Default)]
pub struct TrayState(Arc<Mutex<Option<TrayMetrics>>>);

struct TrayMetrics {
    cpu: MenuItem<Wry>,
    ram: MenuItem<Wry>,
    gpu: MenuItem<Wry>,
    cpu_temperature: MenuItem<Wry>,
    gpu_temperature: MenuItem<Wry>,
    network: MenuItem<Wry>,
    uptime: MenuItem<Wry>,
}

pub fn install(app: &App, state: &TrayState) -> tauri::Result<()> {
    let cpu = MenuItem::with_id(app, "metric-cpu", "CPU: aguardando", false, None::<&str>)?;
    let ram = MenuItem::with_id(app, "metric-ram", "RAM: aguardando", false, None::<&str>)?;
    let gpu = MenuItem::with_id(app, "metric-gpu", "GPU: aguardando", false, None::<&str>)?;
    let cpu_temperature = MenuItem::with_id(
        app,
        "metric-cpu-temperature",
        "CPU TEMP: N/D",
        false,
        None::<&str>,
    )?;
    let gpu_temperature = MenuItem::with_id(
        app,
        "metric-gpu-temperature",
        "GPU TEMP: N/D",
        false,
        None::<&str>,
    )?;
    let network = MenuItem::with_id(
        app,
        "metric-network",
        "Rede: aguardando",
        false,
        None::<&str>,
    )?;
    let uptime = MenuItem::with_id(
        app,
        "metric-uptime",
        "Uptime: aguardando",
        false,
        None::<&str>,
    )?;
    let show = MenuItem::with_id(app, "show", "Abrir UsbMonitor", true, None::<&str>)?;
    let release_serial = MenuItem::with_id(
        app,
        "release-serial",
        "Liberar porta serial",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Encerrar", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &cpu,
            &ram,
            &gpu,
            &cpu_temperature,
            &gpu_temperature,
            &network,
            &uptime,
            &show,
            &release_serial,
            &quit,
        ],
    )?;
    *state
        .0
        .lock()
        .map_err(|error| std::io::Error::other(format!("Estado da bandeja inválido: {error}")))? =
        Some(TrayMetrics {
            cpu,
            ram,
            gpu,
            cpu_temperature,
            gpu_temperature,
            network,
            uptime,
        });
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id("system-metrics")
        .icon(icon)
        .tooltip("UsbMonitor Rust")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "release-serial" => {
                if let Some(state) = app.try_state::<SerialState>() {
                    if let Err(error) = serial::disconnect(&state) {
                        eprintln!("Não foi possível liberar a porta serial pela bandeja: {error}");
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn update(app: &AppHandle, state: &TrayState, metrics: &SystemMetrics) {
    let cpu = metrics
        .cpu_usage_percent
        .map_or_else(|| "N/D".to_owned(), |usage| format!("{usage:.0}%"));
    let ram = if metrics.ram_total_bytes > 0 {
        format!(
            "{:.1}/{:.1} GB ({:.0}%)",
            metrics.ram_used_bytes as f64 / 1_073_741_824.0,
            metrics.ram_total_bytes as f64 / 1_073_741_824.0,
            metrics.ram_used_bytes as f64 / metrics.ram_total_bytes as f64 * 100.0
        )
    } else {
        "N/D".to_owned()
    };
    let gpu = metrics
        .gpu_usage_percent
        .map_or_else(|| "N/D".to_owned(), |usage| format!("{usage}%"));
    let cpu_temperature = metrics
        .cpu_temperature_c
        .map_or_else(|| "N/D".to_owned(), |value| format!("{value}°C"));
    let gpu_temperature = metrics
        .gpu_temperature_c
        .map_or_else(|| "N/D".to_owned(), |value| format!("{value}°C"));
    let network = format!(
        "↑ {:.1} ↓ {:.1} MB/s",
        metrics.upload_bytes_per_sec as f64 / 1_048_576.0,
        metrics.download_bytes_per_sec as f64 / 1_048_576.0
    );
    let uptime = format!(
        "{}d {}h {}m",
        metrics.uptime_seconds / 86_400,
        metrics.uptime_seconds % 86_400 / 3_600,
        metrics.uptime_seconds % 3_600 / 60
    );

    let tray_metrics = match state.0.lock() {
        Ok(items) => items,
        Err(error) => {
            eprintln!("Falha ao atualizar a bandeja: {error}");
            return;
        }
    };
    if let Some(items) = tray_metrics.as_ref() {
        for (item, text) in [
            (&items.cpu, format!("CPU: {cpu}")),
            (&items.ram, format!("RAM: {ram}")),
            (&items.gpu, format!("GPU: {gpu}")),
            (
                &items.cpu_temperature,
                format!("CPU TEMP: {cpu_temperature}"),
            ),
            (
                &items.gpu_temperature,
                format!("GPU TEMP: {gpu_temperature}"),
            ),
            (&items.network, format!("Rede: {network}")),
            (&items.uptime, format!("Uptime: {uptime}")),
        ] {
            if let Err(error) = item.set_text(text) {
                eprintln!("Falha ao atualizar uma métrica na bandeja: {error}");
            }
        }
    }
    drop(tray_metrics);

    if let Some(icon) = app.tray_by_id("system-metrics") {
        let tooltip =
            format!("CPU {cpu} {cpu_temperature} | RAM {ram} | GPU {gpu} {gpu_temperature}");
        if let Err(error) = icon.set_tooltip(Some(tooltip)) {
            eprintln!("Falha ao atualizar o resumo da bandeja: {error}");
        }
    }
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            eprintln!("Não foi possível mostrar a janela: {error}");
        }
        if let Err(error) = window.set_focus() {
            eprintln!("Não foi possível focar a janela: {error}");
        }
    }
}
