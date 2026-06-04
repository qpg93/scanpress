use anyhow::{Context, Result};
use mupdf::{Colorspace, Document, Matrix};

pub(crate) struct EncodedJpeg {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub page_width_pt: f32,
    pub page_height_pt: f32,
}

pub(crate) fn render_page_to_jpeg(
    document: &Document,
    page_index: i32,
    dpi: u32,
    quality: u8,
    grayscale: bool,
) -> Result<EncodedJpeg> {
    let page = document
        .load_page(page_index)
        .with_context(|| format!("Failed to load page {}", page_index + 1))?;

    let bounds = page
        .bounds()
        .with_context(|| format!("Failed to read page {} dimensions", page_index + 1))?;
    let page_width_pt = bounds.x1 - bounds.x0;
    let page_height_pt = bounds.y1 - bounds.y0;

    let scale = dpi as f32 / 72.0;

    let pixmap = page
        .to_pixmap(
            &Matrix::new_scale(scale, scale),
            &Colorspace::device_rgb(),
            false,
            true,
        )
        .with_context(|| format!("Failed to render page {}", page_index + 1))?;

    let w = pixmap.width() as u32;
    let h = pixmap.height() as u32;
    let samples = pixmap.samples();

    let mut buf = ::image::ImageBuffer::<::image::Rgb<u8>, _>::from_raw(w, h, samples.to_vec())
        .context("Failed to create RGB image buffer")?;

    if grayscale {
        for pixel in buf.pixels_mut() {
            let y = ((299 * pixel[0] as u32 + 587 * pixel[1] as u32 + 114 * pixel[2] as u32 + 500)
                / 1000) as u8;
            pixel[0] = y;
            pixel[1] = y;
            pixel[2] = y;
        }
    }

    let mut bytes = Vec::new();
    ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode_image(&::image::DynamicImage::ImageRgb8(buf))?;

    Ok(EncodedJpeg {
        bytes,
        width: w,
        height: h,
        page_width_pt,
        page_height_pt,
    })
}
