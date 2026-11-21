The renderer assets are from Mozilla's Apache 2.0 licensed PDF.js npm distribution `pdfjs-dist@5.7.284`. Package integrity is `sha512-h4EdYQczmGhbOlqc3PPZwxevn7ApdWPbovAuWXOB/DjIyigSnwfy2oze7c6mRcSr9XgLp3eN3EeL4DyySTPMFw==`.

`pdf.min.mjs` and `pdf.worker.min.mjs` are the upstream legacy build. `cmaps`, `standard_fonts`, and the JavaScript image decoders in `decoders` are copied from the same distribution with their licenses. `reader.mjs` is Hagency's display-only entry point. The local preview serves these resources together with the PDF. It uses no network service or stock PDF.js viewer.

Source: https://github.com/mozilla/pdf.js/releases/tag/v5.7.284
