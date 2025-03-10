use crate::spider::error::SpiderError;
use anyhow::Result;
use regex::Regex;
use std::path::Path;
use url::Url;

/// Extract the base domain from a URL
///
/// If the URL contains a subdomain (except 'www'), the base domain includes the subdomain.
/// If the URL does not contain a subdomain, the base domain does not include a subdomain.
/// If the URL contains 'www', the base domain does not include 'www'.
pub fn extract_base_domain(url_str: &str) -> Result<String, SpiderError> {
    let url = Url::parse(url_str).map_err(SpiderError::UrlParse)?;

    let host = url
        .host_str()
        .ok_or_else(|| SpiderError::InvalidUrl(format!("No host in URL: {}", url_str)))?;

    // Strip 'www.' prefix if present
    if let Some(stripped) = host.strip_prefix("www.") {
        return Ok(stripped.to_string());
    }

    Ok(host.to_string())
}

/// Check if a URL is in the same base domain
pub fn is_same_domain(url_str: &str, base_domain: &str) -> Result<bool, SpiderError> {
    let url = Url::parse(url_str).map_err(SpiderError::UrlParse)?;

    let host = url
        .host_str()
        .ok_or_else(|| SpiderError::InvalidUrl(format!("No host in URL: {}", url_str)))?;

    // Strip 'www.' prefix if present
    let normalized_host = if let Some(stripped) = host.strip_prefix("www.") {
        stripped
    } else {
        host
    };

    // Check if domains are exactly the same
    if normalized_host == base_domain {
        return Ok(true);
    }

    // Check if the URL is a subdomain of the base domain
    if normalized_host.ends_with(&format!(".{}", base_domain)) {
        return Ok(true);
    }

    Ok(false)
}

/// Normalize a URL by handling redirects
pub fn normalize_url(url_str: &str) -> Result<String, SpiderError> {
    let url = Url::parse(url_str).map_err(SpiderError::UrlParse)?;

    Ok(url.to_string())
}

/// Resolve a relative URL against a base URL
pub fn resolve_url(base_url: &str, relative_url: &str) -> Result<String, SpiderError> {
    let base = Url::parse(base_url).map_err(SpiderError::UrlParse)?;

    let absolute_url = base.join(relative_url).map_err(SpiderError::UrlParse)?;

    Ok(absolute_url.to_string())
}

/// Check if a URL contains any of the skip patterns
pub fn should_skip_url(url: &str, skip_patterns: &[String]) -> bool {
    skip_patterns.iter().any(|pattern| url.contains(pattern))
}

/// Check if a URL has a subdomain that matches any of the skip patterns
pub fn should_skip_subdomain(
    url_str: &str,
    skip_subdomain_patterns: &[String],
) -> Result<bool, SpiderError> {
    let url = Url::parse(url_str).map_err(SpiderError::UrlParse)?;

    let host = url
        .host_str()
        .ok_or_else(|| SpiderError::InvalidUrl(format!("No host in URL: {}", url_str)))?;

    // Skip check if host starts with www.
    let normalized_host = if let Some(stripped) = host.strip_prefix("www.") {
        stripped
    } else {
        host
    };

    Ok(skip_subdomain_patterns
        .iter()
        .any(|pattern| normalized_host.starts_with(pattern)))
}

/// Check if a URL is a priority URL
pub fn is_priority_url(url: &str, priority_paths: &[String]) -> bool {
    priority_paths.iter().any(|path| url.contains(path))
}

/// Detect if a list of URLs contains a pattern that would indicate massive links
pub fn detect_massive_links_pattern(urls: &[String], threshold: usize) -> Option<String> {
    if urls.len() < threshold {
        return None;
    }

    // Simple pattern detection: look for URLs that follow a numeric pattern
    let re = Regex::new(r"(.*?)(\d+)(.*)").unwrap();

    let mut patterns: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for url in urls {
        if let Some(captures) = re.captures(url) {
            if captures.len() >= 4 {
                let prefix = captures.get(1).unwrap().as_str();
                let suffix = captures.get(3).unwrap().as_str();
                let pattern = format!("{}*{}", prefix, suffix);

                *patterns.entry(pattern).or_insert(0) += 1;
            }
        }
    }

    // Find patterns that exceed the threshold
    patterns
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .max_by_key(|(_, count)| *count)
        .map(|(pattern, _)| pattern)
}

