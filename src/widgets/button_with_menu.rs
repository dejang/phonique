use std::{any::Any, cell::Cell};

use iced::{
    Background, Border, Color, Length, Point, Rectangle, Theme,
    advanced::{
        Layout, Widget, renderer,
        widget::{Id, Operation, Tree, operate, operation::Outcome},
    },
    mouse::Button,
    widget::Text,
};

use super::context_menu::{ContextMenu, MenuStyle};

pub struct Style {
    background_color: Background,
    text_color: Color,
    border_color: Color,
    shadow_color: Color,
}

pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, &Status) -> Style + 'a>;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Active,
    Pressed,
    Disabled,
}

pub trait Catalog {
    type Class<'a>;
    fn default<'a>() -> Self::Class<'a>;
    fn style(&self, class: &Self::Class<'_>, status: &Status) -> Style;
}

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: &Status) -> Style {
        class(self, status)
    }
}

fn default(theme: &Theme, status: &Status) -> Style {
    let extended_palette = theme.extended_palette();

    match status {
        Status::Active => Style {
            background_color: Background::Color(extended_palette.primary.base.color),
            text_color: extended_palette.primary.base.text,
            border_color: extended_palette.primary.strong.color,
            shadow_color: Color::TRANSPARENT,
        },
        Status::Pressed => Style {
            background_color: Background::Color(extended_palette.primary.strong.color),
            text_color: extended_palette.primary.strong.text,
            border_color: extended_palette.primary.strong.color,
            shadow_color: Color::TRANSPARENT,
        },
        Status::Disabled => Style {
            background_color: Background::Color(extended_palette.secondary.weak.color),
            text_color: extended_palette.secondary.weak.text,
            border_color: extended_palette.primary.base.color,
            shadow_color: Color::TRANSPARENT,
        },
    }
}

pub struct ButtonWithMenu<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + iced::widget::text::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    content: iced::Element<'a, Message, Theme, Renderer>,
    menu_options: &'a [&'a str],
    width: Length,
    height: Length,
    padding: f32,
    font_size: f32,
    menu_style: MenuStyle,
    on_option_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    id: Option<&'a Id>,
}

impl<'a, Message, Theme, Renderer> ButtonWithMenu<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + iced::widget::text::Catalog + 'a,
    Renderer: iced::advanced::text::Renderer + 'a,
{
    pub fn new(content: &'a str, menu_options: &'a [&'a str]) -> Self {
        Self {
            content: Text::new(content).into(),
            menu_options,
            width: Length::Shrink,
            height: Length::Shrink,
            padding: 8.0,
            font_size: 14.0,
            menu_style: MenuStyle::default(),
            on_option_select: None,
            id: None,
        }
    }

    pub fn set_id(mut self, id: &'a Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
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

    pub fn on_option_select<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) -> Message + 'a,
    {
        self.on_option_select = Some(Box::new(f));
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ButtonWithMenu<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: Catalog + iced::widget::text::Catalog + iced::widget::container::Catalog,
    Renderer: iced::advanced::text::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        iced::Size {
            width: self.width,
            height: self.height,
        }
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content]);
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        let padding = iced::Padding::new(self.padding);
        let limits = limits.width(self.width).height(self.height).shrink(padding);

        let content_layout =
            self.content
                .as_widget()
                .layout(&mut tree.children[0], renderer, &limits);

        let size = limits
            .resolve(self.width, self.height, content_layout.size())
            .expand(padding);

        iced::advanced::layout::Node::with_children(
            size,
            vec![content_layout.move_to(iced::Point::new(padding.left, padding.top))],
        )
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(Status::Active)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_mut::<Status>();
        match event {
            iced::Event::Mouse(mouse_event) => {
                match mouse_event {
                    iced::mouse::Event::ButtonPressed(Button::Left) => {
                        if cursor.is_over(layout.bounds()) {
                            // Toggle between Active and Pressed
                            *state = if *state == Status::Pressed {
                                Status::Active
                            } else {
                                Status::Pressed
                            };
                            shell.request_redraw();
                        } else if *state == Status::Pressed {
                            // Click outside while menu is open - close it
                            *state = Status::Active;
                            shell.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: iced::advanced::Layout<'_>,
        _renderer: &Renderer,
        _viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<Status>();
        match state {
            Status::Pressed => {
                let bounds = layout.bounds();
                let overlay_position = Point::new(
                    bounds.x + translation.x,
                    bounds.y + bounds.height + translation.y + 2.0, // 2px gap
                );

                if let Some(on_select) = &self.on_option_select {
                    Some(iced::advanced::overlay::Element::new(Box::new(
                        ContextMenu::new(self.menu_options, overlay_position)
                            .min_width(bounds.width)
                            .font_size(self.font_size)
                            .menu_style(self.menu_style.clone())
                            .on_select(|index| (on_select)(index)),
                    )))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<Status>();
        operation.custom(self.id, layout.bounds(), state);
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();
        let style_fn = <Theme as Catalog>::default();
        let state = tree.state.downcast_ref::<Status>();
        let button_style = <Theme as Catalog>::style(theme, &style_fn, state);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: button_style.border_color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                shadow: Default::default(),
                snap: true,
            },
            button_style.background_color,
        );

        if let Some(content_layout) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                &renderer::Style {
                    text_color: button_style.text_color,
                },
                content_layout,
                cursor,
                viewport,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<ButtonWithMenu<'a, Message, Theme, Renderer>>
    for iced::Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + iced::advanced::text::Renderer,
    Theme: 'a + Catalog + iced::widget::text::Catalog + iced::widget::container::Catalog,
{
    fn from(value: ButtonWithMenu<'a, Message, Theme, Renderer>) -> Self {
        Self::new(value)
    }
}

/// Create a [`Task`](iced::Task) to signal the overlay has been clicked
pub fn clicked_overlay(target: Id) -> iced::Task<bool> {
    struct ClickOverlay {
        target_id: Id,
        clicked: bool,
    };

    impl Operation<bool> for ClickOverlay {
        fn container(
            &mut self,
            id: Option<&Id>,
            _bounds: Rectangle,
            operate_on_children: &mut dyn FnMut(&mut dyn Operation<bool>),
        ) {
            if let Some(current_id) = id {
                if self.target_id.eq(current_id) {
                    return;
                }
            }

            operate_on_children(self);
        }

        fn custom(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
            if let Some(current_id) = id {
                if self.target_id.eq(current_id) {
                    self.clicked = true;
                    let state = state.downcast_mut::<Status>().unwrap();
                    log::info!("{state:?}");
                    *state = Status::Active;
                }
            }
        }

        fn finish(&self) -> Outcome<bool> {
            Outcome::Some(true)
        }
    }

    operate(ClickOverlay {
        target_id: target,
        clicked: false,
    })
}
