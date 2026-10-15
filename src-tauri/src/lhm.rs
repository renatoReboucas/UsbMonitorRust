//! Temperatura da CPU pelo **serviço** do LibreHardwareMonitor
//! (<https://github.com/jacobtread/lhm-service>).
//!
//! Placas que não publicam uma zona térmica ACPI — o caso típico de desktops
//! AM4/Ryzen — deixam [`crate::cpu::temperature_celsius`] vazio. O serviço do
//! LibreHardwareMonitor é quem carrega o driver de kernel e publica os sensores
//! em um named pipe, então este aplicativo continua rodando **sem** privilégios
//! de administrador. Sem o serviço instalado, toda leitura fica em `None`.
//!
//! O cliente embutido (`lhm-client 0.0.0`) fala exatamente o protocolo do
//! instalador publicado pelo projeto. As versões 0.1.0 em diante do serviço
//! trocaram as mensagens do pipe, então **não** são compatíveis com este
//! cliente; trocar de lado quebra a integração.

use std::{
    io::{Error as IoError, ErrorKind},
    time::Duration,
};

use lhm_client::{ComputerOptions, Hardware, HardwareType, LHMClient, Sensor, SensorType};
use tokio::{runtime::Runtime, time::timeout};

/// O serviço é local; se não responder nesse prazo, é melhor seguir sem ele.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
/// Conectar cria o canal do pipe; rápido, mas não instantâneo.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

fn timed_out(_: tokio::time::error::Elapsed) -> IoError {
    IoError::new(
        ErrorKind::TimedOut,
        "o serviço do LibreHardwareMonitor não respondeu",
    )
}

/// Sensores do LibreHardwareMonitor que são um delta até o limite, não uma
/// temperatura (`Core Distance to TjMax` vem negativo).
fn is_delta_sensor(sensor: &Sensor) -> bool {
    sensor.name.to_ascii_lowercase().contains("distance")
}

/// Todos os sensores de temperatura abaixo de um hardware, incluindo filhos.
fn collect_temperatures<'a>(hardware: &'a Hardware, out: &mut Vec<&'a Sensor>) {
    out.extend(
        hardware
            .sensors
            .iter()
            .filter(|sensor| matches!(sensor.ty, SensorType::Temperature)),
    );
    for child in &hardware.children {
        collect_temperatures(child, out);
    }
}

/// Percorre a árvore atrás de hardware de CPU e reúne suas temperaturas. Os
/// outros ramos (GPU, disco) ficam de fora, senão a GPU entraria como "CPU".
fn collect_cpu_temperatures<'a>(hardware: &'a [Hardware], out: &mut Vec<&'a Sensor>) {
    for item in hardware {
        if matches!(item.ty, HardwareType::Cpu) {
            collect_temperatures(item, out);
        } else {
            collect_cpu_temperatures(&item.children, out);
        }
    }
}

/// Maior leitura válida entre os sensores, arredondada para graus inteiros.
fn hottest_temperature(sensors: &[&Sensor]) -> Option<u32> {
    sensors
        .iter()
        .filter(|sensor| !is_delta_sensor(sensor))
        .map(|sensor| sensor.value)
        .filter(|value| value.is_finite() && *value > 0.0)
        .reduce(f32::max)
        .map(|value| value.round() as u32)
}

/// Temperatura da CPU, em graus Celsius, a partir da árvore de hardware.
fn cpu_temperature(hardware: &[Hardware]) -> Option<u32> {
    let mut sensors = Vec::new();
    collect_cpu_temperatures(hardware, &mut sensors);
    hottest_temperature(&sensors)
}

/// Todos os sensores abaixo de um hardware, incluindo sub-hardwares.
fn collect_sensors<'a>(hardware: &'a Hardware, out: &mut Vec<&'a Sensor>) {
    out.extend(hardware.sensors.iter());
    for child in &hardware.children {
        collect_sensors(child, out);
    }
}

fn is_gpu(hardware: &Hardware) -> bool {
    matches!(
        hardware.ty,
        HardwareType::GpuNvidia | HardwareType::GpuAmd | HardwareType::GpuIntel
    )
}

/// Primeiro hardware de GPU na árvore (a LHM publica uma entrada por placa).
fn find_gpu(hardware: &[Hardware]) -> Option<&Hardware> {
    for item in hardware {
        if is_gpu(item) {
            return Some(item);
        }
        if let Some(found) = find_gpu(&item.children) {
            return Some(found);
        }
    }
    None
}

