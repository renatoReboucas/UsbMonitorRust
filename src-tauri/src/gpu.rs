use std::{
    ffi::{c_char, c_int, c_uint, CStr},
    path::PathBuf,
};

use libloading::Library;

use crate::telemetry::SystemMetrics;

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

pub struct NvidiaTelemetry {
    _library: Library,
    shutdown: NvmlShutdown,
    device: NvmlDevice,
    get_utilization: NvmlGetUtilization,
    get_temperature: NvmlGetTemperature,
    get_memory: NvmlGetMemory,
    get_name: NvmlGetName,
}

impl NvidiaTelemetry {
    pub fn load() -> Result<Self, String> {
        let path =
            PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32")
                .join("nvml.dll");
        let library = unsafe { Library::new(&path) }.map_err(|error| {
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

    pub fn collect(&self, metrics: &mut SystemMetrics) {
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
            name[name.len() - 1] = 0;
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
