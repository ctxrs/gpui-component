use std::{
    ops::Range,
    rc::Rc,
    sync::{Arc, Mutex},
};

use gpui::{
    point, px, quad, transparent_black, App, BorderStyle, Bounds, Corners, CursorStyle, Edges,
    Element, ElementId, GlobalElementId, Half, HighlightStyle, Hitbox, HitboxBehavior, Hsla,
    InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    SharedString, StyledText, TextLayout, Window,
};

use crate::{
    global_state::GlobalState,
    input::Selection,
    text::{node::LinkMark, style::InlineCodeStyle},
    ActiveTheme,
};

/// A inline element used to render a inline text and support selectable.
///
/// All text in TextView (including the CodeBlock) used this for text rendering.
pub(super) struct Inline {
    id: ElementId,
    text: SharedString,
    links: Rc<Vec<(Range<usize>, LinkMark)>>,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    code_ranges: Vec<Range<usize>>,
    inline_code_style: Option<InlineCodeStyle>,
    styled_text: StyledText,

    state: Arc<Mutex<InlineState>>,
}

/// The inline text state, used RefCell to keep the selection state.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct InlineState {
    hovered_index: Option<usize>,
    /// The text that actually rendering, matched with selection.
    pub(super) text: SharedString,
    pub(super) selection: Option<Selection>,
}

impl InlineState {
    /// Save actually rendered text for selected text to use.
    pub(crate) fn set_text(&mut self, text: SharedString) {
        self.text = text;
    }
}

impl Inline {
    pub(super) fn new(
        id: impl Into<ElementId>,
        state: Arc<Mutex<InlineState>>,
        links: Vec<(Range<usize>, LinkMark)>,
        highlights: Vec<(Range<usize>, HighlightStyle)>,
        code_ranges: Vec<Range<usize>>,
        inline_code_style: Option<InlineCodeStyle>,
    ) -> Self {
        let text = state.lock().unwrap().text.clone();
        Self {
            id: id.into(),
            links: Rc::new(links),
            highlights,
            code_ranges,
            inline_code_style,
            text: text.clone(),
            styled_text: StyledText::new(text),
            state,
        }
    }

    /// Get link at given mouse position.
    fn link_for_position(
        layout: &TextLayout,
        links: &Vec<(Range<usize>, LinkMark)>,
        position: Point<Pixels>,
    ) -> Option<LinkMark> {
        let offset = layout.index_for_position(position).ok()?;
        for (range, link) in links.iter() {
            if range.contains(&offset) {
                return Some(link.clone());
            }
        }

        None
    }

    /// Paint selected bounds for debug.
    #[allow(unused)]
    fn paint_selected_bounds(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        window.paint_quad(gpui::PaintQuad {
            bounds,
            background: cx.theme().blue.alpha(0.01).into(),
            corner_radii: gpui::Corners::default(),
            border_color: gpui::transparent_black(),
            border_style: BorderStyle::default(),
            border_widths: gpui::Edges::all(px(0.)),
        });
    }

    fn layout_selections(
        &self,
        text_layout: &TextLayout,
        window: &mut Window,
        cx: &mut App,
    ) -> (bool, bool, Option<Selection>) {
        let Some(text_view_state) = GlobalState::global(cx).text_view_state() else {
            return (false, false, None);
        };

        let text_view_state = text_view_state.read(cx);
        let is_selectable = text_view_state.is_selectable();
        if !text_view_state.has_selection() {
            return (is_selectable, false, None);
        }

        let line_height = window.line_height();
        let selection_bounds = text_view_state.selection_bounds();

        // Use for debug selection bounds
        // self.paint_selected_bounds(selection_bounds, window, cx);

        let mut selection: Option<Selection> = None;
        let mut offset = 0;
        let mut chars = self.text.chars().peekable();
        while let Some(c) = chars.next() {
            let Some(pos) = text_layout.position_for_index(offset) else {
                offset += c.len_utf8();
                continue;
            };

            let mut char_width = line_height.half();
            if let Some(next_pos) = text_layout.position_for_index(offset + 1) {
                if next_pos.y == pos.y {
                    char_width = next_pos.x - pos.x;
                }
            }

            if point_in_text_selection(pos, char_width, &selection_bounds, line_height) {
                if selection.is_none() {
                    selection = Some((offset..offset).into());
                }

                let next_offset = offset + c.len_utf8();
                selection.as_mut().unwrap().end = next_offset;
            }

            offset += c.len_utf8();
        }

        (true, true, selection)
    }

