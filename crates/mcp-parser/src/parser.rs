use mcp_core::types::SelectorConfig;
use scraper::{Html, Selector};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Invalid selector '{0}': {1}")]
    InvalidSelector(String, String),
    #[error("No results found")]
    NoResults,
    #[error("Failed to extract URL: {0}")]
    UrlExtraction(String),
}

pub struct HtmlSearchParser {
    selectors: HashMap<String, CompiledSelectors>,
}

struct CompiledSelectors {
    container: Selector,
    title: Selector,
    url: Selector,
    snippet: Selector,
    date: Option<Selector>,
    thumbnail: Option<Selector>,
    next_page: Option<Selector>,
}

impl HtmlSearchParser {
    pub fn new() -> Self {
        Self {
            selectors: HashMap::new(),
        }
    }

    pub fn register_engine(
        &mut self,
        engine_name: &str,
        config: &SelectorConfig,
    ) -> Result<(), ParserError> {
        let compiled = CompiledSelectors {
            container: Selector::parse(&config.result_container)
                .map_err(|e| ParserError::InvalidSelector(config.result_container.clone(), e.to_string()))?,
            title: Selector::parse(&config.title_selector)
                .map_err(|e| ParserError::InvalidSelector(config.title_selector.clone(), e.to_string()))?,
            url: Selector::parse(&config.url_selector)
                .map_err(|e| ParserError::InvalidSelector(config.url_selector.clone(), e.to_string()))?,
            snippet: Selector::parse(&config.snippet_selector)
                .map_err(|e| ParserError::InvalidSelector(config.snippet_selector.clone(), e.to_string()))?,
            date: config.date_selector.as_ref()
                .map(|s| Selector::parse(s))
                .transpose()
                .map_err(|e| ParserError::InvalidSelector(
                    config.date_selector.clone().unwrap_or_default(),
                    e.to_string(),
                ))?,
            thumbnail: config.thumbnail_selector.as_ref()
                .map(|s| Selector::parse(s))
                .transpose()
                .map_err(|e| ParserError::InvalidSelector(
                    config.thumbnail_selector.clone().unwrap_or_default(),
                    e.to_string(),
                ))?,
            next_page: config.next_page_selector.as_ref()
                .map(|s| Selector::parse(s))
                .transpose()
                .map_err(|e| ParserError::InvalidSelector(
                    config.next_page_selector.clone().unwrap_or_default(),
                    e.to_string(),
                ))?,
        };

        self.selectors.insert(engine_name.to_string(), compiled);
        Ok(())
    }

    pub fn parse(
        &self,
        engine_name: &str,
        html: &str,
        base_url: &str,
    ) -> Result<Vec<ParsedResult>, ParserError> {
        let selectors = self.selectors
            .get(engine_name)
            .ok_or_else(|| ParserError::NoResults)?;

        let document = Html::parse_document(html);
        let mut results = Vec::new();

        for element in document.select(&selectors.container) {
            let title = element
                .select(&selectors.title)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let url = element
                .select(&selectors.url)
                .next()
                .and_then(|e| e.value().attr("href"))
                .map(|href| normalize_url(href, base_url))
                .unwrap_or_default();

            let snippet = element
                .select(&selectors.snippet)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            let date = selectors.date.as_ref().and_then(|sel| {
                element.select(sel).next()
                    .map(|e| e.text().collect::<String>().trim().to_string())
            });

            let thumbnail = selectors.thumbnail.as_ref().and_then(|sel| {
                element.select(sel).next()
                    .and_then(|e| e.value().attr("src").map(String::from))
            });

            if !title.is_empty() || !url.is_empty() {
                results.push(ParsedResult {
                    title,
                    url,
                    snippet,
                    published_date: date,
                    thumbnail,
                });
            }
        }

        let _next_page = selectors.next_page.as_ref().and_then(|sel| {
            document.select(sel).next()
                .and_then(|e| e.value().attr("href"))
                .map(String::from)
        });

        if results.is_empty() {
            return Err(ParserError::NoResults);
        }

        Ok(results)
    }

