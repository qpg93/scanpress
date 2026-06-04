use crate::compress::renderer;
use crate::compress::CompressionProgressTracker;
use anyhow::{Context, Result};
use mupdf::Document;
use printpdf::{
    ColorBits, ColorSpace, Image, ImageFilter, ImageTransform, ImageXObject, Mm, PdfDocument,
    PdfDocumentReference, Px,
};

/// JPEG encoder writes `/ColorTransform 0` for RGB, but printpdf's DCT
/// (JPEG) filter requires `/ColorTransform 1` for proper colour decoding.
const COLOR_TRANSFORM_FIX: &[u8] = b"/ColorTransform 0";

fn points_to_mm(points: f32) -> f32 {
    if points <= 0.0 { 1.0 } else { points * 25.4 / 72.0 }
}

pub(crate) fn build_pdf_bytes(
    document: &Document,
    page_count: u32,
    dpi: u32,
    quality: u8,
    grayscale: bool,
    mut tracker: Option<&mut CompressionProgressTracker>,
) -> Result<Vec<u8>> {
    if page_count == 0 {
        anyhow::bail!("Input PDF has no pages.")
    }

    let mut output_doc: Option<PdfDocumentReference> = None;
    for page_index in 0..page_count {
        let jpeg = renderer::render_page_to_jpeg(document, page_index as i32, dpi, quality, grayscale)?;
        let page_width_mm = points_to_mm(jpeg.page_width_pt);
        let page_height_mm = points_to_mm(jpeg.page_height_pt);

        let image = Image::from(ImageXObject {
            width: Px(jpeg.width as usize),
            height: Px(jpeg.height as usize),
            color_space: ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: true,
            image_data: jpeg.bytes,
            image_filter: Some(ImageFilter::DCT),
            smask: None,
            clipping_bbox: None,
        });

        if output_doc.is_none() {
            output_doc = Some(PdfDocument::empty("Compressed PDF"));
        }

        let doc = output_doc.as_ref().context("Failed to create output PDF.")?;
        let (pdf_page, layer) = doc.add_page(Mm(page_width_mm), Mm(page_height_mm), "Layer 1");

        image.add_to_layer(
            doc.get_page(pdf_page).get_layer(layer),
            ImageTransform {
                translate_x: Some(Mm(0.0)),
                translate_y: Some(Mm(0.0)),
                rotate: None,
                scale_x: Some(jpeg.page_width_pt / jpeg.width as f32),
                scale_y: Some(jpeg.page_height_pt / jpeg.height as f32),
                dpi: Some(72.0),
            },
        );

        if let Some(t) = tracker.as_deref_mut() {
            t.advance(&format!("Rendered page {} / {}", page_index + 1, page_count));
        }
    }

    let mut pdf_bytes = output_doc
        .context("Failed to create output PDF.")?
        .save_to_bytes()
        .context("Failed to generate output PDF.")?;

    while let Some(pos) = pdf_bytes.windows(COLOR_TRANSFORM_FIX.len()).position(|w| w == COLOR_TRANSFORM_FIX) {
        pdf_bytes[pos + COLOR_TRANSFORM_FIX.len() - 1] = b'1';
    }

    Ok(pdf_bytes)
}
