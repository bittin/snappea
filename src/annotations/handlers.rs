//! Annotation message handlers
//!
//! Handles DrawMsg for all annotation drawing operations.

use crate::domain::{
    Annotation, ArrowAnnotation, CircleOutlineAnnotation, LineAnnotation, MAGNIFIER_MAX_ZOOM,
    MAGNIFIER_MIN_ZOOM, MagnifierAnnotation, PencilAnnotation, PixelateAnnotation,
    RectOutlineAnnotation, RedactAnnotation, TextAnnotation, TextEditing,
};
use crate::screenshot::Args;
use crate::session::messages::{DrawAction, DrawMsg, TextAction};

/// Handle a DrawMsg, modifying Args state
///
/// Returns true if the message was handled, false otherwise.
/// The caller is responsible for returning Task::none().
pub fn handle_draw_msg(args: &mut Args, msg: DrawMsg) {
    match msg {
        DrawMsg::Arrow(action) => handle_arrow(args, action),
        DrawMsg::Circle(action) => handle_circle(args, action),
        DrawMsg::Rectangle(action) => handle_rectangle(args, action),
        DrawMsg::Line(action) => handle_line(args, action),
        DrawMsg::Pencil(action) => handle_pencil(args, action),
        DrawMsg::Text(action) => handle_text(args, action),
        DrawMsg::Magnifier(action) => handle_magnifier(args, action),
        DrawMsg::MagnifierSelect(index) => {
            args.annotations.selected_magnifier = index;
        }
        DrawMsg::MagnifierMove(index, x, y) => {
            args.annotations.selected_magnifier = Some(index);
            args.annotations.edit_selected_magnifier(|m| {
                let r = m.radius();
                m.set_geometry(x, y, r);
            });
        }
        DrawMsg::MagnifierResize(index, radius) => {
            args.annotations.selected_magnifier = Some(index);
            args.annotations.edit_selected_magnifier(|m| {
                let (cx, cy) = m.center();
                m.set_geometry(cx, cy, radius);
            });
        }
        DrawMsg::MagnifierSetZoom(index, zoom) => {
            args.annotations.selected_magnifier = Some(index);
            let zoom = zoom.clamp(MAGNIFIER_MIN_ZOOM, MAGNIFIER_MAX_ZOOM);
            args.annotations
                .edit_selected_magnifier(|m| m.magnification = zoom);
        }
        DrawMsg::Redact(action) => handle_redact(args, action),
        DrawMsg::Pixelate(action) => handle_pixelate(args, action),
        DrawMsg::ClearShapes => args.annotations.clear_shapes(),
        DrawMsg::ClearRedactions => args.annotations.clear_redactions(),
        DrawMsg::Undo => args.annotations.undo(),
        DrawMsg::Redo => args.annotations.redo(),
    }
}

// ============================================================================
// Arrow handlers
// ============================================================================

fn handle_arrow(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.arrow_mode = !args.annotations.arrow_mode;
            if !args.annotations.arrow_mode {
                args.annotations.arrow_drawing = None;
            } else {
                disable_other_modes(args, Mode::Arrow);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.arrow_mode {
                args.annotations.arrow_drawing = Some((x, y));
            }
        }
        // Only freehand tracks intermediate drag positions.
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.arrow_drawing.take() {
                let arrow = ArrowAnnotation {
                    start_x,
                    start_y,
                    end_x: x,
                    end_y: y,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    thickness: args.ui.shape_thickness,
                };
                args.annotations.arrows.push(arrow.clone());
                args.annotations.add(Annotation::Arrow(arrow));
            }
        }
    }
}

// ============================================================================
// Circle handlers
// ============================================================================

fn handle_circle(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.circle_mode = !args.annotations.circle_mode;
            if !args.annotations.circle_mode {
                args.annotations.circle_drawing = None;
            } else {
                disable_other_modes(args, Mode::Circle);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.circle_mode {
                args.annotations.circle_drawing = Some((x, y));
            }
        }
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.circle_drawing.take() {
                let circle = CircleOutlineAnnotation {
                    start_x,
                    start_y,
                    end_x: x,
                    end_y: y,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    thickness: args.ui.shape_thickness,
                };
                args.annotations.circles.push(circle.clone());
                args.annotations.add(Annotation::Circle(circle));
            }
        }
    }
}

// ============================================================================
// Rectangle handlers
// ============================================================================

fn handle_rectangle(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.rect_outline_mode = !args.annotations.rect_outline_mode;
            if !args.annotations.rect_outline_mode {
                args.annotations.rect_outline_drawing = None;
            } else {
                disable_other_modes(args, Mode::Rectangle);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.rect_outline_mode {
                args.annotations.rect_outline_drawing = Some((x, y));
            }
        }
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.rect_outline_drawing.take() {
                let rect = RectOutlineAnnotation {
                    start_x,
                    start_y,
                    end_x: x,
                    end_y: y,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    thickness: args.ui.shape_thickness,
                };
                args.annotations.rect_outlines.push(rect.clone());
                args.annotations.add(Annotation::Rectangle(rect));
            }
        }
    }
}

