use std::{
    collections::VecDeque,
    fs,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use sysinfo::{Networks, System};
use tauri::{AppHandle, Manager};

use crate::{
    cpu,
    gpu::NvidiaTelemetry,
    ram,
    tray::{self, TrayState},
};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const LOG_CAPACITY: usize = 3_600;

#[derive(Clone, Default, Serialize)]
pub struct SystemMetrics {
    pub timestamp_unix_ms: u64,
    pub computer_name: Option<String>,
    pub os_version: Option<String>,
    pub cpu_name: Option<String>,
    pub cpu_usage_percent: Option<f32>,
    pub cpu_frequency_mhz: Option<u64>,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub gpu_name: Option<String>,
    pub gpu_usage_percent: Option<u32>,
    pub gpu_temperature_c: Option<u32>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub upload_bytes_per_sec: u64,
    pub download_bytes_per_sec: u64,
    pub network_interfaces: usize,
    pub uptime_seconds: u64,
}

#[derive(Default)]
struct TelemetryData {
    latest: SystemMetrics,
    history: VecDeque<SystemMetrics>,
}

#[derive(Clone, Default)]
pub struct TelemetryState(Arc<RwLock<TelemetryData>>);

impl TelemetryState {
    pub fn start(&self, app: AppHandle, tray_state: TrayState) -> std::io::Result<()> {
        let state = self.clone();
        thread::Builder::new()
            .name("system-telemetry".to_owned())
            .spawn(move || collect_loop(state, app, tray_state))?;
        Ok(())
    }

    pub fn latest(&self) -> Result<SystemMetrics, String> {
        self.0
            .read()
            .map(|data| data.latest.clone())
            .map_err(|_| "A leitura das métricas do sistema falhou.".to_owned())
    }

    pub fn history(&self) -> Result<Vec<SystemMetrics>, String> {
        self.0
            .read()
            .map(|data| data.history.iter().cloned().collect())
            .map_err(|_| "A leitura do histórico de métricas falhou.".to_owned())
    }

    fn record(&self, sample: SystemMetrics) {
        match self.0.write() {
            Ok(mut data) => {
                data.latest = sample.clone();
                data.history.push_back(sample);
                if data.history.len() > LOG_CAPACITY {
                    data.history.pop_front();
                }
            }
            Err(error) => eprintln!("Falha ao atualizar o histórico de métricas: {error}"),
        }
    }
}

fn collect_loop(state: TelemetryState, app: AppHandle, tray_state: TrayState) {
    let mut system = System::new();
    system.refresh_cpu_all();
    system.refresh_memory();
    let mut networks = Networks::new_with_refreshed_list();
    let mut previous_network = network_totals(&networks);
    let mut last_sample = Instant::now();
    let mut last_tray_update = Instant::now();
    let nvml = match NvidiaTelemetry::load() {
        Ok(nvml) => Some(nvml),
        Err(error) => {
            eprintln!("Telemetria NVIDIA indisponível: {error}");
            None
        }
    };

    loop {
        thread::sleep(SAMPLE_INTERVAL);
        let elapsed = last_sample.elapsed().as_secs_f64().max(0.001);
        last_sample = Instant::now();
        system.refresh_cpu_all();
        system.refresh_memory();
        networks.refresh(true);

        let (received, transmitted) = network_totals(&networks);
        let (ram_used_bytes, ram_total_bytes) = ram::usage_bytes(&system);
        let mut snapshot = SystemMetrics {
            timestamp_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64,
            computer_name: System::host_name(),
            os_version: System::os_version(),
            cpu_name: cpu::name(&system),
            cpu_usage_percent: cpu::usage(&system),
            cpu_frequency_mhz: cpu::frequency_mhz(&system),
            ram_used_bytes,
            ram_total_bytes,
            upload_bytes_per_sec: rate(transmitted, previous_network.1, elapsed),
            download_bytes_per_sec: rate(received, previous_network.0, elapsed),
            network_interfaces: networks.len(),
            uptime_seconds: System::uptime(),
            ..SystemMetrics::default()
        };
        previous_network = (received, transmitted);
        if let Some(nvml) = &nvml {
            nvml.collect(&mut snapshot);
        }
        state.record(snapshot.clone());
        if last_tray_update.elapsed() >= Duration::from_secs(5) {
            last_tray_update = Instant::now();
            tray::update(&app, &tray_state, &snapshot);
        }
    }
}

fn network_totals(networks: &Networks) -> (u64, u64) {
    networks
        .values()
        .fold((0, 0), |(received, transmitted), data| {
            (
                received.saturating_add(data.total_received()),
                transmitted.saturating_add(data.total_transmitted()),
            )
        })
}

fn rate(current: u64, previous: u64, elapsed_seconds: f64) -> u64 {
    (current.saturating_sub(previous) as f64 / elapsed_seconds.max(0.001)) as u64
}

pub fn export_logs(app: &AppHandle, state: &TelemetryState) -> Result<String, String> {
    let samples = state.history()?;
    if samples.is_empty() {
        return Err("Ainda não há amostras para exportar.".to_owned());
    }
    let directory = app
        .path()
        .download_dir()
        .map_err(|error| format!("Não foi possível localizar a pasta Downloads: {error}"))?
        .join("UsbMonitorRust");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Não foi possível criar a pasta de exportação: {error}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = directory.join(format!("telemetria-{timestamp}.csv"));
    let mut csv = String::from(
        "timestamp_unix_ms,cpu_name,cpu_usage_percent,cpu_frequency_mhz,ram_used_bytes,ram_total_bytes,gpu_name,gpu_usage_percent,gpu_temperature_c,vram_used_bytes,vram_total_bytes,upload_bytes_per_sec,download_bytes_per_sec,network_interfaces,uptime_seconds\r\n",
    );
    for sample in samples {
        let row = [
            sample.timestamp_unix_ms.to_string(),
            csv_text(sample.cpu_name.as_deref()),
            csv_number(sample.cpu_usage_percent),
            csv_number(sample.cpu_frequency_mhz),
            sample.ram_used_bytes.to_string(),
            sample.ram_total_bytes.to_string(),
            csv_text(sample.gpu_name.as_deref()),
            csv_number(sample.gpu_usage_percent),
            csv_number(sample.gpu_temperature_c),
            csv_number(sample.vram_used_bytes),
            csv_number(sample.vram_total_bytes),
            sample.upload_bytes_per_sec.to_string(),
            sample.download_bytes_per_sec.to_string(),
            sample.network_interfaces.to_string(),
            sample.uptime_seconds.to_string(),
        ];
        csv.push_str(&row.join(","));
        csv.push_str("\r\n");
    }
    fs::write(&path, csv).map_err(|error| format!("Não foi possível salvar o CSV: {error}"))?;
    Ok(path.display().to_string())
}

fn csv_text(value: Option<&str>) -> String {
    let value = value.unwrap_or_default().replace('"', "\"\"");
    format!("\"{value}\"")
}

fn csv_number<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_rate_handles_counter_reset_and_elapsed_time() {
        assert_eq!(rate(90, 100, 1.0), 0);
        assert_eq!(rate(300, 100, 2.0), 100);
    }

    #[test]
    fn csv_text_quotes_values_and_escapes_quotes() {
        assert_eq!(csv_text(Some("GPU \"A\"")), "\"GPU \"\"A\"\"\"");
        assert_eq!(csv_text(None), "\"\"");
    }
}
