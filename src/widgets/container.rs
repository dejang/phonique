use iced::{
    Border, Color, Element, Length, Padding, Point, Shadow, Theme,
    advanced::{
        Widget,
        layout::{self},
        renderer,
        widget::{self, Tree},
    },
    alignment::{Horizontal, Vertical},
    overlay::menu,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id(widget::Id);

impl Id {
    /// Creates a custom [`Id`].
    pub fn new(id: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(widget::Id::new(id))
    }

    /// Creates a unique [`Id`].
    ///
    /// This function produces a different [`Id`] every time it is called.
    pub fn unique() -> Self {
        Self(widget::Id::unique())
    }
}

impl From<Id> for widget::Id {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl From<&'static str> for Id {
    fn from(value: &'static str) -> Self {
        Id::new(value)
    }
}

impl From<&i32> for Id {
    fn from(value: &i32) -> Self {
        Id::new(std::borrow::Cow::Owned(value.to_string()))
    }
}

pub struct Container<'a, T, Message, Theme, Renderer = iced::Renderer>
where
    Message: 'a + Clone,
    Theme: Catalog + menu::Catalog,
    T: ToString + Clone,
{
    id: Option<Id>,
    content: Element<'a, Message, Theme, Renderer>,
    padding: Padding,
    width: Length,
    height: Length,
    horizontal_alignment: Horizontal,
    vertical_alignment: Vertical,
    class: <Theme as Catalog>::Class<'a>,
    on_select: Option<Box<dyn Fn(Option<Id>) -> Message + 'a>>,
    menu_state: MenuState<'a, T>,
    on_menu_select: Option<Box<dyn Fn(Option<Id>, T) -> Message + 'a>>,
    on_menu_open: Option<Message>,
    on_menu_close: Option<Message>,
    on_menu_hover: Option<Box<dyn Fn(T) -> Message + 'a>>,
    menu_class: <Theme as menu::Catalog>::Class<'a>,
    style_fn: Option<StyleFn<'a, Theme>>,
    border_fn: Option<Box<dyn Fn(&Theme, &Status) -> Border + 'a>>,
}

#[derive(Debug, Clone)]
pub struct MenuState<'a, T> {
    pub selected: Option<usize>,
    pub options: &'a [T],
}

impl<'a, T> MenuState<'a, T> {
    pub fn new(options: &'a [T]) -> Self {
        Self {
            options,
            selected: None,
        }
    }
}

