use crate::common::{HttpClient, TtlCache};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, info};

const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";

#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Debug, Deserialize, Clone)]
struct GeocodingResult {
    name: String,
    latitude: f64,
    longitude: f64,
    country: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ForecastResponse {
    latitude: f64,
    longitude: f64,
    daily: DailyForecast,
    daily_units: DailyUnits,
}

#[derive(Debug, Deserialize)]
struct DailyForecast {
    time: Vec<String>,
    #[serde(rename = "temperature_2m_max")]
    temperature_max: Vec<Option<f64>>,
    #[serde(rename = "temperature_2m_min")]
    temperature_min: Vec<Option<f64>>,
    #[serde(rename = "apparent_temperature_max")]
    apparent_temperature_max: Vec<Option<f64>>,
    #[serde(rename = "apparent_temperature_min")]
    apparent_temperature_min: Vec<Option<f64>>,
    #[serde(rename = "precipitation_sum")]
    precipitation: Vec<Option<f64>>,
    #[serde(rename = "precipitation_probability_max")]
    precipitation_probability: Vec<Option<i32>>,
    #[serde(rename = "weather_code")]
    weather_code: Vec<Option<i32>>,
    #[serde(rename = "wind_speed_10m_max")]
    wind_speed: Vec<Option<f64>>,
    #[serde(rename = "relative_humidity_2m_mean")]
    humidity: Vec<Option<i32>>,
    sunrise: Vec<Option<String>>,
    sunset: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DailyUnits {
    #[serde(rename = "temperature_2m_max")]
    _temperature_max: String,
    #[serde(rename = "precipitation_sum")]
    _precipitation: String,
    #[serde(rename = "wind_speed_10m_max")]
    _wind_speed: String,
    #[serde(rename = "relative_humidity_2m_mean")]
    _humidity: String,
}

#[derive(Clone)]
pub struct WeatherService {
    client: HttpClient,
    geo_cache: TtlCache<GeocodingResult>,
    forecast_cache: TtlCache<String>,
}

impl WeatherService {
    pub fn new() -> Self {
        Self {
            client: HttpClient::new().expect("Failed to create HTTP client"),
            geo_cache: TtlCache::new(Duration::from_secs(3600)),
            forecast_cache: TtlCache::new(Duration::from_secs(600)),
        }
    }

    pub async fn get_weather(&self, city: &str, days: u8) -> Result<String> {
        let days = days.clamp(1, 7);
        let cache_key = format!("{}-{}", city.to_lowercase(), days);

        if let Some(cached) = self.forecast_cache.get(&cache_key).await {
            debug!("Weather cache hit for {}", city);
            return Ok(cached);
        }

        let geo = self.geocode(city).await?;
        info!("Resolved '{}' to {:?}", city, geo);

        let forecast = self.fetch_forecast(geo.latitude, geo.longitude, days).await?;
        let markdown = self.format_weather(&geo, &forecast, days);

        self.forecast_cache.set(cache_key, markdown.clone()).await;
        Ok(markdown)
    }

    async fn geocode(&self, city: &str) -> Result<GeocodingResult> {
        let cache_key = city.to_lowercase();
        if let Some(cached) = self.geo_cache.get(&cache_key).await {
            debug!("Geocoding cache hit for {}", city);
            return Ok(cached);
        }

        let url = format!("{}?name={}&count=1&language=zh&format=json", GEOCODING_URL, city);
        let resp: GeocodingResponse = self
            .client
            .inner()
            .get(&url)
            .send()
            .await
            .context("Geocoding request failed")?
            .json()
            .await
            .context("Failed to parse geocoding response")?;

        let result = resp
            .results
            .and_then(|r| r.into_iter().next())
            .context(format!("City '{}' not found", city))?;

        self.geo_cache.set(cache_key, result.clone()).await;
        Ok(result)
    }

    async fn fetch_forecast(&self, lat: f64, lon: f64, days: u8) -> Result<ForecastResponse> {
        let url = format!(
            "{}?latitude={}&longitude={}&daily=temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,precipitation_sum,precipitation_probability_max,weather_code,wind_speed_10m_max,relative_humidity_2m_mean,sunrise,sunset&timezone=auto&forecast_days={}",
            FORECAST_URL, lat, lon, days
        );

        let resp: ForecastResponse = self
            .client
            .inner()
            .get(&url)
            .send()
            .await
            .context("Forecast request failed")?
            .json()
            .await
            .context("Failed to parse forecast response")?;

        Ok(resp)
    }

    fn format_weather(&self, geo: &GeocodingResult, forecast: &ForecastResponse, days: u8) -> String {
        let country = geo.country.as_deref().unwrap_or("Unknown");
        let mut md = format!(
            "## {} ({}) 天气预报\n\n",
            geo.name, country
        );

        let daily = &forecast.daily;
        for i in 0..(days as usize).min(daily.time.len()) {
            let date = &daily.time[i];
            let temp_max = daily.temperature_max[i].map(|t| format!("{:.1}", t)).unwrap_or_else(|| "-".to_string());
            let temp_min = daily.temperature_min[i].map(|t| format!("{:.1}", t)).unwrap_or_else(|| "-".to_string());
            let feel_max = daily.apparent_temperature_max[i].map(|t| format!("{:.1}", t)).unwrap_or_else(|| "-".to_string());
            let feel_min = daily.apparent_temperature_min[i].map(|t| format!("{:.1}", t)).unwrap_or_else(|| "-".to_string());
            let precip = daily.precipitation[i].map(|p| format!("{:.1}", p)).unwrap_or_else(|| "-".to_string());
            let precip_prob = daily.precipitation_probability[i].map(|p| format!("{}", p)).unwrap_or_else(|| "-".to_string());
            let wind = daily.wind_speed[i].map(|w| format!("{:.1}", w)).unwrap_or_else(|| "-".to_string());
            let humidity = daily.humidity[i].map(|h| format!("{}", h)).unwrap_or_else(|| "-".to_string());
            let weather = daily.weather_code[i].map(weather_code_desc).unwrap_or_else(|| "未知".to_string());
            let sunrise = daily.sunrise[i].as_deref().unwrap_or("-");
            let sunset = daily.sunset[i].as_deref().unwrap_or("-");

            md.push_str(&format!(
                "### {}\n- **天气**: {}\n- **温度**: {}°C / {}°C (体感 {}°C / {}°C)\n- **降水**: {}mm (概率 {}%)\n- **风速**: {}km/h\n- **湿度**: {}%\n- **日出/日落**: {} / {}\n\n",
                date, weather, temp_max, temp_min, feel_max, feel_min, precip, precip_prob, wind, humidity, sunrise, sunset
            ));
        }

        md
    }
}

fn weather_code_desc(code: i32) -> String {
    match code {
        0 => "晴朗".to_string(),
        1 | 2 | 3 => "多云".to_string(),
        45 | 48 => "雾".to_string(),
        51 | 53 | 55 => "毛毛雨".to_string(),
        56 | 57 => "冻雨".to_string(),
        61 | 63 | 65 => "雨".to_string(),
        66 | 67 => "冻雨".to_string(),
        71 | 73 | 75 => "雪".to_string(),
        77 => "雪粒".to_string(),
        80 | 81 | 82 => "阵雨".to_string(),
        85 | 86 => "阵雪".to_string(),
        95 => "雷雨".to_string(),
        96 | 99 => "雷雨伴冰雹".to_string(),
        _ => format!("天气代码 {}", code),
    }
}