// ============================================================================
// Line handlers
// ============================================================================

fn handle_line(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.line_mode = !args.annotations.line_mode;
            if !args.annotations.line_mode {
                args.annotations.line_drawing = None;
            } else {
                disable_other_modes(args, Mode::Line);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.line_mode {
                args.annotations.line_drawing = Some((x, y));
            }
        }
        // A line is defined by its endpoints; intermediate drag positions are
        // only used for the live preview, which reads the cursor directly.
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.line_drawing.take() {
                let line = LineAnnotation {
                    start_x,
                    start_y,
                    end_x: x,
                    end_y: y,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    thickness: args.ui.shape_thickness,
                };
                args.annotations.lines.push(line.clone());
                args.annotations.add(Annotation::Line(line));
            }
        }
    }
}

// ============================================================================
// Pencil (freehand) handlers
// ============================================================================

/// Minimum distance (in global logical units) between two consecutive sampled
/// points. Filters out redundant samples from high-frequency mouse motion.
const PENCIL_MIN_STEP: f32 = 1.5;

fn handle_pencil(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.pencil_mode = !args.annotations.pencil_mode;
            if !args.annotations.pencil_mode {
                args.annotations.pencil_drawing = None;
            } else {
                disable_other_modes(args, Mode::Pencil);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.pencil_mode {
                args.annotations.pencil_drawing = Some(vec![(x, y)]);
            }
        }
        DrawAction::Move(x, y) => {
            if let Some(points) = args.annotations.pencil_drawing.as_mut() {
                let far_enough = points.last().is_none_or(|(lx, ly)| {
                    (x - lx).hypot(y - ly) >= PENCIL_MIN_STEP
                });
                if far_enough {
                    points.push((x, y));
                }
            }
        }
        DrawAction::End(x, y) => {
            if let Some(mut points) = args.annotations.pencil_drawing.take() {
                if points.last().is_none_or(|(lx, ly)| *lx != x || *ly != y) {
                    points.push((x, y));
                }
                let pencil = PencilAnnotation {
                    points,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    thickness: args.ui.shape_thickness,
                };
                // A single click produces one point and nothing visible — drop it
                // so it doesn't occupy a slot in the undo history.
                if pencil.is_valid() {
                    args.annotations.pencils.push(pencil.clone());
                    args.annotations.add(Annotation::Pencil(pencil));
                }
            }
        }
    }
}

// ============================================================================
// Text handlers
// ============================================================================

/// Commit whatever text is being edited, if it has any visible content.
///
/// Shared by the explicit commit action, by starting a new text elsewhere, and
/// by leaving text mode — so a typed label is never silently lost.
pub fn commit_editing_text(args: &mut Args) {
    args.annotations.commit_text_editing();
}

fn handle_text(args: &mut Args, action: TextAction) {
    match action {
        TextAction::ModeToggle => {
            args.annotations.text_mode = !args.annotations.text_mode;
            if args.annotations.text_mode {
                disable_other_modes(args, Mode::Text);
                args.detection.clear();
            } else {
                // Leaving the tool keeps what was typed.
                commit_editing_text(args);
            }
        }
        TextAction::Begin(x, y) => {
            if args.annotations.text_mode {
                // Clicking elsewhere finishes the previous label first.
                commit_editing_text(args);
                args.annotations.selected_text = None;
                args.annotations.text_editing = Some(TextEditing {
                    x,
                    y,
                    content: String::new(),
                    font_size: args.ui.text_font_size,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                    replacing: None,
                });
            }
        }
        TextAction::Insert(s) => {
            if let Some(editing) = args.annotations.text_editing.as_mut() {
                editing.content.push_str(&s);
            }
        }
        TextAction::Backspace => {
            if let Some(editing) = args.annotations.text_editing.as_mut() {
                editing.content.pop();
            }
        }
        TextAction::Newline => {
            if let Some(editing) = args.annotations.text_editing.as_mut() {
                editing.content.push('\n');
            }
        }
        TextAction::Commit => commit_editing_text(args),
        TextAction::SetFontSize(size) => {
            args.ui.text_font_size = size;
            // Applies to the label being typed and to the selected one, so the
            // picker doubles as an editor for existing text.
            if let Some(editing) = args.annotations.text_editing.as_mut() {
                editing.font_size = size;
            }
            args.annotations.edit_selected_text(|t| t.font_size = size);
        }
        TextAction::Select(index) => {
            // Selecting a different label finishes any open edit first.
            commit_editing_text(args);
            args.annotations.selected_text = index;
        }
        TextAction::Move(index, x, y) => {
            args.annotations.selected_text = Some(index);
            args.annotations.edit_selected_text(|t| {
                t.x = x;
                t.y = y;
            });
        }
        TextAction::EditExisting(index) => {
            commit_editing_text(args);
            let Some(unified_idx) = args.annotations.unified_index_of_text(index) else {
                return;
            };
            let Some(text) = args.annotations.texts.get(index).cloned() else {
                return;
            };
            args.annotations.selected_text = Some(index);
            // Match the label's own size/colour while editing it.
            args.ui.text_font_size = text.font_size;
            args.annotations.text_editing = Some(TextEditing {
                x: text.x,
                y: text.y,
                content: text.content,
                font_size: text.font_size,
                color: text.color,
                shadow: text.shadow,
                replacing: Some(unified_idx),
            });
        }
    }
}

