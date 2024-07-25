#[macro_use]
extern crate cronframe;

use chrono::{Duration, Local};
use cronframe::{Any, Arc, CronFrame, CronFrameExpr, JobBuilder, Lazy, Mutex, Once, Sender};

#[cron_obj]
#[derive(Clone)]
struct WeatherAlert {
    city: String,
    schedule: CronFrameExpr,
}

// get it from https://openweathermap.org
const API_KEY: &str = "";

#[cron_impl]
impl WeatherAlert {
    #[mt_job(expr = "schedule")]
    fn weather_alert(self) {
        println!("Weather alert function for {}", self.city);

        let url_coord = format!(
            "http://api.openweathermap.org/geo/1.0/direct?q={}&limit={}&appid={}",
            self.city, 5, API_KEY
        );

        let resp: serde_json::Value = reqwest::blocking::get(url_coord).unwrap().json().unwrap();

        let latitude = resp[0]["lat"].clone();
        let longitude = resp[0]["lon"].clone();

        println!("latitude = {latitude}");
        println!("longitude = {longitude}");

        let url_weather = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}",
            latitude, longitude, API_KEY
        );

        let resp: serde_json::Value = reqwest::blocking::get(url_weather).unwrap().json().unwrap();

        let weather_id: i32 = resp["weather"][0]["id"]
            .clone()
            .to_string()
            .parse()
            .unwrap();

        println!("weather_id = {weather_id}");

        println!("{}: ", Local::now());
        match weather_id {
            200..=232 => println!("!! Thread Carefully: Thunderstorm in {} !!", self.city),
            500..=531 => println!("! Thread Carefully: Rain in {} !", self.city),
            600..=622 => println!("! Thread Carefully: Snow in {} !", self.city),
            781 => println!("!!! Seek Shelter: Tornado in {} !!!", self.city),
            _ => println!("Nothing to worry about in {}", self.city),
        }
    }
}

fn main() {
    let cronframe = CronFrame::default();

    let alert_schedule = CronFrameExpr::new("0", "0/10", "5-6,14-16", "*", "*", "Mon-Fri", "*", 0);

    let mut venice = WeatherAlert::new_cron_obj("Venice".into(), alert_schedule);

    venice.cf_gather(cronframe.clone());

    cronframe.scheduler();

    loop {
        std::thread::sleep(Duration::milliseconds(500).to_std().unwrap());
    }
}
