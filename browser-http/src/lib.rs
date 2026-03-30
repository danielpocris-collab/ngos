//! NGOS Browser HTTP Stack
//!
//! HTTP/1.1 client - 100% Proprietary, no external deps

pub use browser_core::{BrowserError, BrowserResult, Url};

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// HTTP client
pub struct HttpClient {
    config: HttpClientConfig,
}

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub max_connections: usize,
    pub connection_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            max_connections: 6,
            connection_timeout_ms: 30000,
            read_timeout_ms: 60000,
            user_agent: String::from("NGOS-Browser/0.1.0"),
        }
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
        }
    }

    pub fn with_config(config: HttpClientConfig) -> Self {
        Self { config }
    }

    /// Fetch a URL (HTTP only, no HTTPS yet)
    pub fn get(&self, url: &Url) -> BrowserResult<HttpResponse> {
        if url.scheme != "http" {
            return Err(BrowserError::Network(format!(
                "Unsupported scheme: {}",
                url.scheme
            )));
        }

        let port = url.port.unwrap_or(80);
        let addr = format!("{}:{}", url.host, port);

        let mut stream =
            TcpStream::connect(&addr).map_err(|e| BrowserError::Network(e.to_string()))?;

        stream
            .set_read_timeout(Some(Duration::from_millis(self.config.read_timeout_ms)))
            .ok();

        // Build HTTP/1.1 GET request
        let request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
             Accept-Language: en-US,en;q=0.5\r\n\
             Connection: close\r\n\
             \r\n",
            url.path, url.host, self.config.user_agent
        );

        stream
            .write_all(request.as_bytes())
            .map_err(|e| BrowserError::Network(e.to_string()))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|e| BrowserError::Network(e.to_string()))?;

        HttpResponse::parse(&response)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP Response
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn parse(raw: &[u8]) -> BrowserResult<Self> {
        // Find header/body separator (\r\n\r\n)
        let separator = b"\r\n\r\n";
        let split_idx = raw
            .windows(separator.len())
            .position(|w| w == separator)
            .ok_or_else(|| {
                BrowserError::Parse("Invalid HTTP response: no header/body separator".into())
            })?;

        let header_bytes = &raw[..split_idx];
        let body = raw[split_idx + separator.len()..].to_vec();

        let header_str = core::str::from_utf8(header_bytes)
            .map_err(|_| BrowserError::Parse("Invalid UTF-8 in HTTP headers".into()))?;

        let mut lines = header_str.split("\r\n");

        // Parse status line: "HTTP/1.1 200 OK"
        let status_line = lines
            .next()
            .ok_or_else(|| BrowserError::Parse("No HTTP status line".into()))?;

        let mut parts = status_line.split_whitespace();
        let _version = parts
            .next()
            .ok_or_else(|| BrowserError::Parse("No HTTP version".into()))?;
        let status = parts
            .next()
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(|| BrowserError::Parse("Invalid status code".into()))?;
        let status_text = parts.next().unwrap_or("OK").to_string();

        // Parse headers
        let mut headers = Vec::new();
        for line in lines {
            if let Some((key, value)) = line.split_once(':') {
                headers.push((key.trim().to_string(), value.trim().to_string()));
            }
        }

        Ok(Self {
            status,
            status_text,
            headers,
            body,
        })
    }

    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.as_str())
    }

    pub fn content_length(&self) -> Option<usize> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, v)| v.parse::<usize>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_response() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello, World!";
        let resp = HttpResponse::parse(raw).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.status_text, "OK");
        assert_eq!(resp.content_type(), Some("text/html"));
        assert_eq!(resp.content_length(), Some(13));
        assert_eq!(resp.body, b"Hello, World!");
    }

    #[test]
    fn create_http_client() {
        let client = HttpClient::new();
        assert_eq!(client.config.max_connections, 6);
    }
}
