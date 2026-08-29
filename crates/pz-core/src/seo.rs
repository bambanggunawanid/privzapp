//! Per-tool SEO copy: search-facing titles, meta descriptions and FAQs.
//!
//! This is the single source the app (hydrated pages) and the build-time
//! prerenderer (`tools/seo-gen`) both render, so crawlers and users see the
//! same content. A test at the bottom keeps it in lockstep with `TOOLS`.
//!
//! Copy rules: title ≤ 65 chars with the primary keyword first;
//! description 80–165 chars, states the job + the privacy differentiator;
//! every FAQ answers the "is it safe / is it free" questions searchers ask.

pub struct ToolSeo {
    pub slug: &'static str,
    /// `<title>` for the tool page.
    pub title: &'static str,
    /// Meta description; also shown on-page as the intro paragraph.
    pub description: &'static str,
    /// Question/answer pairs rendered on-page and as FAQPage JSON-LD.
    pub faq: &'static [(&'static str, &'static str)],
}

pub const TOOL_SEO: &[ToolSeo] = &[
    ToolSeo {
        slug: "edit-pdf",
        title: "Edit PDF — Sign, Draw & Add Images Free | PrivZapp",
        description: "Edit a PDF free in your browser: sign by hand, draw, stamp images, rotate, add page numbers, watermark, crop and reorganize pages. No uploads.",
        faq: &[
            (
                "How do I sign a PDF with my own handwriting?",
                "Open the PDF in the editor, pick the pen, and draw your signature right on the page with your mouse, finger or stylus. Click Apply and download the signed PDF.",
            ),
            (
                "Can I add an image or stamp to a PDF?",
                "Yes — choose Add image, pick the picture, then drag a rectangle on the page where it should go. It's embedded into the PDF when you apply.",
            ),
            (
                "Is signing here safe for contracts?",
                "Safer than upload-based editors: the document, your signature and everything you draw stay on your device. There is no server copy to leak.",
            ),
            (
                "Which editing tools are included?",
                "Pen and signature drawing, typed text boxes, image stamps, rotate, add page numbers, text watermark, crop margins, reorganize/delete/duplicate pages, append another PDF, and export plain, compressed or AES-256 password-protected — each applies instantly and you keep editing.",
            ),
        ],
    },
    ToolSeo {
        slug: "merge-pdf",
        title: "Merge PDF — Combine PDF Files Free & Private | PrivZapp",
        description: "Merge and combine PDF files into one document, free and in your browser. Your PDFs are never uploaded — everything runs on your device.",
        faq: &[
            (
                "How do I combine PDF files into one?",
                "Choose or drag in two or more PDFs, arrange them in the order you want, and click Merge PDF. You instantly download a single combined PDF.",
            ),
            (
                "Is it safe to merge confidential PDFs here?",
                "Yes. PrivZapp processes files with WebAssembly inside your browser. Your PDFs never leave your device and there is no server that could store them.",
            ),
            (
                "Is there a file size or page limit?",
                "No artificial limits and no premium tier — merging is free forever. The only practical limit is your device's memory.",
            ),
        ],
    },
    ToolSeo {
        slug: "split-pdf",
        title: "Split PDF — Extract Pages from PDF Free | PrivZapp",
        description: "Split a PDF online free: extract a page range like 1-3,5 or burst every page into its own file — privately, with no upload to any server.",
        faq: &[
            (
                "How do I extract specific pages from a PDF?",
                "Pick your PDF, type a page range such as 1-3,5, and click Split PDF. Leave the range empty to save every page as a separate PDF.",
            ),
            (
                "Are my documents uploaded when I split them?",
                "No. The split happens entirely in your browser via WebAssembly; the PDF never leaves your device.",
            ),
            (
                "Is splitting PDFs free?",
                "Yes — every PrivZapp tool is free forever, funded by donations instead of your data.",
            ),
        ],
    },
    ToolSeo {
        slug: "rotate-pdf",
        title: "Rotate PDF Pages Online Free — No Upload | PrivZapp",
        description: "Rotate PDF pages by 90, 180 or 270 degrees and save the result instantly. Free, no watermarks, and your file never leaves your device.",
        faq: &[
            (
                "How do I rotate a PDF and save it?",
                "Choose the PDF, pick 90°, 180° or 270°, click Rotate PDF and download the rotated copy. The original file is untouched.",
            ),
            (
                "Does rotating reduce quality?",
                "No — rotation only changes each page's orientation flag, so text and images stay exactly as sharp as before.",
            ),
            (
                "Is my PDF uploaded to a server?",
                "Never. PrivZapp runs completely in your browser; there is nowhere for your document to go.",
            ),
        ],
    },
    ToolSeo {
        slug: "compress-pdf",
        title: "Compress PDF — Reduce PDF File Size Free | PrivZapp",
        description: "Compress PDF files to a smaller size right in your browser — free, no upload, no watermark. Great for email attachments and uploads with size limits.",
        faq: &[
            (
                "How does PDF compression work here?",
                "PrivZapp recompresses the internal streams of your PDF and removes unused objects. If the result would be bigger than the original, you get the original back — never a worse file.",
            ),
            (
                "Will my PDF lose quality?",
                "No. The compression is lossless: text stays selectable and images are not re-encoded.",
            ),
            (
                "Is it safe for contracts and private documents?",
                "Yes. The file is processed on your device and never uploaded, so nobody else can ever see it.",
            ),
        ],
    },
    ToolSeo {
        slug: "images-to-pdf",
        title: "JPG to PDF — Convert Images to PDF Free | PrivZapp",
        description: "Convert JPG, PNG, WebP and other images to a single PDF, free and private. Photos and scans become one PDF without ever being uploaded.",
        faq: &[
            (
                "How do I convert photos to one PDF?",
                "Select or drag in your images in the order you want, adjust quality if needed, and click Images to PDF. Each image becomes one page.",
            ),
            (
                "Which image formats can I turn into a PDF?",
                "JPG, PNG, WebP, GIF, BMP, TIFF and more — anything PrivZapp can decode gets embedded into the PDF.",
            ),
            (
                "Are my photos uploaded anywhere?",
                "No. Conversion happens in your browser with WebAssembly, so private photos and scanned documents stay on your device.",
            ),
        ],
    },
    ToolSeo {
        slug: "watermark-pdf",
        title: "Watermark PDF — Add Text Watermark Free | PrivZapp",
        description: "Stamp a text watermark like CONFIDENTIAL or DRAFT across every PDF page — free, instant, and fully private with no upload.",
        faq: &[
            (
                "How do I add a watermark to a PDF?",
                "Choose the PDF, type your watermark text, and click Watermark PDF. The text is stamped diagonally across the middle of every page.",
            ),
            (
                "Can anyone else see the document I watermark?",
                "No — the watermark is applied on your device. The file is never transmitted anywhere.",
            ),
            (
                "Is the watermark tool free?",
                "Yes, free forever, with no trial limits and no watermark-on-your-watermark tricks.",
            ),
        ],
    },
    ToolSeo {
        slug: "reorder-pdf",
        title: "Reorder PDF Pages Online Free — No Upload | PrivZapp",
        description: "Rearrange PDF pages into any order, duplicate pages or drop the ones you don't need — free and private, right in your browser.",
        faq: &[
            (
                "How do I change the page order of a PDF?",
                "Pick the PDF and type the new order, e.g. 3,1,2. Repeat a page number to duplicate it, or leave one out to remove it.",
            ),
            (
                "Can I duplicate a page inside a PDF?",
                "Yes — list the page twice (e.g. 1,1,2) and it appears twice in the output.",
            ),
            (
                "Is my PDF kept on a server afterwards?",
                "No. There is no server: reordering runs entirely on your device and nothing is retained.",
            ),
        ],
    },
    ToolSeo {
        slug: "page-numbers-pdf",
        title: "Add Page Numbers to PDF Online Free | PrivZapp",
        description: "Add page numbers to every PDF page free — stamped as 'page / total' at the bottom, in your browser, with nothing uploaded anywhere.",
        faq: &[
            (
                "How do I add page numbers to a PDF?",
                "Choose the PDF and click Add Page Numbers. Every page gets a centered 'page / total' label in the bottom margin.",
            ),
            (
                "Will the numbers overlap my content?",
                "They sit in the bottom margin at a modest size, below where typical layouts place content.",
            ),
            (
                "Is my document uploaded to be numbered?",
                "No — numbering runs on your device via WebAssembly; the PDF never leaves your browser.",
            ),
        ],
    },
    ToolSeo {
        slug: "crop-pdf",
        title: "Crop PDF — Trim Page Margins Online Free | PrivZapp",
        description: "Crop PDF pages by trimming margins from any side — free, instant, and private. Great for cutting whitespace before printing or reading.",
        faq: &[
            (
                "How do I crop the margins of a PDF?",
                "Enter how many points to trim from the left, top, right and bottom (72 points = 1 inch) and click Crop PDF. Every page is cropped the same way.",
            ),
            (
                "Does cropping delete page content?",
                "The content outside the new page box is hidden, not destroyed — cropping again with zero margins can't restore it in this tool, so keep your original file.",
            ),
            (
                "Is the PDF uploaded while cropping?",
                "No. The crop is applied in your browser and the file never leaves your device.",
            ),
        ],
    },
    ToolSeo {
        slug: "extract-text-pdf",
        title: "PDF to Text — Extract Text from PDF Free | PrivZapp",
        description: "Extract all text from a PDF into a plain .txt file, free and in your browser. Nothing is uploaded — ideal for confidential documents.",
        faq: &[
            (
                "How do I get the text out of a PDF?",
                "Choose the PDF and click PDF to Text. You download a .txt file with the text of every page.",
            ),
            (
                "Why did I get a 'no extractable text' error?",
                "Your PDF is probably scanned images rather than digital text. That needs OCR, which PrivZapp doesn't support yet.",
            ),
            (
                "Is this safe for contracts and private files?",
                "Yes — extraction runs entirely on your device. No server ever sees the document.",
            ),
        ],
    },
    ToolSeo {
        slug: "pdf-to-images",
        title: "PDF to JPG — Convert PDF Pages to Images | PrivZapp",
        description: "Convert PDF pages to JPG, PNG or WebP images free in your browser. Choose the resolution and page range. Your PDF is never uploaded to a server.",
        faq: &[
            (
                "How do I convert a PDF to JPG?",
                "Drop the PDF in, pick JPG as the format and a render scale, then run it. Every page comes back as its own image — one page downloads as a single file, several arrive as a .zip.",
            ),
            (
                "What resolution will the images be?",
                "1x renders at the PDF's natural 72 DPI; 2x, 3x and 4x multiply that (2x = 144 DPI, 4x = 288 DPI). Higher scales give sharper images and bigger files — 2x suits screens, 3x-4x suits print or OCR.",
            ),
            (
                "Can I convert only some pages?",
                "Yes. Leave the page box empty to convert the whole document, or type a range like 1-3,5 to pick exactly the pages you want.",
            ),
            (
                "Is my PDF uploaded to convert it?",
                "No. The pages are rendered by your own browser and the images are packaged on your device — the file never leaves it, and the tool keeps working offline.",
            ),
        ],
    },
    ToolSeo {
        slug: "repair-pdf",
        title: "Repair PDF — Fix a Corrupt PDF Online Free | PrivZapp",
        description: "Try to repair a damaged PDF by rebuilding its structure and cross-reference table — free, on your device, with nothing uploaded.",
        faq: &[
            (
                "How does PDF repair work?",
                "PrivZapp re-parses the file leniently, recovers every readable object, renumbers them and writes a clean new PDF structure.",
            ),
            (
                "Can every broken PDF be fixed?",
                "No — if the page data itself is destroyed, no tool can recover it. Structural damage (bad offsets, truncated xref) usually can be.",
            ),
            (
                "Is my broken file uploaded for repair?",
                "No. Repair happens in your browser; the file never leaves your device.",
            ),
        ],
    },
    ToolSeo {
        slug: "protect-pdf",
        title: "Protect PDF — Password Protect a PDF Free | PrivZapp",
        description: "Password-protect a PDF with standard AES-256 encryption that opens in any PDF viewer — free, and the password never leaves your device.",
        faq: &[
            (
                "How do I put a password on a PDF?",
                "Choose the PDF, enter a password, and click Protect PDF. The output is a standard encrypted PDF that any modern viewer can open with that password.",
            ),
            (
                "How strong is the protection?",
                "AES-256 (PDF 2.0 standard security). Unlike many sites, your password and file are never transmitted — encryption happens on your device.",
            ),
            (
                "What if I forget the PDF password?",
                "There's no backdoor. Keep the password safe — without it the file stays locked.",
            ),
        ],
    },
    ToolSeo {
        slug: "unlock-pdf",
        title: "Unlock PDF — Remove PDF Password Online Free | PrivZapp",
        description: "Remove a password you know from a PDF and save an unlocked copy, free and in your browser — the file and password are never uploaded.",
        faq: &[
            (
                "How do I remove a password from a PDF?",
                "Choose the protected PDF, type its password, and click Unlock PDF. You download a copy that opens without a password.",
            ),
            (
                "Can this crack a PDF whose password I lost?",
                "No — you must know the password. PrivZapp removes protection you're authorized to remove; it doesn't break encryption.",
            ),
            (
                "Is entering my password here safe?",
                "Yes. The password is used inside your browser only and never transmitted — there is no server in the picture.",
            ),
        ],
    },
    ToolSeo {
        slug: "convert-img",
        title: "Convert Image — PNG, JPG, WebP Converter Free | PrivZapp",
        description: "Convert images between PNG, JPG, WebP, GIF, BMP, TIFF, ICO and QOI free in your browser. No upload, no account, no quality games.",
        faq: &[
            (
                "How do I convert PNG to JPG (or JPG to PNG)?",
                "Choose your images, pick the target format from the dropdown, set quality for lossy formats, and click Convert Image. Each file downloads in the new format.",
            ),
            (
                "Which formats are supported?",
                "PNG, JPG, WebP, GIF, BMP, TIFF, ICO and QOI — any of them to any other, including batch conversion of many files at once.",
            ),
            (
                "Are my pictures uploaded during conversion?",
                "Never. The converter is WebAssembly running in your browser, so photos stay on your device.",
            ),
        ],
    },
    ToolSeo {
        slug: "resize-img",
        title: "Resize Image Online Free — Exact Pixels | PrivZapp",
        description: "Resize images to exact pixel dimensions or scale them with the aspect ratio kept — free, private, in your browser, with no upload.",
        faq: &[
            (
                "How do I resize an image to specific dimensions?",
                "Enter a width and height in pixels and click Resize Image. Leave one field empty to keep the aspect ratio automatically.",
            ),
            (
                "Does resizing reduce image quality?",
                "PrivZapp uses high-quality Lanczos resampling, the same filter professional tools use, so downscaled images stay crisp.",
            ),
            (
                "Can I resize many images at once?",
                "Yes — select multiple files and they are all resized with the same settings in one click.",
            ),
        ],
    },
    ToolSeo {
        slug: "compress-img",
        title: "Compress Image — Reduce JPG & PNG Size Free | PrivZapp",
        description: "Compress JPG and PNG images to a smaller file size free, with a quality slider — processed in your browser, never uploaded anywhere.",
        faq: &[
            (
                "How do I reduce an image's file size?",
                "Choose your images, set the quality slider, and click Compress Image. If compression can't beat the original size, you get the original back — never a bigger file.",
            ),
            (
                "What's the best quality setting?",
                "80 is a great default for photos: much smaller files with virtually invisible differences. Go lower for thumbnails, higher for print.",
            ),
            (
                "Is this image compressor really private?",
                "Yes. Compression runs on your device via WebAssembly. Your photos are never sent to a server.",
            ),
        ],
    },
    ToolSeo {
        slug: "rotate-img",
        title: "Rotate Image Online Free — JPG, PNG & More | PrivZapp",
        description: "Rotate photos by 90, 180 or 270 degrees, free and in your browser. Batch-rotate many images at once; nothing is ever uploaded.",
        faq: &[
            (
                "How do I rotate a photo and save it?",
                "Choose your images, pick the angle, and click Rotate Image. Each rotated copy downloads immediately in its original format.",
            ),
            (
                "Does rotating lose quality?",
                "90/180/270° rotations remap pixels without resampling; only lossy formats like JPG re-encode, at the quality you'd expect.",
            ),
            (
                "Can I rotate many pictures at once?",
                "Yes — select any number of images and they all rotate by the same angle in one click.",
            ),
        ],
    },
    ToolSeo {
        slug: "flip-img",
        title: "Flip Image — Mirror a Photo Online Free | PrivZapp",
        description: "Mirror an image horizontally or vertically, free and private. Fix selfies and scans in your browser — photos never leave your device.",
        faq: &[
            (
                "How do I mirror a photo?",
                "Choose the image, pick horizontal (left ↔ right) or vertical (top ↕ bottom), and click Flip Image.",
            ),
            (
                "Why do selfies need flipping?",
                "Front cameras usually store a mirrored preview; flipping horizontally restores how you actually look to others.",
            ),
            (
                "Is my photo uploaded to be flipped?",
                "No — the flip happens on your device via WebAssembly and the file never leaves your browser.",
            ),
        ],
    },
    ToolSeo {
        slug: "upscale-img",
        title: "Upscale Image — Enlarge 2x or 4x Online Free | PrivZapp",
        description: "Enlarge images 2x or 4x with sharp Lanczos resampling, free and in your browser. No account, no upload, no watermark on the result.",
        faq: &[
            (
                "How do I make an image bigger without heavy blur?",
                "Choose the image, pick 2× or 4×, and click Upscale Image. PrivZapp uses Lanczos resampling — the sharpest classical enlargement filter.",
            ),
            (
                "Is this AI upscaling?",
                "No — it's high-quality classical resampling that runs instantly on your device. AI upscalers invent detail; this faithfully enlarges what's there.",
            ),
            (
                "Are my pictures uploaded to be enlarged?",
                "Never. Upscaling runs in your browser, so private photos stay private.",
            ),
        ],
    },
    ToolSeo {
        slug: "grayscale-img",
        title: "Grayscale Image — Black & White Converter Free | PrivZapp",
        description: "Convert photos to black and white free in your browser. True luminance grayscale, batch support, and nothing ever uploaded.",
        faq: &[
            (
                "How do I make a photo black and white?",
                "Choose your images and click Grayscale Image. Each one is converted using proper luminance weighting and downloads in its original format.",
            ),
            (
                "Does transparency survive the conversion?",
                "Yes — alpha channels are preserved for formats that support them, like PNG and WebP.",
            ),
            (
                "Is the converter private?",
                "Completely: the conversion runs on your device and photos never reach a server.",
            ),
        ],
    },
    ToolSeo {
        slug: "blur-img",
        title: "Blur Image Online Free — Gaussian Blur | PrivZapp",
        description: "Blur a picture with adjustable gaussian strength, free and in your browser. Soften screenshots or backgrounds without uploading anything.",
        faq: &[
            (
                "How do I blur a photo?",
                "Choose the image, set the strength slider, and click Blur Image. Higher strength means a softer, more diffuse result.",
            ),
            (
                "Can blurring hide sensitive text reliably?",
                "Strong blur makes text unreadable, but for true redaction crop the region out instead — blur can sometimes be partially reversed.",
            ),
            (
                "Is my image uploaded to be blurred?",
                "No — the blur is computed on your device; the picture never leaves your browser.",
            ),
        ],
    },
    ToolSeo {
        slug: "watermark-img",
        title: "Watermark Image — Add Text to Photos Free | PrivZapp",
        description: "Stamp semi-transparent text across your photos to protect them, free and in your browser — pictures are never uploaded anywhere.",
        faq: &[
            (
                "How do I put a watermark on a picture?",
                "Choose your images, type the watermark text, and click Watermark Image. The text is stamped semi-transparently across the middle.",
            ),
            (
                "Can I watermark many photos at once?",
                "Yes — select a whole batch and every image gets the same stamp in one click.",
            ),
            (
                "Are my photos uploaded to add the watermark?",
                "No. Watermarking runs on your device, so unpublished work never leaves your control.",
            ),
        ],
    },
    ToolSeo {
        slug: "strip-exif",
        title: "Remove EXIF Data — Strip Photo Metadata Free | PrivZapp",
        description: "Remove EXIF metadata — GPS location, camera model, timestamps — from photos before sharing. Free and private: photos never leave your device.",
        faq: &[
            (
                "Why should I remove EXIF data before sharing photos?",
                "Photos often embed the GPS coordinates of your home, your camera's serial number and exact timestamps. Stripping metadata removes that hidden information.",
            ),
            (
                "Does removing metadata change how the photo looks?",
                "The pixels are preserved; the image is re-encoded without any metadata blocks. For JPGs you control the re-encode quality with the slider.",
            ),
            (
                "Is this EXIF remover private itself?",
                "Completely — which is the point. The photo is cleaned on your device and never uploaded, unlike web services that see your photo to clean it.",
            ),
        ],
    },
    ToolSeo {
        slug: "crop-img",
        title: "Crop Image Online Free — No Upload | PrivZapp",
        description: "Crop images to an exact pixel rectangle, free and in your browser. Batch-crop multiple pictures with the same frame, with no upload.",
        faq: &[
            (
                "How do I crop an image to exact pixels?",
                "Enter the X/Y offset of the top-left corner and the width and height you want to keep, then click Crop Image.",
            ),
            (
                "Can I crop several images the same way?",
                "Yes — select multiple images and the same rectangle is applied to each, perfect for screenshots or scans with identical layouts.",
            ),
            (
                "Is the crop tool free and private?",
                "Yes and yes: free forever, and your pictures are processed on your device only.",
            ),
        ],
    },
    ToolSeo {
        slug: "favicon-pack",
        title: "Favicon Generator — PNG/JPG to ICO Pack Free | PrivZapp",
        description: "Turn any PNG or JPG into a complete favicon pack: favicon.ico, PNG sizes, apple-touch-icon and webmanifest in one ZIP — free, no upload.",
        faq: &[
            (
                "What's inside the favicon pack?",
                "A multi-size favicon.ico (16/32/48), favicon-16x16.png, favicon-32x32.png, apple-touch-icon.png (180), android-chrome 192 and 512 PNGs, a site.webmanifest, and a README with the exact HTML snippet to paste.",
            ),
            (
                "How do I add the favicons to my website?",
                "Unzip everything into your site's root folder and paste the four-line snippet from README.txt into your page's <head>. Browsers pick up favicon.ico automatically.",
            ),
            (
                "Does my logo get uploaded to generate the icons?",
                "No — every size is generated in your browser with WebAssembly. Unreleased logos stay on your device.",
            ),
        ],
    },
    ToolSeo {
        slug: "rename-batch",
        title: "Batch Rename Files Online Free | PrivZapp",
        description: "Rename many files at once with a pattern like vacation-{n} — numbering is automatic and extensions are kept. Free, private, no upload.",
        faq: &[
            (
                "How does pattern renaming work?",
                "Type a pattern such as vacation-{n}; {n} becomes 1, 2, 3… in order. Without {n}, a number is appended automatically so names stay unique.",
            ),
            (
                "Do file extensions change?",
                "No — each file keeps its original extension, only the name part changes.",
            ),
            (
                "Are my files uploaded to be renamed?",
                "No. Renaming happens instantly in your browser and the renamed copies download straight back to you.",
            ),
        ],
    },
    ToolSeo {
        slug: "zip-files",
        title: "Compress Files to ZIP Online Free — No Upload | PrivZapp",
        description: "Compress files into a ZIP archive free in your browser. Bundle documents, photos or anything else — nothing is ever uploaded to a server.",
        faq: &[
            (
                "How do I compress files into a ZIP?",
                "Select or drag in any files and click Create ZIP. You download one archive.zip containing all of them, deflate-compressed.",
            ),
            (
                "Is there a limit on files or total size?",
                "No artificial limits — it's free forever. Your device's memory is the only practical bound.",
            ),
            (
                "Is zipping files here safer than a ZIP website?",
                "Yes: typical ZIP sites upload your files to compress them. PrivZapp compresses on your device, so the contents stay yours alone.",
            ),
        ],
    },
    ToolSeo {
        slug: "unzip",
        title: "Unzip Files Online Free — Extract ZIP, No Upload | PrivZapp",
        description: "Open and extract ZIP archives free in your browser, with zip-bomb and path-traversal protection. The archive never leaves your device.",
        faq: &[
            (
                "How do I open a ZIP file without installing anything?",
                "Drop the .zip here and click Extract ZIP — every file inside appears as a download, straight from your browser.",
            ),
            (
                "Is it safe to extract a ZIP from an unknown source?",
                "PrivZapp guards against zip bombs and path-traversal names, and since nothing is executed or uploaded, extraction itself stays on your terms.",
            ),
            (
                "Are the extracted files sent anywhere?",
                "No — the archive is read on your device and the contents go straight back to you.",
            ),
        ],
    },
    ToolSeo {
        slug: "encrypt-file",
        title: "Encrypt File with Password — AES-256 Free | PrivZapp",
        description: "Password-protect any file with AES-256-GCM encryption, free and offline-capable. Keys are derived on your device; nothing is uploaded.",
        faq: &[
            (
                "How strong is the encryption?",
                "AES-256-GCM with a key derived from your password via PBKDF2-HMAC-SHA256 at 600,000 rounds — the same class of cryptography banks rely on.",
            ),
            (
                "What is a .pzv file?",
                "It's a PrivZapp vault: your file, encrypted. Decrypt it any time with the Decrypt File tool and the same password, on any device running PrivZapp.",
            ),
            (
                "What if I forget the password?",
                "The file is unrecoverable — by design. Nobody, including us, can open it without the password, because we never see the file or the password.",
            ),
        ],
    },
    ToolSeo {
        slug: "decrypt-file",
        title: "Decrypt File — Open .pzv Password Vaults | PrivZapp",
        description: "Decrypt PrivZapp .pzv vaults with your password and get the original file back — locally in your browser, with nothing uploaded.",
        faq: &[
            (
                "How do I open a .pzv file?",
                "Choose the .pzv vault, enter the password it was encrypted with, and click Decrypt File. The original file downloads immediately.",
            ),
            (
                "It says wrong password or corrupted — what now?",
                "AES-GCM verifies integrity, so either the password differs from the one used to encrypt, or the file was modified. Check the password first.",
            ),
            (
                "Does decryption happen on a server?",
                "No — the vault and password never leave your device. That's why PrivZapp encryption is safe to use at all.",
            ),
        ],
    },
];

