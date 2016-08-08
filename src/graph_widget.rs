use conrod::{
    Widget, CommonBuilder, UpdateArgs, color, IndexSlot,
    Line, Oval, Polygon,
    LineStyle,
    Colorable, Positionable,
    event, input,
    Point, Scalar
};

use std::io::{self, Write};
use conrod;


struct Graph {
    vertices: Vec<Box<Vertex>>
}

struct Vertex {
    to: Vec<(*mut Vertex, IndexSlot, IndexSlot)>,
    from: Vec<*mut Vertex>,

    label: String,
    position: [f64; 2],
    fill_idx: IndexSlot,
    outline_idx: IndexSlot
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
        - vertex_radius: Scalar { 25.0 }
        - vertex_outline_color: color::Color { color::rgb(0.2, 0.2, 0.2) }
        - vertex_fill_color: color::Color { color::rgb(0.99, 0.99, 1.0) }

        - edge_color: color::Color { color::rgb(0.2, 0.2, 0.2) }
        - arrow_base: Scalar { 15.0 }
        - arrow_height: Scalar { 10.0 }
    }
}

pub struct GraphWidget {
    common: CommonBuilder,
    style: Style
}

impl GraphWidget {
    pub fn new() -> Self {
        GraphWidget {
            common: CommonBuilder::new(),
            style: Style::new()
        }
    }
}

fn dist(a: Point, b: Point) -> Scalar {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];

    (dx*dx + dy*dy).sqrt()
}


fn draw_arrow(start: Point, end: Point, ui: &mut conrod::UiCell, style: &Style,
              parent_idx: conrod::WidgetIndex, line_idx: &IndexSlot, tip_idx: &IndexSlot,
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
    Line::abs(start, new_to)
        .color(edge_color)
        .thickness(2.0)
        .graphics_for(parent_idx)
        .parent(parent_idx)
        .set(line_idx.get(ui), ui);

    Polygon::fill(triangle)
        .color(edge_color)
        .graphics_for(parent_idx)
        .parent(parent_idx)
        .set(tip_idx.get(ui), ui);
}

impl Widget for GraphWidget {
    type State = State;
    type Style = Style;

    fn common(&self) -> &CommonBuilder {
        &self.common
    }

    fn common_mut(&mut self) -> &mut CommonBuilder {
        &mut self.common
    }

    fn init_state(&self) -> Self::State {
        let mut v0 = Box::new(Vertex {
            to: vec![],
            from: vec![],
            label: "Hello world!".to_string(),
            position: [0.0, 0.0],
            fill_idx: IndexSlot::new(),
            outline_idx: IndexSlot::new()
        });

        let v1 = Box::new(Vertex {
            to: vec![(&mut *v0, IndexSlot::new(), IndexSlot::new())],
            from: vec![],
            label: "Holy smokes!".to_string(),
            position: [200.0, 200.0],
            fill_idx: IndexSlot::new(),
            outline_idx: IndexSlot::new()
        });

        State {
            graph: Graph { vertices: vec![v0, v1] },
            mode: Mode::Idle,
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: UpdateArgs<Self>) {
        let UpdateArgs { idx, state, style, rect, mut ui, ..} = args;

        let radius = style.vertex_radius(&ui.theme);

        let in_widget_space = |xy: Point| {
            [rect.x() + xy[0], rect.y() + xy[1]]
        }; 

        let vertex_at_point = |state: &State, xy: Point| {
            state.graph.vertices.iter().position(|v| dist(v.position, xy) < radius)
        };

        for widget_event in ui.widget_input(idx).events() {
            use conrod::input::state::mouse;
            use conrod::input::keyboard;

            match widget_event {
                event::Widget::Press(event::Press {
                    button: event::Button::Mouse(mouse::Button::Left, xy),
                    modifiers: modifiers
                }) => {
                    let clicked_vertex = vertex_at_point(&state, in_widget_space(xy)).clone();

                    match (state.mode.clone(), modifiers, clicked_vertex) {
                        // start creating edge
                        (_, keyboard::SHIFT, Some(index)) =>
                            state.update(|state|
                                state.mode = Mode::CreatingEdge(index, IndexSlot::new(),
                                                                IndexSlot::new(), in_widget_space(xy))),

                        // create node
                        (_, keyboard::SHIFT, None) =>
                            state.update(|state|
                                state.graph.vertices.push(Box::new(Vertex {
                                    to: vec![],
                                    from: vec![],
                                    label: "new node".to_string(),
                                    position: in_widget_space(xy),
                                    fill_idx: IndexSlot::new(),
                                    outline_idx: IndexSlot::new()}))),
                            
                        // start moving vertex
                        (Mode::Idle, _, Some(index)) |
                        (Mode::MovingVertex(_,_), _, Some(index)) =>
                            state.update(|state|
                                state.mode = Mode::MovingVertex(index, state.graph.vertices[index].position)),

                        _ => ()
                    }
                },

                event::Widget::Drag(drag) if drag.button == input::MouseButton::Left => {
                    match state.mode.clone() {
                        Mode::Idle => (),

                        Mode::MovingVertex(index, vpos) =>
                            state.update(|state| {
                                (*state.graph.vertices[index]).position[0] = vpos[0] + drag.total_delta_xy[0];
                                (*state.graph.vertices[index]).position[1] = vpos[1] + drag.total_delta_xy[1];
                            }),

                        Mode::CreatingEdge(index, line_slot, arrow_slot, target) => {
                            state.update(|state| {
                                state.mode = Mode::CreatingEdge(index, line_slot, arrow_slot, in_widget_space(drag.to));
                            });
                        }
                    }
                },

                event::Widget::Release(release) => {
                    if let event::Button::Mouse(input::MouseButton::Left, _) = release.button {
                        if let Mode::CreatingEdge(index, line_slot, arrow_slot, _) = state.mode.clone() {
                        }
                        state.update(|state| state.mode = Mode::Idle);
                    }
                },

                _ => {}
            }
        }

        if let Mode::CreatingEdge(index, line_slot, arrow_slot, target) = state.mode.clone() {
            let start = (*state.graph.vertices[index]).position;
            draw_arrow(start, target, &mut ui, style, idx, &line_slot, &arrow_slot, 0.0);
        }

        let vertex_outline_color = style.vertex_outline_color(&ui.theme);
        let vertex_fill_color = style.vertex_fill_color(&ui.theme);
        for v in state.graph.vertices.iter() {
            // draw outgoing edges
            for &(ref v2, ref line_idx, ref tip_idx) in v.to.iter() {
                draw_arrow(v.position, unsafe { (**v2).position }, &mut ui,
                           style, idx, line_idx, tip_idx, radius);
            }

            // draw the vertex
            Oval::fill([radius*2.0, radius*2.0])
                .xy(v.position)
                .color(vertex_fill_color)
                .graphics_for(idx)
                .parent(idx)
                .set(v.fill_idx.get(&mut ui), &mut ui);

            let linestyle = LineStyle::new().thickness(2.0);
            Oval::outline_styled([radius*2.0, radius*2.0], linestyle)
                .xy(v.position)
                .color(vertex_outline_color)
                .graphics_for(idx)
                .parent(idx)
                .set(v.outline_idx.get(&mut ui), &mut ui);
        }
    }
}
