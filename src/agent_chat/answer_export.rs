//! Show a snapshot of the complete Matrix answer as a document, independent of folding/reveal.
use std::sync::Arc;

use ruma::OwnedUserId;

use crate::shared::{
    attachment_download::{DownloadKind, markdown_source, save_loaded_attachment},
    document_preview::DocumentPreview,
    popup_list::{enqueue_popup_notification, PopupKind},
    web_browser::{SavedCopy, WebBrowserAction},
};

/// The most of an answer's first line that names its tab.
const TITLE_CHARS: usize = 40;

/// What a reader tab is called for an answer: its first line that has text, without the marks
/// Markdown puts around a heading, a quote, a list item or an emphasis.
fn title(body: &str) -> String {
    let line = body
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(['#', '>', '-', '*', '+', ' '])
                .trim_matches(['*', '_', '`', ' '])
        })
        .find(|line| !line.is_empty());
    match line {
        Some(line) if line.chars().count() > TITLE_CHARS => {
            let start: String = line.chars().take(TITLE_CHARS).collect();
            format!("{}…", start.trim_end())
        }
        Some(line) => line.to_owned(),
        None => crate::i18n::tr("Agent answer").to_owned(),
    }
}

/// Opens the answer in the reader's tabs: as the Markdown it is, or as the PDF made of it.
/// The tab can save what it shows as a file. Where this computer can show no PDF, the PDF is
/// saved at once.
pub fn open(body: String, pdf: bool) {
    let account = crate::sliding_sync::current_user_id();
    let epoch = crate::account_session::epoch();
    // Font discovery/layout and PDF encoding must never block chat drawing or sync.
    std::thread::spawn(move || {
        let title = title(&body);
        let filename = format!(
            "Hagency-answer-{}.{}",
            chrono::Local::now().format("%Y%m%d-%H%M%S"),
            if pdf { "pdf" } else { "md" }
        );
        let opened = if pdf {
            pdf_tab(&body, title, filename, account, epoch)
                .map_err(|error| format!("{}: {error}", crate::i18n::tr("Could not export PDF")))
        } else {
            let bytes = rinx_answer_export::markdown(&body);
            markdown_source(bytes.clone()).map(|source| {
                Some(WebBrowserAction::OpenMarkdown {
                    title,
                    source,
                    account,
                    attachment: None,
                    save: Some(SavedCopy { filename, bytes: bytes.into() }),
                })
            })
        };
        match opened {
            Ok(Some(action)) => makepad_widgets::Cx::post_action(action),
            Ok(None) => {}
            Err(error) => enqueue_popup_notification(error, PopupKind::Error, None),
        }
    });
}

fn pdf_tab(
    body: &str,
    title: String,
    filename: String,
    account: Option<OwnedUserId>,
    epoch: u64,
) -> Result<Option<WebBrowserAction>, String> {
    let bytes: Arc<[u8]> = rinx_answer_export::pdf(body)?.into();
    if !DownloadKind::Pdf.can_preview() {
        save_loaded_attachment(filename, bytes);
        return Ok(None);
    }
    let preview = DocumentPreview::new(DownloadKind::Pdf, bytes.to_vec(), epoch)?;
    Ok(Some(WebBrowserAction::OpenAttachment {
        title,
        preview,
        account,
        epoch,
        save: Some(SavedCopy { filename, bytes }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answers_tab_is_named_by_its_first_line_of_text() {
        assert_eq!(title("\n\n## 周报 *摘要*\n正文"), "周报 *摘要");
        assert_eq!(title("> **Note** this"), "Note** this");
        assert_eq!(title("- `cargo test`\n- more"), "cargo test");
        assert_eq!(title("Plain first line\nSecond"), "Plain first line");
        let long = "字".repeat(60);
        let named = title(&long);
        assert_eq!(named.chars().count(), TITLE_CHARS + 1);
        assert!(named.ends_with('…'));
        // Nothing but marks and space: the tab still has a name.
        assert_eq!(title("  \n***\n# \n"), crate::i18n::tr("Agent answer"));
        assert_eq!(title(""), crate::i18n::tr("Agent answer"));
    }
}
