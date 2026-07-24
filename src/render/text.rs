//! Text measurement shared by hit-testing and selection rendering.
//!
//! Uses the same `cosmic-text` shaper as the preview and the saved image, so a
//! label's clickable box matches the glyphs actually drawn.

use crate::domain::{TEXT_LINE_HEIGHT_FACTOR, TextAnnotation};

/// Padding (logical px) added around a text block for hit-testing and for the
/// selection outline, so thin labels are still easy to grab.
pub const TEXT_HIT_PADDING: f32 = 4.0;

/// Measure a laid-out text block, returning `(width, height)` in logical units.
///
/// Returns a single empty line's height for empty input rather than zero, so an
/// empty label still has a grabbable box.
pub fn measure_text(content: &str, font_size: f32) -> (f32, f32) {
    use cosmic::iced::advanced::graphics::text::{cosmic_text, font_system};

    let line_height = font_size * TEXT_LINE_HEIGHT_FACTOR;

    let Ok(mut font_sys) = font_system().write() else {
        // Fall back to a rough estimate rather than reporting a zero-size box.
        let lines = content.lines().count().max(1) as f32;
        let widest = content.lines().map(str::chars).map(Iterator::count).max().unwrap_or(0) as f32;
        return (widest * font_size * 0.6, lines * line_height);
    };

    let metrics = cosmic_text::Metrics::new(font_size.max(1.0), line_height.max(1.0));
    let mut buffer = cosmic_text::Buffer::new(font_sys.raw(), metrics);
    buffer.set_size(None, None);
    buffer.set_text(
        content,
        &cosmic_text::Attrs::new(),
        cosmic_text::Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_sys.raw(), false);

    let mut width = 0.0_f32;
    let mut lines = 0.0_f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        lines += 1.0;
    }

    (width, lines.max(1.0) * line_height)
}

/// The clickable/selectable box of a text annotation in global logical
/// coordinates, as `(x, y, width, height)` including [`TEXT_HIT_PADDING`].
pub fn text_bounds(text: &TextAnnotation) -> (f32, f32, f32, f32) {
    let (w, h) = measure_text(&text.content, text.font_size);
    (
        text.x - TEXT_HIT_PADDING,
        text.y - TEXT_HIT_PADDING,
        w + TEXT_HIT_PADDING * 2.0,
        h + TEXT_HIT_PADDING * 2.0,
    )
}

/// Whether a global logical point falls inside a text annotation's box.
pub fn text_contains(text: &TextAnnotation, gx: f32, gy: f32) -> bool {
    let (x, y, w, h) = text_bounds(text);
    gx >= x && gx <= x + w && gy >= y && gy <= y + h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShapeColor;

    fn ann(content: &str, x: f32, y: f32, size: f32) -> TextAnnotation {
        TextAnnotation {
            x,
            y,
            content: content.to_string(),
            font_size: size,
            color: ShapeColor::default(),
            shadow: false,
        }
    }

    #[test]
    fn empty_text_still_has_a_grabbable_box() {
        // Otherwise a label you just placed couldn't be clicked before typing.
        let (_, h) = measure_text("", 24.0);
        assert!(h > 0.0, "empty text should still occupy one line");
    }

    #[test]
    fn more_lines_make_a_taller_box() {
        let (_, one) = measure_text("a", 20.0);
        let (_, three) = measure_text("a\nb\nc", 20.0);
        assert!(three > one * 2.0, "3 lines: {three} vs 1 line: {one}");
    }

    #[test]
    fn bigger_font_makes_a_taller_box() {
        let (_, small) = measure_text("hello", 12.0);
        let (_, big) = measure_text("hello", 48.0);
        assert!(big > small);
    }

    #[test]
    fn hit_box_includes_padding_and_excludes_far_points() {
        let t = ann("hi", 100.0, 100.0, 24.0);
        let (x, y, w, h) = text_bounds(&t);

        // Padded outwards from the anchor.
        assert!(x < 100.0 && y < 100.0);
        // The anchor itself and the box centre are inside.
        assert!(text_contains(&t, 100.0, 100.0));
        assert!(text_contains(&t, x + w / 2.0, y + h / 2.0));
        // Well outside is not.
        assert!(!text_contains(&t, 100.0, 1000.0));
        assert!(!text_contains(&t, -500.0, 100.0));
    }

    #[test]
    fn hit_box_follows_the_annotation_position() {
        let a = ann("same", 0.0, 0.0, 24.0);
        let b = ann("same", 300.0, 200.0, 24.0);
        assert!(text_contains(&a, 2.0, 2.0));
        assert!(!text_contains(&b, 2.0, 2.0));
        assert!(text_contains(&b, 302.0, 202.0));
    }
}
