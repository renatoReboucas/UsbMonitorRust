use std::{fs, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub display_template: String,
    pub palette: String,
    pub brightness: u8,
    pub rotation: u16,
    pub mirror: bool,
    pub accent_color: String,
    pub cpu_custom: String,
    pub gpu_custom: String,
    pub ram_custom: String,
    pub cpu_alert_c: u32,
    pub gpu_alert_c: u32,
    pub sensor: String,
    pub network: String,
    pub city: String,
    pub start_with_windows: bool,
    pub refresh_interval_secs: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            display_template: "crt".to_owned(),
            palette: "mocha".to_owned(),
            brightness: 80,
            rotation: 0,
            mirror: false,
            accent_color: "#35d7e9".to_owned(),
            cpu_custom: String::new(),
            gpu_custom: String::new(),
            ram_custom: String::new(),
            cpu_alert_c: 85,
            gpu_alert_c: 80,
            sensor: "CPU (Tctl/Tdie)".to_owned(),
            network: "Detecção automática".to_owned(),
            city: "São Paulo, São Paulo".to_owned(),
            start_with_windows: false,
            refresh_interval_secs: 5,
        }
    }
}

impl AppConfig {
    pub fn validate(mut self) -> Result<Self, String> {
        // Paletas do Catppuccin mais as duas do display (Console/verde e Retro).
        // Sem todas na lista, salvar uma delas seria recusado.
        const PALETTES: [&str; 6] = ["latte", "frappe", "macchiato", "mocha", "console", "retro"];
        if self.refresh_interval_secs == 0 || self.refresh_interval_secs > 3600 {
            self.refresh_interval_secs = 5;
        }
        if self.brightness > 100 {
            return Err("O brilho precisa estar entre 0 e 100%.".to_owned());
        }
        if !matches!(self.rotation, 0 | 180) {
            return Err("A rotação precisa ser 0° ou 180°.".to_owned());
        }
        if !valid_template_id(&self.display_template) {
            return Err("O identificador do layout HTML não é válido.".to_owned());
        }
        self.palette.make_ascii_lowercase();
        if !PALETTES.contains(&self.palette.as_str()) {
            return Err("A paleta de cores selecionada não existe.".to_owned());
        }
        for (label, value) in [
            ("CPU", &mut self.cpu_custom),
            ("GPU", &mut self.gpu_custom),
            ("RAM", &mut self.ram_custom),
        ] {
            *value = value.trim().to_owned();
            if value.chars().count() > 24 {
                return Err(format!(
                    "A identificação personalizada de {label} excede 24 caracteres."
                ));
            }
        }
        if self.cpu_alert_c > 150 {
            return Err("O alerta de temperatura da CPU precisa estar entre 0 e 150 °C.".to_owned());
        }
        if self.gpu_alert_c > 150 {
            return Err("O alerta de temperatura da GPU precisa estar entre 0 e 150 °C.".to_owned());
        }
        if !valid_color(&self.accent_color) {
            return Err("A cor de destaque deve estar no formato #RRGGBB.".to_owned());
        }
        self.city = self.city.trim().to_owned();
        if self.city.chars().count() > 100 {
            return Err("O nome da cidade excede 100 caracteres.".to_owned());
        }
        Ok(self)
    }
}

pub struct ConfigStore(pub Mutex<AppConfig>);

pub fn load(app: &AppHandle) -> Result<AppConfig, String> {
    let path = settings_path(app)?;
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let mut config: AppConfig = serde_json::from_str(&contents)
                .map_err(|error| format!("O arquivo de configurações está inválido: {error}"))?;
            if !valid_template_id(&config.display_template) {
                config.display_template = AppConfig::default().display_template;
            }
            if config.city.trim().is_empty() {
                config.city = AppConfig::default().city;
            }
            Ok(config)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(error) => Err(format!(
            "Não foi possível ler as configurações em {}: {error}",
            path.display()
        )),
    }
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = settings_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "O caminho das configurações não tem uma pasta pai.".to_owned())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("Não foi possível criar a pasta de configurações: {error}"))?;
    let contents = serde_json::to_vec_pretty(config)
        .map_err(|error| format!("Não foi possível serializar as configurações: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    fs::write(&temporary, contents)
        .map_err(|error| format!("Não foi possível gravar as configurações: {error}"))?;

    let had_previous = path.exists();
    if had_previous {
        if backup.exists() {
            fs::remove_file(&backup).map_err(|error| {
                format!("Não foi possível preparar o backup de configuração: {error}")
            })?;
        }
        fs::rename(&path, &backup).map_err(|error| {
            format!("Não foi possível proteger a configuração anterior: {error}")
        })?;
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        if had_previous {
            if let Err(restore_error) = fs::rename(&backup, &path) {
                return Err(format!(
                    "Não foi possível salvar as configurações ({error}) nem restaurar o backup ({}): {restore_error}",
                    backup.display()
                ));
            }
        }
        return Err(format!(
            "Não foi possível substituir as configurações: {error}"
        ));
    }
    if had_previous {
        fs::remove_file(&backup).map_err(|error| {
            format!("As configurações foram salvas, mas o backup não pôde ser removido: {error}")
        })?;
    }
    Ok(())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|directory| directory.join("settings.json"))
        .map_err(|error| format!("Não foi possível localizar a pasta de configuração: {error}"))
}

fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_template_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_rotation_brightness_color_and_custom_labels() {
        let valid = AppConfig {
            accent_color: "#a1B2c3".to_owned(),
            ..AppConfig::default()
        };
        assert!(valid.validate().is_ok());

        let invalid = AppConfig {
            rotation: 90,
            ..AppConfig::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn defaults_and_validates_temperature_alerts() {
        let defaults = AppConfig::default();
        assert_eq!(defaults.cpu_alert_c, 85);
        assert_eq!(defaults.gpu_alert_c, 80);

        let too_hot = AppConfig {
            gpu_alert_c: 200,
            ..AppConfig::default()
        };
        assert!(too_hot.validate().is_err());

        let disabled = AppConfig {
            cpu_alert_c: 0,
            gpu_alert_c: 0,
            ..AppConfig::default()
        };
        assert!(disabled.validate().is_ok());
    }

    #[test]
    fn validates_hex_color_without_panicking_on_unicode() {
        assert!(!valid_color("#ééééé"));
        assert!(!valid_color("abcdefg"));
    }

    #[test]
    fn accepts_only_safe_html_template_identifiers() {
        assert!(valid_template_id("my-display-v2"));
        assert!(!valid_template_id("../outside"));
        assert!(!valid_template_id("MyDisplay"));
    }

    #[test]
    fn legacy_theme_settings_fall_back_to_the_default_template() {
        let config: AppConfig = serde_json::from_str(r#"{"theme":"3.5inchTheme1"}"#).unwrap();
        assert_eq!(config.display_template, "crt");
    }

    #[test]
    fn accepts_every_palette_shipped_with_the_ui() {
        for palette in ["latte", "frappe", "macchiato", "mocha", "console", "retro"] {
            let config = AppConfig {
                palette: palette.to_owned(),
                ..AppConfig::default()
            };
            assert!(config.validate().is_ok(), "paleta recusada: {palette}");
        }
    }
}
