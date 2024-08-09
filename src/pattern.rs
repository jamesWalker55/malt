//! Pattern module, represents a user-editable pattern thing.
//! Code based on: https://github.com/tiagolr/gate1

use core::f64::consts as C;

use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CurveType {
    Hold,
    Curve,
    SCurve,
    Pulse,
    Wave,
    Triangle,
    Stairs,
    SmoothStairs,
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
            Self::Pulse => {
                let t = (p1.tension.powi(2) * 100.0).floor().max(1.0); // num waves

                if x == p2.x {
                    return p2.y;
                }

                let cycle_width = (p2.x - p1.x) / t;
                let x_in_cycle = (x - p1.x) % cycle_width;

                if x_in_cycle < cycle_width / 2.0 {
                    if p1.tension >= 0.0 {
                        p1.y
                    } else {
                        p2.y
                    }
                } else if p1.tension >= 0.0 {
                    p2.y
                } else {
                    p1.y
                }
            }
            Self::Wave => {
                let t = 2.0 * ((p1.tension.powi(2) * 100.0).abs() + 1.0).floor() - 1.0; // wave num
                let amp = (p2.y - p1.y) / 2.0;
                let vshift = p1.y + amp;
                let freq = t * 2.0 * C::PI / (2.0 * (p2.x - p1.x));
                -amp * (freq * (x - p1.x)).cos() + vshift
            }
            Self::Triangle => {
                let tt = 2.0 * ((p1.tension.powi(2) * 100.0).abs() + 1.0).floor() - 1.0; // wave num
                let amp = p2.y - p1.y;
                let t = (p2.x - p1.x) * 2.0 / tt;
                amp * (2.0 * ((x - p1.x) / t - (1.0 / 2.0 + (x - p1.x) / t).floor()).abs()) + p1.y
            }
            Self::Stairs => {
                let t = (p1.tension.powi(2) * 150.0).floor().max(2.0); // num waves
                let step_size;
                let step_index;
                let y_step_size;

                if p1.tension >= 0.0 {
                    step_size = (p2.x - p1.x) / t;
                    step_index = ((x - p1.x) / step_size).floor();
                    y_step_size = (p2.y - p1.y) / (t - 1.0);
                } else {
                    step_size = (p2.x - p1.x) / (t - 1.0);
                    step_index = ((x - p1.x) / step_size).ceil();
                    y_step_size = (p2.y - p1.y) / t;
                }

                if x == p2.x {
                    return p2.y;
                }

                p1.y + step_index * y_step_size
            }
            Self::SmoothStairs => {
                let pwr = 4;
                let t = (p1.tension.powi(2) * 150.0).floor().max(1.0); // num waves

                let gx = (p2.x - p1.x) / t; // gridx
                let gy = (p2.y - p1.y) / t; // gridy
                let step_index = ((x - p1.x) / gx).floor();

                let xx1 = p1.x + gx * step_index;
                let xx2 = p1.x + gx * (step_index + 1.0);
                let xx = (xx1 + xx2) / 2.0;

                let yy1 = p1.y + gy * step_index;
                let yy2 = p1.y + gy * (step_index + 1.0);
                let yy = (yy1 + yy2) / 2.0;

                if p1.x == p2.x {
                    return p2.y;
                }

                if x < xx && p1.tension >= 0.0 {
                    return ((x - xx1) / (xx - xx1)).powi(pwr) * (yy - yy1) + yy1;
                }

                if x < xx && p1.tension < 0.0 {
                    return -1.0 * ((1.0 - (x - xx1) / (xx - xx1)).powi(pwr) - 1.0) * (yy - yy1)
                        + yy1;
                }

                if x >= xx && p1.tension >= 0.0 {
                    return -1.0 * ((1.0 - (x - xx) / (xx2 - xx)).powi(pwr) - 1.0) * (yy2 - yy)
                        + yy;
                }

                ((x - xx) / (xx2 - xx)).powi(pwr) * (yy2 - yy) + yy
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
    first_point: Point,
    last_point: Point,
    mid_points: Vec<Point>,
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
    pub(crate) fn new(mut points: Vec<Point>) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }

        // validate last point, must be at end
        let last_point = points.pop().unwrap();
        if last_point.x != 1.0 {
            return None;
        }

        // validate first point, must be at start
        let first_point = points.remove(0);
        if first_point.x != 0.0 {
            return None;
        }

        // points must be sorted
        if points
            .as_slice()
            .windows(2)
            .any(|slice| &slice[0].x > &slice[1].x)
        {
            return None;
        }

        Some(Self {
            first_point,
            last_point,
            mid_points: points,
            // segments: vec![],
        })
    }

    pub(crate) fn insert_point(&mut self, p: Point) -> usize {
        // insert point, keeping the list sorted
        // if multiple points have the same x pos, insert at last of those points
        match self.mid_points.iter().rposition(|p2| p2.x <= p.x) {
            Some(prev_pos) => {
                self.mid_points.insert(prev_pos + 1, p);
                prev_pos + 1
            }
            None => {
                // this is the leftmost point (except the first point)
                self.mid_points.insert(0, p);
                0
            }
        }
    }

    pub(crate) fn remove_point_at_pos(&mut self, x: f64, y: f64) {
        self.mid_points.retain(|p| p.x != x || p.y != y);
    }

    /// Return number of points. Will always be at least 2.
    pub(crate) fn len(&self) -> usize {
        self.mid_points.len() + 2
    }

    pub(crate) fn remove_point(&mut self, mut i: usize) -> Result<()> {
        if i == 0 {
            return Err(PatternError::EndPointConflict);
        }
        if i == self.len() - 1 {
            return Err(PatternError::EndPointConflict);
        }

        // decrement i by 1 to offset by starting point
        i -= 1;

        if i < self.mid_points.len() {
            self.mid_points.remove(i);
            Ok(())
        } else {
            Err(PatternError::PointOutOfBounds)
        }
    }

    pub(crate) fn remove_points_in_range(&mut self, x1: f64, x2: f64) {
        self.mid_points.retain(|p| x1 <= p.x && p.x <= x2);
    }

    #[inline(always)]
    fn invert_point(p: &mut Point) {
        p.y = 1.0 - p.y;
    }

    pub(crate) fn invert(&mut self) {
        Self::invert_point(&mut self.first_point);
        Self::invert_point(&mut self.last_point);
        for p in self.mid_points.iter_mut() {
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
        std::mem::swap(&mut self.first_point, &mut self.last_point);
        self.mid_points.reverse();

        // update x position of points (and tension)
        if self.mid_points.is_empty() {
            // just first and last points
            Self::reverse_point(&mut self.first_point, &self.last_point);
        } else {
            // has at least 1 mid point

            // process first point
            Self::reverse_point(&mut self.first_point, self.mid_points.first().unwrap());

            // process mid points, except last mid-point
            {
                // i have no idea how to get 2 points as mut at the same time
                let slice = self.mid_points.as_mut_slice();
                for i in 0..(slice.len() - 1) {
                    let [ref mut p1, _, ref mut p2] = &mut slice[i..(i + 1)] else {
                        unreachable!("expected slice with exactly 2 elements");
                    };
                    Self::reverse_point(p1, p2);
                }

                // finally process the last mid-point
                let second_last_point = &mut slice[slice.len() - 1];
                Self::reverse_point(second_last_point, &self.last_point);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.mid_points.clear();
        self.first_point = Point::new(0.0, 0.5, 0.0, CurveType::Curve).unwrap();
        self.last_point = Point::new(1.0, 0.5, 0.0, CurveType::Curve).unwrap();
    }

    pub(crate) fn get_y_at(&self, x: f64) -> f64 {
        if self.mid_points.is_empty() {
            // just start / end points
            let p1 = &self.first_point;
            let p2 = &self.last_point;
            return CurveType::get_y(p1, p2, x);
        }

        // handle start point
        {
            let p1 = &self.first_point;
            let p2 = self.mid_points.first().unwrap();
            if p1.x <= x && x <= p2.x {
                return CurveType::get_y(p1, p2, x);
            }
        }

        // handle mid points (except last mid-point)
        for i in 0..(self.mid_points.len() - 1) {
            let p1 = self.mid_points.get(i).unwrap();
            let p2 = self.mid_points.get(i + 1).unwrap();
            if p1.x <= x && x <= p2.x {
                return CurveType::get_y(p1, p2, x);
            }
        }

        // handle last mid-point
        {
            let p1 = self.mid_points.last().unwrap();
            let p2 = &self.last_point;
            if p1.x <= x && x <= p2.x {
                return CurveType::get_y(p1, p2, x);
            }
        }

        panic!("called get_y_at with an out-of-bounds value: {}", x);
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