/// Temperatura válida de um sensor, em graus inteiros.
fn valid_temperature(sensor: &Sensor) -> Option<u32> {
    (matches!(sensor.ty, SensorType::Temperature)
        && !is_delta_sensor(sensor)
        && sensor.value.is_finite()
        && sensor.value > 0.0)
        .then(|| sensor.value.round() as u32)
}

/// `GPU Core` é a borda do chip — a temperatura que os monitores chamam de
/// "GPU". Sem ela, a primeira leitura válida serve.
fn gpu_temperature(sensors: &[&Sensor]) -> Option<u32> {
    sensors
        .iter()
        .filter(|sensor| sensor.name.to_ascii_lowercase().contains("core"))
        .find_map(|sensor| valid_temperature(sensor))
        .or_else(|| sensors.iter().find_map(|sensor| valid_temperature(sensor)))
}

/// Uso da GPU: `GPU Core` (a carga geral) e, na falta dele, qualquer sensor de
/// carga válido. O valor é percentual.
fn gpu_usage(sensors: &[&Sensor]) -> Option<u32> {
    let load = |sensor: &&Sensor| {
        (matches!(sensor.ty, SensorType::Load) && sensor.value.is_finite())
            .then(|| sensor.value.round().clamp(0.0, 100.0) as u32)
    };
    sensors
        .iter()
        .filter(|sensor| sensor.name.to_ascii_lowercase().contains("core"))
        .find_map(load)
        .or_else(|| sensors.iter().find_map(load))
}

/// Converte um sensor de memória em bytes. A LHM 0.9.4 publica a memória de
/// vídeo como `SmallData` em **megabytes**; a `Data` (bytes) existe em outras
/// versões e ambas são aceitas.
fn sensor_bytes(sensor: &Sensor) -> Option<u64> {
    if !sensor.value.is_finite() || sensor.value < 0.0 {
        return None;
    }
    match sensor.ty {
        SensorType::Data => Some(sensor.value.round() as u64),
        SensorType::SmallData => Some((sensor.value as f64 * 1024.0 * 1024.0).round() as u64),
        _ => None,
    }
}

fn memory_sensor(sensors: &[&Sensor], needles: &[&str]) -> Option<u64> {
    for needle in needles {
        for sensor in sensors {
            if sensor.name.to_ascii_lowercase().contains(needle) {
                if let Some(bytes) = sensor_bytes(sensor) {
                    return Some(bytes);
                }
            }
        }
    }
    None
}

/// Memória de vídeo usada e total, preferindo os sensores dedicados da LHM aos
/// do D3D (que ficam disponíveis mesmo sem o sensor de VRAM do driver).
fn gpu_memory(sensors: &[&Sensor]) -> (Option<u64>, Option<u64>) {
    let used = memory_sensor(sensors, &["gpu memory used", "d3d dedicated memory used"]);
    let total = memory_sensor(sensors, &["gpu memory total", "d3d dedicated memory total"]);
    (used, total)
}

/// Leitura da GPU publicada por um hardware da LHM.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct GpuReading {
    pub name: Option<String>,
    pub usage_percent: Option<u32>,
    pub temperature_c: Option<u32>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
}

/// Resultado de um ciclo de leitura: a temperatura da CPU e, quando pedida, a
/// GPU. A LHM cobre NVIDIA, AMD e Intel pelo mesmo serviço.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Readings {
    pub cpu_temperature_c: Option<u32>,
    pub gpu: Option<GpuReading>,
}

fn gpu_reading(hardware: &[Hardware]) -> Option<GpuReading> {
    let gpu = find_gpu(hardware)?;
    let mut sensors = Vec::new();
    collect_sensors(gpu, &mut sensors);
    let (vram_used_bytes, vram_total_bytes) = gpu_memory(&sensors);
    let name = gpu.name.trim();
    Some(GpuReading {
        name: (!name.is_empty()).then(|| name.to_owned()),
        usage_percent: gpu_usage(&sensors),
        temperature_c: gpu_temperature(&sensors),
        vram_used_bytes,
        vram_total_bytes,
    })
}

