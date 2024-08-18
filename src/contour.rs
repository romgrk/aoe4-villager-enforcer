use std::cmp::Ordering;
use imageproc::contours::Contour;
use geo::{Line, Point};
use geo::EuclideanDistance;
use geo::EuclideanLength;

const TOLERANCE: f64 = 10.0;

pub struct Square {
  pub points: [Point; 4],
  pub contour: Contour<i32>,
}

pub fn detect_squares(width: u32, height: u32, contours: &Vec<Contour<i32>>) -> Vec<Square> {
  let mut results = vec![];

  'outer: for (_index, contour) in contours.iter().enumerate() {
    let points = &contour.points;

    if points.len() < 150 {
      continue;
    }

    let mut y_count_by_index = vec![0u32; height as usize];
    let mut x_count_by_index = vec![0u32; width as usize];

    for point in points.iter() {
      x_count_by_index[point.x as usize] += 1;
      y_count_by_index[point.y as usize] += 1;
    }

    let mut x_counts =
      x_count_by_index.into_iter()
        .enumerate()
        .collect::<Vec<_>>();
    x_counts.sort_by(|(_, a), (_, b)| if a > b { Ordering::Less } else { Ordering::Greater });

    let mut y_counts =
      y_count_by_index.into_iter()
        .enumerate()
        .collect::<Vec<_>>();
    y_counts.sort_by(|(_, a), (_, b)| if a > b { Ordering::Less } else { Ordering::Greater });

    if x_counts.len() < 3 || y_counts.len() < 3 {
      continue
    }

    if ratio(x_counts[1].1 as f64, x_counts[2].1 as f64) > 0.35
    || ratio(y_counts[1].1 as f64, y_counts[2].1 as f64) > 0.35 {
      continue;
    }

    let x_left   = std::cmp::min(x_counts[0].0, x_counts[1].0) as f64;
    let x_right  = std::cmp::max(x_counts[0].0, x_counts[1].0) as f64;
    let y_top    = std::cmp::min(y_counts[0].0, y_counts[1].0) as f64;
    let y_bottom = std::cmp::max(y_counts[0].0, y_counts[1].0) as f64;

    let top    = Line::new(Point::new(x_left,  y_top),    Point::new(x_right, y_top));
    let bottom = Line::new(Point::new(x_left,  y_bottom), Point::new(x_right, y_bottom));
    let left   = Line::new(Point::new(x_left,  y_top),    Point::new(x_left,  y_bottom));
    let right  = Line::new(Point::new(x_right, y_top),    Point::new(x_right, y_bottom));

    for point in points.iter() {
      let p = Point::new(point.x as f64, point.y as f64);

      let dt = top.euclidean_distance(&p);
      let db = bottom.euclidean_distance(&p);
      let dl = left.euclidean_distance(&p);
      let dr = right.euclidean_distance(&p);

      if dt > TOLERANCE
        && db > TOLERANCE
        && dl > TOLERANCE
        && dr > TOLERANCE
      {
        continue 'outer;
      }
    }

    let min_ratio = 0.98;
    if   line_length_ratio(top, bottom) < min_ratio
      || line_length_ratio(top, left)   < min_ratio
      || line_length_ratio(top, right)  < min_ratio
    {
      continue;
    }

    results.push(Square {
      points: [
        Point::new(x_left, y_top),
        Point::new(x_right, y_top),
        Point::new(x_right, y_bottom),
        Point::new(x_left, y_bottom),
      ],
      contour: contour.to_owned(),
    });
  }

  results.sort_by(|a, b| if a.points[0].y() < b.points[0].y() { Ordering::Less } else { Ordering::Greater });

  return results;
}

fn line_length_ratio(a: Line, b: Line) -> f64 {
  return ratio(a.euclidean_length(), b.euclidean_length());
}

fn ratio(a: f64, b: f64) -> f64 {
  let r = f64::min(a, b) / f64::max(a, b);
  return r;
}
