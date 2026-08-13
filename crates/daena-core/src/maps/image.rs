use crate::error::CoreError;
use sha2::{Digest, Sha256};
use std::io::Cursor;

/// Encoded-byte budget for an imported base or raster layer. Stricter than the
/// 64 MiB host transfer ceiling so decode and layer allocation stay bounded.
pub const IMAGE_MAX_ENCODED_BYTES: usize = 32 * 1024 * 1024;
/// Pixel-count budget applied before a decoded buffer is allocated.
/// 4096×4096 = 16,777,216; a 5000×5000 header is rejected before IDAT inflate.
pub const IMAGE_MAX_PIXELS: u64 = 16_777_216;
/// RGBA decoded-memory budget for one image or layer (`pixels * 4 + 1 KiB`).
pub const IMAGE_MAX_DECODED_BYTES: usize = (IMAGE_MAX_PIXELS as usize)
    .saturating_mul(4)
    .saturating_add(1024);
/// Upper bound on raster layers on one map.
pub const IMAGE_MAX_RASTER_LAYERS: usize = 16;
/// In-memory undo/redo budget for one Image Map editing session.
pub const IMAGE_MAX_UNDO_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSource {
    pub width: u32,
    pub height: u32,
    pub source_format: &'static str,
    pub mime_type: &'static str,
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Validation(format!("maps: {}", message.into()))
}

pub fn source_format_for_mime(mime_type: &str) -> Result<&'static str, CoreError> {
    match mime_type {
        "image/png" => Ok("png"),
        "image/jpeg" => Ok("jpeg"),
        "image/svg+xml" => Ok("svg"),
        _ => Err(invalid("unsupported image MIME type")),
    }
}

pub fn mime_for_source_format(source_format: &str) -> Result<&'static str, CoreError> {
    match source_format {
        "png" => Ok("image/png"),
        "jpeg" => Ok("image/jpeg"),
        "svg" => Ok("image/svg+xml"),
        _ => Err(invalid("unsupported image sourceFormat")),
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), CoreError> {
    if width == 0 || height == 0 {
        return Err(invalid("image dimensions must be positive"));
    }
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > IMAGE_MAX_PIXELS {
        return Err(pixel_budget_error());
    }
    Ok(())
}

fn encoded_budget_error() -> CoreError {
    invalid(format!(
        "image exceeds the encoded-byte budget of {} bytes ({} MiB); choose a smaller PNG, JPEG, or SVG",
        IMAGE_MAX_ENCODED_BYTES,
        IMAGE_MAX_ENCODED_BYTES / (1024 * 1024)
    ))
}

fn pixel_budget_error() -> CoreError {
    invalid(format!(
        "image exceeds the pixel budget of {IMAGE_MAX_PIXELS} pixels (for example 4096×4096); choose a smaller image"
    ))
}

fn decoded_budget_error() -> CoreError {
    invalid(format!(
        "image exceeds the decoded-memory budget of {} bytes (~{} MiB RGBA); choose a smaller image",
        IMAGE_MAX_DECODED_BYTES,
        IMAGE_MAX_DECODED_BYTES / (1024 * 1024)
    ))
}

pub fn validate_image_source(bytes: &[u8], declared_mime: &str) -> Result<ImageSource, CoreError> {
    if bytes.is_empty() {
        return Err(invalid("image is empty"));
    }
    if bytes.len() > IMAGE_MAX_ENCODED_BYTES {
        return Err(encoded_budget_error());
    }
    let source_format = source_format_for_mime(declared_mime)?;
    let (width, height) = match source_format {
        "png" => decode_png(bytes)?,
        "jpeg" => decode_jpeg(bytes)?,
        "svg" => parse_svg(bytes)?,
        _ => return Err(invalid("unsupported image sourceFormat")),
    };
    check_dimensions(width, height)?;
    Ok(ImageSource {
        width,
        height,
        source_format,
        mime_type: mime_for_source_format(source_format)?,
    })
}

