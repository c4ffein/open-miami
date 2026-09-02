use crate::ecs::world::Wall;
use crate::math::Vec2;

pub fn circle_circle_collision(pos1: Vec2, radius1: f32, pos2: Vec2, radius2: f32) -> bool {
    let distance = pos1.distance(pos2);
    distance < radius1 + radius2
}

/// Swept circle-vs-circle: does a circle of `radius` moving from `from` to
/// `to` touch the circle at `center` at any point along the way? The segment
/// `from -> to` against the target inflated by `radius` — an endpoint-only
/// check can tunnel straight through a target when the closing speed exceeds
/// the combined radii per frame (a bullet meeting a rushing bot head-on).
pub fn swept_circle_circle_collision(
    from: Vec2,
    to: Vec2,
    radius: f32,
    center: Vec2,
    target_radius: f32,
) -> bool {
    let r = radius + target_radius;
    let d = Vec2::new(to.x - from.x, to.y - from.y);
    let f = Vec2::new(from.x - center.x, from.y - center.y);
    let len2 = d.x * d.x + d.y * d.y;
    if len2 <= f32::EPSILON {
        return circle_circle_collision(from, radius, center, target_radius);
    }
    // Closest point of the segment to the centre, clamped to [0, 1].
    let t = (-(f.x * d.x + f.y * d.y) / len2).clamp(0.0, 1.0);
    let cx = from.x + d.x * t - center.x;
    let cy = from.y + d.y * t - center.y;
    cx * cx + cy * cy < r * r
}

pub fn circle_rect_collision(
    circle_pos: Vec2,
    radius: f32,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
) -> bool {
    // Find the closest point on the rectangle to the circle
    let closest_x = circle_pos.x.max(rect_x).min(rect_x + rect_w);
    let closest_y = circle_pos.y.max(rect_y).min(rect_y + rect_h);

    let distance = ((circle_pos.x - closest_x).powi(2) + (circle_pos.y - closest_y).powi(2)).sqrt();
    distance < radius
}

/// Swept circle-vs-rect: does a circle of `radius` moving from `from` to `to`
/// touch the rectangle at any point along the way? Implemented as the segment
/// `from -> to` against the rectangle inflated by `radius` (slightly generous
/// at the corners, which is fine for small, fast projectiles). Unlike an
/// endpoint-only check this cannot tunnel through a wall thinner than one
/// frame's travel.
pub fn swept_circle_rect_collision(
    from: Vec2,
    to: Vec2,
    radius: f32,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
) -> bool {
    let ix = rect_x - radius;
    let iy = rect_y - radius;
    let iw = rect_w + radius * 2.0;
    let ih = rect_h + radius * 2.0;
    point_in_rect(from, ix, iy, iw, ih)
        || point_in_rect(to, ix, iy, iw, ih)
        || line_rect_intersection(from, to, ix, iy, iw, ih)
}

pub fn point_in_rect(point: Vec2, rect_x: f32, rect_y: f32, rect_w: f32, rect_h: f32) -> bool {
    point.x >= rect_x
        && point.x <= rect_x + rect_w
        && point.y >= rect_y
        && point.y <= rect_y + rect_h
}

/// Do the segments `p1 -> p2` and `p3 -> p4` intersect? Touching at an
/// endpoint counts. Collinear segments intersect only when they overlap —
/// the orientation test alone (all four cross products zero) would report
/// any two segments on one line as crossing, so a bot standing on the
/// extension line of a wall's edge would lose line of sight to a player
/// further along that same line.
pub(crate) fn line_segment_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> bool {
    let d1 = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x);
    let d2 = (p2.x - p1.x) * (p4.y - p1.y) - (p2.y - p1.y) * (p4.x - p1.x);
    let d3 = (p4.x - p3.x) * (p1.y - p3.y) - (p4.y - p3.y) * (p1.x - p3.x);
    let d4 = (p4.x - p3.x) * (p2.y - p3.y) - (p4.y - p3.y) * (p2.x - p3.x);

    if d1 == 0.0 && d2 == 0.0 && d3 == 0.0 && d4 == 0.0 {
        // All four points on one line: a 1-D overlap test along the axis the
        // line leans on most (the other axis can be constant).
        let dx = (p2.x - p1.x).abs().max((p4.x - p3.x).abs());
        let dy = (p2.y - p1.y).abs().max((p4.y - p3.y).abs());
        let (a0, a1, b0, b1) = if dx >= dy {
            (p1.x, p2.x, p3.x, p4.x)
        } else {
            (p1.y, p2.y, p3.y, p4.y)
        };
        return a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1);
    }

    // `<=`: lines touching at an endpoint count.
    d1 * d2 <= 0.0 && d3 * d4 <= 0.0
}

