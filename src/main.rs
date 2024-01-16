use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use image::{self, EncodableLayout};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiResponse {
    copyright: Option<String>,
    date: String,
    explanation: String,
    hdurl: Option<String>,
    media_type: String,
    service_version: String,
    title: String,
    url: String,
}

async fn extract_colors_from_image(url: &str) -> Vec<u8> {
    let client = Client::new();
    let response = client.get(url).send().await.unwrap();
    let image_bytes = response.bytes().await.unwrap();

    // Decode the image bytes into a DynamicImage
    let image = image::load_from_memory(&image_bytes).unwrap();

    dominant_color::get_colors(image.to_rgb8().as_bytes(), false)
}

async fn fetch_picture_of_the_day() -> Result<ApiResponse, reqwest::Error> {
    let client = Client::new();

    let api_key = env::var("NASA_API_KEY").expect("Missing API key");
    let api_url = format!("https://api.nasa.gov/planetary/apod?api_key={}", api_key);

    let response = client.get(&api_url).send().await?;
    let apod_data = response.json::<ApiResponse>().await?;

    Ok(apod_data)
}

#[get("/")]
async fn index() -> impl Responder {
    let fallback_url = "https://i.imgur.com/68jyjZT.jpg";

    match fetch_picture_of_the_day().await {
        Ok(apod_data) => {
            // Extract colors from the image
            let is_image_type = apod_data.media_type == "image";
            let colors = extract_colors_from_image(if is_image_type {
                &apod_data.url
            } else {
                &fallback_url
            })
            .await;

            // Convert RGB values to hexadecimal color codes
            let hex_colors: Vec<String> = colors
                .chunks_exact(3)
                .map(|chunk| format!("{:02X}{:02X}{:02X}", chunk[0], chunk[1], chunk[2]))
                .collect();

            let response_body = json!({
                "code": 200,
                "success": true,
                "message": "Dominant colors extracted!",
                "colors": hex_colors,
            });

            HttpResponse::Ok().json(response_body)
        }
        Err(err) => {
            eprintln!("Error fetching picture of the day: {:?}", err);

            let error_response = json!({
                "code": 500,
                "success": false,
                "message": "Error fetching picture of the day",
                "error": err.to_string(),
            });

            HttpResponse::InternalServerError().json(error_response)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("Invalid port number");

    let host = match env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .as_str()
    {
        "development" => "127.0.0.1",
        "production" => "0.0.0.0",
        _ => {
            eprintln!("Invalid environment type. Defaulting to 127.0.0.1");
            "127.0.0.1"
        }
    };

    HttpServer::new(|| App::new().service(index))
        .bind((host, port))?
        .run()
        .await
}
