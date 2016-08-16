#[macro_use] extern crate conrod;
extern crate piston_window;
extern crate find_folder;

use conrod::widget::{Canvas, Widget};
use conrod::color;
use piston_window::{EventLoop, OpenGL, PistonWindow, UpdateEvent, WindowSettings};

mod graph_widget;
use graph_widget::GraphWidget;

fn main() {
    const WIDTH: u32 = 1080;
    const HEIGHT: u32 = 720;

    let opengl = OpenGL::V3_2;

    let mut window: PistonWindow = WindowSettings::new("Text demo", [WIDTH, HEIGHT])
                                   .opengl(opengl).exit_on_esc(true).build().unwrap();
    window.set_ups(60);

    let mut ui = conrod::UiBuilder::new().build();

    let assets = find_folder::Search::KidsThenParents(5, 5).for_folder("assets").unwrap();
    let font_path = assets.join("Hack-Regular.ttf");
    ui.fonts.insert_from_file(font_path).unwrap();

    let mut text_texture_cache =
        conrod::backend::piston_window::GlyphCache::new(&mut window, WIDTH, HEIGHT);

    let image_map = conrod::image::Map::new();

    while let Some(event) = window.next()  {
        if let Some(e) = conrod::backend::piston_window::convert_event(event.clone(), &window) {
            ui.handle_event(e);
        }

        event.update(|_| set_ui(&mut ui.set_widgets()));

        window.draw_2d(&event, |c, g| {
            if let Some(primitives) = ui.draw_if_changed() {
                fn texture_from_image<T>(img: &T) -> &T { img };
                conrod::backend::piston_window::draw(c, g, primitives,
                                                     &mut text_texture_cache,
                                                     &image_map,
                                                     texture_from_image);
            }
        });
    }
}



fn set_ui(ui: &mut conrod::UiCell) {
    use conrod::{Colorable, Positionable};

    Canvas::new()
        .color(color::rgb(0.97, 0.97, 0.97))
        .set(MASTER, ui);

    GraphWidget::new()
        .middle_of(MASTER)
        .set(GRAPH, ui);
}

widget_ids! {
    MASTER,
    GRAPH
}
