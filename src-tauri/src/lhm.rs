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

/// Uma leitura completa: atualiza os sensores e calcula a temperatura da CPU.
async fn read(client: &mut LHMClient) -> std::io::Result<Option<u32>> {
    timeout(REQUEST_TIMEOUT, client.update())
        .await
        .map_err(timed_out)??;
    let hardware = timeout(REQUEST_TIMEOUT, client.get_hardware())
        .await
        .map_err(timed_out)??;
    Ok(cpu_temperature(&hardware))
}

pub struct HardwareMonitor {
    runtime: Runtime,
    client: Option<LHMClient>,
}

impl HardwareMonitor {
    /// Cria o runtime e tenta conectar de imediato. `None` só quando o próprio
    /// runtime não pôde ser criado; serviço ausente é um estado normal.
    pub fn load() -> Option<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| eprintln!("LibreHardwareMonitor: runtime não criado: {error}"))
            .ok()?;

        let mut monitor = Self {
            runtime,
            client: None,
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
        // Só a CPU importa aqui: a GPU vem da NVML e o resto do `sysinfo`.
        // Menos hardware habilitado deixa a atualização mais leve.
        let options = ComputerOptions {
            cpu_enabled: true,
            motherboard_enabled: true,
            controller_enabled: true,
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

    /// Temperatura da CPU em graus Celsius, ou `None` quando o serviço está
    /// ausente/parado ou não há leitura válida.
    pub fn cpu_temperature_celsius(&mut self) -> Option<u32> {
        let mut client = self.client.take()?;

        // Em qualquer erro a conexão é descartada (a próxima tentativa abre uma
        // nova), porque o pipe pode ter ficado em estado indefinido.
        match self.runtime.block_on(read(&mut client)) {
            Ok(temperature) => {
                self.client = Some(client);
                temperature
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

    fn cpu(sensors: Vec<Sensor>) -> Hardware {
        Hardware {
            name: "AMD Ryzen 7 5700X".to_owned(),
            ty: HardwareType::Cpu,
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
}
