use anyhow::Result;
use env_logger::Env;
use log::info;
use std::env;
use std::time::Instant;

use tiny_crawler::spider::{Loader, Spider, SpiderConfig};
use tiny_crawler::spider::config::defaults;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage:");
        println!(
            "  Single URL:  {} crawl <url> [max_depth] [max_loops] [max_concurrent] [min_delay_ms] [max_delay_ms]",
            args[0]
        );
        println!("  Multiple URLs: {} batch [url_file] [max_depth] [max_loops] [max_concurrent] [max_concurrent_sites] [min_delay_ms] [max_delay_ms]", args[0]);
        println!(
            "  - min_delay_ms: Minimum delay between requests in milliseconds (default: {})",
            defaults::MIN_REQUEST_DELAY_MS
        );
        println!(
            "  - max_delay_ms: Maximum delay between requests in milliseconds (default: {})",
            defaults::MAX_REQUEST_DELAY_MS
        );
        println!(
            "  - max_concurrent_sites: Maximum number of sites to crawl in parallel (default: {})",
            defaults::MAX_CONCURRENT_SITES
        );
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "crawl" => {
            if args.len() < 3 {
                println!("URL is required for crawl command");
                return Ok(());
            }

            let url = &args[2];

            // Parse optional arguments
            let max_depth = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_DEPTH);
            let max_loops = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_LOOPS);
            let max_concurrent = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_CONCURRENT);

            // Advanced options
            let min_delay = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(defaults::MIN_REQUEST_DELAY_MS);
            let max_delay = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_REQUEST_DELAY_MS);

            // Create config using builder
            let config = SpiderConfig::builder()
                .max_depth(max_depth)
                .max_loops(max_loops)
                .max_concurrent(max_concurrent)
                .min_request_delay_ms(min_delay)
                .max_request_delay_ms(max_delay)
                .build();

            // Create spider
            let spider = Spider::new(config);

            // Start crawling
            info!("Starting crawl of {}", url);
            info!(
                "Using advanced settings - min delay: {}ms, max delay: {}ms",
                min_delay, max_delay
            );
            let start = Instant::now();

            let result = spider.crawl(url).await?;

            let duration = start.elapsed();
            info!("Crawl completed in {:?}", duration);
            info!("Found {} unique URLs", result.urls.len());
        }
        "batch" => {
            // Parse optional arguments
            let url_file = args.get(2).map(|s| s.as_str()).unwrap_or("input/urls.txt");
            let max_depth = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_DEPTH);
            let max_loops = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_LOOPS);
            let max_concurrent = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_CONCURRENT);
            let max_concurrent_sites = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_CONCURRENT_SITES);

            // Advanced options
            let min_delay = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(defaults::MIN_REQUEST_DELAY_MS);
            let max_delay = args.get(8).and_then(|s| s.parse().ok()).unwrap_or(defaults::MAX_REQUEST_DELAY_MS);

            // Create config using builder
            let config = SpiderConfig::builder()
                .max_depth(max_depth)
                .max_loops(max_loops)
                .max_concurrent(max_concurrent)
                .min_request_delay_ms(min_delay)
                .max_request_delay_ms(max_delay)
                .build();

            // Create loader
            let loader = Loader::new(config, max_concurrent_sites, url_file);

            // Start crawling
            info!("Starting batch crawl from file: {}", url_file);
            info!(
                "Using advanced settings - min delay: {}ms, max delay: {}ms",
                min_delay, max_delay
            );
            let start = Instant::now();

            let results = loader.crawl_all().await?;

            let duration = start.elapsed();
            info!("Batch crawl completed in {:?}", duration);

            // Print results
            for result in results {
                match result {
                    Ok(msg) => info!("{}", msg),
                    Err(err) => info!("Error: {}", err),
                }
            }
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Use 'crawl' or 'batch' commands");
        }
    }

    Ok(())
}
