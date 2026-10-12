use sysinfo::System;

pub fn usage_bytes(system: &System) -> (u64, u64) {
    (system.used_memory(), system.total_memory())
}
