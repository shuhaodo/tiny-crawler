use crate::spider::config::SpiderConfig;
use crate::spider::error::SpiderError;
use crate::spider::network::NetworkClient;
use crate::spider::utils::{
    detect_massive_links_pattern, domain_to_filename, extract_base_domain, is_priority_url,
    is_same_domain, normalize_url, resolve_url, should_skip_subdomain, should_skip_url,
};

use anyhow::Result;
use futures::stream::{self, StreamExt};
use futures::FutureExt;
use log::{debug, info, warn};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::sync::{Arc, Mutex};

/// A URL with additional metadata
#[derive(Debug, Clone, Eq, PartialEq)]
struct UrlEntry {
    /// The URL
    url: String,

    /// The depth of this URL in the crawl
    depth: usize,

    /// Priority score for sorting
    priority: usize,
}

/// Spider crawl result
#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlResult {
    /// The original URL
    pub base_url: String,

    /// The extracted base domain
    pub base_domain: String,

    /// List of found URLs
    pub urls: Vec<String>,

    /// Map of skipped URLs grouped by reason
    pub skipped_urls: HashMap<String, Vec<String>>,

    /// List of patterns detected for massive link sets
    pub massive_link_patterns: Vec<String>,

    /// Map of redirected URLs
    pub redirects: HashMap<String, String>,

    /// List of unreachable URLs
    pub unreachable_urls: Vec<String>,

    /// URLs remaining in the queue
    pub remaining_queue: Vec<String>,

    /// Stats about the crawl
    pub stats: HashMap<String, usize>,
}

/// Spider for crawling websites
pub struct Spider {
    /// Spider configuration
    config: SpiderConfig,

    /// Network client for making requests
    network: NetworkClient,
}

impl Spider {
    /// Create a new spider with the given configuration
    pub fn new(config: SpiderConfig) -> Self {
        // Create a network client
        let network = NetworkClient::new(config.clone()).expect("Failed to create network client");
        
        Self { 
            config,
            network,
        }
    }
    

