//! Live rendering for text annotations on the screenshot overlay.
//!
//! Draws committed text labels plus the one currently being typed. The final
//! saved image is rendered separately by `render::image::draw_texts_on_image`;
//! both anchor the text block at its top-left corner and use the same line
//! height so the preview matches the output.

use cosmic::iced::core::{Point, Rectangle, Size, alignment, text::Renderer as _};
use cosmic::iced::{Color, advanced::text::Text};

use crate::domain::{TEXT_LINE_HEIGHT_FACTOR, TextAnnotation, TextEditing};

/// Caret appended to the in-progress text so the insertion point is visible
/// without needing to measure glyph advances.
const CARET: char = '|';

/// Build the iced text primitive for an annotation string.
fn text_primitive(content: String, font_size: f32, viewport: &Rectangle) -> Text {
    Text {
        content,
        // Generous bounds: annotations wrap only on explicit newlines, and the
        // renderer clips to the viewport anyway.
        bounds: Size::new(viewport.width, viewport.height),
        size: cosmic::iced::Pixels(font_size),
        line_height: cosmic::iced::core::text::LineHeight::Relative(TEXT_LINE_HEIGHT_FACTOR),
        font: cosmic::iced::Font::default(),
        align_x: alignment::Horizontal::Left.into(),
        align_y: alignment::Vertical::Top,
        shaping: cosmic::iced::core::text::Shaping::Advanced,
        wrapping: cosmic::iced::core::text::Wrapping::None,
        ellipsize: cosmic::iced::core::text::Ellipsize::default(),
    }
}

/// Draw one text block, with an optional dark drop shadow for legibility.
fn draw_block(
    renderer: &mut cosmic::Renderer,
    viewport: &Rectangle,
    content: String,
    font_size: f32,
    color: Color,
    shadow: bool,
    local_x: f32,
    local_y: f32,
) {
    if shadow {
        let offset = (font_size * 0.06).max(1.0);
        renderer.fill_text(
            text_primitive(content.clone(), font_size, viewport),
            Point::new(local_x + offset, local_y + offset),
            Color::from_rgba(0.0, 0.0, 0.0, 0.86),
            *viewport,
        );
    }
    renderer.fill_text(
        text_primitive(content, font_size, viewport),
        Point::new(local_x, local_y),
        color,
        *viewport,
    );
}

/// Draw all committed text annotations.
///
/// `output_offset` is the global position of this output's top-left corner, used
/// to convert the annotations' global coordinates to output-local ones.
pub fn draw_texts(
    renderer: &mut cosmic::Renderer,
    viewport: &Rectangle,
    texts: &[TextAnnotation],
    output_offset: (f32, f32),
) {
    for text in texts {
        if !text.is_valid() {
            continue;
        }
        draw_block(
            renderer,
            viewport,
            text.content.clone(),
            text.font_size,
            text.color.into(),
            text.shadow,
            text.x - output_offset.0,
            text.y - output_offset.1,
        );
    }
}

/// Draw the text annotation currently being typed, with a trailing caret.
pub fn draw_text_editing(
    renderer: &mut cosmic::Renderer,
    viewport: &Rectangle,
    editing: &TextEditing,
    font_size: f32,
    color: Color,
    shadow: bool,
    output_offset: (f32, f32),
) {
    let mut content = editing.content.clone();
    content.push(CARET);

    draw_block(
        renderer,
        viewport,
        content,
        font_size,
        color,
        shadow,
        editing.x - output_offset.0,
        editing.y - output_offset.1,
    );
}