/// Check if a line segment intersects with a rectangle
fn line_rect_intersection(
    line_start: Vec2,
    line_end: Vec2,
    rect_x: f32,
    rect_y: f32,
    rect_w: f32,
    rect_h: f32,
) -> bool {
    // Check if line intersects any of the 4 edges of the rectangle
    let top_left = Vec2::new(rect_x, rect_y);
    let top_right = Vec2::new(rect_x + rect_w, rect_y);
    let bottom_left = Vec2::new(rect_x, rect_y + rect_h);
    let bottom_right = Vec2::new(rect_x + rect_w, rect_y + rect_h);

    // Check intersection with each edge
    line_segment_intersection(line_start, line_end, top_left, top_right)
        || line_segment_intersection(line_start, line_end, top_right, bottom_right)
        || line_segment_intersection(line_start, line_end, bottom_right, bottom_left)
        || line_segment_intersection(line_start, line_end, bottom_left, top_left)
}

/// Check if there's a clear line of sight between two points (no walls blocking)
pub fn has_line_of_sight(from: Vec2, to: Vec2, walls: &[Wall]) -> bool {
    for wall in walls {
        if line_rect_intersection(from, to, wall.x, wall.y, wall.width, wall.height) {
            return false; // Wall blocks line of sight
        }
    }
    true // No walls blocking
}

