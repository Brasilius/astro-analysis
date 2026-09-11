use crate::astro::{Star, catalog};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

pub const HYG_URL: &str =
    "https://raw.githubusercontent.com/astronexus/HYG-Database/main/hyg/CURRENT/hygdata_v41.csv";
#[derive(Clone, Debug)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}
pub enum Request {
    Refresh(Option<Location>, bool),
}
pub enum Update {
    Location(Location),
    Catalog(Vec<Star>),
    Weather(String),
    Status(&'static str, String),
    Done,
}
pub fn worker() -> (Sender<Request>, Receiver<Update>) {
    let (tx, rx) = mpsc::channel();
    let (out, updates) = mpsc::channel();
    std::thread::spawn(move || {
        let client = match Client::builder()
            .timeout(Duration::from_secs(45))
            .user_agent("noctiluca/0.1")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = out.send(Update::Status("network", e.to_string()));
                let _ = out.send(Update::Done);
                return;
            }
        };
        while let Ok(Request::Refresh(mut location, refresh_catalog)) = rx.recv() {
            if location.is_none() {
                let result = (|| -> Result<Location, String> {
                    let v: Value = client
                        .get("https://ipapi.co/json/")
                        .send()
                        .and_then(|r| r.error_for_status())
                        .and_then(|r| r.json())
                        .map_err(|e| e.to_string())?;
                    let lat = v["latitude"]
                        .as_f64()
                        .ok_or("No latitude returned; press L to enter coordinates")?;
                    let lon = v["longitude"].as_f64().ok_or("No longitude returned")?;
                    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
                        return Err("Invalid IP coordinates".into());
                    }
                    Ok(Location {
                        lat,
                        lon,
                        label: format!(
                            "{} · IP estimate",
                            v["city"].as_str().unwrap_or("Unknown city")
                        ),
                    })
                })();
                match result {
                    Ok(l) => {
                        let _ = out.send(Update::Location(l.clone()));
                        location = Some(l);
                    }
                    Err(e) => {
                        let _ = out.send(Update::Status("location", e));
                    }
                }
            }
            if let Some(l) = location {
                let url = format!(
                    "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=cloud_cover,is_day&timezone=UTC",
                    l.lat, l.lon
                );
                let result = client
                    .get(url)
                    .send()
                    .and_then(|r| r.error_for_status())
                    .and_then(|r| r.json::<Value>());
                match result {
                    Ok(v) => {
                        if let (Some(cloud), Some(day), Some(time)) = (
                            v["current"]["cloud_cover"].as_f64(),
                            v["current"]["is_day"].as_u64(),
                            v["current"]["time"].as_str(),
                        ) {
                            let _ = out.send(Update::Weather(format!(
                                "{} · cloud {cloud:.0}% · {time}Z",
                                if day == 1 { "DAYLIGHT" } else { "NIGHT" }
                            )));
                        } else {
                            let _ =
                                out.send(Update::Status("weather", "Unexpected response".into()));
                        }
                    }
                    Err(e) => {
                        let _ = out.send(Update::Status("weather", e.to_string()));
                    }
                }
            }
            if refresh_catalog {
                let result = client
                    .get(HYG_URL)
                    .send()
                    .and_then(|r| r.error_for_status())
                    .and_then(|r| r.bytes())
                    .map_err(|e| e.to_string())
                    .and_then(|b| catalog(&b));
                match result {
                    Ok(s) => {
                        let _ = out.send(Update::Catalog(s));
                    }
                    Err(e) => {
                        let _ = out.send(Update::Status("catalog", e));
                    }
                }
            }
            let _ = out.send(Update::Done);
        }
    });
    (tx, updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires public network; downloads the 34 MB HYG catalog"]
    fn live_sources() {
        let (tx, rx) = worker();
        tx.send(Request::Refresh(
            Some(Location {
                lat: 37.7749,
                lon: -122.4194,
                label: "Test".into(),
            }),
            true,
        ))
        .unwrap();
        let (mut catalog_ok, mut weather_ok) = (false, false);
        loop {
            match rx.recv_timeout(Duration::from_secs(100)).unwrap() {
                Update::Catalog(s) => {
                    assert_eq!(s.len(), 5070);
                    catalog_ok = true;
                }
                Update::Weather(s) => {
                    assert!(s.contains("cloud"));
                    weather_ok = true;
                }
                Update::Status(source, error) => panic!("{source}: {error}"),
                Update::Done => break,
                _ => {}
            }
        }
        assert!(catalog_ok && weather_ok);
    }
}
