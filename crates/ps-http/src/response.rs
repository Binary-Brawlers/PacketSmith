//! Response viewing models, metrics calculations, hex dump generator, and syntax formatters.

use std::collections::HashMap;
use std::fmt::Write;
use serde::{Deserialize, Serialize};

pub const LARGE_RESPONSE_THRESHOLD_BYTES: usize = 10 * 1024 * 1024; // 10 MB

/// Status category derived from HTTP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpStatusCategory {
    Informational, // 1xx
    Successful,    // 2xx
    Redirection,   // 3xx
    ClientError,   // 4xx
    ServerError,   // 5xx
    Unknown,
}

impl HttpStatusCategory {
    pub fn from_status_code(code: u16) -> Self {
        match code {
            100..=199 => Self::Informational,
            200..=299 => Self::Successful,
            300..=399 => Self::Redirection,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::Unknown,
        }
    }
}

/// Formats raw byte counts into human-readable strings (B, KB, MB).
pub fn format_byte_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Active mode for rendering response body content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseBodyMode {
    #[default]
    Pretty,
    Raw,
    Hex,
    Preview,
}

/// Active response viewer tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseViewTab {
    #[default]
    Body,
    Headers,
    Cookies,
    Timing,
}

/// Formatter for pretty printing JSON payloads.
pub fn format_json_pretty(text: &str) -> Result<String, serde_json::Error> {
    let val: serde_json::Value = serde_json::from_str(text)?;
    serde_json::to_string_pretty(&val)
}

/// Generates a standard canonical 16-byte-per-line hex dump with ASCII sidebar.
pub fn generate_hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();
    let chunk_size = 16;

    for (chunk_idx, chunk) in bytes.chunks(chunk_size).enumerate() {
        let offset = chunk_idx * chunk_size;
        let _ = write!(output, "{:08x}  ", offset);

        // First 8 bytes
        for i in 0..8 {
            if i < chunk.len() {
                let _ = write!(output, "{:02x} ", chunk[i]);
            } else {
                output.push_str("   ");
            }
        }
        output.push(' ');

        // Next 8 bytes
        for i in 8..16 {
            if i < chunk.len() {
                let _ = write!(output, "{:02x} ", chunk[i]);
            } else {
                output.push_str("   ");
            }
        }

        output.push_str(" |");
        for &b in chunk {
            if (32..=126).contains(&b) {
                output.push(b as char);
            } else {
                output.push('.');
            }
        }
        output.push_str("|\n");
    }

    output
}

/// Parses the `Set-Cookie` header lines into structured cookies.
pub fn parse_cookies_from_headers(headers: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut cookies = Vec::new();
    for (name, val) in headers {
        if name.eq_ignore_ascii_case("set-cookie") {
            for part in val.split(';') {
                let trimmed = part.trim();
                if let Some(eq) = trimmed.find('=') {
                    let k = trimmed[..eq].trim().to_string();
                    let v = trimmed[eq + 1..].trim().to_string();
                    cookies.push((k, v));
                    break;
                }
            }
        }
    }
    cookies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_byte_size() {
        assert_eq!(format_byte_size(500), "500 B");
        assert_eq!(format_byte_size(1536), "1.5 KB");
        assert_eq!(format_byte_size(2 * 1024 * 1024), "2.00 MB");
    }

    #[test]
    fn test_hex_dump_generation() {
        let data = b"Hello, World!";
        let dump = generate_hex_dump(data);
        assert!(dump.starts_with("00000000  48 65 6c 6c 6f 2c 20 57"));
        assert!(dump.contains("|Hello, World!|"));
    }

    #[test]
    fn test_json_pretty() {
        let compact = "{\"message\":\"ok\",\"count\":1}";
        let pretty = format_json_pretty(compact).expect("format json");
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("  \"message\": \"ok\""));
    }
}
