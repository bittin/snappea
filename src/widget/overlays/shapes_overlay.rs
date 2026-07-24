//! ShapesOverlay - Canvas overlay for circle and rectangle outline annotations
//!
//! This widget handles:
//! - Drawing existing circle and rectangle annotations
//! - Live preview during shape drawing
//! - Mouse input for shape creation

use cosmic::iced::widget::canvas;

use crate::domain::{
    CircleOutlineAnnotation, LineAnnotation, PencilAnnotation, Rect, RectOutlineAnnotation,
};

/// Canvas overlay for circle/rectangle outline rendering and input
pub struct ShapesOverlay<'a, Message: Clone + 'static> {
    /// Selection rect in output-local coordinates (x, y, w, h)
    pub selection_rect: Option<(f32, f32, f32, f32)>,
    /// Output rect for global offset
    pub output_rect: Rect,
    /// Existing circles in global coordinates
    pub circles: Vec<CircleOutlineAnnotation>,
    /// Existing rectangle outlines in global coordinates
    pub rect_outlines: Vec<RectOutlineAnnotation>,
    /// Existing straight lines in global coordinates
    pub lines: Vec<LineAnnotation>,
    /// Existing freehand strokes in global coordinates
    pub pencils: Vec<PencilAnnotation>,
    /// Whether circle drawing mode is active
    pub circle_mode: bool,
    /// Whether rectangle outline drawing mode is active
    pub rect_outline_mode: bool,
    /// Whether straight-line drawing mode is active
    pub line_mode: bool,
    /// Whether freehand drawing mode is active
    pub pencil_mode: bool,
    /// Whether text placement mode is active
    pub text_mode: bool,
    /// Current circle drawing start in global coordinates (if any)
    pub circle_drawing: Option<(f32, f32)>,
    /// Current rectangle outline drawing start in global coordinates (if any)
    pub rect_outline_drawing: Option<(f32, f32)>,
    /// Current line drawing start in global coordinates (if any)
    pub line_drawing: Option<(f32, f32)>,
    /// Points collected so far for the in-progress freehand stroke
    pub pencil_drawing: Option<Vec<(f32, f32)>>,
    /// Callback when circle drawing starts
    pub on_circle_start: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when circle drawing ends
    pub on_circle_end: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when rectangle drawing starts
    pub on_rect_start: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when rectangle drawing ends
    pub on_rect_end: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when line drawing starts
    pub on_line_start: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when line drawing ends
    pub on_line_end: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when freehand drawing starts
    pub on_pencil_start: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when the freehand stroke extends to a new point
    pub on_pencil_move: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when freehand drawing ends
    pub on_pencil_end: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Callback when a text annotation is placed at a point
    pub on_text_begin: Option<Box<dyn Fn(f32, f32) -> Message + 'a>>,
    /// Shape color for preview
    pub shape_color: crate::config::ShapeColor,
    /// Whether to draw shadow on shapes
    pub shape_shadow: bool,
}

/// State for ShapesOverlay canvas program
#[derive(Debug, Default)]
pub struct ShapesState {
    /// Whether Ctrl key is currently pressed
    pub ctrl_down: bool,
    /// Whether Ctrl was pressed when drawing started (latched)
    pub ctrl_latched: bool,
}

impl ShapesState {
    /// Latch Ctrl state if a drawing is active
    pub fn latch_ctrl_if_needed(&mut self, drawing_active: bool) {
        if drawing_active && self.ctrl_down {
            self.ctrl_latched = true;
        }
    }
}

impl<'a, Message: Clone + 'static> ShapesOverlay<'a, Message> {
    /// Constrain end point to form a square/circle (when Ctrl is held)
    fn constrain_end(sx: f32, sy: f32, ex: f32, ey: f32) -> (f32, f32) {
        let dx = ex - sx;
        let dy = ey - sy;
        let side = dx.abs().min(dy.abs());
        let sign_x = if dx < 0.0 { -1.0 } else { 1.0 };
        let sign_y = if dy < 0.0 { -1.0 } else { 1.0 };
        (sx + side * sign_x, sy + side * sign_y)
    }

    /// Whether any drawing gesture handled by this overlay is in progress.
    fn drawing_active(&self) -> bool {
        self.circle_drawing.is_some()
            || self.rect_outline_drawing.is_some()
            || self.line_drawing.is_some()
            || self.pencil_drawing.is_some()
    }
}

