use crate::spider::config::SpiderConfig;
use crate::spider::error::SpiderError;
use log::debug;
use rand::Rng;
use reqwest::{Client, Response};
use std::time::Duration;
use url::Url;

/// Handles HTTP client creation and network requests with anti-bot detection measures
pub struct NetworkClient {
    /// The HTTP client
    client: Client,
    
    /// Spider configuration
    config: SpiderConfig,
}

impl NetworkClient {
    /// Create a new network client with the given configuration
    pub fn new(config: SpiderConfig) -> Result<Self, SpiderError> {
        // Create a client with redirect policy, timeouts
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(30))
            // Apply common browser-like headers to avoid detection
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                        .parse()
                        .unwrap(),
                );
                headers.insert(
                    reqwest::header::ACCEPT_LANGUAGE,
                    "en-US,en;q=0.5".parse().unwrap(),
                );
                headers.insert(
                    reqwest::header::ACCEPT_ENCODING,
                    "gzip, deflate, br".parse().unwrap(),
                );
                headers.insert(
                    "DNT", "1".parse().unwrap()
                );
                headers
            })
            .build()
            .map_err(|e| SpiderError::HttpClient(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }
    
    /// Get a random user agent from the config
    fn get_random_user_agent(&self) -> String {
        let user_agents = &self.config.user_agents;
        let idx = rand::thread_rng().gen_range(0..user_agents.len());
        user_agents[idx].clone()
    }
    
    /// Add random delay between requests
    pub async fn apply_delay(&self) {
        // Calculate a random delay between min and max
        let delay_ms = rand::thread_rng().gen_range(
            self.config.min_request_delay_ms..=self.config.max_request_delay_ms
        );
        
        // Apply the delay
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
    
    /// Fetch a URL with anti-bot measures
    pub async fn fetch(&self, url: &str) -> Result<Response, SpiderError> {
        let parsed_url = Url::parse(url)
            .map_err(|e| SpiderError::UrlParse(e))?;
        
        // Apply delay before making the request
        self.apply_delay().await;
        
        // Get a random user agent
        let user_agent = self.get_random_user_agent();
        
        // Start with a basic request
        let mut request = self.client.get(url);
        
        // Set the user agent for this specific request
        request = request.header(reqwest::header::USER_AGENT, user_agent);
        
        // Add a referer header
        // Use a plausible referer (Google, Bing, or current domain)
        let domain = parsed_url.host_str().unwrap_or("example.com");
        let referer = format!("{}://{}/", parsed_url.scheme(), domain);
        request = request.header(reqwest::header::REFERER, referer);
            
        // Include an empty cookies header
        request = request.header(reqwest::header::COOKIE, "");
        
        // Send the request
        let response = request.send().await
            .map_err(|e| SpiderError::NetworkError(format!("Failed to fetch {}: {}", url, e)))?;
            
        // Check response status
        if !response.status().is_success() {
            return Err(SpiderError::HttpStatus(
                format!("HTTP error status: {} for {}", response.status(), url)
            ));
        }
            
        Ok(response)
    }
    
    /// Extract HTML content from a response, handling various content types
    pub async fn extract_html(&self, response: Response) -> Result<String, SpiderError> {
        // Check content type
        let content_type = response.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();
            
        // Only process HTML content
        if !content_type.contains("text/html") && !content_type.contains("application/xhtml+xml") {
            return Err(SpiderError::ContentType(
                format!("Not HTML content: {}", content_type)
            ));
        }
        
        // Get the HTML content
        let html = response.text().await
            .map_err(|e| SpiderError::HtmlParse(format!("Failed to get HTML: {}", e)))?;
            
        
        Ok(html)
    }
    
    /// Write debug HTML to file when no links are found
    pub fn save_debug_html(&self, url: &str, html: &str) -> Result<(), SpiderError> {
        // Extract domain from URL
        let domain = match url::Url::parse(url) {
            Ok(parsed_url) => {
                parsed_url.host_str()
                    .unwrap_or("unknown_domain")
                    .to_string()
                    .replace("www.", "")
            },
            Err(_) => "unknown_domain".to_string(),
        };
        
        // Create domain-specific directory
        let debug_dir = format!("debug/{}", domain);
        std::fs::create_dir_all(&debug_dir)
            .map_err(|e| SpiderError::Io(e))?;
        
        // Create a safe filename from the URL path
        let url_path = url.replace("://", "_")
                          .replace("/", "_")
                          .replace(".", "_");
        let filename = format!("{}/debug_{}.html", debug_dir, url_path);
        
        // Write the HTML to a file
        std::fs::write(&filename, html)
            .map_err(|e| SpiderError::Io(e))?;
            
        debug!("Saved debug HTML to {}", filename);
        
        Ok(())
    }
    
    /// Check if page might require JavaScript
    pub fn requires_javascript(&self, html: &str) -> bool {
        html.contains("document.write") || 
        html.contains("window.location") || 
        html.matches("function(").count() > 10 ||
        (html.contains("</noscript>") && html.matches("<a").count() < 3)
    }
    
    /// Check if page might have anti-bot protection
    pub fn has_anti_bot_protection(&self, html: &str) -> bool {
        html.contains("captcha") || 
        html.contains("CAPTCHA") || 
        html.contains("robot") || 
        html.contains("Robot") ||
        html.contains("automated") || 
        html.contains("Automated")
    }
    
    /// Get HTML content statistics for debugging
    pub fn get_html_stats(&self, html: &str) -> String {
        format!("{} chars, {} divs, {} links, {} scripts",
                html.len(),
                html.matches("<div").count(),
                html.matches("<a ").count(),
                html.matches("<script").count())
    }
}