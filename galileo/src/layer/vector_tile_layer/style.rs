//! See [`VectorTileStyle`].

use crate::render::point_paint::PointPaint;
use crate::Color;
use galileo_mvt::MvtFeature;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Style of a vector tile layer. This specifies how each feature in a tile should be rendered.
///
/// <div class="warning">This exact type is experimental and is likely to change in near future.</div>
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VectorTileStyle {
    /// Rules for feature to be drawn. Rules are traversed in sequence until a rule that corresponds to a current feature
    /// is found, and that rule is used for drawing. If no rule corresponds to the feature, default symbol is used.
    pub rules: Vec<StyleRule>,

    /// Default symbol that is used for features, for which other rules don't apply.
    pub default_symbol: VectorTileSymbol,

    /// Background color of tiles.
    pub background: Color,
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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StyleRule {
    /// If set, a feature must belong to the set layer. If not set, layer is not checked.
    pub layer_name: Option<String>,
    /// Specifies a set of attibutes of a feature that must have the given values for this rule to be applied.
    #[serde(default)]
    pub properties: HashMap<String, String>,
    /// Symbol to draw a feature with.
    pub symbol: VectorTileSymbol,
}

/// Symbol to draw a vector tile feature.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VectorTileSymbol {
    /// If set, points will be drawn with this symbol.
    pub point: Option<PointPaint<'static>>,
    /// If set, lines will be drawn with this symbol.
    pub line: Option<VectorTileLineSymbol>,
    /// If set, polygons will be drawn with this symbol.
    pub polygon: Option<VectorTilePolygonSymbol>,
}

impl VectorTileSymbol {
    /// Creates a new symbol for polygon geometries.
    pub fn polygon(color: Color) -> Self {
        Self {
            point: None,
            line: None,
            polygon: Some(VectorTilePolygonSymbol { fill_color: color }),
        }
    }
}

/// Symbol for point geometries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTilePointSymbol {
    /// Size of the point.
    pub size: f64,
    /// Color of the point.
    pub color: Color,
}

/// Symbol for line geometries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTileLineSymbol {
    /// Width of the line in pixels.
    pub width: f64,
    /// Color of the line in pixels.
    pub stroke_color: Color,
}

/// Symbol for polygon geometries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTilePolygonSymbol {
    /// Color of the fill of polygon.
    pub fill_color: Color,
}
