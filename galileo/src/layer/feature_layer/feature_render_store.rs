use crate::render::render_bundle::{RenderBundle, RenderPrimitive};
use crate::render::{Canvas, PackedBundle, PrimitiveId};
use galileo_types::cartesian::Point3d;
use galileo_types::impls::{Contour, Polygon};
use std::collections::{HashMap, HashSet};

pub(super) struct FeatureRenderStore {
    id: usize,
    min_resolution: f64,
    render_bundles: Vec<RenderBundle>,
    packed_bundles: Vec<Option<Box<dyn PackedBundle>>>,
    feature_render_map: HashMap<usize, RenderMapEntry>,
    buffer_size_limit: usize,
    bundle_indices_to_pack: HashSet<usize>,
    next_index: usize,
}

struct RenderMapEntry {
    bundle_index: usize,
    primitive_ids: Vec<PrimitiveId>,
}

impl FeatureRenderStore {
    pub fn new(id: usize, min_resolution: f64, buffer_size_limit: usize) -> Self {
        Self {
            id,
            min_resolution,
            buffer_size_limit,
            render_bundles: vec![],
            packed_bundles: vec![],
            feature_render_map: HashMap::new(),
            bundle_indices_to_pack: HashSet::new(),
            next_index: 0,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn min_resolution(&self) -> f64 {
        self.min_resolution
    }

    pub fn set_buffer_size_limit(&mut self, limit: usize) {
        self.buffer_size_limit = limit;
    }

    pub fn init_bundle(&mut self, f: impl Fn() -> RenderBundle) {
        if !self.has_not_full_bundles() {
            self.render_bundles.push(f());
            self.packed_bundles.push(None);
        }
    }

    fn has_not_full_bundles(&self) -> bool {
        self.render_bundles
            .iter()
            .any(|bundle| bundle.approx_buffer_size() < self.buffer_size_limit)
    }

    pub fn remove_render(&mut self, render_index: usize) {
        if let Some(RenderMapEntry {
            bundle_index,
            primitive_ids,
        }) = self.feature_render_map.remove(&render_index)
        {
            for id in primitive_ids {
                if let Err(err) = self.render_bundles[bundle_index].remove(id) {
                    log::warn!("Error while removing render primitive: {err:?}.")
                }
            }

            self.bundle_indices_to_pack.insert(bundle_index);
        } else {
            log::error!(
                "Tried to remove render index {render_index} that was not present in the map."
            );
        }
    }

    pub fn add_primitives(
        &mut self,
        primitives: Vec<RenderPrimitive<f64, Point3d, Contour<Point3d>, Polygon<Point3d>>>,
    ) -> usize {
        let curr_bundle_index = self.curr_bundle_index();
        let ids = primitives
            .into_iter()
            .map(|primitive| {
                self.render_bundles[curr_bundle_index].add(primitive, self.min_resolution)
            })
            .collect();

        let next_index = self.next_index;
        self.next_index += 1;

        self.feature_render_map.insert(
            next_index,
            RenderMapEntry {
                bundle_index: curr_bundle_index,
                primitive_ids: ids,
            },
        );

        self.bundle_indices_to_pack.insert(curr_bundle_index);

        next_index
    }

    fn curr_bundle_index(&self) -> usize {
        self.render_bundles.len() - 1
    }

    pub fn update_renders(
        &mut self,
        render_index: usize,
        primitives: Vec<RenderPrimitive<f64, Point3d, Contour<Point3d>, Polygon<Point3d>>>,
    ) {
        let RenderMapEntry {
            bundle_index,
            primitive_ids,
        } = &self.feature_render_map[&render_index];
        if primitive_ids.len() != primitives.len() {
            log::error!("Cannot update feature style. The number of primitives is not equal to what it was.")
        }

        for (id, primitive) in primitive_ids.iter().zip(primitives.into_iter()) {
            if let Err(err) = self.render_bundles[*bundle_index].update(*id, primitive) {
                log::warn!("Failed to update feature style: {err:?}");
            }
        }

        self.bundle_indices_to_pack.insert(*bundle_index);
    }

    pub fn pack(&mut self, canvas: &dyn Canvas) {
        for index in self.bundle_indices_to_pack.drain() {
            self.packed_bundles[index] = Some(canvas.pack_bundle(&self.render_bundles[index]));
        }
    }

    pub fn bundles(&self) -> Vec<&dyn PackedBundle> {
        self.packed_bundles
            .iter()
            .filter_map(|v| v.as_ref().map(|bundle| &**bundle))
            .collect()
    }
}
