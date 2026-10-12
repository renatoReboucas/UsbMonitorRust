use std::{
    ffi::{c_char, c_int, c_uint, CStr},
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use sysinfo::{Networks, System};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::services::ServeDir;

const USB_VID: u16 = 0x1a86;
const USB_PID: u16 = 0x5722;
const ADDRESS: &str = "127.0.0.1:47825";

#[derive(Clone)]
struct AppState {
    serial: Arc<Mutex<Option<(String, Box<dyn SerialPort>)>>>,
    metrics: Arc<RwLock<SystemMetrics>>,
}

#[derive(Serialize)]
struct DeviceStatus {
    connected: bool,
    port: Option<String>,
    description: Option<String>,
    port_open: bool,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Deserialize)]
struct ConnectRequest {
    port: String,
    baud_rate: u32,
}

#[derive(Clone, Default, Serialize)]
struct SystemMetrics {
    computer_name: Option<String>,
    os_version: Option<String>,
    cpu_name: Option<String>,
    cpu_usage_percent: Option<f32>,
    cpu_frequency_mhz: Option<u64>,
    ram_used_bytes: u64,
    ram_total_bytes: u64,
    gpu_name: Option<String>,
    gpu_usage_percent: Option<u32>,
    gpu_temperature_c: Option<u32>,
    vram_used_bytes: Option<u64>,
    vram_total_bytes: Option<u64>,
    upload_bytes_per_sec: u64,
    download_bytes_per_sec: u64,
    network_interfaces: usize,
    uptime_seconds: u64,
}

type NvmlDevice = *mut std::ffi::c_void;
type NvmlInit = unsafe extern "C" fn() -> c_int;
type NvmlShutdown = unsafe extern "C" fn() -> c_int;
type NvmlGetDevice = unsafe extern "C" fn(c_uint, *mut NvmlDevice) -> c_int;
type NvmlGetUtilization = unsafe extern "C" fn(NvmlDevice, *mut GpuUtilization) -> c_int;
type NvmlGetTemperature = unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> c_int;
type NvmlGetMemory = unsafe extern "C" fn(NvmlDevice, *mut GpuMemory) -> c_int;
type NvmlGetName = unsafe extern "C" fn(NvmlDevice, *mut c_char, c_uint) -> c_int;

#[repr(C)]
#[derive(Default)]
struct GpuUtilization {
    gpu: c_uint,
    memory: c_uint,
}

#[repr(C)]
#[derive(Default)]
struct GpuMemory {
    total: u64,
    free: u64,
    used: u64,
}

struct NvidiaTelemetry {
    _library: libloading::Library,
    shutdown: NvmlShutdown,
    device: NvmlDevice,
    get_utilization: NvmlGetUtilization,
    get_temperature: NvmlGetTemperature,
    get_memory: NvmlGetMemory,
    get_name: NvmlGetName,
}

