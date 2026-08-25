// Builds a small, valid two-page PDF fixture in memory (with a correct
// xref table) so tests don't depend on binary files in the repo. Page 1
// carries real text ("Hello PrivZapp") for the text-layer/retype tests.

// Standard Helvetica AFM advance widths for character codes 32..=126.
function helveticaWidths() {
  return [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333,
    278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278,
    584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278,
    500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944,
    667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556,
    278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500,
    278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
  ];
}

function contentStream(text) {
  const s = `BT /F1 24 Tf 72 700 Td (${text}) Tj ET`;
  return `<< /Length ${s.length} >>\nstream\n${s}\nendstream\n`;
}

export function samplePdf() {
  const bodies = [
    "<< /Type /Catalog /Pages 2 0 R >>\n",
    "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\n",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>\n",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>\n",
    // Widths + a standard encoding make the font measurable and
    // ASCII-safe — the precondition for the editor's font-preserving
    // text edits (and for glyph-precise redaction geometry). These are
    // the real Helvetica AFM widths (codes 32..126) so canvas-metric
    // heuristics (bold detection) agree with the rendered layout.
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /FirstChar 32 /LastChar 126 " +
      `/Encoding /WinAnsiEncoding /Widths [${helveticaWidths().join(" ")}] >>\n`,
    contentStream("Hello PrivZapp"),
    contentStream("Second page"),
  ];
  let pdf = "%PDF-1.4\n";
  const offsets = [];
  bodies.forEach((body, i) => {
    offsets.push(pdf.length);
    pdf += `${i + 1} 0 obj\n${body}endobj\n`;
  });
  const xref = pdf.length;
  pdf += `xref\n0 ${bodies.length + 1}\n0000000000 65535 f \n`;
  for (const off of offsets) {
    pdf += `${String(off).padStart(10, "0")} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${bodies.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(pdf, "latin1");
}
