use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager,
};

pub fn install(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Abrir UsbMonitor", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Encerrar", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let icon = tray_image();

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("UsbMonitor Rust")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            eprintln!("Não foi possível mostrar a janela: {error}");
        }
        if let Err(error) = window.set_focus() {
            eprintln!("Não foi possível focar a janela: {error}");
        }
    }
}

fn tray_image() -> Image<'static> {
    let mut pixels = vec![0; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            if (y < 4 && (3..13).contains(&x)) || (y >= 4 && (7..10).contains(&x)) {
                let offset = (y * 16 + x) * 4;
                pixels[offset..offset + 4].copy_from_slice(&[53, 215, 233, 255]);
            }
        }
    }
    Image::new_owned(pixels, 16, 16)
}