impl NvidiaTelemetry {
    fn load() -> Result<Self, String> {
        let path =
            PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32")
                .join("nvml.dll");
        let library = unsafe { libloading::Library::new(&path) }.map_err(|error| {
            format!(
                "Não foi possível carregar NVML ({}): {error}",
                path.display()
            )
        })?;
        let init: NvmlInit = unsafe {
            *library
                .get(b"nvmlInit_v2\0")
                .map_err(|error| format!("NVML não exporta nvmlInit_v2: {error}"))?
        };
        let shutdown: NvmlShutdown = unsafe {
            *library
                .get(b"nvmlShutdown\0")
                .map_err(|error| format!("NVML não exporta nvmlShutdown: {error}"))?
        };
        let get_device: NvmlGetDevice = unsafe {
            *library
                .get(b"nvmlDeviceGetHandleByIndex_v2\0")
                .map_err(|error| {
                    format!("NVML não exporta nvmlDeviceGetHandleByIndex_v2: {error}")
                })?
        };
        let get_utilization: NvmlGetUtilization = unsafe {
            *library
                .get(b"nvmlDeviceGetUtilizationRates\0")
                .map_err(|error| {
                    format!("NVML não exporta nvmlDeviceGetUtilizationRates: {error}")
                })?
        };
        let get_temperature: NvmlGetTemperature = unsafe {
            *library
                .get(b"nvmlDeviceGetTemperature\0")
                .map_err(|error| format!("NVML não exporta nvmlDeviceGetTemperature: {error}"))?
        };
        let get_memory: NvmlGetMemory = unsafe {
            *library
                .get(b"nvmlDeviceGetMemoryInfo\0")
                .map_err(|error| format!("NVML não exporta nvmlDeviceGetMemoryInfo: {error}"))?
        };
        let get_name: NvmlGetName = unsafe {
            *library
                .get(b"nvmlDeviceGetName\0")
                .map_err(|error| format!("NVML não exporta nvmlDeviceGetName: {error}"))?
        };
        if unsafe { init() } != 0 {
            return Err("NVML recusou a inicialização. Verifique o driver NVIDIA.".to_owned());
        }
        let mut device = std::ptr::null_mut();
        if unsafe { get_device(0, &mut device) } != 0 {
            unsafe { shutdown() };
            return Err("NVML não encontrou uma GPU NVIDIA no índice 0.".to_owned());
        }

        Ok(Self {
            _library: library,
            shutdown,
            device,
            get_utilization,
            get_temperature,
            get_memory,
            get_name,
        })
    }

    fn collect(&self, metrics: &mut SystemMetrics) {
        let mut utilization = GpuUtilization::default();
        if unsafe { (self.get_utilization)(self.device, &mut utilization) } == 0 {
            metrics.gpu_usage_percent = Some(utilization.gpu.min(100));
        }
        let mut temperature = 0;
        if unsafe { (self.get_temperature)(self.device, 0, &mut temperature) } == 0 {
            metrics.gpu_temperature_c = Some(temperature);
        }
        let mut memory = GpuMemory::default();
        if unsafe { (self.get_memory)(self.device, &mut memory) } == 0 {
            metrics.vram_used_bytes = Some(memory.used);
            metrics.vram_total_bytes = Some(memory.total);
        }
        let mut name = [0 as c_char; 96];
        if unsafe { (self.get_name)(self.device, name.as_mut_ptr(), name.len() as c_uint) } == 0 {
            let name = unsafe { CStr::from_ptr(name.as_ptr()) }
                .to_string_lossy()
                .trim()
                .to_owned();
            if !name.is_empty() {
                metrics.gpu_name = Some(name);
            }
        }
    }
}

impl Drop for NvidiaTelemetry {
    fn drop(&mut self) {
        unsafe { (self.shutdown)() };
    }
}

fn refresh_metrics(metrics: &RwLock<SystemMetrics>) {
    let mut system = System::new_all();
    system.refresh_all();
    let mut networks = Networks::new_with_refreshed_list();
    let mut last_network = network_totals(&networks);
    let mut last_sample = Instant::now();
    let nvml = match NvidiaTelemetry::load() {
        Ok(nvml) => Some(nvml),
        Err(error) => {
            eprintln!("Telemetria NVIDIA indisponível: {error}");
            None
        }
    };

    loop {
        thread::sleep(Duration::from_secs(1));
        let elapsed = last_sample.elapsed().as_secs_f64().max(0.001);
        last_sample = Instant::now();
        system.refresh_cpu_all();
        system.refresh_memory();
        networks.refresh(true);

        let (received, transmitted) = network_totals(&networks);
        let mut snapshot = SystemMetrics {
            computer_name: System::host_name(),
            os_version: System::os_version(),
            cpu_name: system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().trim().to_owned())
                .filter(|name| !name.is_empty()),
            cpu_usage_percent: Some(system.global_cpu_usage().clamp(0.0, 100.0)),
            cpu_frequency_mhz: system.cpus().first().map(|cpu| cpu.frequency()),
            ram_used_bytes: system.used_memory(),
            ram_total_bytes: system.total_memory(),
            upload_bytes_per_sec: rate(transmitted, last_network.1, elapsed),
            download_bytes_per_sec: rate(received, last_network.0, elapsed),
            network_interfaces: networks.len(),
            uptime_seconds: System::uptime(),
            ..SystemMetrics::default()
        };
        last_network = (received, transmitted);
        if let Some(nvml) = &nvml {
            nvml.collect(&mut snapshot);
        }
        if let Ok(mut current) = metrics.write() {
            *current = snapshot;
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

fn is_target(port: &SerialPortInfo) -> bool {
    matches!(
        &port.port_type,
        SerialPortType::UsbPort(usb) if usb.vid == USB_VID && usb.pid == USB_PID
    )
}

async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<SystemMetrics>, (StatusCode, Json<ApiError>)> {
    let snapshot = state.metrics.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: "A leitura das métricas do sistema falhou.".to_owned(),
            }),
        )
    })?;
    Ok(Json(snapshot.clone()))
}

