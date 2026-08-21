use std::io::{Cursor, Write};

use flate2::write::ZlibEncoder;
use flate2::Compression as ZlibCompression;
use png::{BitDepth, ColorType, Compression, Decoder, Encoder, FilterType};

use crate::provenance::AtlasRenderProvenanceV1;
use crate::request::{AtlasFormat, AtlasRenderRequest};
use crate::{AtlasError, AtlasPhase, AtlasProgress, CODE_ENCODER_FAILED};

pub fn encode_png(
    request: &AtlasRenderRequest,
    rgba: &[u8],
    provenance: &AtlasRenderProvenanceV1,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    progress.report(AtlasPhase::Encoding, 0, 1)?;
    progress.check_cancelled()?;
    let expected = (request.width_px as usize)
        .checked_mul(request.height_px as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AtlasError::limit("PNG buffer size overflowed"))?;
    if rgba.len() != expected {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "RGBA length does not match the requested dimensions",
        ));
    }
    let mut png = Vec::new();
    {
        let mut encoder = Encoder::new(&mut png, request.width_px, request.height_px);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Fast);
        encoder.set_filter(FilterType::NoFilter);
        encoder
            .add_text_chunk("Software".into(), "daena-atlas/1".into())
            .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
        encoder
            .add_text_chunk("Comment".into(), provenance.compact_json()?)
            .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
        let mut writer = encoder
            .write_header()
            .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
        writer
            .finish()
            .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    }
    progress.report(AtlasPhase::Encoding, 1, 1)?;
    Ok(png)
}

pub fn encode_artifact(
    request: &AtlasRenderRequest,
    rgba: &[u8],
    png: &[u8],
    provenance: &AtlasRenderProvenanceV1,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    match request.format {
        AtlasFormat::Png => Ok(png.to_vec()),
        AtlasFormat::Svg => encode_svg(request, png, provenance, progress),
        AtlasFormat::Pdf => encode_pdf(request, rgba, provenance, progress),
    }
}

pub fn encode_svg(
    request: &AtlasRenderRequest,
    png: &[u8],
    provenance: &AtlasRenderProvenanceV1,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    progress.report(AtlasPhase::Encoding, 0, 1)?;
    progress.check_cancelled()?;
    let desc = xml_escape(&provenance.compact_json()?);
    let image = base64_encode(png);
    let svg = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
  <title>daena-atlas</title>\n\
  <desc>{desc}</desc>\n\
  <image width=\"{w}\" height=\"{h}\" href=\"data:image/png;base64,{image}\"/>\n\
</svg>\n",
        w = request.width_px,
        h = request.height_px,
    );
    if svg_has_forbidden_content(&svg) {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "SVG encoder produced forbidden active or remote content",
        ));
    }
    progress.report(AtlasPhase::Encoding, 1, 1)?;
    Ok(svg.into_bytes())
}

#[must_use]
pub fn page_points(px: u32, dpi: u32) -> u32 {
    let dpi = dpi.max(1);
    ((u64::from(px) * 72 + u64::from(dpi) / 2) / u64::from(dpi)) as u32
}

pub fn encode_pdf(
    request: &AtlasRenderRequest,
    rgba: &[u8],
    provenance: &AtlasRenderProvenanceV1,
    progress: &mut dyn AtlasProgress,
) -> Result<Vec<u8>, AtlasError> {
    progress.report(AtlasPhase::Encoding, 0, 1)?;
    progress.check_cancelled()?;
    let width = request.width_px;
    let height = request.height_px;
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "RGBA length does not match the requested dimensions",
        ));
    }
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in rgba.as_chunks::<4>().0 {
        rgb.extend_from_slice(&pixel[..3]);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), ZlibCompression::fast());
    encoder
        .write_all(&rgb)
        .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    let stream = encoder
        .finish()
        .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    let page_w = page_points(width, request.dpi);
    let page_h = page_points(height, request.dpi);
    let info = pdf_escape(&provenance.compact_json()?);
    let image_dict = format!(
        "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>",
        stream.len()
    );
    let content = format!("{page_w} 0 0 {page_h} 0 0 cm /Im0 Do");
    let mut pdf = b"%PDF-1.4\n%".to_vec();
    pdf.extend_from_slice(&[0xE2, 0xE3, 0xCF, 0xD3, b'\n']);
    let mut offsets = Vec::new();
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_w} {page_h}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"4 0 obj\n");
    pdf.extend_from_slice(image_dict.as_bytes());
    pdf.extend_from_slice(b"\nstream\n");
    pdf.extend_from_slice(&stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    let content_obj = format!(
        "5 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len()
    );
    pdf.extend_from_slice(content_obj.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!("6 0 obj\n<< /Producer (daena-atlas/4) /Subject ({info}) >>\nendobj\n").as_bytes(),
    );
    let xref_at = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 7 /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    let text = String::from_utf8_lossy(&pdf);
    if text.contains("/JS")
        || text.contains("/JavaScript")
        || text.contains("/URI")
        || text.contains("http://")
    {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "PDF encoder produced forbidden active or remote content",
        ));
    }
    progress.report(AtlasPhase::Encoding, 1, 1)?;
    Ok(pdf)
}

