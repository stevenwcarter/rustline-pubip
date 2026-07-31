//! `pubip` — a rustline widget showing your public/WAN IP.
//!
//! Fetches a configured URL (default <https://api.ipify.org>, which returns
//! the caller's IP as plain text) through the host's TTL-cached HTTP
//! capability, so the network is hit at most once per `refresh_secs` and a
//! stale answer is served while the endpoint is unreachable.
//!
//! Pure logic lives here and is unit-tested on the host target (`cargo
//! test`); the Extism guest glue below only compiles for wasm32.

use serde::Deserialize;

/// Longest plausible IP literal (an IPv6 address maxes out at 45 chars
/// including an IPv4-mapped tail). Anything longer is not an IP.
pub const MAX_IP_CHARS: usize = 45;

/// This plugin's `[plugins.pubip.options]` table.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Options {
    /// Endpoint returning the caller's IP as a plain-text body.
    pub url: String,
    /// Cache TTL in seconds (the host owns the cache).
    pub refresh_secs: i64,
    /// Render format. `{ip}` is the fetched address.
    pub format: String,
    /// Click-toggle alternate view.
    pub alt_format: String,
    /// Shown when no usable answer exists. Empty renders nothing.
    pub down_format: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            url: "https://api.ipify.org".to_string(),
            refresh_secs: 900,
            format: "\u{f0a5f} {ip}".to_string(), // nf-md-ip_network + space
            alt_format: String::new(),
            down_format: String::new(),
        }
    }
}

/// Trim and validate a response body as an IP-address-looking string, so a
/// captive portal's HTML error page never renders into the status line.
/// Accepts IPv4 (`203.0.113.9`) and IPv6 (`2001:db8::1`) shapes: hex digits,
/// dots, and colons only, at most [`MAX_IP_CHARS`] chars.
pub fn extract_ip(body: &str) -> Option<String> {
    let s = body.trim();
    if s.is_empty() || s.len() > MAX_IP_CHARS {
        return None;
    }
    s.chars()
        .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':')
        .then(|| s.to_string())
}

/// Substitute `{ip}`. Unknown placeholders pass through untouched, the same
/// convention as the built-in widgets' formats.
pub fn render_format(format: &str, ip: &str) -> String {
    format.replace("{ip}", ip)
}

/// The capability manifest, embedded as the `rustline-manifest` custom
/// section so `rustline plugin approve pubip` can read the requests straight
/// out of the distributed `.wasm` — `plugin install` downloads only this
/// binary, so a sidecar file would never reach the user's plugin dir.
#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = "rustline-manifest")]
pub static MANIFEST: [u8; include_bytes!("../manifest.toml").len()] =
    *include_bytes!("../manifest.toml");

#[cfg(target_arch = "wasm32")]
mod guest {
    use super::*;
    use rustline_plugin_sdk::{
        GuestRender, LogLevel, Segment, active_format, export_plugin, http_get_cached, log,
    };

    fn render(input: &GuestRender) -> Vec<Segment> {
        let opts: Options = serde_json::from_value(input.config.clone()).unwrap_or_default();
        let format = active_format(&input.context, "pubip", &opts.format, &opts.alt_format);

        match http_get_cached(&opts.url, opts.refresh_secs, &input.context.now) {
            Ok(r) if r.ok => match extract_ip(&r.body) {
                Some(ip) => vec![Segment::new(render_format(format, &ip))],
                None => {
                    log(
                        LogLevel::Warn,
                        &format!(
                            "pubip: {} returned a body that doesn't look like an IP",
                            opts.url
                        ),
                    );
                    down_segment(&opts.down_format)
                }
            },
            Ok(r) => {
                log(LogLevel::Warn, &format!("pubip: fetch failed: {}", r.error));
                down_segment(&opts.down_format)
            }
            Err(e) => {
                log(LogLevel::Warn, &format!("pubip: host call failed: {e}"));
                down_segment(&opts.down_format)
            }
        }
    }

    /// `down_format` verbatim, or no segment at all when it's empty — the
    /// built-in widgets' collapse-to-nothing convention.
    fn down_segment(down_format: &str) -> Vec<Segment> {
        if down_format.is_empty() {
            Vec::new()
        } else {
            vec![Segment::new(down_format.to_string())]
        }
    }

    export_plugin!(name: "pubip", render: render);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ipv4_with_surrounding_whitespace() {
        assert_eq!(extract_ip("203.0.113.9\n"), Some("203.0.113.9".to_string()));
    }

    #[test]
    fn extracts_ipv6() {
        assert_eq!(
            extract_ip("2001:db8::8a2e:370:7334"),
            Some("2001:db8::8a2e:370:7334".to_string())
        );
    }

    #[test]
    fn rejects_html_error_pages() {
        assert_eq!(extract_ip("<html><body>captive portal</body></html>"), None);
    }

    #[test]
    fn rejects_empty_and_overlong() {
        assert_eq!(extract_ip("   "), None);
        assert_eq!(extract_ip(&"1".repeat(46)), None);
    }

    #[test]
    fn render_format_substitutes_ip_and_keeps_unknown_placeholders() {
        assert_eq!(render_format("IP {ip} {x}", "1.2.3.4"), "IP 1.2.3.4 {x}");
    }

    #[test]
    fn default_options_match_readme() {
        let o = Options::default();
        assert_eq!(o.url, "https://api.ipify.org");
        assert_eq!(o.refresh_secs, 900);
        assert!(o.format.contains("{ip}"));
    }
}