/// Uma leitura completa: atualiza os sensores e extrai CPU (e GPU, quando
/// habilitada) da mesma resposta.
async fn read(client: &mut LHMClient, include_gpu: bool) -> std::io::Result<Readings> {
    timeout(REQUEST_TIMEOUT, client.update())
        .await
        .map_err(timed_out)??;
    let hardware = timeout(REQUEST_TIMEOUT, client.get_hardware())
        .await
        .map_err(timed_out)??;
    Ok(Readings {
        cpu_temperature_c: cpu_temperature(&hardware),
        gpu: if include_gpu {
            gpu_reading(&hardware)
        } else {
            None
        },
    })
}

pub struct HardwareMonitor {
    runtime: Runtime,
    client: Option<LHMClient>,
    /// A GPU só é habilitada quando nenhuma outra fonte pode fornecê-la; assim
    /// a atualização fica mais leve em máquinas com NVIDIA.
    gpu_enabled: bool,
}

impl HardwareMonitor {
    /// Cria o runtime e tenta conectar de imediato. `None` só quando o próprio
    /// runtime não pôde ser criado; serviço ausente é um estado normal.
    pub fn load(gpu_enabled: bool) -> Option<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| eprintln!("LibreHardwareMonitor: runtime não criado: {error}"))
            .ok()?;

        let mut monitor = Self {
            runtime,
            client: None,
            gpu_enabled,
        };
        monitor.connect();

        if monitor.client.is_none() {
            eprintln!(
                "LibreHardwareMonitor: serviço não respondeu; a temperatura da CPU fica em N/D (veja o README)."
            );
        }

        Some(monitor)
    }

    /// `true` quando o serviço respondeu à última tentativa de conexão.
    pub fn is_ready(&self) -> bool {
        self.client.is_some()
    }

    /// Descarta a conexão atual e tenta reconectar (usado quando o serviço é
    /// instalado depois do aplicativo).
    pub fn reconnect(&mut self) {
        self.client = None;
        self.connect();
    }

    fn connect(&mut self) {
        // A CPU e o resto do `sysinfo` vêm de outras fontes; aqui só entram a
        // temperatura da CPU e, quando necessário, a GPU. Menos hardware
        // habilitado deixa a atualização mais leve.
        let options = ComputerOptions {
            cpu_enabled: true,
            motherboard_enabled: true,
            controller_enabled: true,
            gpu_enabled: self.gpu_enabled,
            ..ComputerOptions::default()
        };

        self.client = self.runtime.block_on(async {
            let mut client = match timeout(CONNECT_TIMEOUT, LHMClient::connect()).await {
                Ok(Ok(client)) => client,
                Ok(Err(error)) => {
                    eprintln!("LibreHardwareMonitor: conexão recusada ({error})");
                    return None;
                }
                Err(_) => return None,
            };
            match timeout(REQUEST_TIMEOUT, client.set_options(options)).await {
                Ok(Ok(())) => Some(client),
                Ok(Err(error)) => {
                    eprintln!("LibreHardwareMonitor: falha ao configurar os sensores ({error})");
                    None
                }
                Err(_) => None,
            }
        });
    }

    /// Temperatura da CPU e, quando habilitada, a leitura da GPU. `None`
    /// quando o serviço está ausente/parado ou não há leitura válida.
    pub fn read(&mut self) -> Option<Readings> {
        let mut client = self.client.take()?;

        // Em qualquer erro a conexão é descartada (a próxima tentativa abre uma
        // nova), porque o pipe pode ter ficado em estado indefinido.
        match self.runtime.block_on(read(&mut client, self.gpu_enabled)) {
            Ok(readings) => {
                self.client = Some(client);
                Some(readings)
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(name: &str, value: f32) -> Sensor {
        Sensor {
            name: name.to_owned(),
            ty: SensorType::Temperature,
            value,
        }
    }

    fn typed_sensor(name: &str, ty: SensorType, value: f32) -> Sensor {
        Sensor {
            name: name.to_owned(),
            ty,
            value,
        }
    }

    fn cpu(sensors: Vec<Sensor>) -> Hardware {
        Hardware {
            name: "AMD Ryzen 7 5700X".to_owned(),
            ty: HardwareType::Cpu,
            children: Vec::new(),
            sensors,
        }
    }

    fn gpu(sensors: Vec<Sensor>) -> Hardware {
        Hardware {
            name: "AMD Radeon RX 6800".to_owned(),
            ty: HardwareType::GpuAmd,
            children: Vec::new(),
            sensors,
        }
    }

    #[test]
    fn picks_the_hottest_valid_reading_of_the_cpu() {
        let hardware = vec![
            Hardware {
                name: "NVIDIA GeForce RTX 5060 Ti".to_owned(),
                ty: HardwareType::GpuNvidia,
                children: Vec::new(),
                sensors: vec![sensor("GPU Core", 99.0)],
            },
            cpu(vec![
                sensor("Core (Tctl/Tdie)", 52.4),
                sensor("Core #1", 41.0),
                sensor("Core Distance to TjMax", -38.0),
            ]),
        ];
        assert_eq!(cpu_temperature(&hardware), Some(52));
    }

    #[test]
    fn ignores_missing_and_invalid_readings() {
        let hardware = vec![cpu(vec![
            sensor("Core #1", f32::NAN),
            sensor("Core #2", 0.0),
        ])];
        assert_eq!(cpu_temperature(&hardware), None);
    }

    #[test]
    fn finds_cpu_sensors_nested_in_children() {
        let mut motherboard = Hardware {
            name: "Mainboard".to_owned(),
            ty: HardwareType::Motherboard,
            children: vec![cpu(vec![sensor("CPU Package", 47.2)])],
            sensors: vec![],
        };
        motherboard.children[0].children = Vec::new();
        assert_eq!(cpu_temperature(&[motherboard]), Some(47));
    }

    #[test]
    fn reads_amd_gpu_temperature_usage_and_memory() {
        // Valores como a LHM 0.9.4 publica numa Radeon: memória em MB
        // (`SmallData`), carga em percentual e temperatura em graus.
        let hardware = vec![
            cpu(vec![sensor("Core (Tctl/Tdie)", 55.0)]),
            gpu(vec![
                sensor("GPU Core", 61.4),
                sensor("GPU Hot Spot", 78.0),
                typed_sensor("GPU Core", SensorType::Load, 37.0),
                typed_sensor("GPU Memory", SensorType::Load, 12.0),
                typed_sensor("GPU Memory Used", SensorType::SmallData, 2048.0),
                typed_sensor("GPU Memory Total", SensorType::SmallData, 16384.0),
            ]),
        ];
        assert_eq!(
            gpu_reading(&hardware),
            Some(GpuReading {
                name: Some("AMD Radeon RX 6800".to_owned()),
                usage_percent: Some(37),
                temperature_c: Some(61),
                vram_used_bytes: Some(2048 * 1024 * 1024),
                vram_total_bytes: Some(16384 * 1024 * 1024),
            })
        );
    }

    #[test]
    fn falls_back_to_d3d_memory_sensors_and_keeps_the_gpu_name() {
        let hardware = vec![gpu(vec![
            typed_sensor("D3D Dedicated Memory Used", SensorType::SmallData, 512.0),
            typed_sensor("D3D Dedicated Memory Total", SensorType::SmallData, 8192.0),
        ])];
        let reading = gpu_reading(&hardware).expect("a GPU existe");
        assert_eq!(reading.name.as_deref(), Some("AMD Radeon RX 6800"));
        assert_eq!(reading.vram_used_bytes, Some(512 * 1024 * 1024));
        assert_eq!(reading.vram_total_bytes, Some(8192 * 1024 * 1024));
        assert_eq!(reading.usage_percent, None);
        assert_eq!(reading.temperature_c, None);
    }

    #[test]
    fn accepts_byte_values_when_the_sensor_is_data() {
        let hardware = vec![gpu(vec![
            typed_sensor("GPU Memory Used", SensorType::Data, 1073741824.0),
            typed_sensor("GPU Memory Total", SensorType::Data, 8589934592.0),
        ])];
        let reading = gpu_reading(&hardware).expect("a GPU existe");
        assert_eq!(reading.vram_used_bytes, Some(1073741824));
        assert_eq!(reading.vram_total_bytes, Some(8589934592));
    }

    #[test]
    fn finds_the_gpu_nested_under_other_hardware() {
        let hardware = vec![Hardware {
            name: "Mainboard".to_owned(),
            ty: HardwareType::Motherboard,
            children: vec![gpu(vec![sensor("GPU Core", 50.0)])],
            sensors: vec![],
        }];
        assert_eq!(
            gpu_reading(&hardware).and_then(|gpu| gpu.temperature_c),
            Some(50)
        );
    }

    #[test]
    fn reports_no_gpu_when_none_is_published() {
        let hardware = vec![cpu(vec![sensor("Core #1", 40.0)])];
        assert_eq!(gpu_reading(&hardware), None);
    }
}