    /// Paint the selection background.
    fn paint_selection(
        selection: &Selection,
        text_layout: &TextLayout,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut start = selection.start;
        let mut end = selection.end;
        if end < start {
            std::mem::swap(&mut start, &mut end);
        }
        let Some(start_position) = text_layout.position_for_index(start) else {
            return;
        };
        let Some(end_position) = text_layout.position_for_index(end) else {
            return;
        };

        let line_height = text_layout.line_height();
        if start_position.y == end_position.y {
            window.paint_quad(quad(
                Bounds::from_corners(
                    start_position,
                    point(end_position.x, end_position.y + line_height),
                ),
                px(0.),
                cx.theme().selection,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));
        } else {
            window.paint_quad(quad(
                Bounds::from_corners(
                    start_position,
                    point(bounds.right(), start_position.y + line_height),
                ),
                px(0.),
                cx.theme().selection,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));

            if end_position.y > start_position.y + line_height {
                window.paint_quad(quad(
                    Bounds::from_corners(
                        point(bounds.left(), start_position.y + line_height),
                        point(bounds.right(), end_position.y),
                    ),
                    px(0.),
                    cx.theme().selection,
                    Edges::default(),
                    gpui::transparent_black(),
                    BorderStyle::default(),
                ));
            }

            window.paint_quad(quad(
                Bounds::from_corners(
                    point(bounds.left(), end_position.y),
                    point(end_position.x, end_position.y + line_height),
                ),
                px(0.),
                cx.theme().selection,
                Edges::default(),
                gpui::transparent_black(),
                BorderStyle::default(),
            ));
        }
    }

    fn paint_inline_code(
        ranges: &[Range<usize>],
        style: &InlineCodeStyle,
        text_layout: &TextLayout,
        bounds: &Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if ranges.is_empty() || !style.is_enabled() {
            return;
        }

        let line_height = text_layout.line_height();
        let background = style.background_color.unwrap_or(transparent_black());
        let border_color = style.border_color.unwrap_or(transparent_black());
        let border_width = style.border_width;
        let radius = style.border_radius;
        let pad_x = style.padding_x;
        let pad_y = style.padding_y;

        for range in ranges {
            if range.start >= range.end {
                continue;
            }
            let Some(start_position) = text_layout.position_for_index(range.start) else {
                continue;
            };
            let Some(end_position) = text_layout.position_for_index(range.end) else {
                continue;
            };

            if start_position.y == end_position.y {
                let rect = Bounds::from_corners(
                    start_position,
                    point(end_position.x, end_position.y + line_height),
                );
                Self::paint_inline_code_quad(
                    rect,
                    background,
                    border_color,
                    border_width,
                    radius,
                    pad_x,
                    pad_y,
                    window,
                );
            } else {
                let first = Bounds::from_corners(
                    start_position,
                    point(bounds.right(), start_position.y + line_height),
                );
                Self::paint_inline_code_quad(
                    first,
                    background,
                    border_color,
                    border_width,
                    radius,
                    pad_x,
                    pad_y,
                    window,
                );

                if end_position.y > start_position.y + line_height {
                    let middle = Bounds::from_corners(
                        point(bounds.left(), start_position.y + line_height),
                        point(bounds.right(), end_position.y),
                    );
                    Self::paint_inline_code_quad(
                        middle,
                        background,
                        border_color,
                        border_width,
                        px(0.),
                        pad_x,
                        pad_y,
                        window,
                    );
                }

                let last = Bounds::from_corners(
                    point(bounds.left(), end_position.y),
                    point(end_position.x, end_position.y + line_height),
                );
                Self::paint_inline_code_quad(
                    last,
                    background,
                    border_color,
                    border_width,
                    radius,
                    pad_x,
                    pad_y,
                    window,
                );
            }
        }
    }

    fn paint_inline_code_quad(
        rect: Bounds<Pixels>,
        background: Hsla,
        border_color: Hsla,
        border_width: Pixels,
        radius: Pixels,
        pad_x: Pixels,
        pad_y: Pixels,
        window: &mut Window,
    ) {
        let padded = Bounds::from_corners(
            point(rect.left() - pad_x, rect.top() - pad_y),
            point(rect.right() + pad_x, rect.bottom() + pad_y),
        );
        window.paint_quad(quad(
            padded,
            Corners::all(radius),
            background,
            Edges::all(border_width),
            border_color,
            BorderStyle::Solid,
        ));
    }
}

