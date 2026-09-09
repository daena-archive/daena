//! Host appearance token catalog and builtin light/dark maps.

use crate::rpc::{AppearancePreference, AppearanceResolved, PluginAppearance};
use crate::ContractError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    pub tokens: ThemePackTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ThemePackTokens {
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

pub const THEME_TOKEN_IDS: &[&str] = &[
    "ink",
    "ink-soft",
    "ink-faint",
    "ink-muted",
    "line",
    "line-soft",
    "line-strong",
    "surface",
    "surface-muted",
    "surface-warm",
    "surface-subtle",
    "surface-quiet",
    "canvas",
    "accent",
    "accent-dark",
    "accent-soft",
    "accent-bg",
    "on-accent",
    "on-bright-accent",
    "brass-ink",
    "danger",
    "danger-bg",
    "danger-line",
    "success",
    "success-bg",
    "success-line",
    "warning",
    "warning-bg",
    "warning-line",
    "info",
    "info-bg",
    "info-line",
    "rail-bg",
    "rail-surface",
    "rail-surface-strong",
    "rail-popover",
    "rail-text",
    "rail-text-soft",
    "rail-text-muted",
    "rail-text-faint",
    "rail-accent",
    "rail-accent-hover",
    "rail-online",
    "rail-offline",
    "rail-border",
];

const BUILTIN_LIGHT: &[(&str, &str)] = &[
    ("ink", "#25251f"),
    ("ink-soft", "#77766d"),
    ("ink-faint", "#aaa79d"),
    ("ink-muted", "#62594e"),
    ("line", "#e4e1d8"),
    ("line-soft", "#e9e1d4"),
    ("line-strong", "#d9cdbd"),
    ("surface", "#fffefa"),
    ("surface-muted", "#f4f2ec"),
    ("surface-warm", "#f4eee3"),
    ("surface-subtle", "#f7f3ec"),
    ("surface-quiet", "#fffcf7"),
    ("canvas", "#f7f6f2"),
    ("accent", "#b4773f"),
    ("accent-dark", "#365342"),
    ("accent-soft", "#c99965"),
    ("accent-bg", "#f2e4d2"),
    ("on-accent", "#fffefa"),
    ("on-bright-accent", "#fffefa"),
    ("brass-ink", "#2f2619"),
    ("danger", "#a14f42"),
    ("danger-bg", "#fdf2ef"),
    ("danger-line", "#e7c4bc"),
    ("success", "#557d63"),
    ("success-bg", "#eef5ef"),
    ("success-line", "#c8d8cb"),
    ("warning", "#8a5f24"),
    ("warning-bg", "#fff8ed"),
    ("warning-line", "#ead7bc"),
    ("info", "#4e6f7c"),
    ("info-bg", "#e8f1f3"),
    ("info-line", "#bfd3d9"),
    ("rail-bg", "#283a30"),
    ("rail-surface", "#3b5243"),
    ("rail-surface-strong", "#486052"),
    ("rail-popover", "#2f4a38"),
    ("rail-text", "#eef0e9"),
    ("rail-text-soft", "#b9c8bc"),
    ("rail-text-muted", "#aab9ad"),
    ("rail-text-faint", "#91a397"),
    ("rail-accent", "#d5ab6c"),
    ("rail-accent-hover", "#e1bc82"),
    ("rail-online", "#88c18e"),
    ("rail-offline", "#777f78"),
    ("rail-border", "#486052"),
];

const BUILTIN_DARK: &[(&str, &str)] = &[
    ("ink", "#f2eee4"),
    ("ink-soft", "#d8d1c3"),
    ("ink-faint", "#a49e92"),
    ("ink-muted", "#b8b1a5"),
    ("line", "#31443a"),
    ("line-soft", "#26372f"),
    ("line-strong", "#435a4e"),
    ("surface", "#131f1b"),
    ("surface-muted", "#182720"),
    ("surface-warm", "#182720"),
    ("surface-subtle", "#15231d"),
    ("surface-quiet", "#101b17"),
    ("canvas", "#0e1714"),
    ("accent", "#c58a4a"),
    ("accent-dark", "#557d63"),
    ("accent-soft", "#d7a25c"),
    ("accent-bg", "#3c3525"),
    ("on-accent", "#fffefa"),
    ("on-bright-accent", "#25251f"),
    ("brass-ink", "#2f2619"),
    ("danger", "#e09a8d"),
    ("danger-bg", "#321f1c"),
    ("danger-line", "#70443d"),
    ("success", "#8aad69"),
    ("success-bg", "#1b2d20"),
    ("success-line", "#3f5d44"),
    ("warning", "#e4c786"),
    ("warning-bg", "#3c3525"),
    ("warning-line", "#6c5830"),
    ("info", "#91a4ae"),
    ("info-bg", "#1d2930"),
    ("info-line", "#3f5662"),
    ("rail-bg", "#283a30"),
    ("rail-surface", "#3b5243"),
    ("rail-surface-strong", "#486052"),
    ("rail-popover", "#2f4a38"),
    ("rail-text", "#eef0e9"),
    ("rail-text-soft", "#b9c8bc"),
    ("rail-text-muted", "#aab9ad"),
    ("rail-text-faint", "#91a397"),
    ("rail-accent", "#d5ab6c"),
    ("rail-accent-hover", "#e1bc82"),
    ("rail-online", "#88c18e"),
    ("rail-offline", "#777f78"),
    ("rail-border", "#486052"),
];

const TEXT_CONTRAST_PAIRS: &[(&str, &str)] = &[("ink", "surface"), ("ink", "canvas")];
const CHROME_CONTRAST_PAIRS: &[(&str, &str)] = &[("rail-text", "rail-bg")];
const MIN_TEXT_CONTRAST: f64 = 4.5;
const MIN_CHROME_CONTRAST: f64 = 3.0;

pub fn is_theme_token_id(value: &str) -> bool {
    THEME_TOKEN_IDS.contains(&value)
}

pub fn parse_theme_color(value: &str) -> Result<String, ContractError> {
    let trimmed = value.trim();
    if !trimmed.starts_with('#') {
        return Err(ContractError(format!("invalid theme color: {value}")));
    }
    let hex = &trimmed[1..];
    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ContractError(format!("invalid theme color: {value}")));
    }
    let normalized = match hex.len() {
        3 => format!(
            "#{}{}{}{}{}{}",
            hex.as_bytes()[0] as char,
            hex.as_bytes()[0] as char,
            hex.as_bytes()[1] as char,
            hex.as_bytes()[1] as char,
            hex.as_bytes()[2] as char,
            hex.as_bytes()[2] as char
        ),
        6 | 8 => format!("#{hex}"),
        _ => return Err(ContractError(format!("invalid theme color: {value}"))),
    };
    Ok(normalized.to_ascii_lowercase())
}