    /// Crawl a website starting from the given URL
    pub async fn crawl(&self, start_url: &str) -> Result<CrawlResult> {
        // Extract base domain from start URL
        let base_domain = extract_base_domain(start_url)?;
        let normalized_start_url = normalize_url(start_url)?;

        // Print configuration
        info!("Spider configuration:");
        info!("  max_depth: {}", self.config.max_depth);
        info!("  max_loops: {}", self.config.max_loops);
        info!("  max_concurrent: {}", self.config.max_concurrent);
        info!("  pattern_threshold: {}", self.config.pattern_threshold);
        info!("  skip_patterns: {:?}", self.config.skip_patterns);
        info!(
            "  skip_subdomain_patterns: {:?}",
            self.config.skip_subdomain_patterns
        );
        info!("  priority_paths: {:?}", self.config.priority_paths);

        info!(
            "Starting crawl of {} (base domain: {})",
            normalized_start_url, base_domain
        );

        // Initialize shared state
        let visited_urls = Arc::new(Mutex::new(HashSet::new()));
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let found_urls = Arc::new(Mutex::new(Vec::new()));
        let skipped_urls = Arc::new(Mutex::new(HashMap::<String, Vec<String>>::new()));
        let massive_link_patterns = Arc::new(Mutex::new(HashSet::new()));
        let redirects = Arc::new(Mutex::new(HashMap::new()));
        let unreachable_urls = Arc::new(Mutex::new(Vec::new()));

        // Add start URL to queue
        queue.lock().unwrap().push_back(UrlEntry {
            url: normalized_start_url.clone(),
            depth: 0,
            priority: 100, // Start URL gets top priority
        });

        // Process URLs from the queue until empty or max_loops reached
        let mut processed_urls_count = 0;
        let mut loop_count = 0;

        while loop_count < self.config.max_loops {
            loop_count += 1;
            // Get next batch of URLs to process
            let batch = {
                let mut queue_lock = queue.lock().unwrap();

                if queue_lock.is_empty() {
                    info!("Queue is empty, crawl complete");
                    break;
                }

                // Sort queue by priority (higher is better)
                let mut entries: Vec<_> = queue_lock.drain(..).collect();
                entries.sort_by(|a, b| b.priority.cmp(&a.priority));

                // Take up to max_concurrent URLs
                let batch_size = std::cmp::min(self.config.max_concurrent, entries.len());
                let batch: Vec<_> = entries.drain(..batch_size).collect();

                // Put remaining entries back in queue
                queue_lock.extend(entries);

                batch
            };

            // Detect massive link patterns
            if let Some(pattern) = detect_massive_links_pattern(
                &batch.iter().map(|e| e.url.clone()).collect::<Vec<_>>(),
                self.config.pattern_threshold,
            ) {
                info!("Detected massive link pattern: {}", pattern);
                massive_link_patterns.lock().unwrap().insert(pattern);
            }

            // Get the batch length here before we move it
            let batch_len = batch.len();

            // Process batch in parallel
            let futures = batch.into_iter().map(|entry| {
                // Skip URLs that exceed max depth
                if entry.depth >= self.config.max_depth {
                    {
                        let mut skipped = skipped_urls.lock().unwrap();
                        let reason = "max_depth_exceeded".to_string();
                        skipped
                            .entry(reason)
                            .or_default()
                            .push(entry.url.clone());
                    }
                    return futures::future::ready(()).boxed();
                }

                // Skip URLs that match massive link patterns
                let patterns = massive_link_patterns.lock().unwrap().clone();
                let matches_pattern = patterns.iter().any(|pattern| {
                    let pattern_parts: Vec<&str> = pattern.split('*').collect();
                    if pattern_parts.len() == 2 {
                        entry.url.starts_with(pattern_parts[0])
                            && entry.url.ends_with(pattern_parts[1])
                    } else {
                        false
                    }
                });

                if matches_pattern {
                    {
                        let mut skipped = skipped_urls.lock().unwrap();
                        let reason = "massive_link_pattern".to_string();
                        skipped
                            .entry(reason)
                            .or_default()
                            .push(entry.url.clone());
                    }
                    return futures::future::ready(()).boxed();
                }

                // Skip URLs that match skip patterns
                if should_skip_url(&entry.url, &self.config.skip_patterns) {
                    {
                        let mut skipped = skipped_urls.lock().unwrap();
                        let reason = "skip_pattern".to_string();
                        skipped
                            .entry(reason)
                            .or_default()
                            .push(entry.url.clone());
                    }
                    return futures::future::ready(()).boxed();
                }

                // Skip URLs that match subdomain patterns
                let skip_subdomain_result =
                    should_skip_subdomain(&entry.url, &self.config.skip_subdomain_patterns);
                match skip_subdomain_result {
                    Ok(should_skip) => {
                        if should_skip {
                            {
                                let mut skipped = skipped_urls.lock().unwrap();
                                let reason = "subdomain_pattern".to_string();
                                skipped
                                    .entry(reason)
                                    .or_default()
                                    .push(entry.url.clone());
                            }
                            return futures::future::ready(()).boxed();
                        }
                    }
                    Err(e) => {
                        debug!("Error checking subdomain pattern for {}: {}", entry.url, e);
                        // Continue processing, don't skip on error
                    }
                }

                // Process URL
                let url = entry.url.clone();
                let depth = entry.depth;

                // Clone all the shared state we need
                let visited_urls_clone = visited_urls.clone();
                let queue_clone = queue.clone();
                let found_urls_clone = found_urls.clone();
                let base_domain_clone = base_domain.clone();
                let redirects_clone = redirects.clone();
                let unreachable_urls_clone = unreachable_urls.clone();
                let priority_paths_clone = self.config.priority_paths.clone();

                async move {
                    self.process_url(
                        &url,
                        depth,
                        visited_urls_clone,
                        queue_clone,
                        found_urls_clone,
                        &base_domain_clone,
                        redirects_clone,
                        unreachable_urls_clone,
                        &priority_paths_clone,
                    )
                    .await;
                }
                .boxed()
            });

            // Wait for all URLs in batch to be processed
            stream::iter(futures)
                .buffer_unordered(self.config.max_concurrent)
                .collect::<Vec<_>>()
                .await;

            processed_urls_count += batch_len;

            // Collect detailed statistics
            let queue_len = queue.lock().unwrap().len();
            let visited_count = visited_urls.lock().unwrap().len();
            let found_count = found_urls.lock().unwrap().len();
            let skipped_count: usize = skipped_urls.lock().unwrap().values().map(|v| v.len()).sum();
            let patterns_count = massive_link_patterns.lock().unwrap().len();
            let redirects_count = redirects.lock().unwrap().len();
            let unreachable_count = unreachable_urls.lock().unwrap().len();

            info!("--- Loop stats for {} (loop #{}) ---", base_domain, loop_count);
            info!(
                "  Processed: {} URLs total ({} in this batch)",
                processed_urls_count, batch_len
            );
            info!("  Queue: {} remaining URLs", queue_len);
            info!("  Visited: {} URLs", visited_count);
            info!("  Found: {} unique URLs", found_count);
            info!("  Skipped: {} URLs", skipped_count);
            info!("  Patterns: {} detected", patterns_count);
            info!("  Redirects: {} captured", redirects_count);
            info!("  Unreachable: {} URLs", unreachable_count);
        }

        // Collect results
        let mut urls = found_urls.lock().unwrap().clone();
        urls.sort();
        urls.dedup();

        let skipped = skipped_urls.lock().unwrap().clone();
        let patterns = massive_link_patterns
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect();
        let redirect_map = redirects.lock().unwrap().clone();
        let unreachable = unreachable_urls.lock().unwrap().clone();

        // Create result
        let result = CrawlResult {
            base_url: normalized_start_url,
            base_domain,
            urls,
            skipped_urls: skipped,
            massive_link_patterns: patterns,
            redirects: redirect_map,
            unreachable_urls: unreachable,
            remaining_queue: Vec::new(), // Will be populated later
            stats: HashMap::new(),       // Will be populated later
        };

        // Create the stats map
        let mut stats = HashMap::new();
        stats.insert("loops".to_string(), loop_count);
        stats.insert("processed_urls".to_string(), processed_urls_count);
        stats.insert(
            "visited_urls".to_string(),
            visited_urls.lock().unwrap().len(),
        );
        stats.insert("found_urls".to_string(), result.urls.len());
        stats.insert(
            "skipped_urls".to_string(),
            result.skipped_urls.values().map(|v| v.len()).sum(),
        );
        stats.insert("redirects".to_string(), result.redirects.len());
        stats.insert(
            "unreachable_urls".to_string(),
            result.unreachable_urls.len(),
        );
        stats.insert(
            "patterns_detected".to_string(),
            result.massive_link_patterns.len(),
        );

        // Get remaining URLs in the queue
        let remaining_urls: Vec<String> = queue
            .lock()
            .unwrap()
            .iter()
            .map(|entry| entry.url.clone())
            .collect();

        // Add the remaining URLs to the result
        let mut result_with_queue = result;
        result_with_queue.remaining_queue = remaining_urls.clone();
        result_with_queue.stats = stats;

        // Save the updated result
        self.save_result(&result_with_queue)?;

        // Print final statistics (but not the queue contents)
        info!("=== Final crawl statistics ===");
        info!("  Base URL: {}", result_with_queue.base_url);
        info!("  Base domain: {}", result_with_queue.base_domain);
        info!("  Found: {} unique URLs", result_with_queue.urls.len());
        info!("  Skipped: {} URLs", result_with_queue.skipped_urls.len());
        info!(
            "  Redirects: {} captured",
            result_with_queue.redirects.len()
        );
        info!(
            "  Unreachable: {} URLs",
            result_with_queue.unreachable_urls.len()
        );
        info!(
            "  Patterns detected: {}",
            result_with_queue.massive_link_patterns.len()
        );
        info!("  Number of loops: {}", loop_count);
        info!("  Total URLs processed: {}", processed_urls_count);
        info!("  URLs remaining in queue: {}", remaining_urls.len());

        Ok(result_with_queue)
    }