impl IntoElement for Inline {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Inline {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_element_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text_style = window.text_style();

        let runs = if self.code_ranges.is_empty() || self.inline_code_style.is_none() {
            let mut runs = Vec::new();
            let mut ix = 0;
            for (range, highlight) in self.highlights.iter() {
                if ix < range.start {
                    runs.push(text_style.clone().to_run(range.start - ix));
                }
                runs.push(text_style.clone().highlight(*highlight).to_run(range.len()));
                ix = range.end;
            }
            if ix < self.text.len() {
                runs.push(text_style.to_run(self.text.len() - ix));
            }
            runs
        } else {
            let inline_code_style = self
                .inline_code_style
                .as_ref()
                .expect("inline code style");
            let mut breakpoints = Vec::new();
            breakpoints.push(0);
            breakpoints.push(self.text.len());
            for (range, _) in self.highlights.iter() {
                breakpoints.push(range.start);
                breakpoints.push(range.end);
            }
            for range in self.code_ranges.iter() {
                breakpoints.push(range.start);
                breakpoints.push(range.end);
            }
            breakpoints.sort_unstable();
            breakpoints.dedup();

            let mut runs = Vec::new();
            let mut highlight_index = 0;
            let mut code_index = 0;

            for window_bounds in breakpoints.windows(2) {
                let start = window_bounds[0];
                let end = window_bounds[1];
                if start == end {
                    continue;
                }

                while highlight_index < self.highlights.len()
                    && self.highlights[highlight_index].0.end <= start
                {
                    highlight_index += 1;
                }

                let highlight = if highlight_index < self.highlights.len() {
                    let (range, style) = &self.highlights[highlight_index];
                    if range.start <= start && range.end >= end {
                        Some(*style)
                    } else {
                        None
                    }
                } else {
                    None
                };

                while code_index < self.code_ranges.len()
                    && self.code_ranges[code_index].end <= start
                {
                    code_index += 1;
                }

                let is_code = if code_index < self.code_ranges.len() {
                    let range = &self.code_ranges[code_index];
                    range.start <= start && range.end >= end
                } else {
                    false
                };

                let mut run_style = text_style.clone();
                if let Some(style) = highlight {
                    run_style = run_style.highlight(style);
                }
                if is_code {
                    if let Some(font_family) = inline_code_style.font_family.as_ref() {
                        run_style.font_family = font_family.clone();
                    }
                    if let Some(font_size) = inline_code_style.font_size {
                        run_style.font_size = font_size.into();
                    }
                    if let Some(color) = inline_code_style.text_color {
                        run_style.color = color;
                    }
                }

                runs.push(run_style.to_run(end - start));
            }

            runs
        };

        self.styled_text = StyledText::new(self.text.clone()).with_runs(runs);
        let (layout_id, _) =
            self.styled_text
                .request_layout(global_element_id, inspector_id, window, cx);

        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.styled_text
            .prepaint(id, inspector_id, bounds, &mut (), window, cx);

        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        hitbox
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let current_view = window.current_view();
        let hitbox = prepaint;
        let mut state = self.state.lock().unwrap();

        let text_layout = self.styled_text.layout().clone();
        if let Some(style) = self.inline_code_style.as_ref() {
            Self::paint_inline_code(&self.code_ranges, style, &text_layout, &bounds, window, cx);
        }
        self.styled_text
            .paint(global_id, None, bounds, &mut (), &mut (), window, cx);

        // layout selections
        let (is_selectable, is_selection, selection) =
            self.layout_selections(&text_layout, window, cx);

        state.selection = selection;

        if is_selection || is_selectable {
            window.set_cursor_style(CursorStyle::IBeam, &hitbox);
        }

        // link cursor pointer
        let mouse_position = window.mouse_position();
        if let Some(link) = Self::link_for_position(&text_layout, &self.links, mouse_position) {
            if !link.requires_modifiers || window.modifiers().secondary() {
                window.set_cursor_style(CursorStyle::PointingHand, &hitbox);
            }
        }

        if let Some(selection) = &state.selection {
            Self::paint_selection(selection, &text_layout, &bounds, window, cx);
        }

        // mouse move, update hovered link
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let text_layout = text_layout.clone();
            let mut hovered_index = state.hovered_index;
            move |event: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.is_hovered(window) {
                    return;
                }