impl<'a, Message: Clone + 'static> canvas::Program<Message, cosmic::Theme, cosmic::Renderer>
    for ShapesOverlay<'a, Message>
{
    type State = ShapesState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &cosmic::iced::Event,
        bounds: cosmic::iced::core::Rectangle,
        cursor: cosmic::iced::core::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use cosmic::iced::core::keyboard;
        use cosmic::iced::core::mouse::{Button, Event as MouseEvent};

        // Margin for shape clamping (0 = clamp to exact edge)
        const ANNOTATION_MARGIN: f32 = 0.0;

        // Helper to clamp and check inner bounds
        let (inner_x, inner_y, inner_w, inner_h) = if let Some((x, y, w, h)) = self.selection_rect {
            (
                x + ANNOTATION_MARGIN,
                y + ANNOTATION_MARGIN,
                w - 2.0 * ANNOTATION_MARGIN,
                h - 2.0 * ANNOTATION_MARGIN,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let clamp_pos = |px: f32, py: f32| -> (f32, f32) {
            if let Some((x, y, w, h)) = self.selection_rect {
                let min_x = x + ANNOTATION_MARGIN;
                let max_x = x + w - ANNOTATION_MARGIN;
                let min_y = y + ANNOTATION_MARGIN;
                let max_y = y + h - ANNOTATION_MARGIN;
                (px.clamp(min_x, max_x), py.clamp(min_y, max_y))
            } else {
                (px, py)
            }
        };

        match event {
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(mods)) => {
                state.ctrl_down = mods.control();
                state.latch_ctrl_if_needed(self.drawing_active());
                return Some(canvas::Action::capture());
            }
            // Freehand needs every intermediate position to build its polyline.
            canvas::Event::Mouse(MouseEvent::CursorMoved { .. }) => {
                if self.pencil_mode && self.pencil_drawing.is_some() {
                    let Some(pos) = cursor.position_in(bounds) else {
                        return None;
                    };
                    let (cx, cy) = clamp_pos(pos.x, pos.y);
                    let gx = cx + self.output_rect.left as f32;
                    let gy = cy + self.output_rect.top as f32;
                    if let Some(ref cb) = self.on_pencil_move {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
            }
            canvas::Event::Mouse(MouseEvent::ButtonPressed(Button::Left)) => {
                let Some(pos) = cursor.position_in(bounds) else {
                    return None;
                };
                // Check if inside inner bounds (with margin)
                let inside = inner_w > 0.0
                    && inner_h > 0.0
                    && pos.x >= inner_x
                    && pos.x <= inner_x + inner_w
                    && pos.y >= inner_y
                    && pos.y <= inner_y + inner_h;
                if !inside {
                    return None;
                }

                // Clamp and convert to global coordinates
                let (cx, cy) = clamp_pos(pos.x, pos.y);
                let gx = cx + self.output_rect.left as f32;
                let gy = cy + self.output_rect.top as f32;

                if self.circle_mode {
                    state.ctrl_latched = state.ctrl_down;
                    if let Some(ref cb) = self.on_circle_start {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
                if self.rect_outline_mode {
                    state.ctrl_latched = state.ctrl_down;
                    if let Some(ref cb) = self.on_rect_start {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
                if self.line_mode {
                    state.ctrl_latched = state.ctrl_down;
                    if let Some(ref cb) = self.on_line_start {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
                if self.pencil_mode {
                    if let Some(ref cb) = self.on_pencil_start {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
                // Text is placed on click; typing is then handled by the keyboard.
                if self.text_mode {
                    if let Some(ref cb) = self.on_text_begin {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
            }
            canvas::Event::Mouse(MouseEvent::ButtonReleased(Button::Left)) => {
                let Some(pos) = cursor.position_in(bounds) else {
                    return None;
                };
                // Clamp and convert to global coordinates
                let (cx, cy) = clamp_pos(pos.x, pos.y);
                let gx = cx + self.output_rect.left as f32;
                let gy = cy + self.output_rect.top as f32;

                if self.circle_mode && self.circle_drawing.is_some() {
                    let (sx, sy) = self.circle_drawing.unwrap_or((gx, gy));
                    let (ex, ey) = if state.ctrl_latched || state.ctrl_down {
                        Self::constrain_end(sx, sy, gx, gy)
                    } else {
                        (gx, gy)
                    };
                    state.ctrl_latched = false;
                    if let Some(ref cb) = self.on_circle_end {
                        return Some(canvas::Action::publish(cb(ex, ey)).and_capture());
                    }
                }

                if self.rect_outline_mode && self.rect_outline_drawing.is_some() {
                    let (sx, sy) = self.rect_outline_drawing.unwrap_or((gx, gy));
                    let (ex, ey) = if state.ctrl_latched || state.ctrl_down {
                        Self::constrain_end(sx, sy, gx, gy)
                    } else {
                        (gx, gy)
                    };
                    state.ctrl_latched = false;
                    if let Some(ref cb) = self.on_rect_end {
                        return Some(canvas::Action::publish(cb(ex, ey)).and_capture());
                    }
                }

                if self.line_mode && self.line_drawing.is_some() {
                    let (sx, sy) = self.line_drawing.unwrap_or((gx, gy));
                    let (ex, ey) = if state.ctrl_latched || state.ctrl_down {
                        crate::render::geometry::snap_to_45_degrees(sx, sy, gx, gy)
                    } else {
                        (gx, gy)
                    };
                    state.ctrl_latched = false;
                    if let Some(ref cb) = self.on_line_end {
                        return Some(canvas::Action::publish(cb(ex, ey)).and_capture());
                    }
                }

                if self.pencil_mode && self.pencil_drawing.is_some() {
                    state.ctrl_latched = false;
                    if let Some(ref cb) = self.on_pencil_end {
                        return Some(canvas::Action::publish(cb(gx, gy)).and_capture());
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: cosmic::iced::core::Rectangle,
        cursor: cosmic::iced::core::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use canvas::{Frame, Path, Stroke};
        use cosmic::iced::core::{Color, Point, Size};

        let mut frame = Frame::new(renderer, bounds.size());

        let shadow_stroke = Stroke {
            style: Color::from_rgba(0.0, 0.0, 0.0, 0.9).into(),
            width: 5.0,
            ..Stroke::default()
        };

        // Draw rectangle outlines with per-annotation colors
        for r in &self.rect_outlines {
            let rect_color: Color = r.color.into();
            let stroke = Stroke {
                style: rect_color.into(),
                width: 3.0,
                ..Stroke::default()
            };
            let x1 = r.start_x - self.output_rect.left as f32;
            let y1 = r.start_y - self.output_rect.top as f32;
            let x2 = r.end_x - self.output_rect.left as f32;
            let y2 = r.end_y - self.output_rect.top as f32;
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            let path = Path::rectangle(
                Point::new(min_x, min_y),
                Size::new((max_x - min_x).max(1.0), (max_y - min_y).max(1.0)),
            );
            if r.shadow {
                frame.stroke(&path, shadow_stroke);
            }
            frame.stroke(&path, stroke);
        }

        // Draw straight lines with per-annotation colors
        for l in &self.lines {
            let line_color: Color = l.color.into();
            let stroke = Stroke {
                style: line_color.into(),
                width: 3.0,
                ..Stroke::default()
            };
            let x1 = l.start_x - self.output_rect.left as f32;
            let y1 = l.start_y - self.output_rect.top as f32;
            let x2 = l.end_x - self.output_rect.left as f32;
            let y2 = l.end_y - self.output_rect.top as f32;
            let path = Path::new(|b| {
                b.move_to(Point::new(x1, y1));
                b.line_to(Point::new(x2, y2));
            });
            if l.shadow {
                frame.stroke(&path, shadow_stroke);
            }
            frame.stroke(&path, stroke);
        }

        // Draw freehand strokes with per-annotation colors
        for p in &self.pencils {
            if !p.is_valid() {
                continue;
            }
            let pencil_color: Color = p.color.into();
            let stroke = Stroke {
                style: pencil_color.into(),
                width: 3.0,
                ..Stroke::default()
            };
            let ox = self.output_rect.left as f32;
            let oy = self.output_rect.top as f32;
            let path = Path::new(|b| {
                b.move_to(Point::new(p.points[0].0 - ox, p.points[0].1 - oy));
                for (px, py) in &p.points[1..] {
                    b.line_to(Point::new(px - ox, py - oy));
                }
            });
            if p.shadow {
                frame.stroke(&path, shadow_stroke);
            }
            frame.stroke(&path, stroke);
        }

        // Draw circle/ellipse outlines with per-annotation colors
        for c in &self.circles {
            let circle_color: Color = c.color.into();
            let stroke = Stroke {
                style: circle_color.into(),
                width: 3.0,
                ..Stroke::default()
            };
            let x1 = c.start_x - self.output_rect.left as f32;
            let y1 = c.start_y - self.output_rect.top as f32;
            let x2 = c.end_x - self.output_rect.left as f32;
            let y2 = c.end_y - self.output_rect.top as f32;
            let (min_x, max_x) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
            let (min_y, max_y) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
            let cx = (min_x + max_x) * 0.5;
            let cy = (min_y + max_y) * 0.5;
            let rx = ((max_x - min_x) * 0.5).max(1.0);
            let ry = ((max_y - min_y) * 0.5).max(1.0);
            let approx_r = rx.max(ry);
            let segs = ((approx_r * 0.35).clamp(32.0, 128.0)) as usize;
            let step = std::f32::consts::TAU / segs as f32;

            let path = Path::new(|b| {
                b.move_to(Point::new(cx + rx, cy));
                for i in 1..=segs {
                    let t = i as f32 * step;
                    b.line_to(Point::new(cx + rx * t.cos(), cy + ry * t.sin()));
                }
            });
            if c.shadow {
                frame.stroke(&path, shadow_stroke);
            }
            frame.stroke(&path, stroke);
        }

        // Live previews (during drag)
        let constrain = state.ctrl_down || state.ctrl_latched;
        let shape_color: Color = self.shape_color.into();
        let preview_color = Color {
            a: 0.7,
            ..shape_color
        };
        let preview_stroke = Stroke {
            style: preview_color.into(),
            width: 3.0,
            ..Stroke::default()
        };
        let preview_shadow_stroke = Stroke {
            style: Color::from_rgba(0.0, 0.0, 0.0, 0.6).into(),
            width: 5.0,
            ..Stroke::default()
        };

        if let Some((sx_g, sy_g)) = self.rect_outline_drawing
            && let Some(pos) = cursor.position_in(bounds)
        {
            let sx = sx_g - self.output_rect.left as f32;
            let sy = sy_g - self.output_rect.top as f32;
            let mut ex = pos.x;
            let mut ey = pos.y;
            if constrain {
                (ex, ey) = Self::constrain_end(sx, sy, ex, ey);
            }
            let (min_x, max_x) = if sx < ex { (sx, ex) } else { (ex, sx) };
            let (min_y, max_y) = if sy < ey { (sy, ey) } else { (ey, sy) };
            let path = Path::rectangle(
                Point::new(min_x, min_y),
                Size::new((max_x - min_x).max(1.0), (max_y - min_y).max(1.0)),
            );
            if self.shape_shadow {
                frame.stroke(&path, preview_shadow_stroke);
            }
            frame.stroke(&path, preview_stroke);
        }

        if let Some((sx_g, sy_g)) = self.line_drawing
            && let Some(pos) = cursor.position_in(bounds)
        {
            let sx = sx_g - self.output_rect.left as f32;
            let sy = sy_g - self.output_rect.top as f32;
            let (mut ex, mut ey) = (pos.x, pos.y);
            if constrain {
                (ex, ey) = crate::render::geometry::snap_to_45_degrees(sx, sy, ex, ey);
            }
            let path = Path::new(|b| {
                b.move_to(Point::new(sx, sy));
                b.line_to(Point::new(ex, ey));
            });
            if self.shape_shadow {
                frame.stroke(&path, preview_shadow_stroke);
            }
            frame.stroke(&path, preview_stroke);
        }

        // Freehand preview: the stroke collected so far (already includes the
        // latest cursor sample, so no cursor position is needed here).
        if let Some(points) = self.pencil_drawing.as_ref()
            && points.len() >= 2
        {
            let ox = self.output_rect.left as f32;
            let oy = self.output_rect.top as f32;
            let path = Path::new(|b| {
                b.move_to(Point::new(points[0].0 - ox, points[0].1 - oy));
                for (px, py) in &points[1..] {
                    b.line_to(Point::new(px - ox, py - oy));
                }
            });
            if self.shape_shadow {
                frame.stroke(&path, preview_shadow_stroke);
            }
            frame.stroke(&path, preview_stroke);
        }

        if let Some((sx_g, sy_g)) = self.circle_drawing
            && let Some(pos) = cursor.position_in(bounds)
        {
            let sx = sx_g - self.output_rect.left as f32;
            let sy = sy_g - self.output_rect.top as f32;
            let mut ex = pos.x;
            let mut ey = pos.y;
            if constrain {
                (ex, ey) = Self::constrain_end(sx, sy, ex, ey);
            }
            let (min_x, max_x) = if sx < ex { (sx, ex) } else { (ex, sx) };
            let (min_y, max_y) = if sy < ey { (sy, ey) } else { (ey, sy) };
            let cx = (min_x + max_x) * 0.5;
            let cy = (min_y + max_y) * 0.5;
            let rx = ((max_x - min_x) * 0.5).max(1.0);
            let ry = ((max_y - min_y) * 0.5).max(1.0);
            let approx_r = rx.max(ry);
            let segs = ((approx_r * 0.35).clamp(32.0, 128.0)) as usize;
            let step = std::f32::consts::TAU / segs as f32;
            let path = Path::new(|b| {
                b.move_to(Point::new(cx + rx, cy));
                for i in 1..=segs {
                    let t = i as f32 * step;
                    b.line_to(Point::new(cx + rx * t.cos(), cy + ry * t.sin()));
                }
            });
            if self.shape_shadow {
                frame.stroke(&path, preview_shadow_stroke);
            }
            frame.stroke(&path, preview_stroke);
        }

        vec![frame.into_geometry()]
    }
}
