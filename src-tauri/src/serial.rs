use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Serialize;
use serialport::{FlowControl, SerialPort, SerialPortInfo, SerialPortType};

const USB_VID: u16 = 0x1a86;
const USB_PID: u16 = 0x5722;
const BAUD_RATE: u32 = 115_200;
const DISPLAY_WIDTH: u32 = 480;
const DISPLAY_HEIGHT: u32 = 320;
const DISPLAY_BITMAP_COMMAND: u8 = 197;
const SET_ORIENTATION_COMMAND: u8 = 121;
const LANDSCAPE_ORIENTATION: u8 = 3;
const RGB565_PIXEL_BYTES: usize = 2;
const WRITE_CHUNK_BYTES: usize = DISPLAY_WIDTH as usize * 8;

#[derive(Clone, Default)]
pub struct SerialState(Arc<Mutex<Option<SerialConnection>>>);

struct SerialConnection {
    port_name: String,
    port: Box<dyn SerialPort>,
}

#[derive(Serialize)]
pub struct DeviceStatus {
    connected: bool,
    port: Option<String>,
    description: Option<String>,
    port_open: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSendResult {
    pub width: u32,
    pub height: u32,
    pub bytes_sent: usize,
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
    if baud_rate != BAUD_RATE {
        return Err(format!(
            "Este display usa {BAUD_RATE} baud; a velocidade selecionada não é compatível."
        ));
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
    let mut port = serialport::new(port_name, baud_rate)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_secs(5))
        .open()
        .map_err(|error| {
            format!(
                "Não foi possível abrir {port_name}: {error}. A porta pode estar ocupada por outra instância do UsbMonitor (inclusive oculta na bandeja), pelo programa original do display ou por um terminal serial. Feche a outra conexão ou use “Liberar porta serial” no menu da bandeja e tente novamente."
            )
        })?;
    port.write_data_terminal_ready(true)
        .map_err(|error| format!("Não foi possível ativar DTR em {port_name}: {error}"))?;
    port.write_request_to_send(true)
        .map_err(|error| format!("Não foi possível ativar RTS em {port_name}: {error}"))?;
    let mut serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível guardar a conexão serial.".to_owned())?;
    if serial.is_some() {
        return Err("A porta serial foi aberta por outra operação.".to_owned());
    }
    send_orientation(port.as_mut()).map_err(|error| {
        format!("A porta abriu, mas não foi possível inicializar a tela: {error}")
    })?;
    *serial = Some(SerialConnection {
        port_name: port_name.to_owned(),
        port,
    });
    Ok(())
}

pub fn send_png_frame(state: &SerialState, png_bytes: &[u8]) -> Result<FrameSendResult, String> {
    let pixels = decode_rgb565(png_bytes)?;
    let mut serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível acessar a conexão do display.".to_owned())?;
    let connection = serial
        .as_mut()
        .ok_or_else(|| "Abra a conexão COM antes de enviar o display.".to_owned())?;

    let write_result = send_bitmap(connection.port.as_mut(), &pixels);
    if let Err(error) = write_result {
        let port_name = connection.port_name.clone();
        *serial = None;
        return Err(format!(
            "A transmissão pela {port_name} falhou; a conexão foi fechada para evitar deixar a tela em estado incompleto: {error}"
        ));
    }

    Ok(FrameSendResult {
        width: DISPLAY_WIDTH,
        height: DISPLAY_HEIGHT,
        bytes_sent: pixels.len(),
    })
}

pub fn disconnect(state: &SerialState) -> Result<(), String> {
    let mut serial = state
        .0
        .lock()
        .map_err(|_| "Não foi possível fechar a porta serial.".to_owned())?;
    *serial = None;
    Ok(())
}

fn decode_rgb565(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("A prévia do display não é um PNG válido: {error}"))?;
    if reader.info().width != DISPLAY_WIDTH || reader.info().height != DISPLAY_HEIGHT {
        return Err(format!(
            "A imagem precisa ter resolução {DISPLAY_WIDTH} × {DISPLAY_HEIGHT}; recebeu {} × {}.",
            reader.info().width,
            reader.info().height
        ));
    }
    let mut decoded = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut decoded)
        .map_err(|error| format!("Não foi possível decodificar a prévia do display: {error}"))?;
    let data = &decoded[..info.buffer_size()];
    let mut rgb565 =
        Vec::with_capacity(DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize * RGB565_PIXEL_BYTES);
    match info.color_type {
        png::ColorType::Rgb => {
            let (pixels, remainder) = data.as_chunks::<3>();
            if !remainder.is_empty() {
                return Err("A imagem PNG RGB está incompleta.".to_owned());
            }
            for pixel in pixels {
                append_rgb565(&mut rgb565, pixel[0], pixel[1], pixel[2]);
            }
        }
        png::ColorType::Rgba => {
            let (pixels, remainder) = data.as_chunks::<4>();
            if !remainder.is_empty() {
                return Err("A imagem PNG RGBA está incompleta.".to_owned());
            }
            for pixel in pixels {
                append_rgb565(&mut rgb565, pixel[0], pixel[1], pixel[2]);
            }
        }
        png::ColorType::Grayscale => {
            for gray in data {
                append_rgb565(&mut rgb565, *gray, *gray, *gray);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            let (pixels, remainder) = data.as_chunks::<2>();
            if !remainder.is_empty() {
                return Err("A imagem PNG em tons de cinza está incompleta.".to_owned());
            }
            for pixel in pixels {
                append_rgb565(&mut rgb565, pixel[0], pixel[0], pixel[0]);
            }
        }
        png::ColorType::Indexed => {
            return Err("A prévia do display usa um formato PNG não suportado.".to_owned())
        }
    }
    Ok(rgb565)
}