pub fn builtin_theme_tokens(mode: ThemeMode) -> BTreeMap<String, String> {
    let pairs = match mode {
        ThemeMode::Light => BUILTIN_LIGHT,
        ThemeMode::Dark => BUILTIN_DARK,
    };
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

pub fn merge_theme_tokens(
    base: &BTreeMap<String, String>,
    overlay: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ContractError> {
    let mut merged = base.clone();
    for (key, value) in overlay {
        if !is_theme_token_id(key) {
            return Err(ContractError(format!("unknown theme token: {key}")));
        }
        merged.insert(key.clone(), parse_theme_color(value)?);
    }
    Ok(merged)
}

pub fn resolve_theme_tokens(
    mode: ThemeMode,
    overlay: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ContractError> {
    let merged = merge_theme_tokens(&builtin_theme_tokens(mode), overlay)?;
    validate_theme_contrast(&merged)?;
    Ok(merged)
}

pub const APPEARANCE_FEATURE: &str = "appearance@1";
const THEME_PACK_NAME_MAX_CHARS: usize = 128;

pub fn builtin_plugin_appearance() -> PluginAppearance {
    PluginAppearance {
        preference: AppearancePreference::System,
        resolved: AppearanceResolved::Light,
        pack: None,
        tokens: builtin_theme_tokens(ThemeMode::Light),
    }
}

pub fn sanitize_plugin_appearance(
    appearance: PluginAppearance,
) -> Result<PluginAppearance, ContractError> {
    let pack = match appearance.pack {
        None => None,
        Some(pack) => {
            if !crate::is_identifier(&pack.plugin_id) || !crate::is_identifier(&pack.theme_id) {
                return Err(ContractError("invalid appearance pack ref".into()));
            }
            Some(pack)
        }
    };
    let mut tokens = BTreeMap::new();
    for (key, value) in &appearance.tokens {
        if !is_theme_token_id(key) {
            return Err(ContractError(format!("unknown theme token: {key}")));
        }
        tokens.insert(key.clone(), parse_theme_color(value)?);
    }
    for id in THEME_TOKEN_IDS {
        if !tokens.contains_key(*id) {
            return Err(ContractError(format!("missing theme token: {id}")));
        }
    }
    validate_theme_contrast(&tokens)?;
    Ok(PluginAppearance {
        pack,
        tokens,
        ..appearance
    })
}

pub fn validate_theme_pack(pack: &ThemePack) -> Result<(), ContractError> {
    if pack.name.trim().is_empty() {
        return Err(ContractError(format!("theme {} name is required", pack.id)));
    }
    if pack.name.chars().count() > THEME_PACK_NAME_MAX_CHARS {
        return Err(ContractError(format!("theme {} name is too long", pack.id)));
    }
    resolve_theme_tokens(ThemeMode::Light, &pack.tokens.light)?;
    resolve_theme_tokens(ThemeMode::Dark, &pack.tokens.dark)?;
    Ok(())
}

pub fn validate_theme_contrast(tokens: &BTreeMap<String, String>) -> Result<(), ContractError> {
    check_contrast_pairs(tokens, TEXT_CONTRAST_PAIRS, MIN_TEXT_CONTRAST)?;
    check_contrast_pairs(tokens, CHROME_CONTRAST_PAIRS, MIN_CHROME_CONTRAST)?;
    Ok(())
}

fn check_contrast_pairs(
    tokens: &BTreeMap<String, String>,
    pairs: &[(&str, &str)],
    minimum: f64,
) -> Result<(), ContractError> {
    for &(foreground, background) in pairs {
        let fg = tokens
            .get(foreground)
            .ok_or_else(|| ContractError(format!("missing theme token: {foreground}")))?;
        let bg = tokens
            .get(background)
            .ok_or_else(|| ContractError(format!("missing theme token: {background}")))?;
        let ratio = contrast_ratio(fg, bg)?;
        if ratio + f64::EPSILON < minimum {
            return Err(ContractError(format!(
                "theme contrast {foreground}/{background} is {ratio:.2}, expected at least {minimum}"
            )));
        }
    }
    Ok(())
}

fn contrast_ratio(a: &str, b: &str) -> Result<f64, ContractError> {
    let (l1, l2) = (relative_luminance(a)?, relative_luminance(b)?);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    Ok((lighter + 0.05) / (darker + 0.05))
}

fn relative_luminance(color: &str) -> Result<f64, ContractError> {
    let (r, g, b) = rgb_channels(color)?;
    Ok(0.2126 * linear_channel(r) + 0.7152 * linear_channel(g) + 0.0722 * linear_channel(b))
}

fn rgb_channels(color: &str) -> Result<(u8, u8, u8), ContractError> {
    let normalized = parse_theme_color(color)?;
    let hex = &normalized[1..];
    if hex.len() == 8 {
        let alpha = u8::from_str_radix(&hex[6..8], 16)
            .map_err(|_| ContractError(format!("invalid theme color: {color}")))?;
        if alpha != 0xff {
            return Err(ContractError(format!(
                "theme contrast requires opaque colors, got {color}"
            )));
        }
    }
    let read = |offset: usize| {
        u8::from_str_radix(&hex[offset..offset + 2], 16)
            .map_err(|_| ContractError(format!("invalid theme color: {color}")))
    };
    Ok((read(0)?, read(2)?, read(4)?))
}

fn linear_channel(value: u8) -> f64 {
    let srgb = f64::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_plugin_appearance_accepts_builtin_and_rejects_unknown_keys() {
        let clean = sanitize_plugin_appearance(builtin_plugin_appearance()).unwrap();
        assert_eq!(clean.tokens.len(), THEME_TOKEN_IDS.len());
        let mut dirty = builtin_plugin_appearance();
        dirty.tokens.insert("not-a-token".into(), "#ffffff".into());
        assert!(sanitize_plugin_appearance(dirty).is_err());
        let mut incomplete = builtin_plugin_appearance();
        incomplete.tokens.remove("ink");
        assert!(sanitize_plugin_appearance(incomplete).is_err());
        let mut low_contrast = builtin_plugin_appearance();
        low_contrast.tokens.insert("ink".into(), "#ffffff".into());
        low_contrast
            .tokens
            .insert("surface".into(), "#ffffff".into());
        low_contrast
            .tokens
            .insert("canvas".into(), "#ffffff".into());
        assert!(sanitize_plugin_appearance(low_contrast).is_err());
        let mut invalid_pack = builtin_plugin_appearance();
        invalid_pack.pack = Some(crate::rpc::AppearancePackRef {
            plugin_id: "".into(),
            theme_id: "parchment".into(),
        });
        assert!(sanitize_plugin_appearance(invalid_pack).is_err());
        let mut valid_pack = builtin_plugin_appearance();
        valid_pack.pack = Some(crate::rpc::AppearancePackRef {
            plugin_id: "com.example.parchment".into(),
            theme_id: "parchment".into(),
        });
        assert_eq!(
            sanitize_plugin_appearance(valid_pack)
                .unwrap()
                .pack
                .unwrap()
                .theme_id,
            "parchment"
        );
    }

    #[test]
    fn builtin_maps_cover_the_catalog() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let tokens = builtin_theme_tokens(mode);
            assert_eq!(tokens.len(), THEME_TOKEN_IDS.len());
            for id in THEME_TOKEN_IDS {
                let value = tokens.get(*id).expect(id);
                assert_eq!(parse_theme_color(value).unwrap(), *value);
            }
        }
        assert_eq!(
            BUILTIN_LIGHT.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            THEME_TOKEN_IDS
        );
        assert_eq!(
            BUILTIN_DARK.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            THEME_TOKEN_IDS
        );
    }

    #[test]
    fn builtin_themes_meet_contrast() {
        resolve_theme_tokens(ThemeMode::Light, &BTreeMap::new()).unwrap();
        resolve_theme_tokens(ThemeMode::Dark, &BTreeMap::new()).unwrap();
    }

    #[test]
    fn sparse_overlay_fills_from_builtin() {
        let mut overlay = BTreeMap::new();
        overlay.insert("accent".into(), "#b4773f".into());
        let merged = resolve_theme_tokens(ThemeMode::Light, &overlay).unwrap();
        assert_eq!(merged.get("accent").unwrap(), "#b4773f");
        assert_eq!(merged.get("canvas").unwrap(), "#f7f6f2");
    }

    #[test]
    fn unknown_token_and_color_fail_closed() {
        let mut overlay = BTreeMap::new();
        overlay.insert("nope".into(), "#ffffff".into());
        assert!(merge_theme_tokens(&builtin_theme_tokens(ThemeMode::Light), &overlay).is_err());
        overlay.clear();
        overlay.insert("accent".into(), "red".into());
        assert!(merge_theme_tokens(&builtin_theme_tokens(ThemeMode::Light), &overlay).is_err());
    }

    #[test]
    fn short_hex_normalizes() {
        assert_eq!(parse_theme_color("#abc").unwrap(), "#aabbcc");
        assert_eq!(parse_theme_color("#AABBCC").unwrap(), "#aabbcc");
        assert_eq!(parse_theme_color("#aabbccdd").unwrap(), "#aabbccdd");
    }

    #[test]
    fn contrast_rejects_unreadable_overlay() {
        let mut overlay = BTreeMap::new();
        overlay.insert("ink".into(), "#fffefa".into());
        overlay.insert("surface".into(), "#fffefa".into());
        assert!(resolve_theme_tokens(ThemeMode::Light, &overlay).is_err());
    }

    #[test]
    fn contrast_rejects_translucent_hex() {
        let mut overlay = BTreeMap::new();
        overlay.insert("ink".into(), "#25251f80".into());
        assert!(resolve_theme_tokens(ThemeMode::Light, &overlay).is_err());
        overlay.insert("ink".into(), "#25251fff".into());
        assert!(resolve_theme_tokens(ThemeMode::Light, &overlay).is_ok());
    }

    #[test]
    fn theme_pack_requires_readable_modes() {
        let pack = ThemePack {
            id: "parchment".into(),
            name: "Parchment".into(),
            tokens: ThemePackTokens {
                light: BTreeMap::from([("accent".into(), "#b4773f".into())]),
                dark: BTreeMap::from([("accent".into(), "#c58a4a".into())]),
            },
        };
        assert!(validate_theme_pack(&pack).is_ok());
        let mut bad = pack.clone();
        bad.name = " ".into();
        assert!(validate_theme_pack(&bad).is_err());
        bad = pack.clone();
        bad.name = "P".repeat(129);
        assert!(validate_theme_pack(&bad).is_err());
        bad = pack;
        bad.tokens.light.insert("ink".into(), "#fffefa".into());
        bad.tokens.light.insert("surface".into(), "#fffefa".into());
        assert!(validate_theme_pack(&bad).is_err());
    }
}