pub fn parse_pdf_media_box(bytes: &[u8]) -> Result<[u32; 4], AtlasError> {
    let text = String::from_utf8_lossy(bytes);
    let start = text
        .find("/MediaBox")
        .ok_or_else(|| AtlasError::new(CODE_ENCODER_FAILED, "PDF MediaBox is missing"))?;
    let slice = &text[start..];
    let open = slice
        .find('[')
        .ok_or_else(|| AtlasError::new(CODE_ENCODER_FAILED, "PDF MediaBox is malformed"))?;
    let close = slice
        .find(']')
        .ok_or_else(|| AtlasError::new(CODE_ENCODER_FAILED, "PDF MediaBox is malformed"))?;
    let nums = slice[open + 1..close]
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    if nums.len() != 4 {
        return Err(AtlasError::new(
            CODE_ENCODER_FAILED,
            "PDF MediaBox must have four numbers",
        ));
    }
    Ok([nums[0], nums[1], nums[2], nums[3]])
}

fn svg_has_forbidden_content(svg: &str) -> bool {
    let lower = svg.to_ascii_lowercase();
    lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("onload=")
        || svg.contains("href=\"http://")
        || svg.contains("href=\"https://")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn pdf_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut index = 0;
    while index + 3 <= data.len() {
        let n = (u32::from(data[index]) << 16)
            | (u32::from(data[index + 1]) << 8)
            | u32::from(data[index + 2]);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push(TABLE[(n & 63) as usize] as char);
        index += 3;
    }
    match data.len() - index {
        1 => {
            let n = u32::from(data[index]) << 16;
            out.push(TABLE[((n >> 18) & 63) as usize] as char);
            out.push(TABLE[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = (u32::from(data[index]) << 16) | (u32::from(data[index + 1]) << 8);
            out.push(TABLE[((n >> 18) & 63) as usize] as char);
            out.push(TABLE[((n >> 12) & 63) as usize] as char);
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPng {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub comment: Option<String>,
}

pub fn decode_png(bytes: &[u8]) -> Result<DecodedPng, AtlasError> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder
        .read_info()
        .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    let (width, height, comment) = {
        let info = reader.info();
        (
            info.width,
            info.height,
            info.uncompressed_latin1_text
                .iter()
                .find(|chunk| chunk.keyword == "Comment")
                .map(|chunk| chunk.text.clone()),
        )
    };
    let mut rgba = vec![0_u8; reader.output_buffer_size()];
    reader
        .next_frame(&mut rgba)
        .map_err(|error| AtlasError::new(CODE_ENCODER_FAILED, error.to_string()))?;
    Ok(DecodedPng {
        width,
        height,
        rgba,
        comment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::AtlasRenderRequest;

    #[test]
    fn png_round_trip_preserves_pixels_and_provenance() {
        let request = AtlasRenderRequest::spike_png(4, 2).unwrap();
        let rgba = [10_u8, 20, 30, 255, 40, 50, 60, 255]
            .repeat(2)
            .into_iter()
            .cycle()
            .take(4 * 2 * 4)
            .collect::<Vec<_>>();
        let provenance = AtlasRenderProvenanceV1::spike(&request, b"sha256:ab", "sha256:cd");
        let encoded = encode_png(&request, &rgba, &provenance, &mut crate::NoopProgress).unwrap();
        let decoded = decode_png(&encoded).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.rgba, rgba);
        assert!(decoded.comment.unwrap().contains("daena-atlas-relief"));
        assert!(!String::from_utf8_lossy(&encoded).contains("http"));
    }

    #[test]
    fn svg_embeds_png_without_remote_or_script_content() {
        let request = {
            let mut request = AtlasRenderRequest::spike_png(4, 2).unwrap();
            request.format = AtlasFormat::Svg;
            request.normalize().unwrap()
        };
        let rgba = [10_u8, 20, 30, 255].repeat(8);
        let provenance = AtlasRenderProvenanceV1::spike(&request, b"sha256:ab", "sha256:cd");
        let png = encode_png(&request, &rgba, &provenance, &mut crate::NoopProgress).unwrap();
        let svg = encode_svg(&request, &png, &provenance, &mut crate::NoopProgress).unwrap();
        let text = String::from_utf8(svg).unwrap();
        assert!(text.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(text.contains("data:image/png;base64,"));
        assert!(!text.contains("<script"));
        assert!(!text.contains("href=\"http://"));
        assert!(text.contains("daena-atlas-relief"));
    }

    #[test]
    fn pdf_page_box_matches_dpi_and_has_no_active_content() {
        let mut request = AtlasRenderRequest::spike_png(300, 150).unwrap();
        request.format = AtlasFormat::Pdf;
        request.dpi = 300;
        let request = request.normalize().unwrap();
        let rgba = [10_u8, 20, 30, 255].repeat(300 * 150);
        let provenance = AtlasRenderProvenanceV1::spike(&request, b"sha256:ab", "sha256:cd");
        let pdf = encode_pdf(&request, &rgba, &provenance, &mut crate::NoopProgress).unwrap();
        assert_eq!(parse_pdf_media_box(&pdf).unwrap(), [0, 0, 72, 36]);
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(!text.contains("/JS"));
        assert!(!text.contains("/URI"));
        assert!(!text.contains("http://"));
    }
}
