use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, GeometrySpecialization};
use crate::geometry_type::{ContourGeometryType, GeometryType};
use crate::segment::Segment;

pub trait Contour {
    type Point;

    fn is_closed(&self) -> bool;

    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point>;

    fn iter_points_closing(&self) -> impl Iterator<Item = &Self::Point> {
        Box::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        ))
    }

    fn iter_segments(&self) -> impl Iterator<Item = Segment<'_, Self::Point>> {
        ContourSegmentIterator::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        ))
    }

    fn project_points<Proj>(
        &self,
        projection: &Proj,
    ) -> Option<crate::cartesian::impls::contour::Contour<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point>,
    {
        Some(crate::cartesian::impls::contour::Contour::new(
            self.iter_points()
                .map(|p| projection.project(p))
                .collect::<Option<Vec<Proj::OutPoint>>>()?,
            self.is_closed(),
        ))
    }
}

pub trait ClosedContour {
    type Point;
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

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let points = self
            .iter_points()
            .map(|p| projection.project(p))
            .collect::<Option<Vec<Proj::OutPoint>>>()?;
        Some(Geom::Contour(crate::cartesian::impls::contour::Contour {
            points,
            is_closed: true,
        }))
    }
}
