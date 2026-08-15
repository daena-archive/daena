use std::io::Cursor;

use png::{BitDepth, ColorType, Compression, Decoder, Encoder, FilterType};

use crate::provenance::AtlasRenderProvenanceV1;
use crate::request::AtlasRenderRequest;
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
        let rgba = vec![10_u8, 20, 30, 255, 40, 50, 60, 255]
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
}
