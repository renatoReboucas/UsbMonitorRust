//! Telemetria de GPUs AMD pela **ADL** (`atiadlxx.dll`), a API que o driver da
//! AMD publica no `System32` do Windows.
//!
//! A NVML cobre placas NVIDIA e o serviço do LibreHardwareMonitor é o caminho
//! universal (Intel incluída), mas nenhum dos dois é a via oficial para uma
//! Radeon. A ADL é: não exige privilégios de administrador e lê direto do
//! driver.
//!
//! Todas as chamadas são opcionais. Drivers antigos ou placas integradas não
//! exportam parte das funções (Overdrive5/OverdriveN variam por geração), então
//! cada símbolo ausente apenas deixa o campo correspondente em `None` e a
//! telemetria segue pelo LibreHardwareMonitor. Nada aqui pode derrubar o
//! aplicativo: erro de driver vira ausência de leitura.

use std::{
    ffi::{c_int, c_void},
    path::PathBuf,
};

use libloading::Library;

use crate::telemetry::SystemMetrics;

/// `ADL_MAX_PATH`, o tamanho dos buffers de texto da ADL.
const ADL_MAX_PATH: usize = 256;
/// `ADL_OK`.
const ADL_OK: c_int = 0;
/// `ADLODNTemperatureType::EDGE` — a temperatura de borda do chip, a que os
/// monitores mostram como "temperatura da GPU".
const ADL_ODN_TEMP_EDGE: c_int = 1;
/// `ATI_VENDOR_ID` — identifica os adaptadores da AMD na enumeração.
const ATI_VENDOR_ID: c_int = 0x1002;

/// `ADL_MAIN_MALLOC_CALLBACK`: a ADL aloca seus buffers com esta função e
/// libera com o mesmo alocador, então precisa ser um `malloc` de verdade.
type AdlMallocCallback = unsafe extern "C" fn(c_int) -> *mut c_void;

type AdlMainControlCreate =
    unsafe extern "C" fn(AdlMallocCallback, c_int, *mut *mut c_void) -> c_int;
type AdlMainControlDestroy = unsafe extern "C" fn(*mut c_void) -> c_int;
type AdlNumberOfAdaptersGet = unsafe extern "C" fn(*mut c_void, *mut c_int) -> c_int;
type AdlAdapterInfoGet = unsafe extern "C" fn(*mut c_void, *mut AdlAdapterInfo, c_int) -> c_int;
type AdlDedicatedVramUsageGet = unsafe extern "C" fn(*mut c_void, c_int, *mut c_int) -> c_int;
type AdlMemoryInfoX4Get = unsafe extern "C" fn(*mut c_void, c_int, *mut AdlMemoryInfoX4) -> c_int;
type AdlOverdrive5CurrentActivityGet =
    unsafe extern "C" fn(*mut c_void, c_int, *mut AdlPmActivity) -> c_int;
type AdlOverdrive5TemperatureGet =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, *mut AdlTemperature) -> c_int;
type AdlOverdriveNTemperatureGet =
    unsafe extern "C" fn(*mut c_void, c_int, c_int, *mut c_int) -> c_int;
type AdlOverdriveNPerformanceStatusGet =
    unsafe extern "C" fn(*mut c_void, c_int, *mut AdlOdnPerformanceStatus) -> c_int;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

unsafe extern "C" fn adl_malloc(size: c_int) -> *mut c_void {
    malloc(size.max(0) as usize)
}

/// `ADLAdapterInfo`. O layout espelha o cabeçalho em C (inteiros de 4 bytes e
/// strings de 256 bytes embutidas), inclusive o `iSize` que a ADL exige que
/// esteja preenchido antes de cada chamada.
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct AdlAdapterInfo {
    size: c_int,
    adapter_index: c_int,
    udid: [u8; ADL_MAX_PATH],
    bus_number: c_int,
    device_number: c_int,
    function_number: c_int,
    vendor_id: c_int,
    adapter_name: [u8; ADL_MAX_PATH],
    display_name: [u8; ADL_MAX_PATH],
    present: c_int,
    exists: c_int,
    driver_path: [u8; ADL_MAX_PATH],
    driver_path_ext: [u8; ADL_MAX_PATH],
    pnp_string: [u8; ADL_MAX_PATH],
    os_display_index: c_int,
}

