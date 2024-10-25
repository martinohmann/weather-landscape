use flo_curves::bezier::{BezierCurveFactory, Coord2, Curve};
use imageproc::drawing::BresenhamLineIter;
use std::collections::BTreeMap;

pub fn fit_curve_to_points(points: &[Coord2], max_error: f64) -> BTreeMap<i64, i64> {
    let curves = Curve::fit_from_points(points, max_error).unwrap_or_default();

    let mut points = BTreeMap::new();

    for curve in curves {
        collect_cubic_bezier_curve_points(
            curve.start_point.into(),
            curve.end_point.into(),
            curve.control_points.0.into(),
            curve.control_points.1.into(),
            &mut points,
        );
    }

    points
}

fn collect_cubic_bezier_curve_points(
    start: (f32, f32),
    end: (f32, f32),
    control_a: (f32, f32),
    control_b: (f32, f32),
    points: &mut BTreeMap<i64, i64>,
) {
    // Bezier Curve function from: https://pomax.github.io/bezierinfo/#control
    let cubic_bezier_curve = |t: f32| {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let x = (start.0 * mt3)
            + (3.0 * control_a.0 * mt2 * t)
            + (3.0 * control_b.0 * mt * t2)
            + (end.0 * t3);
        let y = (start.1 * mt3)
            + (3.0 * control_a.1 * mt2 * t)
            + (3.0 * control_b.1 * mt * t2)
            + (end.1 * t3);
        (x.round(), y.round()) // round to nearest pixel, to avoid ugly line artifacts
    };

    let distance = |point_a: (f32, f32), point_b: (f32, f32)| {
        ((point_a.0 - point_b.0).powi(2) + (point_a.1 - point_b.1).powi(2)).sqrt()
    };

    // Approximate curve's length by adding distance between control points.
    let curve_length_bound: f32 =
        distance(start, control_a) + distance(control_a, control_b) + distance(control_b, end);

    // Use hyperbola function to give shorter curves a bias in number of line segments.
    let num_segments: i32 = ((curve_length_bound.powi(2) + 800.0).sqrt() / 8.0) as i32;

    // Sample points along the curve and connect them with line segments.
    let t_interval = 1f32 / (num_segments as f32);

    let mut t1 = 0f32;
    for i in 0..num_segments {
        let t2 = (i as f32 + 1.0) * t_interval;

        let start = cubic_bezier_curve(t1);
        let end = cubic_bezier_curve(t2);

        let line_points = BresenhamLineIter::new(start, end);

        for (x, y) in line_points {
            points.insert(x as i64, y as i64);
        }
        t1 = t2;
    }
}