pub fn validate_raster_png(
    bytes: &[u8],
    expected_width: u32,
    expected_height: u32,
) -> Result<(), CoreError> {
    let source = validate_image_source(bytes, "image/png")?;
    if source.width != expected_width || source.height != expected_height {
        return Err(invalid(
            "raster layer PNG dimensions must match the base image",
        ));
    }
    Ok(())
}

pub fn encode_transparent_png(width: u32, height: u32) -> Result<Vec<u8>, CoreError> {
    check_dimensions(width, height)?;
    let pixel_bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(decoded_budget_error)?;
    if pixel_bytes > IMAGE_MAX_DECODED_BYTES {
        return Err(decoded_budget_error());
    }
    let data = vec![0_u8; pixel_bytes];
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::FilterType::NoFilter);
        let mut writer = encoder
            .write_header()
            .map_err(|error| invalid(format!("failed to encode PNG: {error}")))?;
        writer
            .write_image_data(&data)
            .map_err(|error| invalid(format!("failed to encode PNG: {error}")))?;
        writer
            .finish()
            .map_err(|error| invalid(format!("failed to encode PNG: {error}")))?;
    }
    if png.len() > IMAGE_MAX_ENCODED_BYTES {
        return Err(encoded_budget_error());
    }
    Ok(png)
}

pub fn content_hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn decode_png(bytes: &[u8]) -> Result<(u32, u32), CoreError> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: IMAGE_MAX_DECODED_BYTES,
        },
    );
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder
        .read_info()
        .map_err(|error| match error {
            png::DecodingError::LimitsExceeded => pixel_budget_error(),
            other => invalid(format!("PNG content does not match image/png: {other}")),
        })?;
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    check_dimensions(width, height)?;
    let buffer_size = reader.output_buffer_size();
    if buffer_size > IMAGE_MAX_DECODED_BYTES {
        return Err(decoded_budget_error());
    }
    let mut buffer = vec![0_u8; buffer_size];
    reader
        .next_frame(&mut buffer)
        .map_err(|error| invalid(format!("PNG IDAT could not be decoded: {error}")))?;
    Ok((width, height))
}

fn decode_jpeg(bytes: &[u8]) -> Result<(u32, u32), CoreError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder
        .read_info()
        .map_err(|error| invalid(format!("JPEG content does not match image/jpeg: {error}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| invalid("JPEG is missing frame dimensions"))?;
    check_dimensions(info.width.into(), info.height.into())?;
    let decoded_bytes = usize::from(info.width)
        .saturating_mul(usize::from(info.height))
        .saturating_mul(4);
    if decoded_bytes > IMAGE_MAX_DECODED_BYTES {
        return Err(decoded_budget_error());
    }
    decoder
        .decode()
        .map_err(|error| invalid(format!("JPEG scan data could not be decoded: {error}")))?;
    Ok((info.width.into(), info.height.into()))
}

fn parse_svg(bytes: &[u8]) -> Result<(u32, u32), CoreError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("SVG must be UTF-8"))?;
    let document =
        roxmltree::Document::parse(text).map_err(|error| invalid(format!("malformed SVG: {error}")))?;
    reject_unsafe_svg(&document)?;
    let root = document.root_element();
    if !root.has_tag_name("svg") {
        return Err(invalid("SVG root element must be svg"));
    }
    if let Some(view_box) = root.attribute("viewBox").or_else(|| root.attribute("viewbox")) {
        let parts = view_box
            .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(invalid("SVG viewBox is invalid"));
        }
        return Ok((parse_svg_length(parts[2])?, parse_svg_length(parts[3])?));
    }
    let width = root
        .attribute("width")
        .ok_or_else(|| invalid("SVG is missing viewBox or width/height"))?;
    let height = root
        .attribute("height")
        .ok_or_else(|| invalid("SVG is missing viewBox or width/height"))?;
    Ok((parse_svg_length(width)?, parse_svg_length(height)?))
}