/// `ADLPMActivity` (Overdrive5): uso, clocks e nível de desempenho atuais.
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
struct AdlPmActivity {
    size: c_int,
    engine_clock: c_int,
    memory_clock: c_int,
    vddc: c_int,
    activity_percent: c_int,
    current_performance_level: c_int,
    current_bus_speed: c_int,
    current_bus_lanes: c_int,
    maximum_bus_lanes: c_int,
    reserved: c_int,
}

/// `ADLTemperature` (Overdrive5): temperatura em milésimos de grau Celsius.
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
struct AdlTemperature {
    size: c_int,
    temperature: c_int,
}

/// `ADLODNPerformanceStatus` (OverdriveN).
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
struct AdlOdnPerformanceStatus {
    core_clock: c_int,
    memory_clock: c_int,
    dcef_clock: c_int,
    gfx_clock: c_int,
    uvd_clock: c_int,
    vce_clock: c_int,
    gpu_activity_percent: c_int,
    current_core_performance_level: c_int,
    current_memory_performance_level: c_int,
    current_dcef_performance_level: c_int,
    current_gfx_performance_level: c_int,
    uvd_performance_level: c_int,
    vce_performance_level: c_int,
    current_bus_speed: c_int,
    current_bus_lanes: c_int,
    maximum_bus_lanes: c_int,
    vddc: c_int,
    vddci: c_int,
}

/// `ADLMemoryInfoX4`: `iMemorySize` é o total de VRAM em **bytes**.
#[repr(C)]
#[allow(dead_code)]
struct AdlMemoryInfoX4 {
    memory_size: i64,
    memory_type: [u8; ADL_MAX_PATH],
    memory_bandwidth: i64,
    hyper_memory_size: i64,
    invisible_memory_size: i64,
    visible_memory_size: i64,
    vram_vendor_rev_id: i64,
    memory_bandwidth_x2: i64,
    memory_bit_rate_x2: i64,
}

