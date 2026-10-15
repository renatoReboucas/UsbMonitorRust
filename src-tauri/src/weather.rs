use std::{
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

const CACHE_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherSnapshot {
    pub city: String,
    pub temperature_c: f64,
    pub description: String,
    pub icon: String,
    pub humidity_percent: u8,
    pub high_c: f64,
    pub low_c: f64,
    pub observed_at_unix_ms: u64,
}

#[derive(Default)]
pub struct WeatherState(Mutex<Option<CachedWeather>>);

struct CachedWeather {
    city: String,
    fetched_at: Instant,
    snapshot: WeatherSnapshot,
}

#[derive(Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Deserialize)]
struct GeocodingResult {
    name: String,
    feature_code: Option<String>,
    population: Option<u64>,
    country_code: Option<String>,
    country: Option<String>,
    admin1: Option<String>,
    latitude: f64,
    longitude: f64,
    timezone: Option<String>,
}

#[derive(Deserialize)]
struct ForecastResponse {
    current: CurrentWeather,
    daily: DailyWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    relative_humidity_2m: u8,
    weather_code: u8,
}

#[derive(Deserialize)]
struct DailyWeather {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

pub async fn current(state: &WeatherState, city: &str) -> Result<WeatherSnapshot, String> {
    let city = city.trim();
    if city.is_empty() || city.chars().count() > 100 {
        return Err("Informe uma cidade com até 100 caracteres.".to_owned());
    }

    {
        let cache = state
            .0
            .lock()
            .map_err(|_| "Não foi possível consultar o cache meteorológico.".to_owned())?;
        if let Some(cached) = cache.as_ref() {
            if cached.city.eq_ignore_ascii_case(city) && cached.fetched_at.elapsed() < CACHE_TTL {
                return Ok(cached.snapshot.clone());
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("UsbMonitorRust/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Não foi possível iniciar o cliente meteorológico: {error}"))?;

    let geocoding: GeocodingResponse = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[
            ("name", city),
            ("count", "100"),
            ("language", "pt"),
            ("format", "json"),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("Falha ao localizar “{city}” no serviço meteorológico: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Resposta de localização inválida para “{city}”: {error}"))?;

    let results = geocoding.results.unwrap_or_default();
    let query = parse_city_query(city)?;
    let place = select_place(&results, &query)
        .ok_or_else(|| format!("A cidade “{city}” não foi encontrada."))?;

    let latitude = place.latitude.to_string();
    let longitude = place.longitude.to_string();
    let timezone = place.timezone.as_deref().unwrap_or("auto");
    let forecast: ForecastResponse = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            (
                "current",
                "temperature_2m,relative_humidity_2m,weather_code",
            ),
            ("daily", "temperature_2m_max,temperature_2m_min"),
            ("forecast_days", "1"),
            ("timezone", timezone),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("Falha ao consultar a previsão de “{city}”: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Resposta meteorológica inválida para “{city}”: {error}"))?;

    let high_c = *forecast
        .daily
        .temperature_2m_max
        .first()
        .ok_or_else(|| "A previsão diária não trouxe a máxima.".to_owned())?;
    let low_c = *forecast
        .daily
        .temperature_2m_min
        .first()
        .ok_or_else(|| "A previsão diária não trouxe a mínima.".to_owned())?;
    let (description, icon) = weather_description(forecast.current.weather_code);
    let snapshot = WeatherSnapshot {
        city: format!(
            "{}, {}",
            place.name,
            place
                .admin1
                .as_deref()
                .or(place.country.as_deref())
                .unwrap_or("—")
        ),
        temperature_c: forecast.current.temperature_2m,
        description: description.to_owned(),
        icon: icon.to_owned(),
        humidity_percent: forecast.current.relative_humidity_2m,
        high_c,
        low_c,
        observed_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64,
    };

    let mut cache = state
        .0
        .lock()
        .map_err(|_| "Não foi possível atualizar o cache meteorológico.".to_owned())?;
    *cache = Some(CachedWeather {
        city: city.to_owned(),
        fetched_at: Instant::now(),
        snapshot: snapshot.clone(),
    });
    Ok(snapshot)
}

#[derive(Debug, PartialEq, Eq)]
struct CityQuery {
    name: String,
    region: Option<String>,
    country: Option<String>,
}

fn parse_city_query(city: &str) -> Result<CityQuery, String> {
    let mut parts = city
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let name = parts.next().unwrap_or_default();
    let second = parts.next();
    let third = parts.next();
    if parts.next().is_some() {
        return Err("Use o formato Cidade, Estado, País.".to_owned());
    }
    if name.is_empty() {
        return Err("Informe uma cidade válida.".to_owned());
    }
    let (region, country) = match (second, third) {
        (Some(scope), Some(country)) => (Some(scope.to_owned()), Some(country.to_owned())),
        (Some(scope), None) => (Some(scope.to_owned()), None),
        (None, _) => (None, None),
    };
    Ok(CityQuery {
        name: name.to_owned(),
        region,
        country,
    })
}

fn select_place<'a>(
    results: &'a [GeocodingResult],
    query: &CityQuery,
) -> Option<&'a GeocodingResult> {
    results
        .iter()
        .filter(|result| result.name.eq_ignore_ascii_case(&query.name))
        .filter(|result| {
            query.region.as_deref().is_none_or(|region| {
                result
                    .admin1
                    .as_deref()
                    .is_some_and(|admin| admin.eq_ignore_ascii_case(region))
                    || result
                        .country
                        .as_deref()
                        .is_some_and(|country| country.eq_ignore_ascii_case(region))
                    || (matches!(region.to_ascii_lowercase().as_str(), "brasil" | "brazil")
                        && result.country_code.as_deref() == Some("BR"))
            })
        })
        .filter(|result| {
            query.country.as_deref().is_none_or(|country| {
                if matches!(country.to_ascii_lowercase().as_str(), "brasil" | "brazil") {
                    result.country_code.as_deref() == Some("BR")
                } else {
                    result
                        .country
                        .as_deref()
                        .is_some_and(|found| found.eq_ignore_ascii_case(country))
                }
            })
        })
        .max_by_key(|result| {
            let feature_priority = match result.feature_code.as_deref() {
                Some("PPLC") => 3,
                Some("PPLA") => 2,
                Some("PPL") => 1,
                _ => 0,
            };
            (feature_priority, result.population.unwrap_or_default())
        })
}

/// Traduz o código do Open-Meteo em um rótulo e no **nome do ícone Lucide** que
/// a interface usa. O nome (e não um glifo) mantém o ícone vetorial e coerente
/// entre o app desktop e os layouts do display; ver `frontend/icons.js`.
fn weather_description(code: u8) -> (&'static str, &'static str) {
    match code {
        0 => ("CÉU LIMPO", "sun"),
        1 => ("POUCO NUBLADO", "cloud-sun"),
        2 => ("PARCIALMENTE NUBLADO", "cloud-sun"),
        3 => ("NUBLADO", "cloud"),
        45 | 48 => ("NEBLINA", "cloud-fog"),
        51 | 53 | 55 | 56 | 57 => ("GAROA", "cloud-drizzle"),
        61 | 63 | 65 | 66 | 67 => ("CHUVA", "cloud-rain"),
        71 | 73 | 75 | 77 => ("NEVE", "snowflake"),
        80..=82 => ("PANCADAS", "cloud-rain"),
        85 | 86 => ("NEVE", "snowflake"),
        95 | 96 | 99 => ("TEMPORAL", "cloud-lightning"),
        _ => ("CONDIÇÃO ATUAL", "cloud"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_open_meteo_weather_codes_to_labels_and_lucide_icons() {
        assert_eq!(weather_description(45), ("NEBLINA", "cloud-fog"));
        assert_eq!(weather_description(61), ("CHUVA", "cloud-rain"));
        assert_eq!(weather_description(0), ("CÉU LIMPO", "sun"));
    }

    #[test]
    fn default_city_identifies_sao_paulo_and_state() {
        assert_eq!(
            crate::config::AppConfig::default().city,
            "São Paulo, São Paulo"
        );
    }

    #[test]
    fn city_query_parser_accepts_brazilian_state_names() {
        assert_eq!(
            parse_city_query("São Paulo, São Paulo, Brasil").unwrap(),
            CityQuery {
                name: "São Paulo".into(),
                region: Some("São Paulo".into()),
                country: Some("Brasil".into()),
            }
        );
    }

    #[test]
    fn city_query_parser_accepts_city_and_country() {
        let query = parse_city_query("Lisboa, Portugal").unwrap();
        assert_eq!(query.name, "Lisboa");
        assert_eq!(query.region.as_deref(), Some("Portugal"));
        assert_eq!(query.country, None);
    }

    #[test]
    fn city_lookup_respects_state_and_prefers_a_city_over_administrative_results() {
        let results = vec![
            GeocodingResult {
                name: "São Paulo".into(),
                feature_code: Some("ADM2".into()),
                population: Some(12_000_000),
                country_code: Some("BR".into()),
                country: Some("Brasil".into()),
                admin1: Some("São Paulo".into()),
                latitude: -23.5,
                longitude: -46.6,
                timezone: None,
            },
            GeocodingResult {
                name: "São Paulo".into(),
                feature_code: Some("PPLA".into()),
                population: Some(11_000_000),
                country_code: Some("BR".into()),
                country: Some("Brasil".into()),
                admin1: Some("São Paulo".into()),
                latitude: -23.5,
                longitude: -46.6,
                timezone: None,
            },
            GeocodingResult {
                name: "São Paulo".into(),
                feature_code: Some("PPLA".into()),
                population: Some(1_000),
                country_code: Some("BR".into()),
                country: Some("Brasil".into()),
                admin1: Some("Outro estado".into()),
                latitude: -10.0,
                longitude: -40.0,
                timezone: None,
            },
        ];
        let query = parse_city_query("São Paulo, São Paulo").unwrap();
        assert_eq!(
            select_place(&results, &query)
                .unwrap()
                .feature_code
                .as_deref(),
            Some("PPLA")
        );
        let wrong_state = parse_city_query("São Paulo, Outro estado").unwrap();
        assert_eq!(
            select_place(&results, &wrong_state).unwrap().population,
            Some(1_000)
        );
    }
}