fn reject_unsafe_svg(document: &roxmltree::Document) -> Result<(), CoreError> {
    for node in document.descendants() {
        if let Some(pi) = node.pi() {
            if pi.target.eq_ignore_ascii_case("xml-stylesheet") {
                return Err(invalid(
                    "SVG contains unsupported active or external content",
                ));
            }
        }
        if !node.is_element() {
            continue;
        }
        let name = node.tag_name().name();
        if forbidden_svg_element(name) {
            return Err(invalid(
                "SVG contains unsupported active or external content",
            ));
        }
        if name.eq_ignore_ascii_case("style") {
            if let Some(text) = node.text() {
                reject_css_urls(text)?;
            }
        }
        for attribute in node.attributes() {
            let local = attribute.name();
            if local.len() >= 2 && local.as_bytes()[..2].eq_ignore_ascii_case(b"on") {
                return Err(invalid(
                    "SVG contains unsupported active or external content",
                ));
            }
            if local.eq_ignore_ascii_case("href")
                || local.eq_ignore_ascii_case("src")
                || attribute.namespace() == Some("http://www.w3.org/1999/xlink")
                    && local.eq_ignore_ascii_case("href")
            {
                reject_external_reference(attribute.value())?;
            }
            if local.eq_ignore_ascii_case("style") {
                reject_css_urls(attribute.value())?;
            }
            if local.eq_ignore_ascii_case("base") {
                reject_external_reference(attribute.value())?;
            }
        }
    }
    Ok(())
}

fn forbidden_svg_element(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "script"
            | "foreignobject"
            | "iframe"
            | "embed"
            | "object"
            | "video"
            | "audio"
            | "handler"
            | "listener"
            | "set"
            | "animate"
            | "animatetransform"
            | "animatemotion"
    )
}

fn reject_external_reference(value: &str) -> Result<(), CoreError> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('#') {
        return Ok(());
    }
    let lowered = value.to_ascii_lowercase();
    if lowered.starts_with("http:")
        || lowered.starts_with("https:")
        || lowered.starts_with("file:")
        || lowered.starts_with("data:")
        || lowered.starts_with("javascript:")
        || lowered.starts_with("//")
    {
        return Err(invalid(
            "SVG contains unsupported active or external content",
        ));
    }
    Err(invalid(
        "SVG contains unsupported active or external content",
    ))
}

fn reject_css_urls(text: &str) -> Result<(), CoreError> {
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("@import") {
        return Err(invalid(
            "SVG contains unsupported active or external content",
        ));
    }
    let mut search = lowered.as_str();
    while let Some(index) = search.find("url(") {
        let rest = search[index + 4..].trim_start();
        let rest = rest
            .strip_prefix('"')
            .or_else(|| rest.strip_prefix('\''))
            .unwrap_or(rest)
            .trim_start();
        if !rest.starts_with('#') {
            return Err(invalid(
                "SVG contains unsupported active or external content",
            ));
        }
        search = &search[index + 4..];
    }
    Ok(())
}