/// Lê uma string de tamanho fixo terminada em NUL, tolerando ausência do NUL.
fn fixed_str(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn zeroed<T>() -> T {
    // As structs da ADL são POD e vêm todas do driver: zerar é o estado inicial
    // correto (e o que o `iSize` espera até ser preenchido).
    unsafe { std::mem::zeroed() }
}

/// GPUs AMD expostas pela ADL neste computador.
pub struct AmdTelemetry {
    _library: Library,
    context: *mut c_void,
    adapter_index: c_int,
    name: String,
    destroy: AdlMainControlDestroy,
    activity: Option<AdlOverdrive5CurrentActivityGet>,
    temperature5: Option<AdlOverdrive5TemperatureGet>,
    temperature_n: Option<AdlOverdriveNTemperatureGet>,
    performance: Option<AdlOverdriveNPerformanceStatusGet>,
    vram_usage: Option<AdlDedicatedVramUsageGet>,
    memory_info: Option<AdlMemoryInfoX4Get>,
}

impl AmdTelemetry {
    /// Carrega a ADL e localiza o primeiro adaptador AMD presente. Erro só
    /// quando a DLL não existe ou não há GPU AMD — casos em que a telemetria
    /// simplesmente segue para NVML/LibreHardwareMonitor.
    pub fn load() -> Result<Self, String> {
        let path =
            PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32")
                .join("atiadlxx.dll");
        let library = unsafe { Library::new(&path) }.map_err(|error| {
            format!(
                "Não foi possível carregar a ADL ({}): {error}",
                path.display()
            )
        })?;

        // Símbolos essenciais: sem eles não há como falar com a ADL.
        let main_create: AdlMainControlCreate = unsafe {
            *library
                .get(b"ADL2_Main_Control_Create\0")
                .map_err(|error| format!("A ADL não exporta ADL2_Main_Control_Create: {error}"))?
        };
        let destroy: AdlMainControlDestroy = unsafe {
            *library
                .get(b"ADL2_Main_Control_Destroy\0")
                .map_err(|error| format!("A ADL não exporta ADL2_Main_Control_Destroy: {error}"))?
        };
        let number_of_adapters: AdlNumberOfAdaptersGet = unsafe {
            *library
                .get(b"ADL2_Adapter_NumberOfAdapters_Get\0")
                .map_err(|error| {
                    format!("A ADL não exporta ADL2_Adapter_NumberOfAdapters_Get: {error}")
                })?
        };
        let adapter_info: AdlAdapterInfoGet = unsafe {
            *library
                .get(b"ADL2_Adapter_AdapterInfo_Get\0")
                .map_err(|error| {
                    format!("A ADL não exporta ADL2_Adapter_AdapterInfo_Get: {error}")
                })?
        };

        let mut context = std::ptr::null_mut();
        if unsafe { main_create(adl_malloc, 1, &mut context) } != ADL_OK || context.is_null() {
            return Err("A ADL recusou a inicialização do controle principal.".to_owned());
        }

        // As funções abaixo variam por geração da placa: ausentes viram `None`.
        let activity = unsafe {
            library
                .get::<AdlOverdrive5CurrentActivityGet>(b"ADL2_Overdrive5_CurrentActivity_Get\0")
                .ok()
                .map(|symbol| *symbol)
        };
        let temperature5 = unsafe {
            library
                .get::<AdlOverdrive5TemperatureGet>(b"ADL2_Overdrive5_Temperature_Get\0")
                .ok()
                .map(|symbol| *symbol)
        };
        let temperature_n = unsafe {
            library
                .get::<AdlOverdriveNTemperatureGet>(b"ADL2_OverdriveN_Temperature_Get\0")
                .ok()
                .map(|symbol| *symbol)
        };
        let performance = unsafe {
            library
                .get::<AdlOverdriveNPerformanceStatusGet>(
                    b"ADL2_OverdriveN_PerformanceStatus_Get\0",
                )
                .ok()
                .map(|symbol| *symbol)
        };
        let vram_usage = unsafe {
            library
                .get::<AdlDedicatedVramUsageGet>(b"ADL2_Adapter_DedicatedVRAMUsage_Get\0")
                .ok()
                .map(|symbol| *symbol)
        };
        let memory_info = unsafe {
            library
                .get::<AdlMemoryInfoX4Get>(b"ADL2_Adapter_MemoryInfoX4_Get\0")
                .ok()
                .map(|symbol| *symbol)
        };

        let mut monitor = Self {
            _library: library,
            context,
            adapter_index: -1,
            name: String::new(),
            destroy,
            activity,
            temperature5,
            temperature_n,
            performance,
            vram_usage,
            memory_info,
        };

        let (index, name) = monitor.find_adapter(number_of_adapters, adapter_info)?;
        monitor.adapter_index = index;
        monitor.name = name;
        Ok(monitor)
    }

    /// Primeiro adaptador AMD presente, com o nome que o driver reporta.
    fn find_adapter(
        &self,
        number_of_adapters: AdlNumberOfAdaptersGet,
        adapter_info: AdlAdapterInfoGet,
    ) -> Result<(c_int, String), String> {
        let mut count = 0;
        if unsafe { number_of_adapters(self.context, &mut count) } != ADL_OK || count <= 0 {
            return Err("A ADL não encontrou adaptadores de vídeo.".to_owned());
        }

        let mut infos: Vec<AdlAdapterInfo> = vec![zeroed(); count as usize];
        let entry_size = std::mem::size_of::<AdlAdapterInfo>() as c_int;
        for info in &mut infos {
            info.size = entry_size;
        }
        let buffer_size = entry_size * count;
        if unsafe { adapter_info(self.context, infos.as_mut_ptr(), buffer_size) } != ADL_OK {
            return Err("A ADL não devolveu a lista de adaptadores.".to_owned());
        }

        for info in &infos {
            if info.vendor_id == ATI_VENDOR_ID && info.present != 0 {
                let name = fixed_str(&info.adapter_name);
                let name = if name.is_empty() {
                    "GPU AMD".to_owned()
                } else {
                    name
                };
                return Ok((info.adapter_index, name));
            }
        }
        Err("Nenhum adaptador AMD presente foi encontrado pela ADL.".to_owned())
    }

    /// Preenche apenas os campos ainda vazios: a NVML tem prioridade e o
    /// LibreHardwareMonitor cobre o que sobrar.
    pub fn collect(&self, metrics: &mut SystemMetrics) {
        if metrics.gpu_usage_percent.is_none() {
            if let Some(activity) = self.activity {
                let mut value = AdlPmActivity {
                    size: std::mem::size_of::<AdlPmActivity>() as c_int,
                    ..AdlPmActivity::default()
                };
                if unsafe { activity(self.context, self.adapter_index, &mut value) } == ADL_OK {
                    metrics.gpu_usage_percent = Some(value.activity_percent.clamp(0, 100) as u32);
                }
            }
        }
        if metrics.gpu_usage_percent.is_none() {
            if let Some(performance) = self.performance {
                let mut status = AdlOdnPerformanceStatus::default();
                if unsafe { performance(self.context, self.adapter_index, &mut status) } == ADL_OK {
                    metrics.gpu_usage_percent =
                        Some(status.gpu_activity_percent.clamp(0, 100) as u32);
                }
            }
        }

        if metrics.gpu_temperature_c.is_none() {
            if let Some(temperature) = self.temperature_n {
                let mut millidegrees = 0;
                if unsafe {
                    temperature(
                        self.context,
                        self.adapter_index,
                        ADL_ODN_TEMP_EDGE,
                        &mut millidegrees,
                    )
                } == ADL_OK
                    && millidegrees > 0
                {
                    metrics.gpu_temperature_c =
                        Some(((millidegrees as f32) / 1000.0).round() as u32);
                }
            }
        }
        if metrics.gpu_temperature_c.is_none() {
            if let Some(temperature) = self.temperature5 {
                let mut value = AdlTemperature {
                    size: std::mem::size_of::<AdlTemperature>() as c_int,
                    temperature: 0,
                };
                if unsafe { temperature(self.context, self.adapter_index, 0, &mut value) } == ADL_OK
                    && value.temperature > 0
                {
                    metrics.gpu_temperature_c =
                        Some(((value.temperature as f32) / 1000.0).round() as u32);
                }
            }
        }

        if metrics.vram_used_bytes.is_none() {
            if let Some(vram_usage) = self.vram_usage {
                let mut used_mb = 0;
                if unsafe { vram_usage(self.context, self.adapter_index, &mut used_mb) } == ADL_OK
                    && used_mb >= 0
                {
                    metrics.vram_used_bytes = Some(used_mb as u64 * 1024 * 1024);
                }
            }
        }
        if metrics.vram_total_bytes.is_none() {
            if let Some(memory_info) = self.memory_info {
                let mut memory: AdlMemoryInfoX4 = zeroed();
                if unsafe { memory_info(self.context, self.adapter_index, &mut memory) } == ADL_OK
                    && memory.memory_size > 0
                {
                    metrics.vram_total_bytes = Some(memory.memory_size as u64);
                }
            }
        }

        if metrics.gpu_name.is_none() && !self.name.is_empty() {
            metrics.gpu_name = Some(self.name.clone());
        }
    }
}

impl Drop for AmdTelemetry {
    fn drop(&mut self) {
        if !self.context.is_null() {
            unsafe { (self.destroy)(self.context) };
            self.context = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_info_layout_matches_the_c_abi() {
        // 15 campos + strings de 256 bytes: o `iSize` que a ADL cobra precisa
        // bater com o do cabeçalho em C (1572 bytes em x64).
        assert_eq!(std::mem::size_of::<AdlAdapterInfo>(), 1572);
        assert_eq!(std::mem::size_of::<AdlPmActivity>(), 40);
        assert_eq!(std::mem::size_of::<AdlTemperature>(), 8);
        assert_eq!(std::mem::size_of::<AdlOdnPerformanceStatus>(), 72);
        assert_eq!(std::mem::size_of::<AdlMemoryInfoX4>(), 320);
    }

    #[test]
    fn reads_nul_terminated_strings_and_tolerates_missing_terminator() {
        let mut buffer = [0u8; 8];
        buffer[..4].copy_from_slice(b"RX 6");
        assert_eq!(fixed_str(&buffer), "RX 6");

        let missing = b"Radeon".to_vec();
        assert_eq!(fixed_str(&missing), "Radeon");
    }
}
