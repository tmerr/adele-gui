#[macro_use] extern crate conrod;
extern crate piston_window;
extern crate find_folder;

use conrod::widget::primitive::shape::rectangle::Rectangle;
use conrod::widget::primitive::text::Text;
use conrod::widget::{Canvas, Widget, TextEdit};
use conrod::Sizeable;
use conrod::color;
use piston_window::{EventLoop, OpenGL, PistonWindow, UpdateEvent, WindowSettings};

mod graph_widget;
use graph_widget::GraphWidget;

const WIDTH: u32 = 1080;
const HEIGHT: u32 = 720;

fn main() {

    let opengl = OpenGL::V3_2;

    let mut window: PistonWindow = WindowSettings::new("Text demo", [WIDTH, HEIGHT])
                                   .opengl(opengl)
                                   .exit_on_esc(false)
                                   .build().unwrap();
    window.set_ups(60);

    let mut ui = conrod::UiBuilder::new().build();

    let assets = find_folder::Search::KidsThenParents(5, 5).for_folder("assets").unwrap();
    let font_path = assets.join("Hack-Regular.ttf");
    ui.fonts.insert_from_file(font_path).unwrap();

    let mut text_texture_cache =
        conrod::backend::piston_window::GlyphCache::new(&mut window, WIDTH, HEIGHT);

    let image_map = conrod::image::Map::new();

    let mut typetext = "Type text goes here".to_string();

    while let Some(event) = window.next()  {
        if let Some(e) = conrod::backend::piston_window::convert_event(event.clone(), &window) {
            ui.handle_event(e);
        }

        event.update(|_| set_ui(&mut ui.set_widgets(), &mut typetext));

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



fn set_ui(ui: &mut conrod::UiCell, typetext: &mut String) {
    use conrod::{Colorable, Positionable};

    let hdivide = 0.7_f64;
    let left_width = hdivide * (WIDTH as f64);
    let right_width = (1.0 - hdivide) * (WIDTH as f64);

    let vdivide = 0.6_f64;
    let vgap = 6.0;
    let top_height = vdivide * (HEIGHT as f64) - 3.0*vgap/2.0;
    let bottom_height = (1.0 - vdivide) * (HEIGHT as f64) - 3.0*vgap/2.0;

    let textmargin = 10.0;
    let fontsize = 12_u32;

    Canvas::new()
        .color(color::rgb(0.97, 0.97, 0.97))
        .set(MASTER, ui);

    GraphWidget::new()
        .mid_left_of(MASTER)
        .w_h(left_width, 720_f64)
        .set(GRAPH, ui);

    Rectangle::fill_with([right_width, top_height], color::rgb(0.9, 0.9, 0.9))
        .top_right_with_margins_on(MASTER, vgap, 0.0)
        .set(TYPE_TEXT_BG, ui);

    for edit in TextEdit::new(typetext)
        .top_right_with_margin_on(TYPE_TEXT_BG, textmargin)
        .w_h(right_width - 2.0*textmargin, top_height - 2.0*textmargin)
        .font_size(fontsize)
        .color(color::BLACK)
        .set(TYPE_TEXT, ui)
    {
        *typetext = edit;
    }

    Rectangle::fill_with([right_width, bottom_height], color::rgb(0.9, 0.9, 0.9))
        .bottom_right_with_margins_on(MASTER, vgap, 0.0)
        .set(GRAPH_TEXT_BG, ui);

    Text::new("Graph text goes here")
        .top_right_with_margin_on(GRAPH_TEXT_BG, textmargin)
        .w_h(right_width - 2.0*textmargin, bottom_height - 2.0*textmargin)
        .font_size(fontsize)
        .color(color::BLACK)
        .wrap_by_word()
        .align_text_left()
        .set(GRAPH_TEXT, ui);
}

widget_ids! {
    MASTER,
    GRAPH,
    TYPE_TEXT_BG,
    TYPE_TEXT,
    GRAPH_TEXT_BG,
    GRAPH_TEXT
}
