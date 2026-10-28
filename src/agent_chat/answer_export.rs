//! Export a snapshot of the complete Matrix answer, independent of folding/reveal.
use crate::shared::popup_list::{enqueue_popup_notification, PopupKind};

pub fn save(body: String, pdf: bool) {
    // Font discovery/layout and PDF encoding must never block chat drawing or sync.
    std::thread::spawn(move || {
        let bytes = if pdf {
            rinx_answer_export::pdf(&body)
        } else {
            Ok(rinx_answer_export::markdown(&body))
        };
        match bytes {
            Ok(bytes) => {
                let extension = if pdf { "pdf" } else { "md" };
                let filename = format!(
                    "Hagency-answer-{}.{}",
                    chrono::Local::now().format("%Y%m%d-%H%M%S"),
                    extension
                );
                crate::shared::attachment_download::save_loaded_attachment(filename, bytes.into());
            }
            Err(error) => enqueue_popup_notification(
                format!("{}: {error}", crate::i18n::tr("Could not export PDF")),
                PopupKind::Error,
                None,
            ),
        }
    });
}
