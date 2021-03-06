use conrod::{color, event, input, Point, Scalar};

use conrod::color::Colorable;
use conrod::{Positionable, Sizeable};

use conrod::widget;
use conrod::widget::Widget;
use conrod::widget::primitive;
use conrod::widget::IndexSlot;

use conrod;
use std;


struct Graph {
    vertices: Vec<Box<Vertex>>
}

struct Vertex {
    outs: Vec<(*mut Vertex, IndexSlot, IndexSlot)>,
    ins: Vec<*mut Vertex>,

    label: String,
    position: [f64; 2],
    fill_idx: IndexSlot,
    outline_idx: IndexSlot,
    text_idx: IndexSlot
}


#[derive(Clone)]
enum Mode {
    // Moving a vertex from an initial location.
    MovingVertex(usize, Point),

    // Creating an edge starting at the given vertex.
    // The IndexSlots are used for the visual line and arrow.
    CreatingEdge(usize, IndexSlot, IndexSlot, Point),

    Idle
}

pub struct State {
    graph: Graph,
    mode: Mode,
}

widget_style!{
    style Style {
        - vertex_radius: Scalar { 35.0 }
        - vertex_outline_color: color::Color { color::rgb(0.2, 0.2, 0.2) }
        - vertex_fill_color: color::Color { color::rgb(0.99, 0.99, 1.0) }

        - edge_color: color::Color { color::rgb(0.2, 0.2, 0.2) }
        - arrow_base: Scalar { 15.0 }
        - arrow_height: Scalar { 10.0 }
    }
}

pub struct GraphWidget {
    common: widget::CommonBuilder,
    style: Style
}

impl GraphWidget {
    pub fn new() -> Self {
        GraphWidget {
            common: widget::CommonBuilder::new(),
            style: Style::new()
        }
    }
}

fn dist(a: Point, b: Point) -> Scalar {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];

    (dx*dx + dy*dy).sqrt()
}

// Note: The vertices are listed in the order that they are stored in the graph.
fn graph_to_string(g: &Graph) -> String {
    fn join<'a, I>(mut input: I, separator: &str) -> String
    where I: Iterator<Item=&'a str> {
        let first = input.next().unwrap_or(&"").to_string();
        input.fold(first, |acc, s| acc + separator + s)
    }

    let dec_lines: Vec<_> = g.vertices.iter().map(|v| v.label.clone() + ";").collect();
    let declarations: String = join(dec_lines.iter().map(|d| d.as_str()), "\n");
    let con_lines: Vec<_> = g.vertices.iter()
        .flat_map(|source| {
            source.outs.iter().map(move |target| {
                let left = (**source).label.clone();
                let right = unsafe { (*target.0).label.as_str() };
                left + " => " + right + ";"
            })
        }).collect();
    let connections = join(con_lines.iter().map(|c| c.as_str()), "\n");
    
    declarations + "\n\n" + &connections
}


fn draw_arrow(start: Point, end: Point, ui: &mut conrod::UiCell, style: &Style,
              parent_idx: widget::Index, line_idx: &IndexSlot, tip_idx: &IndexSlot,
              subtract: Scalar) {
    let arrow_height = style.arrow_height(&ui.theme);

    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let dist = (dx*dx + dy*dy).sqrt();
    let norm_dx = dx / dist;
    let norm_dy = dy / dist;
    let new_dist = dist - subtract - arrow_height;
    let new_dx = norm_dx * new_dist;
    let new_dy = norm_dy * new_dist;

    let new_to = [start[0] + new_dx, start[1] + new_dy];
    
    // arrow at the end of the line.
    //         _      _
    //        /0\     |
    //       /   \    | h
    //      /1   2\   |
    //      -------   -
    //         b
    //
    // We can use the normalized vector from before to build up
    // the triangle. Keep in mind how 90 degree rotations work
    // in 2D space:
    //
    // rotate left: (x, y) => (-y, x)
    // rotate right: (x, y) => (y, -x)
    // rotate 180: (x, y) => (-y, -x)

    let b = style.arrow_base(&ui.theme);
    let h = style.arrow_height(&ui.theme);

    let h_vector = [norm_dx * h, norm_dy * h];
    let b_vector = [norm_dx * b / 2.0, norm_dy * b / 2.0];

    let triangle = vec![
        [new_to[0] + h_vector[0], new_to[1] + h_vector[1]],
        [new_to[0] - b_vector[1], new_to[1] + b_vector[0]],
        [new_to[0] + b_vector[1], new_to[1] - b_vector[0]]
    ];
    
    let edge_color = style.edge_color(&ui.theme);
    primitive::line::Line::abs(start, new_to)
        .color(edge_color)
        .thickness(2.0)
        .graphics_for(parent_idx)
        .parent(parent_idx)
        .set(line_idx.get(ui), ui);

    primitive::shape::polygon::Polygon::fill(triangle)
        .color(edge_color)
        .graphics_for(parent_idx)
        .parent(parent_idx)
        .set(tip_idx.get(ui), ui);
}

