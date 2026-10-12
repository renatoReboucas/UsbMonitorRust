use std::{sync::Mutex, time::Duration};

use serde::Serialize;
use serialport::{SerialPort, SerialPortInfo, SerialPortType};

const USB_VID: u16 = 0x1a86;
const USB_PID: u16 = 0x5722;

#[derive(Default)]
pub struct SerialState(Mutex<Option<(String, Box<dyn SerialPort>)>>);

#[derive(Serialize)]
pub struct DeviceStatus {
    connected: bool,
    port: Option<String>,
    description: Option<String>,
    port_open: bool,
}

pub fn status(state: &SerialState) -> Result<DeviceStatus, String> {
    let ports = serialport::available_ports()
        .map_err(|error| format!("Falha ao consultar portas seriais: {error}"))?;
    let target = ports.into_iter().find(is_target);
    let serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível consultar o estado da porta serial.".to_owned())?;

    Ok(DeviceStatus {
        connected: target.is_some(),
        port: target.as_ref().map(|device| device.port_name.clone()),
        description: target.as_ref().map(|device| match &device.port_type {
            SerialPortType::UsbPort(usb) => usb
                .product
                .clone()
                .unwrap_or_else(|| format!("USB {:04X}:{:04X}", usb.vid, usb.pid)),
            _ => "Dispositivo serial".to_owned(),
        }),
        port_open: serial.is_some(),
    })
}

pub fn connect(state: &SerialState, port_name: &str, baud_rate: u32) -> Result<(), String> {
    if baud_rate == 0 {
        return Err("A velocidade serial precisa ser maior que zero.".to_owned());
    }
    {
        let serial = state
            .0
            .lock()
            .map_err(|_| "Não foi possível consultar o estado da porta serial.".to_owned())?;
        if serial.is_some() {
            return Err("A porta serial já está aberta.".to_owned());
        }
    }
    let ports = serialport::available_ports()
        .map_err(|error| format!("Falha ao consultar portas seriais: {error}"))?;
    if !ports
        .iter()
        .any(|device| device.port_name == port_name && is_target(device))
    {
        return Err("A porta não corresponde ao display USB VID_1A86&PID_5722.".to_owned());
    }
    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|error| {
            format!(
                "Não foi possível abrir {port_name}: {error}. A porta pode estar ocupada por outra instância do UsbMonitor (inclusive oculta na bandeja), pelo programa original do display ou por um terminal serial. Feche a outra conexão ou use “Liberar porta serial” no menu da bandeja e tente novamente."
            )
        })?;
    let mut serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível guardar a conexão serial.".to_owned())?;
    if serial.is_some() {
        return Err("A porta serial foi aberta por outra operação.".to_owned());
    }
    *serial = Some((port_name.to_owned(), port));
    Ok(())
}

pub fn disconnect(state: &SerialState) -> Result<(), String> {
    let mut serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível fechar a porta serial.".to_owned())?;
    *serial = None;
    Ok(())
}

fn is_target(port: &SerialPortInfo) -> bool {
    matches!(
        &port.port_type,
        SerialPortType::UsbPort(usb) if usb.vid == USB_VID && usb.pid == USB_PID
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn target_identity_matches_the_observed_usb_adapter() {
        assert_eq!(super::USB_VID, 0x1a86);
        assert_eq!(super::USB_PID, 0x5722);
    }
}
