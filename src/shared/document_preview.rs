//! Short-lived, local documents for the docked WebView.
use std::{
    fmt,
    net::TcpListener,
    sync::{Arc, OnceLock},
    time::Duration,
};

use ruma::html::{ElementAttributesSchemes, Html, ListBehavior, PropertiesNames, SanitizerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use super::attachment_download::DownloadKind;

pub const HTML_LIMIT: u64 = 512 * 1024;
pub const PDF_LIMIT: u64 = 24 * 1024 * 1024;

fn static_html(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(bytes).map_err(|_| "HTML must contain UTF-8 text".to_owned())?;
    let html = Html::parse(source.trim_start_matches('\u{feff}'));
    html.sanitize_with(
        &SanitizerConfig::strict()
            .allow_elements(
                [
                    "style",
                    "article",
                    "main",
                    "section",
                    "header",
                    "footer",
                    "div",
                    "figure",
                    "figcaption",
                ],
                ListBehavior::Add,
            )
            .remove_elements(["script", "iframe", "object", "embed", "svg", "math"])
            .remove_attributes([PropertiesNames {
                parent: "a",
                properties: &["href", "target"],
            }])
            .allow_schemes(
                [ElementAttributesSchemes {
                    element: "img",
                    attr_schemes: &[PropertiesNames {
                        parent: "src",
                        properties: &["data"],
                    }],
                }],
                ListBehavior::Override,
            ),
    );
    Ok(format!(
        "<!doctype html><html><head><meta charset=utf-8>\
         <meta name=viewport content='width=device-width, initial-scale=1'><style>\
         :root{{color-scheme:light dark;font:16px/1.55 system-ui,sans-serif}}\
         body{{max-width:76ch;margin:2rem auto;padding:0 1.25rem;overflow-wrap:anywhere}}\
         pre{{overflow:auto;padding:.65rem;background:rgba(128,128,128,.12)}}\
         table{{border-collapse:collapse}}td,th{{border:1px solid #888;padding:.35rem .55rem}}\
         </style></head><body>{html}</body></html>"
    )
    .into_bytes())
}

/// Owns exactly one preview URL. The listener and plaintext are released when
/// the reader tab closes. A random route keeps unrelated loopback clients out.
#[derive(Clone)]
pub struct DocumentPreview(Arc<Preview>);

struct Preview {
    url: Url,
    task: tokio::task::JoinHandle<()>,
    _slot: tokio::sync::OwnedSemaphorePermit,
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl fmt::Debug for DocumentPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DocumentPreview")
    }
}

impl DocumentPreview {
    pub fn url(&self) -> &Url {
        &self.0.url
    }

