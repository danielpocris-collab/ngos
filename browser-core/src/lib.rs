//! NGOS Browser Core
//!
//! Core types, traits, and error definitions shared across all browser crates.
//!
//! Canonical subsystem role:
//! - subsystem: browser core support
//! - owner layer: application support layer
//! - semantic owner: `browser-core`
//! - truth path role: shared browser-facing support contracts for browser
//!   vertical crates
//!
//! Canonical contract families defined here:
//! - browser error contracts
//! - browser shared result contracts
//! - browser shared core type contracts
//!
//! This crate may define browser-vertical support contracts, but it must not
//! redefine kernel, runtime, or product-level OS truth.

use thiserror::Error;

/// Browser result type
pub type BrowserResult<T> = Result<T, BrowserError>;

/// Browser error types
#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Layout error: {0}")]
    Layout(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("JavaScript error: {0}")]
    JavaScript(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// URL structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl Url {
    pub fn parse(s: &str) -> Result<Self, BrowserError> {
        if let Some(path) = s.strip_prefix("file://") {
            let normalized = if path.is_empty() { "/" } else { path };
            return Ok(Self {
                scheme: String::from("file"),
                host: String::new(),
                port: None,
                path: String::from(normalized),
                query: None,
                fragment: None,
            });
        }

        // Simple URL parser
        let without_fragment = s.split('#').next().unwrap_or(s);
        let (url_part, fragment) = if s.contains('#') {
            (
                without_fragment,
                Some(String::from(&s[s.find('#').unwrap() + 1..])),
            )
        } else {
            (without_fragment, None)
        };

        let without_query = url_part.split('?').next().unwrap_or(url_part);
        let (scheme_host_path, query) = if url_part.contains('?') {
            (
                without_query,
                Some(String::from(&url_part[url_part.find('?').unwrap() + 1..])),
            )
        } else {
            (without_query, None)
        };

        let parts: Vec<&str> = scheme_host_path.split("://").collect();
        let (scheme, rest) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1])
        } else {
            return Err(BrowserError::Parse("Invalid URL scheme".into()));
        };

        let (host_port, path) = match rest.split_once('/') {
            Some((hp, p)) => (hp, format!("/{}", p)),
            None => (rest, String::from("/")),
        };

        let (host, port) = match host_port.split_once(':') {
            Some((h, p)) => {
                let port_num = p
                    .parse::<u16>()
                    .map_err(|_| BrowserError::Parse("Invalid port".into()))?;
                (h.to_string(), Some(port_num))
            }
            None => (host_port.to_string(), None),
        };

        Ok(Self {
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        })
    }

    pub fn origin(&self) -> String {
        if self.scheme == "file" {
            return String::from("file://");
        }
        match self.port {
            Some(port) => format!("{}://{}:{}", self.scheme, self.host, port),
            None => format!("{}://{}", self.scheme, self.host),
        }
    }
}

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub user_agent: String,
    pub accept_languages: Vec<String>,
    pub max_connections: usize,
    pub cache_size_mb: usize,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            user_agent: String::from("NGOS-Browser/0.1.0"),
            accept_languages: vec![String::from("en-US"), String::from("en")],
            max_connections: 6,
            cache_size_mb: 100,
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}", self.scheme, self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{}", port)?;
        }
        write!(f, "{}", self.path)?;
        if let Some(ref query) = self.query {
            write!(f, "?{}", query)?;
        }
        if let Some(ref fragment) = self.fragment {
            write!(f, "#{}", fragment)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_simple() {
        let url = Url::parse("http://example.com").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.path, "/");
    }

    #[test]
    fn parse_url_with_port() {
        let url = Url::parse("https://example.com:8080/path").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn parse_url_with_query() {
        let url = Url::parse("http://example.com/search?q=rust").unwrap();
        assert_eq!(url.query, Some(String::from("q=rust")));
    }

    #[test]
    fn parse_url_with_fragment() {
        let url = Url::parse("http://example.com/page#section").unwrap();
        assert_eq!(url.fragment, Some(String::from("section")));
    }

    #[test]
    fn parse_file_url() {
        let url =
            Url::parse("file://C:/Users/pocri/OneDrive/Desktop/experiment/docs/ui-preview.html")
                .unwrap();
        assert_eq!(url.scheme, "file");
        assert_eq!(url.host, "");
        assert!(url.path.contains("ui-preview.html"));
    }
}
