//! Icon loading: embedded SVGs -> ARGB32 pixmaps for the StatusNotifierItem.

use ksni::Icon;
use resvg::{tiny_skia, usvg};
use std::sync::LazyLock;

const ICON_LIGHT_SVG: &str = include_str!("../assets/icon-light.svg");
const ICON_DARK_SVG: &str = include_str!("../assets/icon-dark.svg");
const ICON_ACTIVE_SVG: &str = include_str!("../assets/icon-active.svg");

/// Rendered tray icons, built once on first use.
pub struct Icons {
    pub light: Icon,
    pub dark: Icon,
    pub active: Icon,
}

pub static ICONS: LazyLock<Icons> = LazyLock::new(|| Icons {
    light: render_svg(ICON_LIGHT_SVG).expect("embedded light icon renders"),
    dark: render_svg(ICON_DARK_SVG).expect("embedded dark icon renders"),
    active: render_svg(ICON_ACTIVE_SVG).expect("embedded active icon renders"),
});

/// Render an SVG source into a 64x64 ARGB32 pixmap (what SNI expects).
fn render_svg(svg: &str) -> Option<Icon> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();

    let mut pixmap = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    // tiny-skia data is RGBA; SNI pixmaps are ARGB32 in network byte order,
    // i.e. the byte layout per pixel must become A, R, G, B.
    let mut data = pixmap.take();
    for px in data.as_chunks_mut::<4>().0 {
        px.rotate_right(1);
    }

    Some(Icon {
        width: size.width() as i32,
        height: size.height() as i32,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_embedded_icons() {
        let icons = &*ICONS;
        for icon in [&icons.light, &icons.dark, &icons.active] {
            assert_eq!(icon.width, 64);
            assert_eq!(icon.height, 64);
            assert_eq!(icon.data.len(), (64 * 64 * 4) as usize);
        }
    }
}
