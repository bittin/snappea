//! Annotation types for drawing on screenshots
//!
//! All annotation types store coordinates in global logical coordinates.

use crate::config::ShapeColor;

/// Arrow annotation for drawing on screenshots
#[derive(Clone, Debug, PartialEq)]
pub struct ArrowAnnotation {
    /// Start point in global logical coordinates
    pub start_x: f32,
    pub start_y: f32,
    /// End point in global logical coordinates
    pub end_x: f32,
    pub end_y: f32,
    /// Color of this arrow
    pub color: ShapeColor,
    /// Whether to draw shadow/border
    pub shadow: bool,
    /// Stroke thickness in logical units
    pub thickness: f32,
}

/// Straight line annotation (an arrow without the head)
#[derive(Clone, Debug, PartialEq)]
pub struct LineAnnotation {
    /// Start point in global logical coordinates
    pub start_x: f32,
    pub start_y: f32,
    /// End point in global logical coordinates
    pub end_x: f32,
    pub end_y: f32,
    /// Color of this line
    pub color: ShapeColor,
    /// Whether to draw shadow/border
    pub shadow: bool,
    /// Stroke thickness in logical units
    pub thickness: f32,
}

/// Freehand (pencil) annotation: an open polyline through every sampled point.
#[derive(Clone, Debug, PartialEq)]
pub struct PencilAnnotation {
    /// Sampled points in global logical coordinates, in draw order
    pub points: Vec<(f32, f32)>,
    /// Color of this stroke
    pub color: ShapeColor,
    /// Whether to draw shadow/border
    pub shadow: bool,
    /// Stroke thickness in logical units
    pub thickness: f32,
}

impl PencilAnnotation {
    /// A stroke needs at least two points to be visible.
    pub fn is_valid(&self) -> bool {
        self.points.len() >= 2
    }
}

/// Selectable stroke thickness range for shape annotations (logical units).
pub const SHAPE_THICKNESS_MIN: f32 = 1.0;
pub const SHAPE_THICKNESS_MAX: f32 = 12.0;
pub const SHAPE_THICKNESS_DEFAULT: f32 = 3.0;
/// How much wider than the stroke the dark legibility outline is drawn.
pub const SHAPE_OUTLINE_EXTRA: f32 = 2.0;

/// Line-height multiplier applied to the font size when laying out text.
///
/// Matches cosmic-viewer's text tool so wrapped/multi-line text has the same
/// vertical rhythm in both apps.
pub const TEXT_LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Selectable text sizes (logical units), mirroring cosmic-viewer's presets.
pub const TEXT_SIZE_PRESETS: [f32; 7] = [12.0, 16.0, 20.0, 24.0, 32.0, 40.0, 64.0];

/// Default text size used for new text annotations.
pub const TEXT_SIZE_DEFAULT: f32 = 24.0;

/// A text label placed on the screenshot.
///
/// `x`/`y` are the **top-left** corner of the text block in global logical
/// coordinates. Both the live preview and the final image render from this same
/// origin, so what you type is where it lands.
#[derive(Clone, Debug, PartialEq)]
pub struct TextAnnotation {
    /// Top-left of the text block in global logical coordinates
    pub x: f32,
    pub y: f32,
    /// The text itself; may contain newlines
    pub content: String,
    /// Font size in logical units
    pub font_size: f32,
    /// Color of the glyphs
    pub color: ShapeColor,
    /// Whether to draw a dark drop shadow behind the glyphs (legibility)
    pub shadow: bool,
}

impl TextAnnotation {
    /// Empty or whitespace-only text renders nothing, so it isn't worth keeping.
    pub fn is_valid(&self) -> bool {
        !self.content.trim().is_empty()
    }
}

/// An in-progress text annotation being typed.
///
/// Kept separate from the committed [`TextAnnotation`] list so an unfinished
/// edit never enters the undo history.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEditing {
    /// Top-left of the text block in global logical coordinates
    pub x: f32,
    pub y: f32,
    /// Text typed so far
    pub content: String,
    /// Styling captured when the edit began, so committing needs nothing from
    /// the UI state. This is what lets any code path that ends an edit save the
    /// label instead of discarding it.
    pub font_size: f32,
    pub color: ShapeColor,
    pub shadow: bool,
    /// When re-editing an existing label, its index in the unified annotation
    /// list. On commit the entry is replaced in place (keeping its z-order and
    /// undo position) instead of a duplicate being appended.
    pub replacing: Option<usize>,
}

impl TextEditing {
    /// Turn the in-progress edit into a committed annotation.
    pub fn into_annotation(self) -> TextAnnotation {
        TextAnnotation {
            x: self.x,
            y: self.y,
            content: self.content,
            font_size: self.font_size,
            color: self.color,
            shadow: self.shadow,
        }
    }
}

