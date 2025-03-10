/// Default configuration constants
pub mod defaults {
    /// Default maximum depth for recursive crawling
    pub const MAX_DEPTH: usize = 10;

    /// Default maximum number of URLs to process
    pub const MAX_LOOPS: usize = 50;

    /// Default maximum number of concurrent requests
    pub const MAX_CONCURRENT: usize = 30;
    
    /// Default maximum number of concurrent sites to crawl
    pub const MAX_CONCURRENT_SITES: usize = 5;

    /// Default threshold for detecting massive link patterns
    pub const PATTERN_THRESHOLD: usize = 500;

    /// Default minimum delay between requests in milliseconds
    pub const MIN_REQUEST_DELAY_MS: u64 = 100;

    /// Default maximum delay between requests in milliseconds
    pub const MAX_REQUEST_DELAY_MS: u64 = 2000;


    /// Default path patterns to skip
    pub const SKIP_PATTERNS: &[&str] = &[
        "/blogs/",
        "/blog/",
        "/docs/",
        "/library/",
        "/images/",
        "/feed/",
        "/wp-content/",
        "/wp-includes/",
        "/cdn-cgi/",
        "/assets/",
        "/static/",
        "/media/",
        "/api/",
        "/downloads/",
        "/files/",
        "/archive/",
        "/resources/",
    ];

    /// Default subdomain patterns to skip
    pub const SKIP_SUBDOMAIN_PATTERNS: &[&str] = &[
        "docs.",
        "api.",
        "cdn.",
        "static.",
        "media.",
        "assets.",
        "files.",
        "download.",
        "images.",
        "library.",
        "archive.",
        "resources.",
    ];

    /// Default priority paths
    pub const PRIORITY_PATHS: &[&str] = &["/contact", "/about", "/faq", "/help", "/support"];

    /// Default user agents
    pub const USER_AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.0 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.107 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 14_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (iPad; CPU OS 14_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1",
    ];
}

/// Configuration for the spider
#[derive(Debug, Clone)]
pub struct SpiderConfig {
    /// Maximum depth for recursive crawling
    pub max_depth: usize,

    /// Maximum number of URLs to process
    pub max_loops: usize,

    /// Maximum number of concurrent requests
    pub max_concurrent: usize,

    /// Threshold for detecting massive link patterns
    pub pattern_threshold: usize,

    /// List of path patterns to skip
    pub skip_patterns: Vec<String>,

    /// List of subdomain patterns to skip
    pub skip_subdomain_patterns: Vec<String>,

    /// List of high value paths to prioritize
    pub priority_paths: Vec<String>,

    /// Minimum delay between requests in milliseconds
    pub min_request_delay_ms: u64,

    /// Maximum delay between requests in milliseconds
    pub max_request_delay_ms: u64,

    /// List of user agents to rotate through for requests
    pub user_agents: Vec<String>,

}

impl Default for SpiderConfig {
    fn default() -> Self {
        use defaults::*;

        Self {
            max_depth: MAX_DEPTH,
            max_loops: MAX_LOOPS,
            max_concurrent: MAX_CONCURRENT,
            pattern_threshold: PATTERN_THRESHOLD,
            skip_patterns: SKIP_PATTERNS.iter().map(|s| s.to_string()).collect(),
            skip_subdomain_patterns: SKIP_SUBDOMAIN_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            priority_paths: PRIORITY_PATHS.iter().map(|s| s.to_string()).collect(),
            min_request_delay_ms: MIN_REQUEST_DELAY_MS,
            max_request_delay_ms: MAX_REQUEST_DELAY_MS,
            user_agents: USER_AGENTS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl SpiderConfig {
    /// Create a new SpiderConfig with custom values
    pub fn new(max_depth: usize, max_loops: usize, max_concurrent: usize) -> Self {
        Self {
            max_depth,
            max_loops,
            max_concurrent,
            ..Default::default()
        }
    }

    /// Create a builder for more granular configuration
    pub fn builder() -> SpiderConfigBuilder {
        SpiderConfigBuilder::default()
    }
}

/// Builder for SpiderConfig to allow for more granular configuration
pub struct SpiderConfigBuilder {
    config: SpiderConfig,
}

impl Default for SpiderConfigBuilder {
    fn default() -> Self {
        Self {
            config: SpiderConfig::default(),
        }
    }
}

impl SpiderConfigBuilder {
    /// Set the maximum crawl depth
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.config.max_depth = max_depth;
        self
    }

    /// Set the maximum number of loops
    pub fn max_loops(mut self, max_loops: usize) -> Self {
        self.config.max_loops = max_loops;
        self
    }

    /// Set the maximum number of concurrent requests
    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.config.max_concurrent = max_concurrent;
        self
    }

    /// Set the pattern threshold for detecting massive link patterns
    pub fn pattern_threshold(mut self, threshold: usize) -> Self {
        self.config.pattern_threshold = threshold;
        self
    }

    /// Set the minimum request delay in milliseconds
    pub fn min_request_delay_ms(mut self, delay: u64) -> Self {
        self.config.min_request_delay_ms = delay;
        self
    }

    /// Set the maximum request delay in milliseconds
    pub fn max_request_delay_ms(mut self, delay: u64) -> Self {
        self.config.max_request_delay_ms = delay;
        self
    }


    /// Add skip patterns
    pub fn add_skip_patterns(mut self, patterns: &[&str]) -> Self {
        self.config
            .skip_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    /// Replace all skip patterns
    pub fn skip_patterns(mut self, patterns: &[&str]) -> Self {
        self.config.skip_patterns = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add subdomain skip patterns
    pub fn add_skip_subdomain_patterns(mut self, patterns: &[&str]) -> Self {
        self.config
            .skip_subdomain_patterns
            .extend(patterns.iter().map(|s| s.to_string()));
        self
    }

    /// Replace all subdomain skip patterns
    pub fn skip_subdomain_patterns(mut self, patterns: &[&str]) -> Self {
        self.config.skip_subdomain_patterns = patterns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add priority paths
    pub fn add_priority_paths(mut self, paths: &[&str]) -> Self {
        self.config
            .priority_paths
            .extend(paths.iter().map(|s| s.to_string()));
        self
    }

    /// Replace all priority paths
    pub fn priority_paths(mut self, paths: &[&str]) -> Self {
        self.config.priority_paths = paths.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add user agents
    pub fn add_user_agents(mut self, agents: &[&str]) -> Self {
        self.config
            .user_agents
            .extend(agents.iter().map(|s| s.to_string()));
        self
    }

    /// Replace all user agents
    pub fn user_agents(mut self, agents: &[&str]) -> Self {
        self.config.user_agents = agents.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build the final SpiderConfig
    pub fn build(self) -> SpiderConfig {
        self.config
    }
}
