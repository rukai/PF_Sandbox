/// Checks if segment p1q1 intersects with segment p2q2
/// Implemented as described here http://www.geeksforgeeks.org/check-if-two-given-line-segments-intersect/
pub fn segments_intersect(p1: (f32, f32), q1: (f32, f32), p2: (f32, f32), q2: (f32, f32)) -> bool {
    let o1 = triplet_orientation(p1, q1, p2);
    let o2 = triplet_orientation(p1, q1, q2);
    let o3 = triplet_orientation(p2, q2, p1);
    let o4 = triplet_orientation(p2, q2, q1);

    // general case
    (o1 != o2 && o3 != o4) ||
    // colinear cases
    (o1 == 0 && point_on_segment(p1, p2, q1)) ||
    (o2 == 0 && point_on_segment(p1, q2, q1)) ||
    (o3 == 0 && point_on_segment(p2, p1, q2)) ||
    (o4 == 0 && point_on_segment(p2, q1, q2))
}

/// Returns the orientation of triplet (p, q, r)
/// 0 - colinear
/// 1 - clockwise
/// 2 - counter clockwise
fn triplet_orientation(p: (f32, f32), q: (f32, f32), r: (f32, f32)) -> u8 {
    let val = (q.1 - p.1) * (r.0 - q.0) -
              (q.0 - p.0) * (r.1 - q.1);

    if val > 0.0 {
        1
    } else if val < 0.0 {
        2
    } else {
        0
    }
}

/// Given p, q, r are colinear,
/// checks if point q lies on segment pr
fn point_on_segment(p: (f32, f32), q: (f32, f32), r: (f32, f32)) -> bool {
    q.0 <= p.0.max(r.0) && q.0 >= p.0.min(r.0) && 
    q.1 <= p.1.max(r.1) && q.1 >= p.1.min(r.1)
}
