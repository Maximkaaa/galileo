use crate::layer::feature_layer::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{LineCap, LinePaint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::cartesian::traits::cartesian_point::NewCartesianPoint3d;
use galileo_types::contour::Contour;
use galileo_types::geo::impls::projection::dimensions::AddDimensionProjection;
use galileo_types::geo::impls::projection::identity::IdentityProjection;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::{CartesianSpace2d, CartesianSpace3d};
use galileo_types::multi_contour::MultiContour;
use std::marker::PhantomData;

pub struct SimpleContourSymbol<Space> {
    pub color: Color,
    pub width: f64,

    space: PhantomData<Space>,
}

impl<Space> SimpleContourSymbol<Space> {
    pub fn new(color: Color, width: f64) -> Self {
        Self {
            color,
            width,
            space: Default::default(),
        }
    }

    fn render_internal(
        &self,
        geometry: &galileo_types::cartesian::impls::contour::Contour<Point3d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        vec![bundle.add_line(
            geometry,
            LinePaint {
                color: self.color,
                width: self.width,
                offset: 0.0,
                line_cap: LineCap::Butt,
            },
            1000.0,
        )]
    }
}

// impl<F, C> Symbol<F, C> for SimpleContourSymbol<CartesianSpace2d>
// where
//     C: Contour,
//     C::Point: NewCartesianPoint2d,
// {
//     fn render(
//         &self,
//         _feature: &F,
//         geometry: &C,
//         bundle: &mut Box<dyn RenderBundle>,
//     ) -> Vec<PrimitiveId> {
//         let projection = AddDimensionProjection::new(0.0);
//         self.render_internal(&geometry.project_points(&projection).unwrap(), bundle)
//     }
//
//     fn update(
//         &self,
//         _feature: &F,
//         _renders_ids: &[PrimitiveId],
//         _bundle: &mut Box<dyn UnpackedBundle>,
//     ) {
//         todo!()
//     }
// }

impl<F> Symbol<F, Geom<Point2d>> for SimpleContourSymbol<CartesianSpace2d> {
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Contour(contour) => {
                let projection = AddDimensionProjection::new(0.0);
                self.render_internal(&contour.project_points(&projection).unwrap(), bundle)
            }
            Geom::MultiContour(contours) => {
                let projection = AddDimensionProjection::new(0.0);
                contours
                    .contours()
                    .flat_map(|c| {
                        self.render_internal(&c.project_points(&projection).unwrap(), bundle)
                    })
                    .collect()
            }
            _ => vec![],
        }
    }
}

impl<F, C> Symbol<F, C> for SimpleContourSymbol<CartesianSpace3d>
where
    C: Contour,
    C::Point: NewCartesianPoint3d,
{
    fn render(
        &self,
        _feature: &F,
        geometry: &C,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let projection = IdentityProjection::<C::Point, Point3d, CartesianSpace3d>::new();
        let id = bundle.add_line(
            &geometry.project_points(&projection).unwrap(),
            LinePaint {
                color: self.color,
                width: self.width,
                offset: 0.0,
                line_cap: LineCap::Butt,
            },
            10000.0,
        );

        vec![id]
    }

    fn update(
        &self,
        _feature: &F,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}
