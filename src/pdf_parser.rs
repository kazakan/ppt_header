use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::SlideData;

#[derive(Debug, Deserialize)]
struct PdfPageTitle {
    page: usize,
    title: String,
}

#[wasm_bindgen(inline_js = "
export async function parsePdfTitlesFromBytes(bytes) {
  if (!globalThis.pdfjsLib) {
    throw new Error('PDF.js is not loaded.');
  }

  const loadingTask = globalThis.pdfjsLib.getDocument({ data: bytes });
  const pdf = await loadingTask.promise;

  const bookmarkTitles = new Map();

  async function destinationToPage(dest) {
    if (!dest) {
      return null;
    }

    let resolved = dest;
    if (typeof dest === 'string') {
      resolved = await pdf.getDestination(dest);
    }
    if (!resolved || !resolved[0]) {
      return null;
    }

    const pageRef = resolved[0];
    const pageIndex = await pdf.getPageIndex(pageRef);
    return pageIndex + 1;
  }

  async function walkOutline(items) {
    if (!Array.isArray(items)) {
      return;
    }

    for (const item of items) {
      const title = (item.title || '').trim();
      if (title) {
        const page = await destinationToPage(item.dest);
        if (page && !bookmarkTitles.has(page)) {
          bookmarkTitles.set(page, title);
        }
      }
      if (item.items && item.items.length > 0) {
        await walkOutline(item.items);
      }
    }
  }

  const outline = await pdf.getOutline();
  await walkOutline(outline || []);

  const result = [];
  for (let pageNumber = 1; pageNumber <= pdf.numPages; pageNumber += 1) {
    let title = bookmarkTitles.get(pageNumber);

    if (!title) {
      const page = await pdf.getPage(pageNumber);
      const textContent = await page.getTextContent();

      for (const item of textContent.items) {
        const text = (item.str || '').trim();
        if (text) {
          title = text;
          break;
        }
      }
    }

    if (!title) {
      title = `Page ${pageNumber}`;
    }

    result.push({ page: pageNumber, title });
  }

  return JSON.stringify(result);
}
")]
extern "C" {
    #[wasm_bindgen(catch, js_name = parsePdfTitlesFromBytes)]
    async fn parse_pdf_titles_from_bytes(bytes: js_sys::Uint8Array)
        -> Result<JsValue, JsValue>;
}

pub async fn parse_pdf(data: &[u8]) -> Result<Vec<SlideData>, String> {
    let bytes = js_sys::Uint8Array::from(data);
    let value = parse_pdf_titles_from_bytes(bytes)
        .await
        .map_err(js_error_to_string)?;

    let json = value
        .as_string()
        .ok_or_else(|| "PDF parser returned an unexpected result".to_string())?;

    let pages: Vec<PdfPageTitle> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to decode PDF extraction output: {e}"))?;

    Ok(pages
        .into_iter()
        .map(|p| SlideData {
            page: p.page,
            title: p.title,
            selected: true,
        })
        .collect())
}

fn js_error_to_string(err: JsValue) -> String {
    err.as_string()
        .or_else(|| js_sys::JSON::stringify(&err).ok().and_then(|v| v.as_string()))
        .unwrap_or_else(|| "Unknown JavaScript error".to_string())
}