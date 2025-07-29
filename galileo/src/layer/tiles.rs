use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use crate::render::PackedBundle;
use crate::tile_schema::TileIndex;
use crate::TileSchema;

#[derive(Clone)]
pub struct RenderedState {
    pub bundle: Arc<dyn PackedBundle>,
    /// tile was already rendered at the map before.
    pub rendered_before: bool,
}

#[derive(Clone)]
pub(crate) struct DisplayedTile<StyleId: Copy> {
    index: TileIndex,
    pub(crate) bundle: Arc<dyn PackedBundle>,
    style_id: StyleId,
    pub(crate) opacity: f32,
    displayed_at: web_time::Instant,
}

impl<StyleId: Copy + PartialEq> PartialEq for DisplayedTile<StyleId> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.style_id == other.style_id
    }
}

impl<StyleId: Copy> DisplayedTile<StyleId> {
    pub(crate) fn is_opaque(&self) -> bool {
        self.opacity >= 0.999
    }
}

pub(crate) trait TileProvider<StyleId> {
    #[allow(dead_code)]
    fn get_tile(&self, index: TileIndex, style_id: StyleId) -> Option<Arc<dyn PackedBundle>>;

    fn get_rendered_state(&self, index: TileIndex, style_id: StyleId) -> Option<RenderedState>;
    fn set_rendered_before(&self, index: TileIndex, style_id: StyleId);
}

pub(crate) struct TilesContainer<StyleId, Provider>
where
    StyleId: Copy,
    Provider: TileProvider<StyleId>,
{
    pub(crate) tiles: Mutex<Vec<DisplayedTile<StyleId>>>,
    tile_schema: TileSchema,
    pub(crate) tile_provider: Provider,
}

impl<StyleId, Provider> TilesContainer<StyleId, Provider>
where
    StyleId: Copy + PartialEq,
    Provider: TileProvider<StyleId>,
{
    pub(crate) fn new(tile_schema: TileSchema, tile_provider: Provider) -> Self {
        Self {
            tiles: Default::default(),
            tile_schema,
            tile_provider,
        }
    }

    pub(crate) fn update_displayed_tiles(
        &self,
        needed_indices: impl IntoIterator<Item = TileIndex>,
        style_id: StyleId,
    ) -> bool {
        let mut displayed_tiles = self.tiles.lock();

        let mut needed_tiles = vec![];
        let mut to_substitute = vec![];

        let now = web_time::Instant::now();
        let fade_in_time = self.fade_in_time();
        let mut requires_redraw = false;

        for index in needed_indices {
            if let Some(displayed) = displayed_tiles
                .iter_mut()
                .find(|displayed| displayed.index == index && displayed.style_id == style_id)
            {
                // fade tiles in
                if !displayed.is_opaque() {
                    to_substitute.push(index);
                    displayed.opacity = ((now.duration_since(displayed.displayed_at)).as_secs_f64()
                        / fade_in_time.as_secs_f64())
                    .min(1.0) as f32;
                    requires_redraw = true;
                } else {
                    // Tile is fully opaque, mark it as rendered
                    self.tile_provider
                        .set_rendered_before(displayed.index, style_id);
                }

                needed_tiles.push(displayed.clone());
            } else {
                match self.tile_provider.get_rendered_state(index, style_id) {
                    None => to_substitute.push(index),
                    Some(RenderedState {
                        bundle,
                        rendered_before,
                    }) => {
                        let opacity = if rendered_before { 1.0 } else { 0.0 };

                        needed_tiles.push(DisplayedTile {
                            index,
                            bundle,
                            style_id,
                            opacity,
                            displayed_at: now,
                        });

                        if !rendered_before {
                            to_substitute.push(index);
                            requires_redraw = true;
                        }
                    }
                }
            }
        }

        let mut new_displayed = vec![];
        for displayed in displayed_tiles.iter() {
            if needed_tiles.iter().any(|new| new == displayed) {
                continue;
            }

            let Some(displayed_bbox) = self.tile_schema.tile_bbox(displayed.index) else {
                continue;
            };

            for subst in &to_substitute {
                let Some(subst_bbox) = self.tile_schema.tile_bbox(*subst) else {
                    continue;
                };

                if displayed_bbox.intersects(subst_bbox) {
                    new_displayed.push(displayed.clone());
                    break;
                }
            }
        }

        new_displayed.append(&mut needed_tiles);
        *displayed_tiles = new_displayed;

        requires_redraw
    }

    fn fade_in_time(&self) -> Duration {
        Duration::from_millis(300)
    }
}