    /// Process a single URL
    async fn process_url(
        &self,
        url: &str,
        depth: usize,
        visited_urls: Arc<Mutex<HashSet<String>>>,
        queue: Arc<Mutex<VecDeque<UrlEntry>>>,
        found_urls: Arc<Mutex<Vec<String>>>,
        base_domain: &str,
        redirects: Arc<Mutex<HashMap<String, String>>>,
        unreachable_urls: Arc<Mutex<Vec<String>>>,
        priority_paths: &[String],
    ) {
        // Mark URL as visited
        {
            let mut visited = visited_urls.lock().unwrap();
            if visited.contains(url) {
                debug!("Already visited {}", url);
                return;
            }
            visited.insert(url.to_string());
        }

        // Add URL to found_urls
        {
            let mut found = found_urls.lock().unwrap();
            found.push(url.to_string());
        }

        // Fetch the URL using our network client
        let response = match self.network.fetch(url).await {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to fetch {}: {}", url, e);
                
                // Add to unreachable_urls
                {
                    let mut unreachable = unreachable_urls.lock().unwrap();
                    unreachable.push(url.to_string());
                }
                
                return;
            }
        };

        // Check for redirects
        if response.url().as_str() != url {
            // Add to redirects map
            {
                let mut redirect_map = redirects.lock().unwrap();
                redirect_map.insert(url.to_string(), response.url().to_string());
            }
        }