                let current = hovered_index;
                let updated = text_layout.index_for_position(event.position).ok();
                //  notify update when hovering over different links
                if current != updated {
                    hovered_index = updated;
                    cx.notify(current_view);
                }
            }
        });

        if !is_selection {
            // click to open link
            window.on_mouse_event({
                let links = self.links.clone();
                let text_layout = text_layout.clone();

                move |event: &MouseUpEvent, phase, _, cx| {
                    if !bounds.contains(&event.position) || !phase.bubble() {
                        return;
                    }

                    if let Some(link) =
                        Self::link_for_position(&text_layout, &links, event.position)
                    {
                        if link.requires_modifiers && !event.modifiers.secondary() {
                            return;
                        }
                        cx.stop_propagation();
                        cx.open_url(&link.url);
                    }
                }
            });
        }
    }
}

/// Check if a `pos` is within a `bounds`, considering multi-line selections.
fn point_in_text_selection(
    pos: Point<Pixels>,
    char_width: Pixels,
    bounds: &Bounds<Pixels>,
    line_height: Pixels,
) -> bool {
    let top = bounds.top();
    let bottom = bounds.bottom();
    let left = bounds.left();
    let right = bounds.right();

    // Out of the vertical bounds
    if pos.y + line_height < top || pos.y >= bottom {
        return false;
    }

    let single_line = (bottom - top) <= line_height;
    if single_line {
        // If it's a single line selection, just check horizontal bounds
        return pos.x + char_width.half() >= left && pos.x + char_width.half() <= right;
    }

    let is_above = pos.y <= top;
    let is_below = pos.y + line_height >= bottom;

    if is_above {
        return pos.x + char_width.half() >= left;
    } else if is_below {
        return pos.x + char_width.half() <= right;
    } else {
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::point_in_text_selection;
    use gpui::{point, px, size, Bounds};

    #[test]
    fn test_point_in_text_selection() {
        let line_height = px(20.);
        let char_width = px(10.);
        let bounds = Bounds {
            origin: point(px(50.), px(50.)),
            size: size(px(100.), px(100.)),
        };

        // First line but haft line height, true
        // | p --------|
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(50.), px(40.)),
            char_width,
            &bounds,
            line_height
        ));

        // First line in selection, true
        // | p --------|
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(50.), px(50.)),
            char_width,
            &bounds,
            line_height
        ));
        // First line, but left out of selection, false
        // p |-----------|
        //   | selection |
        //   |-----------|
        assert!(!point_in_text_selection(
            point(px(40.), px(50.)),
            char_width,
            &bounds,
            line_height
        ));
        // First line but right out of selection, true
        // |-----------| p
        // | selection |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(160.), px(50.)),
            char_width,
            &bounds,
            line_height
        ));

        // Middle line in selection, true
        // |-----------|
        // |     p     |
        // |-----------|
        assert!(point_in_text_selection(
            point(px(100.), px(70.)),
            char_width,
            &bounds,
            line_height
        ));
        // Middle line, but left out of selection, true
        //   |-----------|
        // p | selection |
        //   |-----------|
        assert!(point_in_text_selection(
            point(px(40.), px(70.)),
            char_width,
            &bounds,
            line_height
        ));
        // Middle line, but right out of selection, true
        // |-----------|
        // | selection | p
        // |-----------|
        assert!(point_in_text_selection(
            point(px(160.), px(70.)),
            char_width,
            &bounds,
            line_height
        ));

        // Last line in selection, true
        // |-----------|
        // | selection |
        // |------- p -|
        assert!(point_in_text_selection(
            point(px(100.), px(140.)),
            char_width,
            &bounds,
            line_height
        ));
        // Last line, but left out of selection, true
        //
        //   |-----------|
        //   | selection |
        // p |-----------|
        assert!(point_in_text_selection(
            point(px(40.), px(140.)),
            char_width,
            &bounds,
            line_height
        ));
        // Last line, but right out of selection, false
        // |-----------|
        // | selection |
        // |-----------| p
        assert!(!point_in_text_selection(
            point(px(160.), px(140.)),
            char_width,
            &bounds,
            line_height
        ));

        // Out of vertical bounds (top), false
        //       p
        // |-----------|
        // | selection |
        // |-----------|
        assert!(!point_in_text_selection(
            point(px(100.), px(20.)),
            char_width,
            &bounds,
            line_height
        ));
        // Out of vertical bounds (bottom), false
        // |-----------|
        // | selection |
        // |-----------|
        //       p
        assert!(!point_in_text_selection(
            point(px(100.), px(160.)),
            char_width,
            &bounds,
            line_height
        ));
    }
}
