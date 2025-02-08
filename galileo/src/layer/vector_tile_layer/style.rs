//! See [`VectorTileStyle`].

use std::collections::HashMap;

use galileo_mvt::MvtFeature;
use serde::{Deserialize, Serialize};

use crate::render::point_paint::PointPaint;
use crate::render::text::TextStyle;
use crate::render::{LineCap, LinePaint, PolygonPaint};
use crate::Color;

/// Style of a vector tile layer. This specifies how each feature in a tile should be rendered.
///
/// <div class="warning">This exact type is experimental and is likely to change in near future.</div>
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTileStyle {
    /// Rules for feature to be drawn. Rules are traversed in sequence until a rule that corresponds to a current feature
    /// is found, and that rule is used for drawing. If no rule corresponds to the feature, default symbol is used.
    pub rules: Vec<StyleRule>,

    /// Default symbol that is used for features, for which other rules don't apply.
    pub default_symbol: VectorTileDefaultSymbol,

    /// Background color of tiles.
    pub background: Color,
}

/// Default symbol of the vector tile.
///
/// These symbols are applied to the features in the tile if no of the style rules are selected for
/// this feature.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTileDefaultSymbol {
    /// Symbol for point objects.
    pub point: Option<VectorTilePointSymbol>,
    /// Symbol for line objects.
    pub line: Option<VectorTileLineSymbol>,
    /// Symbol for polygon objects.
    pub polygon: Option<VectorTilePolygonSymbol>,
    /// Symbol for point objects that should have text labels.
    pub label: Option<VectorTileLabelSymbol>,
}

impl VectorTileStyle {
    /// Get a rule for the given feature.
    pub fn get_style_rule(&self, layer_name: &str, feature: &MvtFeature) -> Option<&StyleRule> {
        self.rules.iter().find(|&rule| {
            let layer_name_check_passed = match &rule.layer_name {
                Some(name) => name == layer_name,
                None => true,
            };
            layer_name_check_passed
                && (rule.properties.is_empty()
                    || rule.properties.iter().all(|(key, value)| {
                        feature.properties.get(key).map(|v| v.to_string())
                            == Some(value.to_string())
                    }))
        })
    }
}

/// A rule that specifies what kind of features can be drawing with the given symbol.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleRule {
    /// If set, a feature must belong to the set layer. If not set, layer is not checked.
    pub layer_name: Option<String>,
    /// Specifies a set of attributes of a feature that must have the given values for this rule to be applied.
    #[serde(default)]
    pub properties: HashMap<String, String>,
    /// Symbol to draw a feature with.
    #[serde(default)]
    pub symbol: VectorTileSymbol,
}

/// Symbol of an object in a vector tile.
///
/// An the object has incompatible type with the symbol, the object is not renderred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VectorTileSymbol {
    /// Do not render object.
    None,
    /// Symbol for a point object.
    #[serde(rename = "point")]
    Point(VectorTilePointSymbol),
    /// Symbol for a line object.
    #[serde(rename = "line")]
    Line(VectorTileLineSymbol),
    /// Symbol for a polygon object.
    #[serde(rename = "polygon")]
    Polygon(VectorTilePolygonSymbol),
    /// Symbol for a point object that is renderred as a text label.
    #[serde(rename = "label")]
    Label(VectorTileLabelSymbol),
}

impl Default for VectorTileSymbol {
    fn default() -> Self {
        Self::None
    }
}

impl VectorTileSymbol {
    pub(crate) fn line(&self) -> Option<&VectorTileLineSymbol> {
        match self {
            Self::Line(symbol) => Some(symbol),
            _ => None,
        }
    }

    pub(crate) fn polygon(&self) -> Option<&VectorTilePolygonSymbol> {
        match self {
            Self::Polygon(symbol) => Some(symbol),
            _ => None,
        }
    }

    pub(crate) fn point(&self) -> Option<&VectorTilePointSymbol> {
        match self {
            Self::Point(symbol) => Some(symbol),
            _ => None,
        }
    }

    pub(crate) fn label(&self) -> Option<&VectorTileLabelSymbol> {
        match self {
            Self::Label(symbol) => Some(symbol),
            _ => None,
        }
    }
}

/// Symbol for point geometries.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTilePointSymbol {
    /// Size of the point.
    pub size: f64,
    /// Color of the point.
    pub color: Color,
}

impl From<VectorTilePointSymbol> for PointPaint<'_> {
    fn from(value: VectorTilePointSymbol) -> Self {
        PointPaint::circle(value.color, value.size as f32)
    }
}

/// Symbol for line geometries.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTileLineSymbol {
    /// Width of the line in pixels.
    pub width: f64,
    /// Color of the line in pixels.
    pub stroke_color: Color,
}

impl From<VectorTileLineSymbol> for LinePaint {
    fn from(value: VectorTileLineSymbol) -> Self {
        Self {
            color: value.stroke_color,
            width: value.width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        }
    }
}

/// Symbol for polygon geometries.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTilePolygonSymbol {
    /// Color of the fill of polygon.
    pub fill_color: Color,
}

impl From<VectorTilePolygonSymbol> for PolygonPaint {
    fn from(value: VectorTilePolygonSymbol) -> Self {
        Self {
            color: value.fill_color,
        }
    }
}

/// Symbol of a point geometry that is renderred as text label on the map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorTileLabelSymbol {
    /// Text of the label with substitutes for feature attributes.
    pub pattern: String,
    /// Style of the text.
    pub text_style: TextStyle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_serialization_point() {
        let symbol = VectorTileSymbol::Point(VectorTilePointSymbol {
            size: 10.0,
            color: Color::BLACK,
        });

        let json = serde_json::to_string_pretty(&symbol).unwrap();
        eprintln!("{json}");

        let value = serde_json::to_value(&symbol).unwrap();
        assert!(value.as_object().unwrap().get("point").is_some());
        assert!(value.as_object().unwrap().get("polygon").is_none());
    }

    #[test]
    fn serialize_with_bincode() {
        let rule = StyleRule {
            layer_name: None,
            properties: HashMap::new(),
            symbol: VectorTileSymbol::None,
        };

        let serialized = bincode::serde::encode_to_vec(&rule, bincode::config::standard()).unwrap();
        let _: (StyleRule, _) =
            bincode::serde::decode_from_slice(&serialized, bincode::config::standard()).unwrap();
    }
}
