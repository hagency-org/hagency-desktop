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

/// The elements of a Matrix message, which the sanitizer's strict mode keeps by itself.
const MESSAGE_ELEMENTS: [&str; 37] = [
    "del", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "p", "a", "ul", "ol", "sup", "sub",
    "li", "b", "i", "u", "strong", "em", "s", "code", "hr", "br", "div", "table", "thead",
    "tbody", "tr", "th", "td", "caption", "pre", "span", "img", "details", "summary",
];

/// What a previewed page keeps beside them: its own style sheet, and the usual sections and
/// text marks of a page.
const PAGE_ELEMENTS: [&str; 22] = [
    "style", "article", "main", "section", "header", "footer", "figure", "figcaption", "nav",
    "aside", "small", "mark", "dl", "dt", "dd", "tfoot", "colgroup", "col", "kbd", "abbr",
    "time", "address",
];

fn static_html(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let source = String::from_utf8(bytes).map_err(|_| crate::i18n::tr("HTML must contain UTF-8 text").to_owned())?;
    let html = Html::parse(source.trim_start_matches('\u{feff}'));
    // A page is laid out by its classes, ids and inline styles, and a table by the cells that
    // span several. None of these can run code or reach the network: the response's policy
    // allows no script and nothing from outside the document.
    let kept = MESSAGE_ELEMENTS.iter().chain(&PAGE_ELEMENTS).copied();
    let attributes = kept.clone().map(|element| {
        let properties: &[&'static str] = match element {
            "td" | "th" => &["class", "id", "style", "colspan", "rowspan"],
            "col" | "colgroup" => &["class", "id", "style", "span"],
            _ => &["class", "id", "style"],
        };
        PropertiesNames { parent: element, properties }
    });
    let classes = kept.map(|element| PropertiesNames { parent: element, properties: &["*"] });
    html.sanitize_with(
        &SanitizerConfig::strict()
            .allow_elements(PAGE_ELEMENTS, ListBehavior::Add)
            .allow_attributes(attributes, ListBehavior::Add)
            .allow_classes(classes, ListBehavior::Add)
            // Dropped with what they hold. A title names the tab of a browser and is no part
            // of the page; a template is material for a script.
            .remove_elements(["script", "iframe", "object", "embed", "svg", "math", "title", "template"])
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
         table{{border-collapse:collapse}}td,th{{border:1px solid rgba(128,128,128,.55);padding:.35rem .55rem}}\
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
            .map_err(|_| crate::i18n::tr("Close an open attachment preview, then try again.").to_owned())?;
        let document = match kind {
            DownloadKind::Html if bytes.len() as u64 <= HTML_LIMIT => {
                PreviewDocument::Html(static_html(bytes)?)
            }
            DownloadKind::Pdf if bytes.len() as u64 <= PDF_LIMIT && bytes.starts_with(b"%PDF-") => {
                PreviewDocument::Pdf { bytes, texts: reader_texts() }
            }
            _ => return Err(crate::i18n::tr("This file cannot be previewed.").into()),
        };
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        let path = format!("/{}", uuid::Uuid::new_v4());
        let host = address.to_string();
        let entry = match document {
            PreviewDocument::Html(_) => "",
            PreviewDocument::Pdf { .. } => "/",
        };
        let url = Url::parse(&format!("http://{host}{path}{entry}")).map_err(|e| e.to_string())?;
        let runtime = crate::sliding_sync::backend_runtime();
        let listener = {
            let _guard = runtime.enter();
            tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?
        };
        let task = crate::account_session::spawn(async move {
            let server = Arc::new(PreviewServer {
                host,
                path,
                document,
                epoch,
            });
            let mut requests = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    result = listener.accept(), if requests.len() < 8 => {
                        let Ok((stream, _)) = result else { break };
                        let server = Arc::clone(&server);
                        requests.spawn(async move {
                            let _ = tokio::time::timeout(Duration::from_secs(8), server.serve(stream)).await;
                        });
                    }
                    _ = requests.join_next(), if !requests.is_empty() => {}
                }
            }
        });
        Ok(Self(Arc::new(Preview {
            url,
            task,
            _slot: slot,
        })))
    }
}

include!(concat!(env!("OUT_DIR"), "/pdf-preview-assets.rs"));

const STATIC_HTML_POLICY: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; form-action 'none'; base-uri 'none'";
const PDF_READER_POLICY: &str = concat!(
    "default-src 'none'; script-src 'self'; worker-src 'self'; connect-src 'self'; ",
    "style-src 'unsafe-inline'; img-src data: blob:; font-src 'self' blob: data:; ",
    "form-action 'none'; base-uri 'none'",
);

enum PreviewDocument {
    Html(Vec<u8>),
    /// A PDF, and what its reader says in the person's language.
    Pdf { bytes: Vec<u8>, texts: Vec<u8> },
}

/// What the PDF reader's page says, by the names its script knows them by. The page itself
/// holds the English, for a reader these never reach.
fn reader_texts() -> Vec<u8> {
    use crate::i18n::{self, Language, language};
    let texts = serde_json::json!({
        "lang": match language() { Language::Chinese => "zh-CN", Language::English => "en" },
        "loading": i18n::tr("Loading PDF…"),
        "failed": i18n::tr("This PDF cannot be displayed."),
        "page": i18n::tr("Page {n}"),
        "zoomIn": i18n::tr("Zoom in"),
        "zoomOut": i18n::tr("Zoom out"),
        "fit": i18n::tr("Fit width"),
        "find": i18n::tr("Find in document"),
        "previous": i18n::tr("Previous match"),
        "next": i18n::tr("Next match"),
        "none": i18n::tr("No matches"),
    });
    serde_json::to_vec(&texts).unwrap_or_default()
}

