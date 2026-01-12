use std::sync::Arc;

use gpui::{Hsla, IsZero, Pixels, Rems, SharedString, StyleRefinement, px, rems};

use crate::highlighter::HighlightTheme;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InlineCodeStyle {
    pub font_family: Option<SharedString>,
    pub font_size: Option<Pixels>,
    pub text_color: Option<Hsla>,
    pub background_color: Option<Hsla>,
    pub border_color: Option<Hsla>,
    pub border_width: Pixels,
    pub border_radius: Pixels,
    pub padding_x: Pixels,
    pub padding_y: Pixels,
}

impl InlineCodeStyle {
    pub fn is_enabled(&self) -> bool {
        self.font_family.is_some()
            || self.font_size.is_some()
            || self.text_color.is_some()
            || self.background_color.is_some()
            || self.border_color.is_some()
            || !self.border_width.is_zero()
            || !self.border_radius.is_zero()
            || !self.padding_x.is_zero()
            || !self.padding_y.is_zero()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CodeTokenLinks {
    pub enabled: bool,
    pub worktree_id: Option<SharedString>,
}

impl CodeTokenLinks {
    pub fn enabled(worktree_id: Option<SharedString>) -> Self {
        Self {
            enabled: true,
            worktree_id,
        }
    }
}

/// TextViewStyle used to customize the style for [`TextView`].
#[derive(Clone)]
pub struct TextViewStyle {
    /// Gap of each paragraphs, default is 1 rem.
    pub paragraph_gap: Rems,
    /// Base font size for headings, default is 14px.
    pub heading_base_font_size: Pixels,
    /// Function to calculate heading font size based on heading level (1-6).
    ///
    /// The first parameter is the heading level (1-6), the second parameter is the base font size.
    /// The second parameter is the base font size.
    pub heading_font_size: Option<Arc<dyn Fn(u8, Pixels) -> Pixels + Send + Sync + 'static>>,
    /// Highlight theme for code blocks. Default: [`HighlightTheme::default_light()`]
    pub highlight_theme: Arc<HighlightTheme>,
    /// The style refinement for code blocks.
    pub code_block: StyleRefinement,
    /// Inline code styling overrides.
    pub inline_code: InlineCodeStyle,
    /// Token-linkification settings for inline code.
    pub code_token_links: CodeTokenLinks,
    pub is_dark: bool,
}

impl PartialEq for TextViewStyle {
    fn eq(&self, other: &Self) -> bool {
        self.paragraph_gap == other.paragraph_gap
            && self.heading_base_font_size == other.heading_base_font_size
            && self.highlight_theme == other.highlight_theme
            && self.code_block == other.code_block
            && self.inline_code == other.inline_code
            && self.code_token_links == other.code_token_links
            && self.is_dark == other.is_dark
    }
}

impl Default for TextViewStyle {
    fn default() -> Self {
        Self {
            paragraph_gap: rems(1.),
            heading_base_font_size: px(14.),
            heading_font_size: None,
            highlight_theme: HighlightTheme::default_light().clone(),
            code_block: StyleRefinement::default(),
            inline_code: InlineCodeStyle::default(),
            code_token_links: CodeTokenLinks::default(),
            is_dark: false,
        }
    }
}

impl TextViewStyle {
    /// Set paragraph gap, default is 1 rem.
    pub fn paragraph_gap(mut self, gap: Rems) -> Self {
        self.paragraph_gap = gap;
        self
    }

    pub fn heading_font_size<F>(mut self, f: F) -> Self
    where
        F: Fn(u8, Pixels) -> Pixels + Send + Sync + 'static,
    {
        self.heading_font_size = Some(Arc::new(f));
        self
    }

    /// Set style for code blocks.
    pub fn code_block(mut self, style: StyleRefinement) -> Self {
        self.code_block = style;
        self
    }

    /// Set inline code style overrides.
    pub fn inline_code(mut self, style: InlineCodeStyle) -> Self {
        self.inline_code = style;
        self
    }

    /// Enable token linkification for inline code.
    pub fn code_token_links(mut self, options: CodeTokenLinks) -> Self {
        self.code_token_links = options;
        self
    }
}
