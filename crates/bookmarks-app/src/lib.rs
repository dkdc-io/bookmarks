//! Tauri desktop shell for bookmarks.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use bookmarks_core::storage::Storage;
use tauri::webview::NewWindowResponse;

pub fn run_app(storage: Box<dyn Storage>) -> anyhow::Result<()> {
    let server = bookmarks_webapp::spawn_loopback(storage)?;
    let server_addr = server.addr();
    wait_for_health(server_addr)?;

    let url = server.url();
    tauri::Builder::default()
        .setup(move |app| {
            let url = url
                .parse()
                .map_err(|err| format!("invalid webapp URL: {err}"))?;

            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::External(url))
                .on_navigation(move |url| {
                    if is_local_webapp_url(url, server_addr) {
                        true
                    } else {
                        open_external_url(url);
                        false
                    }
                })
                .on_new_window(|url, _features| {
                    open_external_url(&url);
                    NewWindowResponse::Deny
                })
                .title("bookmarks")
                .inner_size(720.0, 800.0)
                .min_inner_size(360.0, 520.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())?;

    drop(server);
    Ok(())
}

fn is_local_webapp_url(url: &tauri::Url, addr: SocketAddr) -> bool {
    let host_matches = match url.host_str() {
        Some("localhost") => addr.ip().is_loopback(),
        Some(host) => host
            .parse()
            .is_ok_and(|ip: std::net::IpAddr| ip == addr.ip()),
        None => false,
    };

    url.scheme() == "http" && host_matches && url.port_or_known_default() == Some(addr.port())
}

fn open_external_url(url: &tauri::Url) {
    if should_open_externally(url) {
        let _ = open::that(url.as_str());
    }
}

fn should_open_externally(url: &tauri::Url) -> bool {
    !matches!(
        url.scheme(),
        "about" | "blob" | "data" | "javascript" | "tauri" | "asset"
    )
}

fn wait_for_health(addr: SocketAddr) -> anyhow::Result<()> {
    for _ in 0..50 {
        if health_check(addr).is_ok_and(|body| body.contains("\"status\":\"ok\"")) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("bookmarks webapp did not become ready at {addr}")
}

fn health_check(addr: SocketAddr) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: bookmarks\r\nConnection: close\r\n\r\n")?;
    let mut body = String::new();
    stream.read_to_string(&mut body)?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_webapp_url_matches_loopback_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 1414));
        let url = "http://127.0.0.1:1414/content".parse().unwrap();
        assert!(is_local_webapp_url(&url, addr));
    }

    #[test]
    fn localhost_matches_loopback_addr() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 1414));
        let url = "http://localhost:1414/content".parse().unwrap();
        assert!(is_local_webapp_url(&url, addr));
    }

    #[test]
    fn external_urls_are_not_local_webapp_urls() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 1414));
        let url = "https://github.com/dkdc-io/bookmarks".parse().unwrap();
        assert!(!is_local_webapp_url(&url, addr));
    }

    #[test]
    fn custom_url_schemes_open_externally() {
        let mailto = "mailto:cody@dkdc.io".parse().unwrap();
        let obsidian = "obsidian://open?vault=notes".parse().unwrap();
        assert!(should_open_externally(&mailto));
        assert!(should_open_externally(&obsidian));
    }

    #[test]
    fn script_like_url_schemes_do_not_open_externally() {
        let javascript = "javascript:alert(1)".parse().unwrap();
        let data = "data:text/html,<h1>x</h1>".parse().unwrap();
        assert!(!should_open_externally(&javascript));
        assert!(!should_open_externally(&data));
    }
}
