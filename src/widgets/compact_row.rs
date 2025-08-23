use crate::{app_state::AudioPlayable, fonts::SANS_BOLD, util::duration_to_str};
use iced::{
    Alignment, Border, Color, Element, Event, Length, Rectangle, Renderer, Shadow, Size, Theme,
    advanced::{
        Clipboard, Layout, Shell, Widget,
        layout::{Node, flex},
        mouse, renderer,
        widget::Tree,
    },
    theme::palette,
    touch,
    widget::{Column, text},
    window,
};
use std::time::{Duration, Instant};

pub fn compact_row<'a, Message, Theme>(
    playable: &'a impl AudioPlayable,
    index: usize,
    is_selected: bool,
    row_sizes: &(Length, Length, Length, Length, Length),
) -> CompactRow<'a, Message, Theme>
where
    Theme: Catalog + iced::widget::text::Catalog + 'a,
    Message: 'a,
{
    let artist = playable.get_artist();
    let title = playable.get_title();
    let album = playable.get_album();
    let genre = playable.get_genre();
    let duration = playable.get_duration();
    let duration_str = duration_to_str(duration);

    let mut artist_title_cell: Column<'_, Message, Theme, Renderer> = Column::new().push(
        text(title)
            .font(SANS_BOLD)
            .size(15)
            .wrapping(text::Wrapping::WordOrGlyph),
    );
    if !artist.is_empty() {
        artist_title_cell =
            artist_title_cell.push(text(artist).size(14).wrapping(text::Wrapping::WordOrGlyph))
    }
    CompactRow::new(is_selected)
        .push(
            text((index).to_string())
                .size(13)
                .width(row_sizes.0)
                .wrapping(text::Wrapping::WordOrGlyph),
        )
        .push(artist_title_cell.width(row_sizes.1))
        .push(
            text(album)
                .width(row_sizes.2)
                .wrapping(text::Wrapping::WordOrGlyph),
        )
        .push(
            text(genre)
                .width(row_sizes.3)
                .wrapping(text::Wrapping::WordOrGlyph),
        )
        .push(
            text(duration_str)
                .size(13)
                .width(row_sizes.4)
                .align_x(Alignment::End),
        )
        .width(Length::Fill)
        .padding(20)
        .spacing(40)
}

#[derive(Default)]
struct CompactRowState {
    last_click: Option<Instant>,
    is_hovered: bool,
}

pub struct CompactRow<'a, Message, Theme, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
    Theme: Catalog,
{
    padding: u16,
    spacing: u16,
    width: Length,
    height: Length,
    vertical_alignment: Alignment,
    content: Vec<Element<'a, Message, Theme, Renderer>>,
    on_select: Option<Message>,
    on_dbl_click: Option<Message>,
    on_right_click: Option<Message>,
    is_selected: bool,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> CompactRow<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Theme: Catalog,
{
    pub fn new(is_selected: bool) -> Self {
        Self {
            padding: 5,
            spacing: 5,
            width: Length::Fill,
            height: Length::Shrink,
            vertical_alignment: Alignment::Center,
            content: vec![],
            on_select: None,
            on_dbl_click: None,
            on_right_click: None,
            is_selected,
            class: Theme::default(),
        }
    }

    pub fn push(mut self, element: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        self.content.push(element.into());
        self
    }

    pub fn padding(mut self, units: u16) -> Self {
        self.padding = units;
        self
    }

    pub fn width(mut self, length: Length) -> Self {
        self.width = length;
        self
    }

    pub fn spacing(mut self, units: u16) -> Self {
        self.spacing = units;
        self
    }

    pub fn on_select(mut self, on_select: Message) -> Self {
        self.on_select = Some(on_select);
        self
    }

    pub fn on_dbl_click(mut self, on_dbl_click: Message) -> Self {
        self.on_dbl_click = Some(on_dbl_click);
        self
    }
    pub fn on_right_click(mut self, on_right_click: Message) -> Self {
        self.on_right_click = Some(on_right_click);
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for CompactRow<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: renderer::Renderer,
    Theme: Catalog,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn children(&self) -> Vec<Tree> {
        self.content.iter().map(Tree::new).collect()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(CompactRowState::default())
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.content);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> Node {
        flex::resolve(
            flex::Axis::Horizontal,
            renderer,
            limits,
            self.width,
            self.height,
            self.padding.into(),
            self.spacing.into(),
            self.vertical_alignment,
            &mut self.content,
            &mut tree.children,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<CompactRowState>();
        let status = if self.is_selected {
            Status::Selected
        } else if state.is_hovered {
            Status::Hovered
        } else {
            Status::Default
        };

        let appearance = theme.style(&self.class, &status);
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: Border::default(),
                shadow: Shadow::default(),
                snap: true,
            },
            appearance.background,
        );

        for ((content, tree), layout) in self
            .content
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((content, tree), layout) in self
            .content
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
        if shell.is_event_captured() {
            return;
        }

        let state = tree.state.downcast_mut::<CompactRowState>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if cursor.is_over(layout.bounds()) {
                    if let Some(last) = state.last_click {
                        if last.elapsed() < Duration::from_millis(500) {
                            // Double click detected
                            if let Some(on_dbl_click) = &self.on_dbl_click {
                                shell.publish(on_dbl_click.clone());
                                shell.capture_event();
                                state.last_click = None;
                                return;
                            }
                        }
                    }
                    state.last_click = Some(Instant::now());

                    if self.on_select.is_some() {
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_select) = &self.on_select {
                    let bounds = layout.bounds();

                    if cursor.is_over(bounds) {
                        shell.publish(on_select.clone());
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if self.on_right_click.is_some() && cursor.is_over(layout.bounds()) {
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                if let Some(on_right_click) = &self.on_right_click {
                    if cursor.is_over(layout.bounds()) {
                        shell.capture_event();
                        shell.publish(on_right_click.clone());
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let was_hovered = state.is_hovered;
                state.is_hovered = cursor.is_over(layout.bounds());
                if !was_hovered && state.is_hovered {
                    shell.request_redraw();
                }
            }
            _ => {}
        }

        if let Event::Window(window::Event::Resized(_now)) = event {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message, Theme, Renderer> From<CompactRow<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
    Theme: 'a + Catalog,
{
    fn from(value: CompactRow<'a, Message, Theme, Renderer>) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Default,
    Hovered,
    Selected,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub background: Color,
}

impl Style {
    pub fn with_background(self, background: impl Into<Color>) -> Self {
        Self {
            background: background.into(),
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
        }
    }
}

pub trait Catalog {
    type Class<'a>;
    fn default<'a>() -> Self::Class<'a>;
    fn style(&self, class: &Self::Class<'_>, status: &Status) -> Style;
}

pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, &Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: &Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: &Status) -> Style {
    let palette = theme.extended_palette();
    match status {
        Status::Default => Style::default(),
        Status::Selected => styled(palette.background.strong),
        Status::Hovered => styled(palette.background.weakest),
    }
}

fn styled(pair: palette::Pair) -> Style {
    Style {
        background: pair.color,
    }
}
