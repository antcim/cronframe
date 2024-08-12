use chrono::Local;

// get it from https://openweathermap.org
const API_KEY: &str = "";

fn main() {
    let city = "Venice";

    println!("Weather alert function for {}", city);

    let url_coord = format!(
        "http://api.openweathermap.org/geo/1.0/direct?q={}&limit={}&appid={}",
        city, 5, API_KEY
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
        200..=232 => println!("!! Thread Carefully: Thunderstorm in {} !!", city),
        500..=531 => println!("! Thread Carefully: Rain in {} !", city),
        600..=622 => println!("! Thread Carefully: Snow in {} !", city),
        781 => println!("!!! Seek Shelter: Tornado in {} !!!", city),
        _ => println!("Nothing to worry about in {}", city),
    }
}