    pub fn extract_next_page(&self, engine_name: &str, html: &str) -> Option<String> {
        let selectors = self.selectors.get(engine_name)?;
        let document = Html::parse_document(html);
        selectors.next_page.as_ref().and_then(|sel| {
            document.select(sel).next()
                .and_then(|e| e.value().attr("href"))
                .map(String::from)
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_date: Option<String>,
    pub thumbnail: Option<String>,
}

fn normalize_url(href: &str, base_url: &str) -> String {
    // Handle Bing redirect URLs: decode base64-encoded actual URL from `u` parameter
    if let Some(actual_url) = decode_bing_redirect(href) {
        return actual_url;
    }

    // Handle Yahoo redirect URLs: extract actual URL from redirect
    if let Some(actual_url) = decode_yahoo_redirect(href) {
        return actual_url;
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    if href.starts_with("//") {
        return format!("https:{}", href);
    }

    if let Ok(base) = url::Url::parse(base_url) {
        if let Ok(resolved) = base.join(href) {
            return resolved.to_string();
        }
    }

    href.to_string()
}

/// Decode Yahoo redirect URLs like:
/// https://r.search.yahoo.com/_ylt=...&RU=https%3a%2f%2frust-lang.org%2f&...
/// Extracts the actual URL from the RU parameter
fn decode_yahoo_redirect(url: &str) -> Option<String> {
    if !url.contains("r.search.yahoo.com") && !url.contains("search.yahoo.com") {
        return None;
    }

    // Look for RU= parameter (URL encoded) - can be preceded by / or &
    // Format: /RU=https%3a%2f%2f.../RK=...
    // or: &RU=https%3a%2f%2f...&...
    let url_upper = url.to_uppercase();
    
    // Find RU= position
    let ru_pos = url_upper.find("RU=")?;
    let ru_value_start = ru_pos + 3;
    
    // Find end of RU parameter (next / or & or end of string)
    let ru_value_end = url[ru_value_start..]
        .find(|c: char| c == '/' || c == '&' || c == ';')
        .map(|i| ru_value_start + i)
        .unwrap_or(url.len());

    let encoded = &url[ru_value_start..ru_value_end];
    
    // URL decode
    if let Ok(decoded) = urlencoding::decode(encoded) {
        if decoded.starts_with("http://") || decoded.starts_with("https://") {
            return Some(decoded.into_owned());
        }
    }
    
    None
}

/// Decode Bing redirect URLs like:
/// https://www.bing.com/ck/a?!&&p=...&u=a1aHR0cHM6Ly9...&ntb=1
/// The `u` parameter contains base64-encoded actual URL
pub fn decode_bing_redirect(url: &str) -> Option<String> {
    if !url.contains("bing.com/ck/a") {
        return None;
    }

    // Try to extract u parameter from the URL
    let url_lower = url.to_lowercase();
    let u_start = url_lower.find("&u=")?;
    let u_value_start = u_start + 3;
    
    // Find end of u parameter (next & or end of string)
    let u_value_end = url_lower[u_value_start..]
        .find('&')
        .map(|i| u_value_start + i)
        .unwrap_or(url.len());

    let encoded = &url[u_value_start..u_value_end];
    
    use base64::Engine;
    
    // Bing uses a custom encoding: "a1" prefix before the base64 data
    // Try stripping the "a1" prefix first
    let candidates: Vec<String> = vec![
        encoded.to_string(),
        encoded.strip_prefix("a1").unwrap_or("").to_string(),
        encoded.strip_prefix("a").unwrap_or("").to_string(),
    ];
    
    for candidate in &candidates {
        if candidate.is_empty() {
            continue;
        }
        
        // Try URL_SAFE_NO_PAD
        if let Ok(decoded_bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(candidate) {
            if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                if decoded_str.starts_with("http://") || decoded_str.starts_with("https://") {
                    return Some(decoded_str);
                }
            }
        }
        
        // Try URL_SAFE with padding
        let padded = match candidate.len() % 4 {
            2 => format!("{}==", candidate),
            3 => format!("{}=", candidate),
            _ => candidate.clone(),
        };
        
        if let Ok(decoded_bytes) = base64::engine::general_purpose::URL_SAFE.decode(&padded) {
            if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                if decoded_str.starts_with("http://") || decoded_str.starts_with("https://") {
                    return Some(decoded_str);
                }
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bing_redirect() {
        // Real Bing redirect URL from live search
        let test_url = "https://www.bing.com/ck/a?!&&p=207d362a9d288e6dfdfe7baccdd054f33ba96ac6ccf4603cbdc1ef27ffbbb025JmltdHM9MTc4MzU1NTIwMA&ptn=3&ver=2&hsh=4&fclid=36e59229-9ed0-6f42-3c08-85b89f876eaf&u=a1aHR0cHM6Ly9ydXN0LWxhbmcub3JnLw&ntb=1";
        
        let result = decode_bing_redirect(test_url);
        assert_eq!(result, Some("https://rust-lang.org/".to_string()));
    }

    #[test]
    fn test_decode_bing_redirect_wikipedia() {
        let test_url = "https://www.bing.com/ck/a?!&&p=07be756f04082cac4ed440b486dcd5045814370ee8f9bba67fe05cb8fc2fee5cJmltdHM9MTc4MzU1NTIwMA&ptn=3&ver=2&hsh=4&fclid=36e59229-9ed0-6f42-3c08-85b89f876eaf&u=a1aHR0cHM6Ly9lbi53aWtpcGVkaWEub3JnL3dpa2kvUnVzdF8ocHJvZ3JhbW1pbmdfbGFuZ3VhZ2Up&ntb=1";
        
        let result = decode_bing_redirect(test_url);
        assert_eq!(result, Some("https://en.wikipedia.org/wiki/Rust_(programming_language)".to_string()));
    }

    #[test]
    fn test_decode_bing_redirect_non_bing() {
        let test_url = "https://google.com/search?q=test";
        let result = decode_bing_redirect(test_url);
        assert_eq!(result, None);
    }
}
