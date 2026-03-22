//! Lua plugin runtime: plugin loading, indicator registry, epoch ticking.

pub mod state;

/// Font size for indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorFont {
    /// ProFont 9pt (6px wide).
    Small,
    /// ProFont 10pt (7px wide).
    Medium,
}

/// A text indicator registered by a Lua plugin.
#[derive(Debug, Clone)]
pub struct Indicator {
    /// Unique name (e.g. "uptime", "battery").
    pub name: String,
    /// Current text value to display.
    pub value: String,
    /// X position on display (0-249).
    pub x: i32,
    /// Y position on display (0-121).
    pub y: i32,
    /// Optional label prefix (e.g. "UP" renders as "UP: {value}").
    pub label: Option<String>,
    /// Font size.
    pub font: IndicatorFont,
    /// Word-wrap width in chars (0 = no wrap).
    pub wrap_width: u32,
}

/// Plugin metadata read from Lua file scope.
#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub author: String,
    pub tag: String, // "default" or "community"
}

/// Plugin config from TOML.
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    /// Extra config keys passed to Lua's on_load(config).
    pub extra: std::collections::HashMap<String, String>,
}

impl PluginConfig {
    pub fn default_for(name: &str, x: i32, y: i32) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            x,
            y,
            extra: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indicator_default_fields() {
        let ind = Indicator {
            name: "test".into(),
            value: "hello".into(),
            x: 10,
            y: 20,
            label: None,
            font: IndicatorFont::Small,
            wrap_width: 0,
        };
        assert_eq!(ind.name, "test");
        assert_eq!(ind.value, "hello");
        assert_eq!(ind.x, 10);
        assert_eq!(ind.y, 20);
        assert!(ind.label.is_none());
        assert_eq!(ind.font, IndicatorFont::Small);
        assert_eq!(ind.wrap_width, 0);
    }

    #[test]
    fn test_indicator_with_label() {
        let ind = Indicator {
            name: "uptime".into(),
            value: "02:15".into(),
            x: 185,
            y: 0,
            label: Some("UP".into()),
            font: IndicatorFont::Small,
            wrap_width: 0,
        };
        assert_eq!(ind.label, Some("UP".into()));
    }

    #[test]
    fn test_indicator_with_wrap() {
        let ind = Indicator {
            name: "status".into(),
            value: "Sniffing the airwaves today".into(),
            x: 125,
            y: 20,
            label: None,
            font: IndicatorFont::Medium,
            wrap_width: 17,
        };
        assert_eq!(ind.wrap_width, 17);
        assert_eq!(ind.font, IndicatorFont::Medium);
    }

    #[test]
    fn test_plugin_config_default() {
        let cfg = PluginConfig::default_for("uptime", 185, 0);
        assert_eq!(cfg.name, "uptime");
        assert!(cfg.enabled);
        assert_eq!(cfg.x, 185);
        assert_eq!(cfg.y, 0);
        assert!(cfg.extra.is_empty());
    }

    #[test]
    fn test_plugin_meta() {
        let meta = PluginMeta {
            name: "uptime".into(),
            version: "1.0.0".into(),
            author: "oxigotchi".into(),
            tag: "default".into(),
        };
        assert_eq!(meta.tag, "default");
    }
}
