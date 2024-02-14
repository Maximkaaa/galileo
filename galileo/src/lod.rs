use serde::{Deserialize, Serialize};

/// Level of detail of a layer or map.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Lod {
    resolution: f64,
    z_index: u32,
}

impl Lod {
    /// Creates new level of detail.
    pub fn new(resolution: f64, z_index: u32) -> Option<Lod> {
        if resolution.is_finite() && resolution != 0.0 {
            Some(Self {
                resolution,
                z_index,
            })
        } else {
            None
        }
    }

    /// Z-index associated with this LOD.
    pub fn z_index(&self) -> u32 {
        self.z_index
    }

    /// Resolution of the LOD.
    pub fn resolution(&self) -> f64 {
        self.resolution
    }
}

impl PartialEq for Lod {
    fn eq(&self, other: &Self) -> bool {
        self.resolution == other.resolution
    }
}

impl Eq for Lod {}

impl PartialOrd for Lod {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Lod {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.resolution
            .partial_cmp(&other.resolution)
            .unwrap_or_else(|| self.z_index.cmp(&other.z_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_comparison() {
        assert_eq!(Lod::new(1.0, 1), Lod::new(1.0, 1));
        assert_eq!(Lod::new(1.0, 1), Lod::new(1.0, 2));
        assert!(Lod::new(2.0, 1) > Lod::new(1.0, 1));
        assert!(Lod::new(2.0, 1) < Lod::new(4.0, 1));
    }

    #[test]
    fn invalid_lod_creation() {
        assert!(Lod::new(1.0, 1).is_some());
        assert!(Lod::new(0.0, 1).is_none());
        assert!(Lod::new(f64::NAN, 1).is_none());
        assert!(Lod::new(f64::INFINITY, 1).is_none());
        assert!(Lod::new(f64::NEG_INFINITY, 1).is_none());
    }
}