    pub fn new(kind: DownloadKind, bytes: Vec<u8>, epoch: u64) -> Result<Self, String> {
        static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
        let slot = SLOTS
            .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Close an open attachment preview, then try again.".to_owned())?;
        let (mime, policy_headers, bytes) = match kind {
            DownloadKind::Html if bytes.len() as u64 <= HTML_LIMIT => (
                "text/html; charset=utf-8",
                "Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; img-src data:; form-action 'none'; base-uri 'none'\r\n",
                static_html(bytes)?,
            ),
            DownloadKind::Pdf if bytes.len() as u64 <= PDF_LIMIT && bytes.starts_with(b"%PDF-") => {
                ("application/pdf", "", bytes)
            }
            _ => return Err("This file cannot be previewed.".into()),
        };
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        let path = format!("/{}", uuid::Uuid::new_v4());
        let host = address.to_string();
        let url = Url::parse(&format!("http://{host}{path}")).map_err(|e| e.to_string())?;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\
             Content-Disposition: inline\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\n\
             X-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\n\
             X-Frame-Options: DENY\r\n{policy_headers}Connection: close\r\n\r\n",
            bytes.len(),
        );
        let runtime = crate::sliding_sync::backend_runtime();
        let listener = {
            let _guard = runtime.enter();
            tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?
        };
        let task = crate::account_session::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let _ = tokio::time::timeout(Duration::from_secs(8), async {
                    let mut request = [0; 8192];
                    let mut read = 0;
                    loop {
                        let count = stream.read(&mut request[read..]).await?;
                        if count == 0 { return Ok::<(), std::io::Error>(()); }
                        let start = read.saturating_sub(3);
                        read += count;
                        if request[start..read].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let mut lines = request[..read].split(|b| *b == b'\n');
                    let mut words = lines.next().unwrap_or_default().split(|b| b.is_ascii_whitespace());
                    let method = words.next();
                    let address_matches = lines.any(|line| line.len() > 5
                        && line[..5].eq_ignore_ascii_case(b"host:")
                        && line[5..].trim_ascii() == host.as_bytes());
                    if matches!(method, Some(b"GET" | b"HEAD"))
                        && words.next() == Some(path.as_bytes())
                        && address_matches
                        && epoch == crate::account_session::epoch()
                    {
                        stream.write_all(response.as_bytes()).await?;
                        if method == Some(b"GET") { stream.write_all(&bytes).await?; }
                    } else {
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await?;
                    }
                    stream.shutdown().await
                }).await;
            }
        });
        Ok(Self(Arc::new(Preview {
            url,
            task,
            _slot: slot,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn request(preview: &DocumentPreview, path: &str, method: &str) -> Vec<u8> {
        let url = preview.url();
        let port = url.port().unwrap();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    }

    #[test]
    fn document_preview_keeps_html_content_and_blocks_active_or_external_content() {
        let bytes = br#"<h1>Quarterly report</h1><script>alert(1)</script>
            <meta http-equiv="refresh" content="0;url=https://example.com/">
            <a href="https://example.com/" onclick="alert(1)">Reference</a>
            <iframe src="https://example.com"></iframe><img src="https://example.com/tracker.png">
            <table><tr><td>Revenue</td></tr></table>"#;
        let preview = DocumentPreview::new(
            DownloadKind::Html,
            bytes.to_vec(),
            crate::account_session::epoch(),
        )
        .unwrap();
        let response = String::from_utf8(request(&preview, preview.url().path(), "GET")).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("Content-Security-Policy: default-src 'none'"));
        assert!(response.contains("<h1>Quarterly report</h1>"));
        assert!(response.contains("<td>Revenue</td>") && response.contains("Reference"));
        assert!(!response.contains("example.com") && !response.contains("alert(1)"));
    }

    #[test]
    fn document_preview_serves_pdf_only_at_its_scoped_url() {
        let pdf = b"%PDF-1.7\nfixture";
        let preview = DocumentPreview::new(
            DownloadKind::Pdf,
            pdf.to_vec(),
            crate::account_session::epoch(),
        )
        .unwrap();
        let url = preview.url().clone();
        assert!(request(&preview, "/missing", "GET").starts_with(b"HTTP/1.1 404"));
        assert!(request(&preview, url.path(), "POST").starts_with(b"HTTP/1.1 404"));
        let response = request(&preview, url.path(), "GET");
        assert!(
            response
                .windows(29)
                .any(|w| w == b"Content-Type: application/pdf")
        );
        assert!(response.ends_with(pdf));
        assert!(!request(&preview, url.path(), "HEAD").ends_with(pdf));
        let stale = DocumentPreview::new(
            DownloadKind::Pdf,
            pdf.to_vec(),
            crate::account_session::epoch().wrapping_sub(1),
        )
        .unwrap();
        assert!(request(&stale, stale.url().path(), "GET").starts_with(b"HTTP/1.1 404"));
        assert!(
            DocumentPreview::new(
                DownloadKind::Pdf,
                b"<!doctype html>".to_vec(),
                crate::account_session::epoch()
            )
            .is_err()
        );
    }
}