async fn status(
    State(state): State<AppState>,
) -> Result<Json<DeviceStatus>, (StatusCode, Json<ApiError>)> {
    let ports = serialport::available_ports().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: format!("Falha ao consultar portas seriais: {error}"),
            }),
        )
    })?;
    let target = ports.into_iter().find(is_target);
    let serial = state.serial.lock().await;

    Ok(Json(DeviceStatus {
        connected: target.is_some(),
        port: target.as_ref().map(|device| device.port_name.clone()),
        description: target.map(|device| match device.port_type {
            SerialPortType::UsbPort(usb) => usb
                .product
                .unwrap_or_else(|| format!("USB {:04X}:{:04X}", usb.vid, usb.pid)),
            _ => "Dispositivo serial".to_owned(),
        }),
        port_open: serial.is_some(),
    }))
}

async fn connect(
    State(state): State<AppState>,
    Json(request): Json<ConnectRequest>,
) -> Result<Json<DeviceStatus>, (StatusCode, Json<ApiError>)> {
    let ports = serialport::available_ports().map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: format!("Falha ao consultar portas seriais: {error}"),
            }),
        )
    })?;
    if !ports
        .iter()
        .any(|device| device.port_name == request.port && is_target(device))
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "A porta não corresponde ao display USB VID_1A86&PID_5722.".to_owned(),
            }),
        ));
    }
    if request.baud_rate == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "A velocidade serial precisa ser maior que zero.".to_owned(),
            }),
        ));
    }

    let port = serialport::new(&request.port, request.baud_rate)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|error| {
            (
                StatusCode::CONFLICT,
                Json(ApiError {
                    error: format!(
                        "Não foi possível abrir {}. Feche o UsbMonitor original e tente novamente: {error}",
                        request.port
                    ),
                }),
            )
        })?;

    *state.serial.lock().await = Some((request.port.clone(), port));
    status(State(state)).await
}

async fn disconnect(
    State(state): State<AppState>,
) -> Result<Json<DeviceStatus>, (StatusCode, Json<ApiError>)> {
    *state.serial.lock().await = None;
    status(State(state)).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = Arc::new(RwLock::new(SystemMetrics::default()));
    let telemetry_metrics = Arc::clone(&metrics);
    thread::Builder::new()
        .name("system-telemetry".to_owned())
        .spawn(move || refresh_metrics(&telemetry_metrics))?;

    let state = AppState {
        serial: Arc::new(Mutex::new(None)),
        metrics,
    };
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let web = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web");
    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/metrics", get(get_metrics))
        .route("/api/connect", post(connect))
        .route("/api/disconnect", post(disconnect))
        .nest_service("/assets", ServeDir::new(assets))
        .fallback_service(ServeDir::new(web))
        .with_state(state);

    let listener = TcpListener::bind(ADDRESS).await?;
    println!("UsbMonitor Rust disponível em http://{ADDRESS}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_observed_ch340_display_adapter() {
        assert_eq!(USB_VID, 0x1a86);
        assert_eq!(USB_PID, 0x5722);
    }

    #[test]
    fn network_rate_handles_counter_reset() {
        assert_eq!(rate(90, 100, 1.0), 0);
        assert_eq!(rate(300, 100, 2.0), 100);
    }
}
