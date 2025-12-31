//! Career Agent Backend - Main Entry Point
//!
//! Starts the web API server for the Career Development Assistant.

use career_agent::api::run_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    println!("╔════════════════════════════════════════════════╗");
    println!("║   Career Agent - AI Career Development Tool    ║");
    println!("║   Sense → Plan → Learn                         ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    // Start server
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    run_server(&host, port).await
}