/// SEO copy for a tool page.
pub fn seo_for(slug: &str) -> Option<&'static ToolSeo> {
    TOOL_SEO.iter().find(|s| s.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TOOLS;

    #[test]
    fn every_tool_has_seo_and_vice_versa() {
        for tool in TOOLS {
            assert!(
                seo_for(tool.slug).is_some(),
                "tool \"{}\" has no SEO entry — add one to pz_core::seo::TOOL_SEO",
                tool.slug
            );
        }
        for seo in TOOL_SEO {
            assert!(
                crate::tool_by_slug(seo.slug).is_some(),
                "SEO entry \"{}\" has no matching tool",
                seo.slug
            );
        }
    }

    #[test]
    fn copy_fits_search_snippets() {
        for seo in TOOL_SEO {
            let t = seo.title.chars().count();
            assert!((25..=65).contains(&t), "{}: title is {t} chars", seo.slug);
            let d = seo.description.chars().count();
            assert!(
                (80..=165).contains(&d),
                "{}: description is {d} chars",
                seo.slug
            );
            assert!(
                seo.faq.len() >= 2,
                "{}: needs at least 2 FAQ entries",
                seo.slug
            );
            assert!(
                seo.title.contains("PrivZapp"),
                "{}: title should carry the brand",
                seo.slug
            );
        }
    }
}
