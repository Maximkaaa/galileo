//! Contour is a sequence of points.
//!
//! Contours can be:
//! * **open** - meaning that the first and the last points of the contour are not connected. For example, a road on
//!   the map can be represented as an open contour.
//! * **closed** - when the first and the last points of the contour are connected. For example, a shoreline can be
//!   represented as a closed contour.
//!
//! Both open and closed contours are represented by the [`Contour`] trait, but there is also a separate [`ClosedContour`]
//! traits for situations when only closed contour makes sense. For example, a [`Polygon`](super::Polygon) can
//! consist only of closed contours. All closed contours also implement the `Contour` trait automatically.
//!
//! # Contour vs OGC LineString
//!
//! In the OGC Simple Feature Access standard, the corresponding geometry type is called a `LineString`. There is
//! a important difference between them thought.
//!
//! `LineString` is considered to be closed when the first and the last points in the sequence are exactly same. `Contour`
//! does not have that requirement. Even more, it should not duplicate the first and the last points. `Contour` trait
//! deals with the last segment of closed contours with [`Contour::iter_points_closing`] and
//! [`Contour::iter_segments`] methods instead.

use crate::cartesian::{CartesianPoint2d, Rect};
use crate::geo::Projection;
use crate::geometry::{CartesianGeometry2dSpecialization, Geom, Geometry, GeometrySpecialization};
use crate::geometry_type::{CartesianSpace2d, ContourGeometryType, GeometryType};
use crate::segment::Segment;

/// Sequence of points. See module level documentation for details.
pub trait Contour {
    /// Type of the points the contour is consisted of.
    type Point;

    /// Whether the contour is closed.
    ///
    /// A closed contour has a segment connecting the last and the first points.
    fn is_closed(&self) -> bool;

    /// Iterate over the points of the contour.
    ///
    /// Note, that the last point shall not be the same as the first one even for the closed contours. If you want to
    /// include the first point at the end of iterator for closed contours, use [`Contour::iter_points_closing`]
    /// instead.
    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point>;

    /// Same as [`Contour::iter_points`] but for closed contours repeats the first point again at the end of the iterator.
    fn iter_points_closing(&self) -> impl Iterator<Item = &Self::Point> {
        Box::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        ))
    }

    /// Iterates over segments of the contour. For closed contours this includes the segment between the last and the
    /// first points of the contour.
    fn iter_segments(&self) -> impl Iterator<Item = Segment<'_, Self::Point>> {
        ContourSegmentIterator::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        ))
    }

    /// Project all the points of the contour with the given `projection`.
    fn project_points<Proj>(
        &self,
        projection: &Proj,
    ) -> Option<crate::impls::Contour<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        Some(crate::impls::Contour::new(
            self.iter_points()
                .map(|p| projection.project(p))
                .collect::<Option<Vec<Proj::OutPoint>>>()?,
            self.is_closed(),
        ))
    }
}

/// A closed contour. See module documentation for details.
pub trait ClosedContour {
    /// Type of the points the contour is consisted of.
    type Point;

    /// Iterate over the points of the contour.
    ///
    /// Note, that the last point shall not be the same as the first one even for the closed contours. If you want to
    /// include the first point at the end of iterator for closed contours, use [`Contour::iter_points_closing`]
    /// instead.
    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point>;
}

impl<P, T: ClosedContour<Point = P>> Contour for T {
    type Point = P;

    fn is_closed(&self) -> bool {
        true
    }

    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point> {
        self.iter_points()
    }
}

/// Iterator of contour points.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ContourPointsIterator<'a, P, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    points_iter: Iter,
    is_closed: bool,
    first_point: Option<&'a P>,
}

impl<'a, P: 'a, Iter> ContourPointsIterator<'a, P, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    fn new(points_iter: Iter, is_closed: bool) -> Self {
        Self {
            points_iter,
            is_closed,
            first_point: None,
        }
    }
}

impl<'a, P, Iter> Iterator for ContourPointsIterator<'a, P, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    type Item = &'a P;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.points_iter.next();
        if self.is_closed && self.first_point.is_none() {
            self.first_point = next;
        }

        if next.is_none() {
            self.first_point.take()
        } else {
            next
        }
    }
}

/// Iterator of contour segements.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ContourSegmentIterator<'a, P: 'a, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    points_iter: ContourPointsIterator<'a, P, Iter>,
    prev_point: Option<&'a P>,
}

impl<'a, P, Iter> ContourSegmentIterator<'a, P, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    fn new(points_iter: ContourPointsIterator<'a, P, Iter>) -> Self {
        Self {
            points_iter,
            prev_point: None,
        }
    }
}

impl<'a, P, Iter> Iterator for ContourSegmentIterator<'a, P, Iter>
where
    Iter: Iterator<Item = &'a P>,
{
    type Item = Segment<'a, P>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_point = self.points_iter.next()?;
        let prev_point = self.prev_point.replace(next_point);

        match prev_point {
            Some(prev) => Some(Segment(prev, next_point)),
            None => self.next(),
        }
    }
}

impl<C, Space> GeometrySpecialization<ContourGeometryType, Space> for C
where
    C: Contour + GeometryType<Type = ContourGeometryType, Space = Space>,
{
    type Point = C::Point;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let points = self
            .iter_points()
            .map(|p| projection.project(p))
            .collect::<Option<Vec<Proj::OutPoint>>>()?;
        Some(Geom::Contour(crate::impls::Contour::new(
            points,
            self.is_closed(),
        )))
    }
}

impl<P, C> CartesianGeometry2dSpecialization<P, ContourGeometryType> for C
where
    P: CartesianPoint2d,
    C: Contour<Point = P>
        + GeometryType<Type = ContourGeometryType, Space = CartesianSpace2d>
        + Geometry<Point = P>,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        self.iter_segments()
            .any(|segment| segment.distance_to_point_sq(point) <= tolerance * tolerance)
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>> {
        Rect::from_points(self.iter_points())
    }
}