/// Redaction annotation (black rectangle) for hiding sensitive content
#[derive(Clone, Debug, PartialEq)]
pub struct RedactAnnotation {
    /// Top-left point in global logical coordinates
    pub x: f32,
    pub y: f32,
    /// Bottom-right point in global logical coordinates
    pub x2: f32,
    pub y2: f32,
}

/// Pixelation annotation for obscuring sensitive content with pixelation effect
#[derive(Clone, Debug, PartialEq)]
pub struct PixelateAnnotation {
    /// Top-left point in global logical coordinates
    pub x: f32,
    pub y: f32,
    /// Bottom-right point in global logical coordinates
    pub x2: f32,
    pub y2: f32,
    /// Block size for this pixelation
    pub block_size: u32,
}

/// Outline rectangle annotation (no fill)
#[derive(Clone, Debug, PartialEq)]
pub struct RectOutlineAnnotation {
    /// Start point in global logical coordinates
    pub start_x: f32,
    pub start_y: f32,
    /// End point in global logical coordinates
    pub end_x: f32,
    pub end_y: f32,
    /// Color of this rectangle
    pub color: ShapeColor,
    /// Whether to draw shadow/border
    pub shadow: bool,
    /// Stroke thickness in logical units
    pub thickness: f32,
}

/// Outline circle/ellipse annotation (no fill)
#[derive(Clone, Debug, PartialEq)]
pub struct CircleOutlineAnnotation {
    /// Start point in global logical coordinates
    pub start_x: f32,
    pub start_y: f32,
    /// End point in global logical coordinates
    pub end_x: f32,
    pub end_y: f32,
    /// Color of this circle
    pub color: ShapeColor,
    /// Whether to draw shadow/border
    pub shadow: bool,
    /// Stroke thickness in logical units
    pub thickness: f32,
}

/// Minimum magnifier zoom factor (matches the config slider)
pub const MAGNIFIER_MIN_ZOOM: f32 = 1.5;
/// Maximum magnifier zoom factor (matches the config slider)
pub const MAGNIFIER_MAX_ZOOM: f32 = 10.0;
/// Minimum magnifier radius in logical units
pub const MAGNIFIER_MIN_RADIUS: f32 = 12.0;

/// Magnifier annotation: a circular loupe that zooms into the content beneath it.
///
/// Defined by a bounding box (like a circle); the interior shows the underlying
/// image content scaled up by `magnification`.
#[derive(Clone, Debug, PartialEq)]
pub struct MagnifierAnnotation {
    /// Start point in global logical coordinates
    pub start_x: f32,
    pub start_y: f32,
    /// End point in global logical coordinates
    pub end_x: f32,
    pub end_y: f32,
    /// Zoom factor applied to the content under the magnifier
    pub magnification: f32,
    /// Color of the magnifier ring
    pub color: ShapeColor,
    /// Whether to draw shadow/border on the ring
    pub shadow: bool,
}

impl MagnifierAnnotation {
    /// Center point in global logical coordinates
    pub fn center(&self) -> (f32, f32) {
        ((self.start_x + self.end_x) * 0.5, (self.start_y + self.end_y) * 0.5)
    }

    /// Radius in logical units (matches `render::geometry::circle_from_points`)
    pub fn radius(&self) -> f32 {
        (((self.end_x - self.start_x).abs() + (self.end_y - self.start_y).abs()) * 0.25).max(1.0)
    }

    /// Rewrite start/end so the loupe is centered at (cx, cy) with the given radius.
    pub fn set_geometry(&mut self, cx: f32, cy: f32, radius: f32) {
        let r = radius.max(MAGNIFIER_MIN_RADIUS);
        self.start_x = cx - r;
        self.start_y = cy - r;
        self.end_x = cx + r;
        self.end_y = cy + r;
    }
}

/// Unified annotation type for ordered drawing and undo/redo
#[derive(Clone, Debug, PartialEq)]
pub enum Annotation {
    Arrow(ArrowAnnotation),
    Line(LineAnnotation),
    Circle(CircleOutlineAnnotation),
    Rectangle(RectOutlineAnnotation),
    Pencil(PencilAnnotation),
    Text(TextAnnotation),
    Magnifier(MagnifierAnnotation),
    Redact(RedactAnnotation),
    Pixelate(PixelateAnnotation),
}

impl Annotation {
    /// Check if this is a shape annotation (everything except redactions)
    pub fn is_shape(&self) -> bool {
        matches!(
            self,
            Annotation::Arrow(_)
                | Annotation::Line(_)
                | Annotation::Circle(_)
                | Annotation::Rectangle(_)
                | Annotation::Pencil(_)
                | Annotation::Text(_)
                | Annotation::Magnifier(_)
        )
    }

    /// Check if this is a redaction annotation (redact, pixelate)
    pub fn is_redaction(&self) -> bool {
        matches!(self, Annotation::Redact(_) | Annotation::Pixelate(_))
    }
}
