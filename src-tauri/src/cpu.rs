use sysinfo::{Components, System};

pub fn name(system: &System) -> Option<String> {
    system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn usage(system: &System) -> Option<f32> {
    Some(system.global_cpu_usage().clamp(0.0, 100.0))
}

pub fn frequency_mhz(system: &System) -> Option<u64> {
    system.cpus().first().map(|cpu| cpu.frequency())
}

/// CPU temperature in Celsius.
///
/// On Windows this reads the ACPI thermal zone through WMI
/// (`MSAcpi_ThermalZoneTemperature`), exposed by `sysinfo::Components`.
/// Many desktop boards do not publish a usable zone, so the result is
/// `None` when no valid reading is available.
pub fn temperature_celsius(components: &Components) -> Option<u32> {
    let mut cpu_zone: Option<f32> = None;
    let mut hottest_zone: Option<f32> = None;

    for component in components {
        let Some(value) = component.temperature() else {
            continue;
        };
        if !value.is_finite() || value <= 0.0 {
            continue;
        }
        let label = component.label().to_ascii_lowercase();
        if label.contains("cpu") || label.contains("core") || label.contains("package") {
            cpu_zone = Some(cpu_zone.map_or(value, |current| current.max(value)));
        }
        hottest_zone = Some(hottest_zone.map_or(value, |current| current.max(value)));
    }

    cpu_zone.or(hottest_zone).map(|value| value.round() as u32)
}
