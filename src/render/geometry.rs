//! Shared geometry calculations for annotations
//!
//! This module contains constants and math shared between
//! screen rendering (iced mesh) and image rendering (tiny-skia).

/// Arrow geometry constants
pub mod arrow {
    /// Default arrow shaft thickness in logical pixels
    pub const THICKNESS: f32 = 4.0;
    /// Default arrowhead size in logical pixels
    pub const HEAD_SIZE: f32 = 16.0;
    /// Shadow/outline thickness offset in logical pixels
    pub const OUTLINE: f32 = 2.0;
    /// Arrowhead angle from shaft in radians (35 degrees)
    pub const HEAD_ANGLE: f32 = 0.610_865_2; // 35.0_f32.to_radians()
    /// Minimum arrow length to be drawn
    pub const MIN_LENGTH: f32 = 5.0;

    /// Calculate arrow head points given start, end, and head size
    /// Returns (head1_x, head1_y, head2_x, head2_y) for the two head lines
    pub fn head_points(
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        head_size: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let length = (dx * dx + dy * dy).sqrt();
        if length < MIN_LENGTH {
            return None;
        }

        // Unit direction vector (pointing from start to end)
        let nx = dx / length;
        let ny = dy / length;

        let cos_a = HEAD_ANGLE.cos();
        let sin_a = HEAD_ANGLE.sin();

        // First head line (rotated clockwise from arrow direction)
        let head1_dx = -nx * cos_a - (-ny) * sin_a;
        let head1_dy = -nx * sin_a + (-ny) * cos_a;
        let head1_x = end_x + head1_dx * head_size;
        let head1_y = end_y + head1_dy * head_size;

        // Second head line (rotated counter-clockwise)
        let head2_dx = -nx * cos_a + (-ny) * sin_a;
        let head2_dy = -nx * (-sin_a) + (-ny) * cos_a;
        let head2_x = end_x + head2_dx * head_size;
        let head2_y = end_y + head2_dy * head_size;

        Some((head1_x, head1_y, head2_x, head2_y))
    }
}

/// Shape (rectangle/circle) geometry constants
pub mod shape {
    /// Default stroke thickness in logical pixels
    pub const THICKNESS: f32 = 3.0;
    /// Border/shadow thickness in logical pixels
    pub const BORDER_THICKNESS: f32 = 5.0;

    /// Ellipse bezier approximation constant: 4/3 * (sqrt(2) - 1)
    pub const BEZIER_K: f32 = 0.552_284_8;
}

/// Mesh rendering constants (for anti-aliased screen preview)
pub mod mesh {
    /// Anti-aliasing feather width in pixels
    pub const FEATHER: f32 = 2.0;
    /// Number of segments for circular caps
    pub const CIRCLE_SEGMENTS: usize = 12;
}

/// Normalize min/max coordinates from arbitrary start/end points
#[inline]
pub fn normalize_rect(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32, f32, f32) {
    let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
    let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
    (min_x, min_y, max_x, max_y)
}

/// Calculate ellipse center and radii from bounding box
#[inline]
pub fn ellipse_from_bounds(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> (f32, f32, f32, f32) {
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let rx = ((max_x - min_x) * 0.5).max(1.0);
    let ry = ((max_y - min_y) * 0.5).max(1.0);
    (cx, cy, rx, ry)
}

/// Calculate circle center and radius from two arbitrary drag points.
///
/// The magnifier is always a true circle; the radius grows with the drag in
/// both dimensions (average of the half-width and half-height).
#[inline]
pub fn circle_from_points(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32, f32) {
    let cx = (x1 + x2) * 0.5;
    let cy = (y1 + y2) * 0.5;
    let radius = (((x2 - x1).abs() + (y2 - y1).abs()) * 0.25).max(1.0);
    (cx, cy, radius)
}

/// Snap a line's end point to the nearest 45° increment around its start,
/// preserving the drag length. Used when Ctrl is held while drawing a line.
#[inline]
pub fn snap_to_45_degrees(sx: f32, sy: f32, ex: f32, ey: f32) -> (f32, f32) {
    let (dx, dy) = (ex - sx, ey - sy);
    let len = dx.hypot(dy);
    if len < f32::EPSILON {
        return (ex, ey);
    }
    const STEP: f32 = std::f32::consts::FRAC_PI_4; // 45°
    let angle = (dy.atan2(dx) / STEP).round() * STEP;
    (sx + len * angle.cos(), sy + len * angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn snap_45_locks_near_horizontal_to_horizontal() {
        // A drag 100 right and 8 up should flatten to pure horizontal.
        let (x, y) = snap_to_45_degrees(0.0, 0.0, 100.0, -8.0);
        assert!(approx(y, 0.0), "expected horizontal, got y={y}");
        assert!(approx(x, 100.31), "length should be preserved, got x={x}");
    }

    #[test]
    fn snap_45_locks_near_vertical_to_vertical() {
        let (x, y) = snap_to_45_degrees(0.0, 0.0, 5.0, 80.0);
        assert!(approx(x, 0.0), "expected vertical, got x={x}");
        assert!(y > 0.0);
    }

    #[test]
    fn snap_45_keeps_diagonal_diagonal() {
        let (x, y) = snap_to_45_degrees(0.0, 0.0, 50.0, 47.0);
        assert!(approx(x, y), "expected 45 degrees, got ({x}, {y})");
    }

    #[test]
    fn snap_45_preserves_drag_length() {
        let (sx, sy) = (10.0f32, 20.0f32);
        let (ex, ey) = (70.0f32, 55.0f32);
        let original = (ex - sx).hypot(ey - sy);
        let (x, y) = snap_to_45_degrees(sx, sy, ex, ey);
        let snapped = (x - sx).hypot(y - sy);
        assert!(approx(original, snapped), "{original} vs {snapped}");
    }

    #[test]
    fn snap_45_handles_zero_length_drag() {
        // Must not produce NaN when start == end.
        let (x, y) = snap_to_45_degrees(3.0, 4.0, 3.0, 4.0);
        assert!(approx(x, 3.0) && approx(y, 4.0));
    }
}