fn parse_svg_length(value: &str) -> Result<u32, CoreError> {
    let value = value.trim();
    let number = value.strip_suffix("px").unwrap_or(value).trim();
    if number.ends_with('%') || number.contains("em") {
        return Err(invalid("SVG size units are unsupported"));
    }
    let parsed = number
        .parse::<f64>()
        .map_err(|_| invalid("SVG size is not a finite number"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(invalid("SVG dimensions must be positive and finite"));
    }
    if parsed > IMAGE_MAX_PIXELS as f64 {
        return Err(pixel_budget_error());
    }
    Ok(parsed.round() as u32)
}

#[cfg(test)]
pub(crate) const VALID_JPEG: &[u8] = include_bytes!("fixtures/valid-8x8.jpg");

#[cfg(test)]
mod tests {
    use super::*;

    fn write_chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(&kind);
        out.extend_from_slice(data);
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in kind.iter().chain(data) {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        out.extend_from_slice(&(crc ^ 0xFFFF_FFFF).to_be_bytes());
    }

    #[test]
    fn rejects_over_budget_png_headers_before_idat() {
        let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
        write_chunk(
            &mut bytes,
            *b"IHDR",
            &[
                0, 1, 0, 0, // width 65536
                0, 1, 0, 0, // height 65536
                8, 6, 0, 0, 0,
            ],
        );
        let error = validate_image_source(&bytes, "image/png")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("pixel budget") || error.contains("PNG"),
            "{error}"
        );
    }

    #[test]
    fn round_trips_transparent_png_and_safe_svg() {
        let png = encode_transparent_png(4, 3).unwrap();
        let source = validate_image_source(&png, "image/png").unwrap();
        assert_eq!(
            source,
            ImageSource {
                width: 4,
                height: 3,
                source_format: "png",
                mime_type: "image/png"
            }
        );
        validate_raster_png(&png, 4, 3).unwrap();
        let mut corrupt = png.clone();
        let idat = corrupt.windows(4).position(|window| window == b"IDAT").unwrap();
        corrupt[idat + 4] ^= 0xff;
        assert!(validate_image_source(&corrupt, "image/png").is_err());

        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 40"></svg>"#;
        let source = validate_image_source(svg, "image/svg+xml").unwrap();
        assert_eq!(source.width, 80);
        assert_eq!(source.height, 40);
    }

    #[test]
    fn rejects_unsafe_svg_event_handlers_and_external_refs() {
        let png = encode_transparent_png(1, 1).unwrap();
        assert!(validate_image_source(&png, "image/jpeg").is_err());
        let scripted = br#"<svg viewBox="0 0 10 10"><script>alert(1)</script></svg>"#;
        assert!(validate_image_source(scripted, "image/svg+xml")
            .unwrap_err()
            .to_string()
            .contains("unsupported active"));
        let newline_onload = b"<svg\nonload=\"alert(1)\" viewBox=\"0 0 10 10\"></svg>";
        assert!(validate_image_source(newline_onload, "image/svg+xml").is_err());
        let spaced_href =
            br#"<svg viewBox="0 0 10 10"><image href = "https://example.test/x.png"/></svg>"#;
        assert!(validate_image_source(spaced_href, "image/svg+xml").is_err());
        let css_url =
            br#"<svg viewBox="0 0 10 10"><rect style="fill:url(https://example.test/x.png)"/></svg>"#;
        assert!(validate_image_source(css_url, "image/svg+xml").is_err());
    }

    #[test]
    fn budget_errors_name_the_recorded_limits_before_allocation() {
        let empty = validate_image_source(&[], "image/png").unwrap_err().to_string();
        assert!(empty.contains("empty"), "{empty}");
        let oversized = vec![0_u8; IMAGE_MAX_ENCODED_BYTES + 1];
        let encoded = validate_image_source(&oversized, "image/png")
            .unwrap_err()
            .to_string();
        assert!(encoded.contains(&IMAGE_MAX_ENCODED_BYTES.to_string()), "{encoded}");
        assert!(encoded.contains("32 MiB"), "{encoded}");
        assert_eq!(IMAGE_MAX_DECODED_BYTES, 16_777_216 * 4 + 1024);
        assert_eq!(IMAGE_MAX_UNDO_BYTES, 64 * 1024 * 1024);
        assert_eq!(IMAGE_MAX_RASTER_LAYERS, 16);
    }

    #[test]
    fn decodes_real_jpeg_and_rejects_header_only_bytes() {
        let source = validate_image_source(VALID_JPEG, "image/jpeg").unwrap();
        assert_eq!(source.width, 8);
        assert_eq!(source.height, 8);
        let mut header_only = vec![
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x08, 8, 0x00, 0x08, 0x00, 0x08, 3,
        ];
        header_only.extend_from_slice(&[0xFF, 0xD9]);
        assert!(validate_image_source(&header_only, "image/jpeg").is_err());
    }
}