fn append_rgb565(output: &mut Vec<u8>, red: u8, green: u8, blue: u8) {
    let packed = (u16::from(red >> 3) << 11) | (u16::from(green >> 2) << 5) | u16::from(blue >> 3);
    output.extend_from_slice(&packed.to_le_bytes());
}

fn command_header(x: u16, y: u16, end_x: u16, end_y: u16, command: u8) -> [u8; 6] {
    [
        (x >> 2) as u8,
        (((x & 0x03) << 6) | (y >> 4)) as u8,
        (((y & 0x0f) << 4) | (end_x >> 6)) as u8,
        (((end_x & 0x3f) << 2) | (end_y >> 8)) as u8,
        end_y as u8,
        command,
    ]
}

fn send_orientation(port: &mut dyn SerialPort) -> std::io::Result<()> {
    port.write_all(&orientation_command())?;
    port.flush()
}

fn orientation_command() -> [u8; 16] {
    let mut command = [0; 16];
    command[5] = SET_ORIENTATION_COMMAND;
    command[6] = LANDSCAPE_ORIENTATION + 100;
    command[7..9].copy_from_slice(&(DISPLAY_WIDTH as u16).to_be_bytes());
    command[9..11].copy_from_slice(&(DISPLAY_HEIGHT as u16).to_be_bytes());
    command
}

fn send_bitmap(port: &mut dyn SerialPort, pixels: &[u8]) -> std::io::Result<()> {
    let header = command_header(
        0,
        0,
        DISPLAY_WIDTH as u16 - 1,
        DISPLAY_HEIGHT as u16 - 1,
        DISPLAY_BITMAP_COMMAND,
    );
    port.write_all(&header)?;
    std::thread::sleep(Duration::from_millis(10));
    for chunk in pixels.chunks(WRITE_CHUNK_BYTES) {
        port.write_all(chunk)?;
    }
    std::thread::sleep(Duration::from_millis(10));
    port.flush()
}

fn is_target(port: &SerialPortInfo) -> bool {
    matches!(
        &port.port_type,
        SerialPortType::UsbPort(usb) if usb.vid == USB_VID && usb.pid == USB_PID
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_landscape_bitmap_header_uses_screen_coordinates() {
        assert_eq!(
            command_header(0, 0, 479, 319, DISPLAY_BITMAP_COMMAND),
            [0, 0, 7, 125, 63, 197]
        );
    }

    #[test]
    fn rgb565_pixels_are_packed_little_endian() {
        let mut pixels = Vec::new();
        append_rgb565(&mut pixels, 255, 0, 0);
        append_rgb565(&mut pixels, 0, 255, 0);
        append_rgb565(&mut pixels, 0, 0, 255);
        assert_eq!(pixels, [0x00, 0xf8, 0xe0, 0x07, 0x1f, 0x00]);
    }

    #[test]
    fn orientation_command_declares_landscape_resolution() {
        assert_eq!(
            orientation_command(),
            [0, 0, 0, 0, 0, 121, 103, 1, 224, 1, 64, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn png_frame_is_decoded_to_the_expected_rgb565_size() {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, DISPLAY_WIDTH, DISPLAY_HEIGHT);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let image = [255, 0, 0, 255].repeat(DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize);
        writer.write_image_data(&image).unwrap();
        drop(writer);

        let pixels = decode_rgb565(&bytes).unwrap();
        assert_eq!(
            pixels.len(),
            DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize * RGB565_PIXEL_BYTES
        );
        assert_eq!(&pixels[..2], &[0x00, 0xf8]);
        assert_eq!(&pixels[pixels.len() - 2..], &[0x00, 0xf8]);
    }

    #[test]
    fn png_frame_rejects_a_non_native_resolution() {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 320, 480);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&vec![0; 320 * 480 * 3]).unwrap();
        drop(writer);

        assert!(decode_rgb565(&bytes).unwrap_err().contains("480 × 320"));
    }

    #[test]
    fn target_identity_matches_the_observed_usb_adapter() {
        assert_eq!(super::USB_VID, 0x1a86);
        assert_eq!(super::USB_PID, 0x5722);
    }
}
