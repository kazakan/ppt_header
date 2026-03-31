
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