        // Normalized current URL (after redirects)
        let current_url = response.url().as_str().to_string();

        // Extract HTML content
        let html = match self.network.extract_html(response).await {
            Ok(html) => html,
            Err(e) => {
                // If we got a content type error, it's likely not HTML
                if let SpiderError::ContentType(_) = e {
                    debug!("Skipping non-HTML content: {}", url);
                } else {
                    warn!("Failed to get HTML from {}: {}", url, e);
                }
                return;
            }
        };

        let document = Html::parse_document(&html);
        let selector = Selector::parse("a[href]").unwrap();
        
        // Count the number of links found
        let link_count = document.select(&selector).count();
        debug!("Found {} links on page {}", link_count, url);
        
        // If we didn't find enough links, log the issue and save debug info
        if link_count == 0 || (link_count < 3 && html.len() > 1000) {
            debug!("Few or no links found ({}) on page", link_count);
            
            // Save HTML for debugging
            let _ = self.network.save_debug_html(url, &html);
            
            // Check for anti-bot protection
            if self.network.has_anti_bot_protection(&html) {
                warn!("Possible anti-bot protection detected on page: {}", url);
            }
            
            // Check for JavaScript-only content
            if self.network.requires_javascript(&html) {
                warn!("Page may require JavaScript to display content: {}", url);
            }
            
            // Basic stats for debugging
            debug!("Page stats: {}", self.network.get_html_stats(&html));
        }

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                // Skip empty links, anchors, javascript, and mailto
                if href.is_empty()
                    || href.starts_with('#')
                    || href.starts_with("javascript:")
                    || href.starts_with("mailto:")
                {
                    debug!("Skipping link: {}", href);
                    continue;
                }

                // Resolve relative URLs
                let absolute_url = match resolve_url(&current_url, href) {
                    Ok(url) => url,
                    Err(e) => {
                        debug!("Failed to resolve URL {}: {}", href, e);
                        continue;
                    }
                };

                // Make sure URL is in the same domain
                match is_same_domain(&absolute_url, base_domain) {
                    Ok(same_domain) => {
                        if !same_domain {
                            debug!("Skipping external URL: {}", absolute_url);
                            continue;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to check domain for {}: {}", absolute_url, e);
                        continue;
                    }
                }

                // Check if URL is already visited or in queue
                let should_add = {
                    let visited = visited_urls.lock().unwrap();
                    if visited.contains(&absolute_url) {
                        debug!("Already visited {}", absolute_url);
                        false
                    } else {
                        // Also check if the URL is already in the queue
                        let q = queue.lock().unwrap();
                        let already_in_queue = q.iter().any(|entry| entry.url == absolute_url);
                        if already_in_queue {
                            debug!("Already in queue {}", absolute_url);
                            false
                        } else {
                            true
                        }
                    }
                };

                if should_add {
                    // Calculate priority: priority paths get higher value
                    let is_priority = is_priority_url(&absolute_url, priority_paths);
                    let priority = if is_priority { 50 } else { 10 };

                    // Add URL to queue
                    {
                        let mut q = queue.lock().unwrap();
                        q.push_back(UrlEntry {
                            url: absolute_url,
                            depth: depth + 1,
                            priority,
                        });
                    }
                }
            }
        }
    }

    /// Save crawl result to file
    fn save_result(&self, result: &CrawlResult) -> Result<(), SpiderError> {
        let filename = domain_to_filename(&result.base_domain);

        let file = File::create(&filename).map_err(SpiderError::Io)?;

        serde_json::to_writer_pretty(file, result).map_err(SpiderError::Json)?;

        info!("Saved results to {}", filename);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Note: The following tests are commented out because they require an external mock HTTP server.
    // In a real environment, these tests would use a library like wiremock or a real server for integration testing.

    /*
    #[tokio::test]
    async fn test_spider_basic() {
        // A test that would verify basic crawling functionality
        // Would need to mock HTTP responses or use a real test server
    }

    #[tokio::test]
    async fn test_spider_redirect() {
        // A test that would verify redirect handling
    }

    #[tokio::test]
    async fn test_spider_non_html() {
        // A test that would verify handling of non-HTML content
    }
    */
}