impl Widget for GraphWidget {
    type State = State;
    type Style = Style;
    type Event = String;

    fn common(&self) -> &widget::CommonBuilder {
        &self.common
    }

    fn common_mut(&mut self) -> &mut widget::CommonBuilder {
        &mut self.common
    }

    fn init_state(&self) -> Self::State {
        let mut v0 = Box::new(Vertex {
            outs: vec![],
            ins: vec![],
            label: "Hello world!".to_string(),
            position: [-200.0, -100.0],
            fill_idx: IndexSlot::new(),
            outline_idx: IndexSlot::new(),
            text_idx: IndexSlot::new()
        });

        let mut v1 = Box::new(Vertex {
            outs: vec![(&mut *v0, IndexSlot::new(), IndexSlot::new())],
            ins: vec![],
            label: "Holy smokes!".to_string(),
            position: [-200.0, 100.0],
            fill_idx: IndexSlot::new(),
            outline_idx: IndexSlot::new(),
            text_idx: IndexSlot::new()
        });
        v0.ins.push(&mut *v1);

        State {
            graph: Graph { vertices: vec![v0, v1] },
            mode: Mode::Idle,
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { idx, state, style, rect, mut ui, ..} = args;

        let radius = style.vertex_radius(&ui.theme);

        let in_widget_space = |xy: Point| {
            [rect.x() + xy[0], rect.y() + xy[1]]
        }; 

        let vertex_at_point = |state: &State, xy: Point| {
            state.graph.vertices.iter().position(|v| dist(v.position, xy) < radius)
        };

        // Clamp a point within this widget's rectangle.
        // The coordinates are in widget space.
        let clamp = |xy: Point, padding: Scalar| {
            let x = rect.x.pad(padding).clamp_value(xy[0]);
            let y = rect.y.pad(padding).clamp_value(xy[1]);
            [x, y]
        };

        /// If there is an edge at the given point, this returns its source vertex's
        /// index, along with the destination vertex's index in the source's `outs` vec.
        fn edge_at_point(state: &State, p: Point) -> Option<(usize, usize)> {
            let width = 6.0; // make the clickable width of the edge bigger than the draw width
            let halfwidth = width/2.0;

            fn dot(r: Point, s: Point) -> Scalar {
                r[0]*s[0] + r[1]*s[1]
            }

            for (vindex, source) in state.graph.vertices.iter().enumerate() {
                let pos = source.outs.iter().position(|&(ref targetptr, _, _)| {
                    let u = source.position;
                    let v = unsafe { (**targetptr).position };

                    let uv = [v[0]-u[0], v[1]-u[1]];
                    let mag_uv = (uv[0]*uv[0] + uv[1]*uv[1]).sqrt();
                    let norm_uv = [uv[0]/mag_uv, uv[1]/mag_uv];

                    // Let `a` be a corner of the rectangle with neighbors
                    // `b` and `c`. Let `p` be the point we're testing.

                    // Remember (x, y) => (-y, x) is a 90 degree counterclockwise rotation.
                    let a = [u[0] - halfwidth * norm_uv[1], u[1] + halfwidth * norm_uv[0]];
                    let ab = uv;
                    // Remember (x, y) => (y, -x) is a 90 degree clockwise rotation.
                    let ac = [width * norm_uv[1], - width * norm_uv[0]];
                    let ap = [p[0]-a[0], p[1]-a[1]];

                    // Notice that ab and ac are perpendicular sides of the rectangle.

                    // To be inside the rectangle we need the scalar projection of
                    // ap onto both ab and ac to be positive and not too large.
                    // So that's what's checked below, it's just written slightly differently
                    // to turn divisions into multiplications.

                    let tmp1 = dot(ap, ab);
                    let tmp2 = dot(ap, ac);
                    return (0_f64 <= tmp1 && tmp1 <= dot(ab, ab)) &&
                           (0_f64 <= tmp2 && tmp2 <= dot(ac, ac));
                });

                if let Some(eindex) = pos {
                    return Some((vindex, eindex));
                }
            }
            return None;
        }

        for widget_event in ui.widget_input(idx).events() {
            use conrod::input::state::mouse;
            use conrod::input::keyboard;

            match widget_event {

                event::Widget::Press(event::Press {
                    button: event::Button::Mouse(mouse::Button::Left, xy),
                    modifiers
                }) => {
                    let clicked_vertex = vertex_at_point(&state, in_widget_space(xy));

                    match (&state.mode, modifiers, clicked_vertex) {
                        // start creating edge
                        (&Mode::Idle, keyboard::SHIFT, Some(index)) =>
                            state.update(|state|
                                state.mode = Mode::CreatingEdge(index, IndexSlot::new(),
                                                                IndexSlot::new(), in_widget_space(xy))),

                        // create node
                        (&Mode::Idle, keyboard::SHIFT, None) =>
                            state.update(|state|
                                state.graph.vertices.push(Box::new(Vertex {
                                    outs: vec![],
                                    ins: vec![],
                                    label: "new node".to_string(),
                                    position: clamp(in_widget_space(xy), radius),
                                    fill_idx: IndexSlot::new(),
                                    outline_idx: IndexSlot::new(),
                                    text_idx: IndexSlot::new()}))),
                            
                        // start moving vertex
                        (&Mode::Idle, _, Some(index)) |
                        (&Mode::MovingVertex(_,_), _, Some(index)) =>
                            state.update(|state|
                                state.mode = Mode::MovingVertex(index, state.graph.vertices[index].position)),

                        _ => ()
                    }
                },

                event::Widget::Drag(drag) if drag.button == input::MouseButton::Left => {
                    match &state.mode {
                        &Mode::Idle => (),

                        // move vertex
                        &Mode::MovingVertex(index, vpos) =>
                            state.update(|state| {
                                let new_vpos = [vpos[0] + drag.total_delta_xy[0],
                                                vpos[1] + drag.total_delta_xy[1]];
                                let clamped = clamp(new_vpos, radius);
                                (*state.graph.vertices[index]).position = clamped;
                            }),

                        // update edge preview
                        &Mode::CreatingEdge(_, _, _, _) => {
                            state.update(|state| {
                                if let Mode::CreatingEdge(_, _, _, ref mut position) = state.mode {
                                    *position = clamp(in_widget_space(drag.to), 0.0);
                                }
                            });
                        }
                    }
                },

                event::Widget::Release(release) => {
                    // finish creating edge
                    if let event::Button::Mouse(input::MouseButton::Left, xy) = release.button {
                        match &state.mode {
                            &Mode::CreatingEdge(src_idx, _, _, _) => {
                                if let Some(target_idx) = vertex_at_point(&state, in_widget_space(xy)) {

                                    state.update(|state| {
                                        let src_ptr: *mut Vertex = &mut *state.graph.vertices[src_idx];
                                        let target_ptr: *mut Vertex = &mut *state.graph.vertices[target_idx];

                                        // don't create redundant edges
                                        if (*state.graph.vertices[src_idx])
                                           .outs.iter().all(|&(p,_,_)| p != target_ptr) {

                                            // steal the index slots from the preview
                                            let m = std::mem::replace(&mut state.mode, Mode::Idle);
                                            let (line_slot, arrow_slot) = match m {
                                                Mode::CreatingEdge(_, line_slot, arrow_slot, _) => (line_slot, arrow_slot),
                                                _ => unreachable!()
                                            };

                                            state.graph.vertices[src_idx].outs.push((target_ptr, line_slot, arrow_slot));
                                            state.graph.vertices[target_idx].ins.push(src_ptr);
                                        }

                                        state.mode = Mode::Idle;
                                    });
                                } else {
                                    // TODO: free index slots?
                                    state.update(|state| state.mode = Mode::Idle);
                                }
                            },

                            &Mode::MovingVertex(_,_) => {
                                state.update(|state| state.mode = Mode::Idle);
                            },

                            &Mode::Idle => ()
                        }
                    }
                },

                event::Widget::Press(event::Press {
                    button: event::Button::Mouse(mouse::Button::Right, xy),
                    ..
                }) => {
                    if let Mode::Idle = state.mode {
                        // TODO: free index slots?

                        // remove vertex
                        if let Some(vindex) = vertex_at_point(&state, in_widget_space(xy)) {
                            let p: *const Vertex = &*state.graph.vertices[vindex];
                            state.update(|state| {
                                for &(other, _, _) in state.graph.vertices[vindex].outs.iter() {
                                    unsafe { (*other).ins.retain(|&q| p != q) };
                                }
                                for &other in state.graph.vertices[vindex].ins.iter() {
                                    unsafe { (*other).outs.retain(|&(q,_,_)| p != q) };
                                }

                                state.graph.vertices.remove(vindex);
                            });
                        // remove edge
                        } else if let Some((vindex, eindex)) = edge_at_point(&state, in_widget_space(xy)) {
                            state.update(|state| {
                                let src_ptr: *const Vertex = &mut *state.graph.vertices[vindex];

                                for &other in state.graph.vertices[vindex].ins.iter() {
                                    unsafe {
                                        (*other).ins.retain(|&ptr| src_ptr != ptr);
                                    }
                                }
                                (*state.graph.vertices[vindex]).outs.remove(eindex);
                            });
                        }
                    }
                }

                _ => {}
            }
        }

        if let &Mode::CreatingEdge(index, ref line_slot, ref arrow_slot, target) = &state.mode {
            let start = (*state.graph.vertices[index]).position;
            draw_arrow(start, target, &mut ui, style, idx, &line_slot, &arrow_slot, 0.0);
        }

        let vertex_outline_color = style.vertex_outline_color(&ui.theme);
        let vertex_fill_color = style.vertex_fill_color(&ui.theme);
        
        state.update(|state| { // need mutation for the TextBox
            for v in state.graph.vertices.iter_mut() {

                // draw outgoing edges
                for &(ref v2, ref line_idx, ref tip_idx) in v.outs.iter() {
                    draw_arrow(v.position, unsafe { (**v2).position }, &mut ui,
                               style, idx, line_idx, tip_idx, radius);
                }

                // draw the vertex
                primitive::shape::oval::Oval::fill([radius*2.0, radius*2.0])
                    .xy(v.position)
                    .color(vertex_fill_color)
                    .graphics_for(idx)
                    .parent(idx)
                    .set(v.fill_idx.get(&mut ui), &mut ui);

                let linestyle = primitive::line::Style::new().thickness(2.0);
                primitive::shape::oval::Oval::outline_styled([radius*2.0, radius*2.0], linestyle)
                    .xy(v.position)
                    .color(vertex_outline_color)
                    .graphics_for(idx)
                    .parent(idx)
                    .set(v.outline_idx.get(&mut ui), &mut ui);

                let position = v.position.clone();
                let i = v.text_idx.get(&mut ui);
                let font_size = 12_u32;
                let char_width = (font_size as f64) * 0.692;
                let box_width = 30.0 + char_width * (v.label.len() as f64);
                for event in widget::text_box::TextBox::new(&mut v.label)
                    .xy(position)
                    .wh([box_width, 25_f64])
                    .font_size(font_size)
                    .align_text_middle()
                    .parent(idx)
                    .set(i, &mut ui)
                {
                    match event {
                        widget::text_box::Event::Update(string) => v.label = string,
                        _ => ()
                    }
                }
            }

            state.graph.vertices.sort_by(|a, b| a.label.cmp(&b.label));
        });

        graph_to_string(&state.graph)
    }
}
