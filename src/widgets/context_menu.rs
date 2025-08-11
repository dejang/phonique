use iced::{
    Background, Border, Color, Point, Rectangle, Shadow, Size,
    advanced::{overlay, renderer, text, widget::Id},
    mouse::Button,
};

#[derive(Clone)]
pub struct MenuStyle {
    pub background_color: Color,
    pub hover_color: Color,
    pub text_color: Color,
    pub border: Border,
    pub shadow: Shadow,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            background_color: Color::WHITE,
            hover_color: Color::from_rgb(0.9, 0.9, 0.9),
            text_color: Color::BLACK,
            border: Border {
                color: Color::from_rgb(0.7, 0.7, 0.7),
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
        }
    }
}

impl MenuStyle {
    pub fn dark() -> Self {
        Self {
            background_color: Color::from_rgb(0.2, 0.2, 0.3),
            hover_color: Color::from_rgb(0.3, 0.3, 0.4),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgb(0.5, 0.5, 0.6),
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
        }
    }
}

pub struct ContextMenu<'a, Message>
where
    Message: Clone,
{
    options: &'a [&'a str],
    position: Point,
    min_width: f32,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    font_size: f32,
    menu_style: MenuStyle,
}

impl<'a, Message> ContextMenu<'a, Message>
where
    Message: Clone,
{
    pub fn new(options: &'a [&'a str], position: Point) -> Self {
        Self {
            options,
            position,
            min_width: 120.0,
            on_select: None,
            font_size: 14.0,
            menu_style: MenuStyle::default(),
        }
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn menu_style(mut self, style: MenuStyle) -> Self {
        self.menu_style = style;
        self
    }

    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> Message + 'a,
    {
        self.on_select = Some(Box::new(f));
        self
    }
}

impl<'a, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for ContextMenu<'a, Message>
where
    Message: Clone,
    Theme: iced::widget::container::Catalog + iced::widget::text::Catalog,
    Renderer: text::Renderer,
{
    fn layout(&mut self, _renderer: &Renderer, _bounds: Size) -> iced::advanced::layout::Node {
        let item_height = 36.0;
        let padding = 8.0;
        let total_height = self.options.len() as f32 * item_height + padding * 2.0;

        let size = Size::new(self.min_width.max(200.0), total_height);

        let mut children = Vec::new();
        for i in 0..self.options.len() {
            let item_y = padding + i as f32 * item_height;
            children.push(
                iced::advanced::layout::Node::new(Size::new(
                    size.width - padding * 2.0,
                    item_height,
                ))
                .move_to(Point::new(padding, item_y)),
            );
        }

        iced::advanced::layout::Node::with_children(size, children).move_to(self.position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        // Draw background and border
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: self.menu_style.border,
                shadow: self.menu_style.shadow,
                snap: true,
            },
            Background::Color(self.menu_style.background_color),
        );

        // Draw menu items
        for (option, item_layout) in self.options.iter().zip(layout.children()) {
            let item_bounds = item_layout.bounds();
            let is_hovered = cursor.is_over(item_bounds);

            // Highlight hovered item
            if is_hovered {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: item_bounds,
                        border: Border::default(),
                        shadow: Default::default(),
                        snap: true,
                    },
                    Background::Color(self.menu_style.hover_color),
                );
            }

            // Draw text
            let text_padding = 12.0;
            let text_y = item_bounds.y + (item_bounds.height - self.font_size) / 2.0;
            let text_position = Point::new(item_bounds.x + text_padding, text_y);
            let text_bounds = Rectangle::new(
                text_position,
                Size::new(item_bounds.width - text_padding * 2.0, self.font_size + 4.0),
            );

            renderer.fill_text(
                text::Text {
                    content: option.to_string(),
                    bounds: text_bounds.size(),
                    size: iced::Pixels(self.font_size),
                    line_height: text::LineHeight::Absolute(iced::Pixels(self.font_size)),
                    font: renderer.default_font(),
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: iced::alignment::Vertical::Top.into(),
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::None,
                },
                text_position,
                self.menu_style.text_color,
                text_bounds,
            );
        }
    }

    fn update(
        &mut self,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
    ) {
        let is_over_overlay = cursor.is_over(layout.bounds());

        // Always capture ALL events when over overlay to prevent propagation
        if is_over_overlay {
            shell.capture_event();
        }

        match event {
            iced::Event::Mouse(mouse_event) => match mouse_event {
                iced::mouse::Event::ButtonPressed(Button::Left) => {
                    if is_over_overlay {
                        if let Some(ref on_select) = self.on_select {
                            for (i, item_layout) in layout.children().enumerate() {
                                if cursor.is_over(item_layout.bounds()) {
                                    shell.publish((on_select)(i));
                                    return;
                                }
                            }
                        }
                    }
                }
                iced::mouse::Event::CursorMoved { .. } => {
                    if is_over_overlay {
                        shell.request_redraw();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}
