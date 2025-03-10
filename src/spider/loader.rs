use crate::spider::config::defaults;
use crate::spider::error::SpiderError;
use crate::spider::{Spider, SpiderConfig};
use futures::stream::{self, StreamExt};
use log::info;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Loader for crawling multiple URLs in parallel
pub struct Loader {
    /// Spider configuration
    config: SpiderConfig,

    /// Maximum number of sites to crawl concurrently
    max_concurrent_sites: usize,

    /// The path to the file containing URLs to crawl
    url_file_path: String,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            config: SpiderConfig::default(),
            max_concurrent_sites: defaults::MAX_CONCURRENT_SITES,
            url_file_path: "input/urls.txt".to_string(),
        }
    }
}

impl Loader {
    /// Create a new loader with custom configuration
    pub fn new(config: SpiderConfig, max_concurrent_sites: usize, url_file_path: &str) -> Self {
        Self {
            config,
            max_concurrent_sites,
            url_file_path: url_file_path.to_string(),
        }
    }

    /// Load URLs from a file
    fn load_urls(&self) -> Result<Vec<String>, SpiderError> {
        let path = Path::new(&self.url_file_path);

        let file = File::open(path).map_err(|e| {
            SpiderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("URL file not found: {} - {}", self.url_file_path, e),
            ))
        })?;

        let reader = BufReader::new(file);
        let mut urls = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(SpiderError::Io)?;
            let trimmed = line.trim();

            // Skip empty lines and comments
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                urls.push(trimmed.to_string());
            }
        }

        Ok(urls)
    }

    /// Crawl all URLs in parallel
    pub async fn crawl_all(&self) -> Result<Vec<Result<String, String>>, SpiderError> {
        // Load URLs from file
        let urls = self.load_urls()?;
        let total_urls = urls.len();

        info!("Loaded {} URLs from {}", total_urls, self.url_file_path);
        info!(
            "Starting crawl with {} concurrent sites",
            self.max_concurrent_sites
        );

        // Keep track of progress
        let processed = Arc::new(Mutex::new(0));

        // Create futures for each URL
        let futures = urls.into_iter().map(|url| {
            let spider = Spider::new(self.config.clone());
            let processed_clone = processed.clone();

            async move {
                let result = match spider.crawl(&url).await {
                    Ok(result) => Ok(format!(
                        "Successfully crawled {}: {} URLs found",
                        url,
                        result.urls.len()
                    )),
                    Err(e) => Err(format!("Failed to crawl {}: {}", url, e)),
                };

                // Update progress
                let mut processed_count = processed_clone.lock().unwrap();
                *processed_count += 1;
                info!("Progress: {}/{} URLs crawled", *processed_count, total_urls);

                result
            }
        });

        // Process futures concurrently with a limit
        let results = stream::iter(futures)
            .buffer_unordered(self.max_concurrent_sites)
            .collect::<Vec<_>>()
            .await;

        info!("Completed crawling all URLs");

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_urls_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let loader = Loader::new(SpiderConfig::default(), 30, path);
        let urls = loader.load_urls().unwrap();

        assert_eq!(urls.len(), 0);
    }

    #[test]
    fn test_load_urls_with_comments_and_empty_lines() {
        let temp_file = NamedTempFile::new().unwrap();
        {
            let mut file = temp_file.reopen().unwrap();
            writeln!(file, "https://example.com").unwrap();
            writeln!(file, "# Comment line").unwrap();
            writeln!(file, "").unwrap();
            writeln!(file, "https://test.com").unwrap();
        }

        let path = temp_file.path().to_str().unwrap();
        let loader = Loader::new(SpiderConfig::default(), 30, path);
        let urls = loader.load_urls().unwrap();

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com");
        assert_eq!(urls[1], "https://test.com");
    }

    #[test]
    fn test_load_urls_file_not_found() {
        let loader = Loader::new(SpiderConfig::default(), 30, "/path/does/not/exist.txt");
        let result = loader.load_urls();

        assert!(result.is_err());
    }
}