/// Generate a filename from a domain
pub fn domain_to_filename(domain: &str) -> String {
    let filename = domain.replace(".", "_").replace(":", "_") + ".json";
    
    // Create output/crawler directory if it doesn't exist
    let output_dir = Path::new("output").join("crawler");
    let _ = std::fs::create_dir_all(&output_dir);
    
    output_dir
        .join(filename)
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_domain_with_subdomain() {
        let url = "https://camps.example.com";
        let result = extract_base_domain(url).unwrap();
        assert_eq!(result, "camps.example.com");
    }

    #[test]
    fn test_extract_base_domain_without_subdomain() {
        let url = "https://example.com/camps";
        let result = extract_base_domain(url).unwrap();
        assert_eq!(result, "example.com");
    }

    #[test]
    fn test_extract_base_domain_with_www() {
        let url = "https://www.example.com";
        let result = extract_base_domain(url).unwrap();
        assert_eq!(result, "example.com");
    }

    #[test]
    fn test_is_same_domain_true() {
        let url = "https://camps.example.com/faq";
        let base_domain = "camps.example.com";
        let result = is_same_domain(url, base_domain).unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_same_domain_false() {
        let url = "https://other.example.com/faq";
        let base_domain = "camps.example.com";
        let result = is_same_domain(url, base_domain).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_same_domain_subdomain() {
        let url = "https://camps.example.com/faq";
        let base_domain = "example.com";
        let result = is_same_domain(url, base_domain).unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_same_domain_www() {
        let url = "https://www.example.com/faq";
        let base_domain = "example.com";
        let result = is_same_domain(url, base_domain).unwrap();
        assert!(result);
    }

    #[test]
    fn test_resolve_url() {
        let base_url = "https://domain.com/faq";
        let relative_url = "/q1";
        let result = resolve_url(base_url, relative_url).unwrap();
        assert_eq!(result, "https://domain.com/q1");
    }

    #[test]
    fn test_should_skip_url() {
        let url = "https://example.com/docs/1";
        let skip_patterns = vec!["/docs/".to_string()];
        assert!(should_skip_url(url, &skip_patterns));
    }

    #[test]
    fn test_should_skip_subdomain() {
        let url = "https://docs.example.com";
        let skip_patterns = vec!["docs.".to_string()];
        assert!(should_skip_subdomain(url, &skip_patterns).unwrap());
    }

    #[test]
    fn test_should_not_skip_normal_subdomain() {
        let url = "https://blog.example.com";
        let skip_patterns = vec!["docs.".to_string(), "api.".to_string()];
        assert!(!should_skip_subdomain(url, &skip_patterns).unwrap());
    }

    #[test]
    fn test_should_skip_with_www_prefix() {
        let url = "https://www.docs.example.com";
        let skip_patterns = vec!["docs.".to_string()];
        assert!(should_skip_subdomain(url, &skip_patterns).unwrap());
    }

    #[test]
    fn test_is_priority_url() {
        let url = "https://example.com/contact";
        let priority_paths = vec!["/contact".to_string()];
        assert!(is_priority_url(url, &priority_paths));
    }

    #[test]
    fn test_detect_massive_links_pattern() {
        let urls = vec![
            "domain.com/a/pattern/1".to_string(),
            "domain.com/a/pattern/2".to_string(),
            "domain.com/a/pattern/3".to_string(),
            "domain.com/a/pattern/4".to_string(),
            "domain.com/a/pattern/5".to_string(),
            "domain.com/other/url".to_string(),
        ];

        let pattern = detect_massive_links_pattern(&urls, 5);
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap(), "domain.com/a/pattern/*");
    }

    #[test]
    fn test_domain_to_filename() {
        let domain = "example.com";
        let filename = domain_to_filename(domain);
        assert_eq!(filename, "output/crawler/example_com.json");
    }
}