impl<'a, T, Message, Theme, Renderer> Container<'a, T, Message, Theme, Renderer>
where
    Theme: Catalog + menu::Catalog,
    Message: 'a + Clone,
    T: ToString + Clone + Eq + PartialEq,
{
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        menu_state: Option<MenuState<'a, T>>,
    ) -> Self {
        let menu_state = if let Some(state) = menu_state {
            state
        } else {
            MenuState {
                selected: None,
                options: &[],
            }
        };
        Self {
            id: None,
            content: content.into(),
            padding: 0.into(),
            width: Length::Shrink,
            height: Length::Shrink,
            horizontal_alignment: Horizontal::Center,
            vertical_alignment: Vertical::Center,
            class: <Theme as Catalog>::default(),
            on_select: None,
            menu_state,
            on_menu_select: None,
            on_menu_open: None,
            on_menu_close: None,
            on_menu_hover: None,
            style_fn: None,
            border_fn: None,
            menu_class: <Theme as menu::Catalog>::default(),
        }
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn width(mut self, value: impl Into<Length>) -> Self {
        self.width = value.into();
        self
    }

    pub fn height(mut self, value: impl Into<Length>) -> Self {
        self.height = value.into();
        self
    }

    pub fn id(mut self, value: impl Into<Id>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn align_x(mut self, value: impl Into<Horizontal>) -> Self {
        self.horizontal_alignment = value.into();
        self
    }

    pub fn align_y(mut self, value: impl Into<Vertical>) -> Self {
        self.vertical_alignment = value.into();
        self
    }

    pub fn style(mut self, value: impl Fn(&Theme, &Status) -> Style + 'a) -> Self {
        self.style_fn = Some(Box::new(value));
        self
    }

    pub fn on_select(mut self, callback: impl Fn(Option<Id>) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }

    pub fn menu_state(mut self, menu_state: MenuState<'a, T>) -> Self {
        self.menu_state = menu_state;
        self
    }

    pub fn on_menu_select(mut self, callback: impl Fn(Option<Id>, T) -> Message + 'a) -> Self {
        self.on_menu_select = Some(Box::new(callback));
        self
    }

    pub fn on_menu_open(mut self, message: Message) -> Self {
        self.on_menu_open = Some(message);
        self
    }

    pub fn on_menu_close(mut self, message: Message) -> Self {
        self.on_menu_close = Some(message);
        self
    }

    pub fn on_menu_hover(mut self, message: impl Fn(T) -> Message + 'a) -> Self {
        self.on_menu_hover = Some(Box::new(message));
        self
    }

    pub fn border(mut self, border: impl Fn(&Theme, &Status) -> Border + 'a) -> Self {
        self.border_fn = Some(Box::new(border));
        self
    }
}

struct State {
    is_selected: bool,
    show_context_menu: bool,
    menu_state: menu::State,
    cursor_position: iced::Point,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_selected: false,
            show_context_menu: false,
            menu_state: menu::State::new(),
            cursor_position: iced::Point::new(0.0, 0.0),
        }
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Container<'a, T, Message, Theme, Renderer>
where
    Theme: Catalog + menu::Catalog,
    Message: 'a + Clone,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer,
    T: ToString + Clone,
{
    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn size(&self) -> iced::Size<Length> {
        iced::Size {
            width: self.width,
            height: self.height,
        }
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        if !state.show_context_menu {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button)) => match button {
                iced::mouse::Button::Left => {
                    state.is_selected = cursor.is_over(layout.bounds());
                    state.show_context_menu = false;
                    if let Some(on_select) = &self.on_select
                        && state.is_selected
                    {
                        shell.publish((on_select)(self.id.clone()));
                    }
                    log::debug!("is selected: {}", state.is_selected);
                    shell.capture_event();
                    shell.request_redraw();
                }
                iced::mouse::Button::Right => {
                    log::debug!("show context menu {}", cursor.is_over(layout.bounds()));
                    if cursor.is_over(layout.bounds()) {
                        state.show_context_menu = true;
                        state.cursor_position = cursor.position().unwrap();
                        shell.capture_event();
                        if let Some(on_menu_open) = &self.on_menu_open {
                            shell.publish(on_menu_open.clone());
                        }
                        shell.request_redraw();
                    } else {
                        state.show_context_menu = false;
                        if let Some(on_menu_close) = &self.on_menu_close {
                            shell.publish(on_menu_close.clone());
                        }
                    }
                }
                _ => {
                    // until the context menu is hidden we capture all events.
                    if state.show_context_menu {
                        shell.capture_event();
                    }
                }
            },
            iced::Event::Window(iced::window::Event::Resized(_)) => {
                shell.request_redraw();
            }
            _ => {}
        };
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        layout::positioned(
            limits,
            self.width,
            self.height,
            self.padding,
            |limits| {
                self.content
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits)
            },
            |content, size| {
                content.align(
                    self.horizontal_alignment.into(),
                    self.vertical_alignment.into(),
                    size,
                )
            },
        )
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let status = if state.is_selected {
            Status::Selected
        } else {
            Status::Default
        };

        let appearance = if let Some(style_fn) = &self.style_fn {
            style_fn(&theme, &status)
        } else {
            theme.get_style(&self.class, &status)
        };

        let border = if let Some(border_fn) = &self.border_fn {
            border_fn(&theme, &status)
        } else {
            Border::default()
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border: border,
                shadow: appearance.shadow,
                snap: true,
            },
            appearance.background_color,
        );
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let card_state = state.state.downcast_mut::<State>();

        if card_state.show_context_menu
            && !self.menu_state.options.is_empty()
            && self.on_menu_select.is_some()
        {
            let on_menu_select = self.on_menu_select.as_ref().unwrap();
            let menu = menu::Menu::new(
                &mut card_state.menu_state,
                self.menu_state.options,
                &mut self.menu_state.selected,
                |option| on_menu_select(self.id.clone(), option),
                self.on_menu_hover.as_deref(),
                &self.menu_class,
            )
            .width(150.0)
            .padding(10.0);

            return Some(menu.overlay(
                Point::new(
                    card_state.cursor_position.x + translation.x,
                    card_state.cursor_position.y + translation.y,
                ),
                *viewport,
                0.0,
            ));
        }

        // If no context menu is shown, check if the content has an overlay
        self.content.as_widget_mut().overlay(
            &mut state.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: layout::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}

impl<'a, T, Message, Theme, Renderer> From<Container<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: Catalog + menu::Catalog + 'a,
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + 'a,
    T: ToString + Clone,
{
    fn from(value: Container<'a, T, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Selected,
    Default,
}

pub struct Style {
    pub background_color: Color,
    pub shadow: Shadow,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background_color: Color::TRANSPARENT,
            shadow: Shadow::default(),
        }
    }
}

pub trait Catalog {
    type Class<'a>;
    fn default<'a>() -> Self::Class<'a>;
    fn get_style(&self, class: &Self::Class<'_>, status: &Status) -> Style;
}

pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, &Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default_style)
    }

    fn get_style(&self, class: &Self::Class<'_>, status: &Status) -> Style {
        class(self, status)
    }
}

pub fn default_style(theme: &Theme, status: &Status) -> Style {
    let palette = theme.extended_palette();
    match status {
        Status::Selected => Style {
            background_color: palette.background.base.color,
            shadow: Shadow::default(),
        },
        Status::Default => Style {
            background_color: palette.background.base.color,
            shadow: Shadow::default(),
        },
    }
}
