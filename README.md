# Tiny Crawler - Web Crawler

Tiny Crawler is a high-performance lightweight web crawler written in Rust. It's designed to be configurable, efficient, and respectful of the websites it crawls.

## Features

- Crawl websites and collect all URLs
- Anti-bot detection measures with randomized delays and rotating user agents
- Configurable crawl depth, concurrency, and request delays
- Batch mode to crawl multiple sites from a file
- Domain-based filtering with priority URL support
- Pattern detection to avoid massive link traps
- Debug HTML capture for troubleshooting

## Installation

### Prerequisites

- Rust and Cargo (1.54.0+)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/shuhaodo/tiny-crawler.git
cd tiny-crawler

# Build the project
cargo build --release
```

## Usage

### Single URL Crawl

To crawl a single website:

```bash
cargo run -- crawl <url> [max_depth] [max_loops] [max_concurrent] [min_delay_ms] [max_delay_ms]
```

Example:

```bash
cargo run -- crawl https://example.com 5 100 20 500 2000
```

### Batch Crawl

To crawl multiple websites from a file:

```bash
cargo run -- batch [url_file] [max_depth] [max_loops] [max_concurrent] [max_concurrent_sites] [min_delay_ms] [max_delay_ms]
```

Example:

```bash
cargo run -- batch input/urls.txt 5 100 20 3 500 2000
```

The URL file should contain one URL per line. Lines starting with `#` are treated as comments.

### Parameters

- `max_depth`: Maximum crawl depth (default: 10)
- `max_loops`: Maximum number of processing loops (default: 50)
- `max_concurrent`: Maximum concurrent requests per website (default: 30)
- `max_concurrent_sites`: Maximum websites to crawl in parallel (default: 5)
- `min_delay_ms`: Minimum delay between requests in milliseconds (default: 100)
- `max_delay_ms`: Maximum delay between requests in milliseconds (default: 2000)

## Output

Tiny Crawler saves crawl results in the `output` directory with one JSON file per domain. The results include:

- List of all found URLs
- Skipped URLs with reasons
- Detected patterns
- Redirects
- Statistics

For debugging purposes, HTML content is saved in the `debug/<domain>/` directory when link extraction issues are detected.

## Configuration

The crawler can be configured through command-line parameters or programmatically. The default configuration is designed to be respectful of websites and avoid detection.

### Skip Patterns

The crawler automatically skips URLs matching common patterns that are typically not useful for crawling (e.g., `/assets/`, `/static/`, `/wp-content/`). This behavior can be customized programmatically.

### Priority URLs

Some paths are given higher priority during crawling (e.g., `/contact`, `/about`). This helps ensure the most important pages are crawled first.

## Project Structure

- `src/spider/crawler.rs`: Main crawling logic
- `src/spider/config.rs`: Configuration parameters
- `src/spider/network.rs`: Network handling
- `src/spider/loader.rs`: Batch loading functionality
- `src/spider/utils.rs`: Utility functions
- `src/spider/error.rs`: Error handling

## License

[MIT License](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