struct PreviewServer {
    host: String,
    path: String,
    document: PreviewDocument,
    epoch: u64,
}

impl PreviewServer {
    fn response(&self, path: &[u8]) -> Option<(&str, &str, &[u8])> {
        let resource = path.strip_prefix(self.path.as_bytes())?;
        match (&self.document, resource) {
            (PreviewDocument::Html(bytes), b"") => {
                Some(("text/html; charset=utf-8", STATIC_HTML_POLICY, bytes))
            }
            (PreviewDocument::Pdf { bytes, .. }, b"/document.pdf") => {
                Some(("application/octet-stream", "default-src 'none'", bytes))
            }
            (PreviewDocument::Pdf { texts, .. }, b"/strings.json") => {
                Some(("application/json; charset=utf-8", "default-src 'none'", texts))
            }
            (PreviewDocument::Pdf { .. }, resource) => {
                let (mime, bytes) = pdf_preview_asset(resource)?;
                Some((mime, PDF_READER_POLICY, bytes))
            }
            _ => None,
        }
    }

    async fn serve(&self, mut stream: tokio::net::TcpStream) -> std::io::Result<()> {
        let mut request = [0; 8192];
        let mut read = 0;
        let header = loop {
            let count = stream.read(&mut request[read..]).await?;
            if count == 0 {
                return Ok(());
            }
            let start = read.saturating_sub(3);
            read += count;
            if let Some(end) = request[start..read]
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
            {
                break &request[..start + end];
            }
        };
        let mut lines = header.split(|b| *b == b'\n');
        let mut words = lines
            .next()
            .unwrap_or_default()
            .split(|b| b.is_ascii_whitespace());
        let method = words.next();
        let path = words.next().unwrap_or_default();
        let host_matches = lines.any(|line| {
            line.len() > 5
                && line[..5].eq_ignore_ascii_case(b"host:")
                && line[5..].trim_ascii() == self.host.as_bytes()
        });
        let authorized = matches!(method, Some(b"GET" | b"HEAD"))
            && host_matches
            && self.epoch == crate::account_session::epoch();
        if let Some((mime, policy, bytes)) = authorized.then(|| self.response(path)).flatten() {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\
                 Content-Disposition: inline\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\n\
                 X-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\n\
                 X-Frame-Options: DENY\r\nContent-Security-Policy: {policy}\r\nConnection: close\r\n\r\n",
                bytes.len(),
            );
            stream.write_all(response.as_bytes()).await?;
            if method == Some(b"GET") {
                stream.write_all(bytes).await?;
            }
        } else {
            stream
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
        }
        stream.shutdown().await
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
    fn document_preview_keeps_the_look_a_page_gives_itself() {
        let page = br#"<!doctype html><html><head><title>Tab name</title>
            <style>.total{font-weight:bold}#sum{color:navy}</style></head>
            <body><nav class="toc">Contents</nav>
            <p class="total lead" id="sum" style="margin:0" onclick="alert(1)">Sum</p>
            <table><tr><td colspan="2" rowspan="3" class="wide">Merged</td></tr></table>
            <code class="language-rust note">let</code><template>Unused</template></body></html>"#;
        let html = String::from_utf8(static_html(page.to_vec()).unwrap()).unwrap();
        for kept in [
            ".total{font-weight:bold}#sum{color:navy}", r#"<nav class="toc">Contents</nav>"#,
            r#"class="total lead""#, r#"id="sum""#, r#"style="margin:0""#, r#"colspan="2""#,
            r#"rowspan="3""#, r#"class="wide""#, r#"class="language-rust note""#, ">Sum</p>",
        ] {
            assert!(html.contains(kept), "{kept} is missing in {html}");
        }
        // What only a browser's tab or a script would use is no part of the page.
        for dropped in ["Tab name", "Unused", "onclick", "alert"] {
            assert!(!html.contains(dropped), "{dropped} is left in {html}");
        }
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
        let viewer = String::from_utf8(request(&preview, url.path(), "GET")).unwrap();
        assert!(viewer.contains("Content-Type: text/html"));
        assert!(viewer.contains("worker-src 'self'"));
        assert!(viewer.contains("src=\"reader.mjs\""));
        // The reader finds text and changes its size; its words come from the app.
        assert!(viewer.contains("id=\"find\"") && viewer.contains("id=\"fit\""));
        let texts = request(&preview, url.join("strings.json").unwrap().path(), "GET");
        assert!(texts.starts_with(b"HTTP/1.1 200 OK\r\nContent-Type: application/json"));
        let body = texts.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        let texts: serde_json::Value = serde_json::from_slice(&texts[body..]).unwrap();
        for name in ["lang", "loading", "failed", "page", "zoomIn", "zoomOut", "fit", "find", "previous", "next", "none"] {
            assert!(texts[name].as_str().is_some_and(|text| !text.is_empty()), "{name} is missing");
        }
        assert!(texts["page"].as_str().unwrap().contains("{n}"));
        let reader = String::from_utf8(request(&preview, url.join("reader.mjs").unwrap().path(), "GET")).unwrap();
        for name in ["zoomIn", "zoomOut", "previous", "none"] {
            assert!(reader.contains(&format!("text.{name}")), "the reader does not use {name}");
        }
        let path = url.join("document.pdf").unwrap();
        let response = request(&preview, path.path(), "GET");
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream"));
        assert!(response.ends_with(pdf));
        assert!(!request(&preview, path.path(), "HEAD").ends_with(pdf));
        let worker = url.join("pdf.worker.min.mjs").unwrap();
        assert!(request(&preview, worker.path(), "HEAD").starts_with(b"HTTP/1.1 200"));
        let unknown = url.join("private.txt").unwrap();
        assert!(request(&preview, unknown.path(), "GET").starts_with(b"HTTP/1.1 404"));
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
