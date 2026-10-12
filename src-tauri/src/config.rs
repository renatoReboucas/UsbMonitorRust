use std::{fs, path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub theme: String,
    pub brightness: u8,
    pub rotation: u16,
    pub mirror: bool,
    pub accent_color: String,
    pub cpu_custom: String,
    pub gpu_custom: String,
    pub ram_custom: String,
    pub sensor: String,
    pub network: String,
    pub city: String,
    pub start_with_windows: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "3.5inchTheme1".to_owned(),
            brightness: 80,
            rotation: 0,
            mirror: false,
            accent_color: "#35d7e9".to_owned(),
            cpu_custom: String::new(),
            gpu_custom: String::new(),
            ram_custom: String::new(),
            sensor: "CPU (Tctl/Tdie)".to_owned(),
            network: "Detecção automática".to_owned(),
            city: String::new(),
            start_with_windows: false,
        }
    }
}

impl AppConfig {
    pub fn validate(mut self) -> Result<Self, String> {
        const THEMES: [&str; 8] = [
            "3.5inchTheme1",
            "3.5inchTheme2",
            "MagicBlue",
            "LandscapeTechnology",
            "LandscapeMagicBlue",
            "LandscapeDeepSpace",
            "AORUS",
            "LandscapeSimpleWhite",
        ];
        if self.brightness > 100 {
            return Err("O brilho precisa estar entre 0 e 100%.".to_owned());
        }
        if !matches!(self.rotation, 0 | 180) {
            return Err("A rotação precisa ser 0° ou 180°.".to_owned());
        }
        if !THEMES.contains(&self.theme.as_str()) {
            return Err("O tema selecionado não existe.".to_owned());
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
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|error| format!("O arquivo de configurações está inválido: {error}")),
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
    fn validates_hex_color_without_panicking_on_unicode() {
        assert!(!valid_color("#ééééé"));
        assert!(!valid_color("abcdefg"));
    }
}