/// Check if there's a clear line of sight with inflated wall boundaries
/// This is used to decide between direct movement and pathfinding
/// Walls are expanded by padding on all sides to prevent enemies from trying
/// to move directly toward targets that are very close to walls
pub fn has_line_of_sight_with_padding(from: Vec2, to: Vec2, walls: &[Wall], padding: f32) -> bool {
    for wall in walls {
        // Inflate wall boundaries by padding amount
        let inflated_x = wall.x - padding;
        let inflated_y = wall.y - padding;
        let inflated_w = wall.width + padding * 2.0;
        let inflated_h = wall.height + padding * 2.0;

        if line_rect_intersection(from, to, inflated_x, inflated_y, inflated_w, inflated_h) {
            return false; // Inflated wall blocks line of sight
        }
    }
    true // No inflated walls blocking
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_rect_overlap_edge_corner_and_miss() {
        // Centre inside the rect.
        assert!(circle_rect_collision(
            Vec2::new(50.0, 50.0),
            1.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
        // Overlapping the right edge from outside.
        assert!(circle_rect_collision(
            Vec2::new(105.0, 50.0),
            10.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
        // Exactly tangent to the edge: touching is not overlapping (`<`).
        assert!(!circle_rect_collision(
            Vec2::new(110.0, 50.0),
            10.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
        // The corner is round: diagonal (7, 7) from the corner is ~9.9 away.
        assert!(circle_rect_collision(
            Vec2::new(107.0, 107.0),
            10.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
        // ... but (8, 8) is ~11.3 away, past the radius, even though the
        // circle's bounding box still overlaps the rect's.
        assert!(!circle_rect_collision(
            Vec2::new(108.0, 108.0),
            10.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
        // Far away.
        assert!(!circle_rect_collision(
            Vec2::new(300.0, 50.0),
            10.0,
            0.0,
            0.0,
            100.0,
            100.0
        ));
    }

    #[test]
    fn swept_circle_rect_catches_a_thin_wall_between_the_endpoints() {
        // A 2 u thin wall; the bullet's endpoints are 50 u either side of it.
        let (wx, wy, ww, wh) = (100.0, 0.0, 2.0, 100.0);
        let from = Vec2::new(50.0, 50.0);
        let to = Vec2::new(150.0, 50.0);
        // The endpoint-only test misses it...
        assert!(!circle_rect_collision(from, 3.0, wx, wy, ww, wh));
        assert!(!circle_rect_collision(to, 3.0, wx, wy, ww, wh));
        // ... the sweep does not.
        assert!(swept_circle_rect_collision(from, to, 3.0, wx, wy, ww, wh));
        // Diagonal crossings too.
        assert!(swept_circle_rect_collision(
            Vec2::new(60.0, 10.0),
            Vec2::new(140.0, 90.0),
            3.0,
            wx,
            wy,
            ww,
            wh
        ));
        // A path beside the wall (past its end) stays clear.
        assert!(!swept_circle_rect_collision(
            Vec2::new(50.0, 120.0),
            Vec2::new(150.0, 120.0),
            3.0,
            wx,
            wy,
            ww,
            wh
        ));
        // Within the radius of the wall's end it clips it (the inflation).
        assert!(swept_circle_rect_collision(
            Vec2::new(50.0, 102.0),
            Vec2::new(150.0, 102.0),
            3.0,
            wx,
            wy,
            ww,
            wh
        ));
        // Either endpoint inside counts.
        assert!(swept_circle_rect_collision(
            from,
            Vec2::new(101.0, 50.0),
            3.0,
            wx,
            wy,
            ww,
            wh
        ));
        // A zero-length sweep degrades to the point test.
        assert!(!swept_circle_rect_collision(
            from, from, 3.0, wx, wy, ww, wh
        ));
    }

    #[test]
    fn swept_circle_circle_catches_head_on_closing_faster_than_the_radii() {
        // A radius-2 bullet jumping 100 u through a radius-5 bot in one step.
        let from = Vec2::new(0.0, 0.0);
        let to = Vec2::new(100.0, 0.0);
        let bot = Vec2::new(50.0, 0.0);
        assert!(!circle_circle_collision(from, 2.0, bot, 5.0));
        assert!(!circle_circle_collision(to, 2.0, bot, 5.0));
        assert!(swept_circle_circle_collision(from, to, 2.0, bot, 5.0));
        // Grazing: the combined radius is 7, so 6 u off the line hits...
        assert!(swept_circle_circle_collision(
            from,
            to,
            2.0,
            Vec2::new(50.0, 6.0),
            5.0
        ));
        // ... and 8 u off it misses.
        assert!(!swept_circle_circle_collision(
            from,
            to,
            2.0,
            Vec2::new(50.0, 8.0),
            5.0
        ));
        // Beyond the sweep's end (closest point clamped to `to`).
        assert!(!swept_circle_circle_collision(
            from,
            to,
            2.0,
            Vec2::new(110.0, 0.0),
            5.0
        ));
        // A zero-length sweep degrades to the static test.
        assert!(swept_circle_circle_collision(bot, bot, 2.0, bot, 5.0));
        assert!(!swept_circle_circle_collision(from, from, 2.0, bot, 5.0));
    }

    #[test]
    fn segments_crossing_parallel_and_touching() {
        let v = Vec2::new;
        // A proper X.
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 10.0),
            v(0.0, 10.0),
            v(10.0, 0.0)
        ));
        // Parallel, offset: never.
        assert!(!line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(0.0, 5.0),
            v(10.0, 5.0)
        ));
        // The lines cross but the segments stop short.
        assert!(!line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(20.0, -5.0),
            v(20.0, 5.0)
        ));
        // Touching at an endpoint (a T and a corner) counts.
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(5.0, 0.0),
            v(5.0, 10.0)
        ));
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 10.0)
        ));
    }

    #[test]
    fn collinear_segments_intersect_only_when_they_overlap() {
        let v = Vec2::new;
        // Same line, disjoint: NOT an intersection.
        assert!(!line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(20.0, 0.0),
            v(30.0, 0.0)
        ));
        // Order-independent.
        assert!(!line_segment_intersection(
            v(30.0, 0.0),
            v(20.0, 0.0),
            v(10.0, 0.0),
            v(0.0, 0.0)
        ));
        // Vertical and diagonal lines too.
        assert!(!line_segment_intersection(
            v(5.0, 0.0),
            v(5.0, 10.0),
            v(5.0, 20.0),
            v(5.0, 30.0)
        ));
        assert!(!line_segment_intersection(
            v(0.0, 0.0),
            v(1.0, 1.0),
            v(2.0, 2.0),
            v(3.0, 3.0)
        ));
        // Same line, overlapping / nested / touching end to end: yes.
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(5.0, 0.0),
            v(15.0, 0.0)
        ));
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(2.0, 0.0),
            v(4.0, 0.0)
        ));
        assert!(line_segment_intersection(
            v(0.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 0.0),
            v(20.0, 0.0)
        ));
        assert!(line_segment_intersection(
            v(5.0, 0.0),
            v(5.0, 10.0),
            v(5.0, 10.0),
            v(5.0, 30.0)
        ));
    }

    #[test]
    fn line_of_sight_blocked_and_clear() {
        let walls = [Wall::new(100.0, 100.0, 50.0, 50.0)];
        // Straight through the wall.
        assert!(!has_line_of_sight(
            Vec2::new(50.0, 125.0),
            Vec2::new(200.0, 125.0),
            &walls
        ));
        // Through the wall diagonally.
        assert!(!has_line_of_sight(
            Vec2::new(90.0, 90.0),
            Vec2::new(160.0, 160.0),
            &walls
        ));
        // Passing beside it.
        assert!(has_line_of_sight(
            Vec2::new(50.0, 50.0),
            Vec2::new(200.0, 50.0),
            &walls
        ));
        // No walls at all.
        assert!(has_line_of_sight(
            Vec2::new(0.0, 0.0),
            Vec2::new(500.0, 500.0),
            &[]
        ));
        // The padded variant blocks a line that only skims the wall.
        assert!(has_line_of_sight(
            Vec2::new(50.0, 95.0),
            Vec2::new(200.0, 95.0),
            &walls
        ));
        assert!(!has_line_of_sight_with_padding(
            Vec2::new(50.0, 95.0),
            Vec2::new(200.0, 95.0),
            &walls,
            25.0
        ));
    }

    #[test]
    fn line_of_sight_along_a_wall_edge_extension_line() {
        let walls = [Wall::new(100.0, 100.0, 50.0, 50.0)];
        // Enemy and player both on y = 100 — the extension of the wall's top
        // edge — with the wall entirely to their right: nothing in between.
        assert!(has_line_of_sight(
            Vec2::new(0.0, 100.0),
            Vec2::new(50.0, 100.0),
            &walls
        ));
        // Same on the wall's left edge line (x = 100), below the wall.
        assert!(has_line_of_sight(
            Vec2::new(100.0, 200.0),
            Vec2::new(100.0, 300.0),
            &walls
        ));
        // Sliding ALONG the edge (overlapping it) still counts as blocked.
        assert!(!has_line_of_sight(
            Vec2::new(0.0, 100.0),
            Vec2::new(120.0, 100.0),
            &walls
        ));
    }
}
