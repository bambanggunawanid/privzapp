// Builds a small, valid two-page PDF fixture in memory (with a correct
// xref table) so tests don't depend on binary files in the repo. Page 1
// carries real text ("Hello PrivZapp") for the text-layer/retype tests.

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
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\n",
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
