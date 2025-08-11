use crate::app_state::AudioPlayable;

pub static CONTAINER_SVG: &[u8] = include_bytes!("../images/placeholder.svg");
pub fn duration_to_str(duration: u64) -> String {
    format!("{}:{:02}", duration / 60, duration % 60)
}

pub fn playable_artwork<'a, Message, T: AudioPlayable + ?Sized>(
    playable: &'a T,
    height: u32,
    width: u32,
) -> iced::Element<'a, Message>
where
    Message: 'a + Clone,
{
    if let Some(bytes) = playable.get_album_art() {
        let handle = iced::widget::image::Handle::from_bytes(bytes.clone());
        iced::widget::Image::new(handle)
            .width(width)
            .height(height)
            .into()
    } else {
        let album_thumbnail_handle = iced::advanced::svg::Handle::from_memory(CONTAINER_SVG);
        iced::widget::svg(album_thumbnail_handle)
            .width(width)
            .height(height)
            .into()
    }
}
