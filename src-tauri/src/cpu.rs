use sysinfo::System;

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