// ============================================================================
// Magnifier handlers
// ============================================================================

fn handle_magnifier(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.magnifier_mode = !args.annotations.magnifier_mode;
            if !args.annotations.magnifier_mode {
                args.annotations.magnifier_drawing = None;
                args.annotations.selected_magnifier = None;
            } else {
                disable_other_modes(args, Mode::Magnifier);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.magnifier_mode {
                args.annotations.magnifier_drawing = Some((x, y));
            }
        }
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.magnifier_drawing.take() {
                let magnifier = MagnifierAnnotation {
                    start_x,
                    start_y,
                    end_x: x,
                    end_y: y,
                    magnification: args.ui.magnifier_magnification,
                    color: args.ui.shape_color,
                    shadow: args.ui.shape_shadow,
                };
                args.annotations.magnifiers.push(magnifier.clone());
                args.annotations.add(Annotation::Magnifier(magnifier));
                // Select the newly created magnifier so it can be tweaked
                args.annotations.selected_magnifier =
                    Some(args.annotations.magnifiers.len() - 1);
            }
        }
    }
}

// ============================================================================
// Redact handlers
// ============================================================================

fn handle_redact(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.redact_mode = !args.annotations.redact_mode;
            if !args.annotations.redact_mode {
                args.annotations.redact_drawing = None;
            } else {
                disable_other_modes(args, Mode::Redact);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.redact_mode {
                args.annotations.redact_drawing = Some((x, y));
            }
        }
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.redact_drawing.take() {
                let redact = RedactAnnotation {
                    x: start_x,
                    y: start_y,
                    x2: x,
                    y2: y,
                };
                args.annotations.redactions.push(redact.clone());
                args.annotations.add(Annotation::Redact(redact));
            }
        }
    }
}

// ============================================================================
// Pixelate handlers
// ============================================================================

fn handle_pixelate(args: &mut Args, action: DrawAction) {
    match action {
        DrawAction::ModeToggle => {
            args.annotations.pixelate_mode = !args.annotations.pixelate_mode;
            if !args.annotations.pixelate_mode {
                args.annotations.pixelate_drawing = None;
            } else {
                disable_other_modes(args, Mode::Pixelate);
                args.detection.clear();
            }
        }
        DrawAction::Start(x, y) => {
            if args.annotations.pixelate_mode {
                args.annotations.pixelate_drawing = Some((x, y));
            }
        }
        DrawAction::Move(..) => {}
        DrawAction::End(x, y) => {
            if let Some((start_x, start_y)) = args.annotations.pixelate_drawing.take() {
                let pixelate = PixelateAnnotation {
                    x: start_x,
                    y: start_y,
                    x2: x,
                    y2: y,
                    block_size: args.ui.pixelation_block_size,
                };
                args.annotations.pixelations.push(pixelate.clone());
                args.annotations.add(Annotation::Pixelate(pixelate));
            }
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Arrow,
    Line,
    Circle,
    Rectangle,
    Pencil,
    Text,
    Magnifier,
    Redact,
    Pixelate,
}

fn disable_other_modes(args: &mut Args, keep: Mode) {
    if keep != Mode::Arrow {
        args.annotations.arrow_mode = false;
        args.annotations.arrow_drawing = None;
    }
    if keep != Mode::Circle {
        args.annotations.circle_mode = false;
        args.annotations.circle_drawing = None;
    }
    if keep != Mode::Rectangle {
        args.annotations.rect_outline_mode = false;
        args.annotations.rect_outline_drawing = None;
    }
    if keep != Mode::Line {
        args.annotations.line_mode = false;
        args.annotations.line_drawing = None;
    }
    if keep != Mode::Pencil {
        args.annotations.pencil_mode = false;
        args.annotations.pencil_drawing = None;
    }
    if keep != Mode::Text {
        args.annotations.text_mode = false;
        // Don't drop a half-typed label when switching tools.
        commit_editing_text(args);
    }
    if keep != Mode::Magnifier {
        args.annotations.magnifier_mode = false;
        args.annotations.magnifier_drawing = None;
        args.annotations.selected_magnifier = None;
    }
    if keep != Mode::Redact {
        args.annotations.redact_mode = false;
        args.annotations.redact_drawing = None;
    }
    if keep != Mode::Pixelate {
        args.annotations.pixelate_mode = false;
        args.annotations.pixelate_drawing = None;
    }
}
