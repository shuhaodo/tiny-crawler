use thiserror::Error;

/// Spider errors
#[derive(Error, Debug)]
pub enum SpiderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Crawl limit reached: {0}")]
    CrawlLimitReached(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("HTTP status error: {0}")]
    HttpStatus(String),
    
    #[error("Content type error: {0}")]
    ContentType(String),
    
    #[error("HTML parse error: {0}")]
    HtmlParse(String),
    
    #[error("HTTP client error: {0}")]
    HttpClient(String),

    #[error("Other error: {0}")]
    Other(String),
}
