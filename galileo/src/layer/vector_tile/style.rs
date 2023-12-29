use crate::primitives::Color;
use galileo_mvt::MvtFeature;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VectorTileStyle {
    pub rules: Vec<StyleRule>,
    pub default_symbol: VectorTileSymbol,
    pub background: Color,
}

impl VectorTileStyle {
    pub fn get_style_rule(&self, layer_name: &str, feature: &MvtFeature) -> Option<&StyleRule> {
        for rule in &self.rules {
            if (rule.layer_name.is_none() || rule.layer_name.as_ref().unwrap() == layer_name)
                && (rule.properties.is_empty()
                    || rule.properties.iter().all(|(key, value)| {
                        feature.properties.get(key).map(|v| v.to_string())
                            == Some(value.to_string())
                    }))
            {
                return Some(rule);
            }
        }

        None
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StyleRule {
    pub layer_name: Option<String>,
    #[serde(default)]
    pub properties: HashMap<String, String>,
    pub symbol: VectorTileSymbol,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VectorTileSymbol {
    pub point: Option<VectorTilePointSymbol>,
    pub line: Option<VectorTileLineSymbol>,
    pub polygon: Option<VectorTilePolygonSymbol>,
}

impl VectorTileSymbol {
    pub fn polygon(color: Color) -> Self {
        Self {
            point: None,
            line: None,
            polygon: Some(VectorTilePolygonSymbol { fill_color: color }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTilePointSymbol {
    pub size: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTileLineSymbol {
    pub width: f64,
    pub stroke_color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTilePolygonSymbol {
    pub fill_color: Color,
}
