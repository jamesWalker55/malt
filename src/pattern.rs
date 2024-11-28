//! Pattern module, represents a user-editable pattern thing.
//! Code based on: https://github.com/tiagolr/gate1

use nih_plug::{nih_debug_assert_failure, nih_error};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CurveType {
    Hold,
    Curve,
    SCurve,
}

impl CurveType {
    fn get_y(p1: &Point, p2: &Point, x: f64) -> f64 {
        match p1.kind {
            Self::Hold => p1.y,
            Self::Curve => {
                if p1.x == p2.x {
                    return p2.y;
                }

                let rise = p1.y > p2.y;
                let tmult = 0.0;
                let ten = (p1.tension + if rise { -tmult / 100.0 } else { tmult / 100.0 })
                    .clamp(-1.0, 1.0);
                let pwr = (ten * 50.0).abs().powf(1.1);

                if ten >= 0.0 {
                    ((x - p1.x) / (p2.x - p1.x)).powf(pwr) * (p2.y - p1.y) + p1.y
                } else {
                    -1.0 * ((1.0 - (x - p1.x) / (p2.x - p1.x)).powf(pwr) - 1.0) * (p2.y - p1.y)
                        + p1.y
                }
            }
            Self::SCurve => {
                if p1.x == p2.x {
                    return p2.y;
                }

                let rise = p1.y > p2.y;
                let tmult = 0.0;
                let ten = (p1.tension + if rise { -tmult / 100.0 } else { tmult / 100.0 })
                    .clamp(-1.0, 1.0);
                let pwr = (ten * 50.0).abs().powf(1.1);

                let xx = (p2.x + p1.x) / 2.0;
                let yy = (p2.y + p1.y) / 2.0;

                if x < xx && ten >= 0.0 {
                    return ((x - p1.x) / (xx - p1.x)).powf(pwr) * (yy - p1.y) + p1.y;
                }

                if x < xx && ten < 0.0 {
                    return -1.0 * ((1.0 - (x - p1.x) / (xx - p1.x)).powf(pwr) - 1.0) * (yy - p1.y)
                        + p1.y;
                }

                if x >= xx && ten >= 0.0 {
                    return -1.0 * ((1.0 - (x - xx) / (p2.x - xx)).powf(pwr) - 1.0) * (p2.y - yy)
                        + yy;
                }

                ((x - xx) / (p2.x - xx)).powf(pwr) * (p2.y - yy) + yy
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Point {
    x: f64,
    y: f64,
    tension: f64,
    kind: CurveType,
}

impl Point {
    pub(crate) fn new(x: f64, y: f64, tension: f64, kind: CurveType) -> Option<Self> {
        if !(0.0..=1.0).contains(&x) {
            return None;
        }
        if !(0.0..=1.0).contains(&y) {
            return None;
        }
        if !(-1.0..=1.0).contains(&tension) {
            return None;
        }

        Some(Self {
            x,
            y,
            tension,
            kind,
        })
    }
}

#[derive(Error, Debug)]
pub(crate) enum PatternError {
    #[error("operation causes conflict with start/end point")]
    EndPointConflict,
    #[error("the specified point is out of bounds")]
    PointOutOfBounds,
}

type Result<T, E = PatternError> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
pub(crate) struct Pattern {
    points: Vec<Point>,
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new(vec![
            Point::new(0.0, 0.0, 0.0, CurveType::Curve).unwrap(),
            Point::new(1.0, 1.0, 0.0, CurveType::Curve).unwrap(),
        ])
        .unwrap()
    }
}

impl Pattern {
    pub(crate) fn new(points: Vec<Point>) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }

        // validate last point, must be at end x=1.0
        if points.last().unwrap().x != 1.0 {
            return None;
        }

        // validate first point, must be at start x=0.0
        if points.first().unwrap().x != 0.0 {
            return None;
        }

        // points must be sorted
        if points
            .as_slice()
            .windows(2)
            .any(|slice| slice[0].x > slice[1].x)
        {
            return None;
        }

        Some(Self { points })
    }

    pub(crate) fn insert_point(&mut self, p: Point) -> usize {
        // insert point, keeping the list sorted
        // if multiple points have the same x pos, insert at last of those points
        match self.points.iter().rposition(|p2| p2.x <= p.x) {
            Some(prev_pos) => {
                if prev_pos == self.len() - 1 {
                    // overlaps with rightmost point
                    // place it just before the last point
                    self.points.insert(prev_pos, p);
                    prev_pos
                // } else if prev_pos == 0 {
                //     // overlaps with leftmost point
                //     // place it just after the first point
                //     self.points.insert(1, p);
                //     1
                } else {
                    // mid point stuff
                    self.points.insert(prev_pos + 1, p);
                    prev_pos + 1
                }
            }
            None => {
                // points must be in range 0.0--1.0
                // and first point must be 0.0
                // so this branch should be impossible to occur
                nih_error!("inserted point is somehow out of bounds");
                self.points.insert(0, p);
                0
            }
        }
    }

    pub(crate) fn remove_point_at_pos(&mut self, x: f64, y: f64) {
        self.points.retain(|p| p.x != x || p.y != y);
    }

    /// Return number of points. Will always be at least 2.
    pub(crate) fn len(&self) -> usize {
        self.points.len()
    }

    pub(crate) fn remove_point(&mut self, i: usize) -> Result<()> {
        if i == 0 {
            return Err(PatternError::EndPointConflict);
        }
        if i == self.len() - 1 {
            return Err(PatternError::EndPointConflict);
        }

        if i < self.len() {
            self.points.remove(i);
            Ok(())
        } else {
            Err(PatternError::PointOutOfBounds)
        }
    }

    pub(crate) fn remove_points_in_range(&mut self, x1: f64, x2: f64) {
        let mut mid_points: Vec<_> = self.points[1..(self.points.len() - 1)].iter().collect();
        mid_points.retain(|p| x1 <= p.x && p.x <= x2);
        mid_points.insert(0, &self.points.first().unwrap());
        mid_points.push(&self.points.last().unwrap());

        // clone all points
        let new_points: Vec<_> = mid_points.iter().map(|p| (*p).clone()).collect();

        self.points = new_points;
    }

    #[inline(always)]
    fn invert_point(p: &mut Point) {
        p.y = 1.0 - p.y;
    }

    pub(crate) fn invert(&mut self) {
        for p in self.points.iter_mut() {
            Self::invert_point(p);
        }
    }

    #[inline(always)]
    fn reverse_point(p: &mut Point, next_point: &Point) {
        p.x = 1.0 - p.x;
        p.tension = next_point.tension * -1.0;
    }

    pub(crate) fn reverse(&mut self) {
        // reverse order of points
        self.points.reverse();

        // i have no idea how to get 2 points as mut at the same time
        let slice = self.points.as_mut_slice();
        for i in 0..(slice.len() - 1) {
            let [ref mut p1, _, ref mut p2] = &mut slice[i..(i + 1)] else {
                unreachable!("expected slice with exactly 2 elements");
            };
            Self::reverse_point(p1, p2);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.points = vec![
            Point::new(0.0, 0.5, 0.0, CurveType::Curve).unwrap(),
            Point::new(1.0, 0.5, 0.0, CurveType::Curve).unwrap(),
        ];
    }

    pub(crate) fn get_y_at(&self, x: f64) -> f64 {
        // handle mid points (except last mid-point)
        for i in 0..(self.points.len() - 1) {
            let p1 = self.points.get(i).unwrap();
            let p2 = self.points.get(i + 1).unwrap();
            if p1.x <= x && x <= p2.x {
                return CurveType::get_y(p1, p2, x);
            }
        }

        nih_debug_assert_failure!("called get_y_at with an out-of-bounds value: {}", x);
        unreachable!();
    }

    pub(crate) fn sine() -> Self {
        Self::new(vec![
            Point::new(0.0, 1.0, 0.33, CurveType::Curve).unwrap(),
            Point::new(0.25, 0.5, -0.33, CurveType::Curve).unwrap(),
            Point::new(0.5, 0.0, 0.33, CurveType::Curve).unwrap(),
            Point::new(0.75, 0.5, -0.33, CurveType::Curve).unwrap(),
            Point::new(1.0, 1.0, 0.0, CurveType::Curve).unwrap(),
        ])
        .unwrap()
    }

    pub(crate) fn triangle() -> Self {
        Self::new(vec![
            Point::new(0.0, 1.0, 0.0, CurveType::Curve).unwrap(),
            Point::new(0.5, 0.0, 0.0, CurveType::Curve).unwrap(),
            Point::new(1.0, 1.0, 0.0, CurveType::Curve).unwrap(),
        ])
        .unwrap()
    }
}
