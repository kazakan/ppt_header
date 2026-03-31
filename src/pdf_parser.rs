use std::collections::HashMap;

use lopdf::Document;

use crate::SlideData;

pub async fn parse_pdf(data: &[u8]) -> Result<Vec<SlideData>, String> {
    let doc = Document::load_mem(data)
        .map_err(|e| format!("Failed to read PDF: {e}"))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        return Ok(Vec::new());
    }

    let bookmark_titles = extract_bookmark_titles(&doc);
    let mut output = Vec::with_capacity(pages.len());

    for (idx, (page_number, _)) in pages.iter().enumerate() {
        let page_index = *page_number as usize;
        let title = bookmark_titles
            .get(&page_index)
            .cloned()
            .or_else(|| extract_page_title(&doc, *page_number))
            .unwrap_or_else(|| format!("Page {}", idx + 1));

        output.push(SlideData {
            page: idx + 1,
            title,
            selected: true,
        });
    }

    Ok(output)
}

fn extract_bookmark_titles(doc: &Document) -> HashMap<usize, String> {
    let mut by_page = HashMap::new();

    if let Ok(toc) = doc.get_toc() {
        for item in toc.toc {
            let title = item.title.trim();
            if !title.is_empty() {
                by_page.entry(item.page).or_insert_with(|| title.to_string());
            }
        }
    }

    by_page
}

fn extract_page_title(doc: &Document, page_number: u32) -> Option<String> {
    let text = doc.extract_text(&[page_number]).ok()?;

    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}
