// Copyright 2021 Tom A. Wagner <tom.a.wagner@protonmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as published by
// the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use super::{Node, Port};

use gtk::{
    glib::{self, clone},
    graphene::{self, Point},
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

use std::{cmp::Ordering, collections::HashMap};

use crate::NodeType;

const CANVAS_SIZE: f64 = 5000.0;

mod imp {
    use super::*;

    use std::cell::RefCell;

    use gtk::{gdk::RGBA, graphene::Rect, gsk::ColorStop};
    use log::warn;
    use once_cell::sync::Lazy;

    #[derive(Default)]
    pub struct GraphView {
        /// Stores nodes and their positions.
        pub(super) nodes: RefCell<HashMap<u32, (Node, Point)>>,
        /// Stores the link and whether it is currently active.
        pub(super) links: RefCell<HashMap<u32, (crate::PipewireLink, bool)>>,
        pub hadjustment: RefCell<Option<gtk::Adjustment>>,
        pub vadjustment: RefCell<Option<gtk::Adjustment>>,
        /// When a node drag is ongoing, this stores the dragged node and the initial coordinates on the widget surface.
        pub drag_state: RefCell<Option<(Node, Point)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphView {
        const NAME: &'static str = "GraphView";
        type Type = super::GraphView;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Scrollable,);

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("graphview");
        }
    }

    impl ObjectImpl for GraphView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_overflow(gtk::Overflow::Hidden);

            let drag_controller = gtk::GestureDrag::new();

            drag_controller.connect_drag_begin(|drag_controller, x, y| {
                let widget = drag_controller
                    .widget()
                    .dynamic_cast::<Self::Type>()
                    .expect("drag-begin event is not on the GraphView");
                let mut drag_state = widget.imp().drag_state.borrow_mut();

                // pick() should at least return the widget itself.
                let target = widget
                    .pick(x, y, gtk::PickFlags::DEFAULT)
                    .expect("drag-begin pick() did not return a widget");
                *drag_state = if target.ancestor(Port::static_type()).is_some() {
                    // The user targeted a port, so the dragging should be handled by the Port
                    // component instead of here.
                    None
                } else if let Some(target) = target.ancestor(Node::static_type()) {
                    // The user targeted a Node without targeting a specific Port.
                    // Drag the Node around the screen.
                    let node = target.dynamic_cast_ref::<Node>().unwrap();

                    // We use the upper-left corner of the widget as the start position instead of the actual
                    // cursor location, this lets us move the node around easier because we don't need to
                    // account for where the cursor is on the node.
                    let alloc = node.allocation();
                    Some((node.clone(), Point::new(alloc.x() as f32, alloc.y() as f32)))
                } else {
                    None
                }
            });
            drag_controller.connect_drag_update(|drag_controller, x, y| {
                let widget = drag_controller
                    .widget()
                    .dynamic_cast::<Self::Type>()
                    .expect("drag-update event is not on the GraphView");
                let drag_state = widget.imp().drag_state.borrow();
                let hadj = widget.imp().hadjustment.borrow();
                let vadj = widget.imp().vadjustment.borrow();

                if let Some((ref node, ref start_point)) = *drag_state {
                    widget.move_node(
                        node,
                        start_point.x() + hadj.as_ref().unwrap().value() as f32 + x as f32,
                        start_point.y() + vadj.as_ref().unwrap().value() as f32 + y as f32,
                    );
                }
            });
            obj.add_controller(&drag_controller);
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.nodes
                .borrow()
                .values()
                .for_each(|(node, _)| node.unparent())
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("hadjustment"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("vadjustment"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("hscroll-policy"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("vscroll-policy"),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "hadjustment" => self.hadjustment.borrow().to_value(),
                "vadjustment" => self.vadjustment.borrow().to_value(),
                "hscroll-policy" | "vscroll-policy" => gtk::ScrollablePolicy::Natural.to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "hadjustment" => {
                    self.set_adjustment(obj, value.get().ok(), gtk::Orientation::Horizontal)
                }
                "vadjustment" => {
                    self.set_adjustment(obj, value.get().ok(), gtk::Orientation::Vertical)
                }
                "hscroll-policy" | "vscroll-policy" => {}
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for GraphView {
        fn size_allocate(&self, widget: &Self::Type, _width: i32, _height: i32, _baseline: i32) {
            for (node, point) in self.nodes.borrow().values() {
                let (_, natural_size) = node.preferred_size();
                node.size_allocate(
                    &gtk::Allocation::new(
                        (f64::from(point.x()) - self.hadjustment.borrow().as_ref().unwrap().value())
                            as i32,
                        (f64::from(point.y()) - self.vadjustment.borrow().as_ref().unwrap().value())
                            as i32,
                        natural_size.width(),
                        natural_size.height(),
                    ),
                    -1,
                )
            }

            if let Some(ref hadjustment) = *self.hadjustment.borrow() {
                self.set_adjustment_values(widget, hadjustment, gtk::Orientation::Horizontal);
            }
            if let Some(ref vadjustment) = *self.vadjustment.borrow() {
                self.set_adjustment_values(widget, vadjustment, gtk::Orientation::Vertical);
            }
        }

        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            let alloc = widget.allocation();

            self.snapshot_background(widget, snapshot);

            // Draw all visible children
            self.nodes
                .borrow()
                .values()
                // Cull nodes from rendering when they are outside the visible canvas area
                .filter(|(node, _)| alloc.intersect(&node.allocation()).is_some())
                .for_each(|(node, _)| self.instance().snapshot_child(node, snapshot));

            self.snapshot_links(widget, snapshot);
        }
    }

    impl ScrollableImpl for GraphView {}

    impl GraphView {
        fn snapshot_background(&self, widget: &super::GraphView, snapshot: &gtk::Snapshot) {
            const GRID_SIZE: f32 = 20.0;
            const GRID_LINE_WIDTH: f32 = 1.0;

            let alloc = widget.allocation();

            // We need to offset the lines between 0 and (excluding) GRID_SIZE so the grid moves with
            // the rest of the view when scrolling.
            // The offset is rounded so the grid is always aligned to a row of pixels.
            let hadj = self
                .hadjustment
                .borrow()
                .as_ref()
                .map(|hadjustment| hadjustment.value())
                .unwrap_or(0.0);
            let hoffset = ((GRID_SIZE - (hadj as f32 % GRID_SIZE)) % GRID_SIZE).floor();
            let vadj = self
                .vadjustment
                .borrow()
                .as_ref()
                .map(|vadjustment| vadjustment.value())
                .unwrap_or(0.0);
            let voffset = ((GRID_SIZE - (vadj as f32 % GRID_SIZE)) % GRID_SIZE).floor();

            snapshot.push_repeat(
                &Rect::new(0.0, 0.0, alloc.width() as f32, alloc.height() as f32),
                Some(&Rect::new(0.0, voffset, alloc.width() as f32, GRID_SIZE)),
            );
            let grid_color = RGBA::new(0.137, 0.137, 0.137, 1.0);
            snapshot.append_linear_gradient(
                &Rect::new(0.0, voffset, alloc.width() as f32, GRID_LINE_WIDTH),
                &Point::new(0.0, 0.0),
                &Point::new(alloc.width() as f32, 0.0),
                &[
                    ColorStop::new(0.0, grid_color),
                    ColorStop::new(1.0, grid_color),
                ],
            );
            snapshot.pop();

            snapshot.push_repeat(
                &Rect::new(0.0, 0.0, alloc.width() as f32, alloc.height() as f32),
                Some(&Rect::new(hoffset, 0.0, GRID_SIZE, alloc.height() as f32)),
            );
            snapshot.append_linear_gradient(
                &Rect::new(hoffset, 0.0, GRID_LINE_WIDTH, alloc.height() as f32),
                &Point::new(0.0, 0.0),
                &Point::new(0.0, alloc.height() as f32),
                &[
                    ColorStop::new(0.0, grid_color),
                    ColorStop::new(1.0, grid_color),
                ],
            );
            snapshot.pop();
        }

        fn snapshot_links(&self, widget: &super::GraphView, snapshot: &gtk::Snapshot) {
            let alloc = widget.allocation();

            let link_cr = snapshot.append_cairo(&graphene::Rect::new(
                0.0,
                0.0,
                alloc.width() as f32,
                alloc.height() as f32,
            ));

            link_cr.set_line_width(2.0);

            let rgba = widget
                .style_context()
                .lookup_color("graphview-link")
                .unwrap_or(gtk::gdk::RGBA::BLACK);

            link_cr.set_source_rgba(
                rgba.red().into(),
                rgba.green().into(),
                rgba.blue().into(),
                rgba.alpha().into(),
            );

            for (link, active) in self.links.borrow().values() {
                // TODO: Do not draw links when they are outside the view
                if let Some((from_x, from_y, to_x, to_y)) = self.get_link_coordinates(link) {
                    link_cr.move_to(from_x, from_y);

                    // Use dashed line for inactive links, full line otherwise.
                    if *active {
                        link_cr.set_dash(&[], 0.0);
                    } else {
                        link_cr.set_dash(&[10.0, 5.0], 0.0);
                    }

                    // If the output port is farther right than the input port and they have
                    // a similar y coordinate, apply a y offset to the control points
                    // so that the curve sticks out a bit.
                    let y_control_offset = if from_x > to_x {
                        f64::max(0.0, 25.0 - (from_y - to_y).abs())
                    } else {
                        0.0
                    };

                    // Place curve control offset by half the x distance between the two points.
                    // This makes the curve scale well for varying distances between the two ports,
                    // especially when the output port is farther right than the input port.
                    let half_x_dist = f64::abs(from_x - to_x) / 2.0;
                    link_cr.curve_to(
                        from_x + half_x_dist,
                        from_y - y_control_offset,
                        to_x - half_x_dist,
                        to_y - y_control_offset,
                        to_x,
                        to_y,
                    );

                    if let Err(e) = link_cr.stroke() {
                        warn!("Failed to draw graphview links: {}", e);
                    };
                } else {
                    warn!("Could not get allocation of ports of link: {:?}", link);
                }
            }
        }

        /// Get coordinates for the drawn link to start at and to end at.
        ///
        /// # Returns
        /// `Some((from_x, from_y, to_x, to_y))` if all objects the links refers to exist as widgets.
        fn get_link_coordinates(&self, link: &crate::PipewireLink) -> Option<(f64, f64, f64, f64)> {
            let nodes = self.nodes.borrow();

            // For some reason, gtk4::WidgetExt::translate_coordinates gives me incorrect values,
            // so we manually calculate the needed offsets here.

            let from_port = &nodes.get(&link.node_from)?.0.get_port(link.port_from)?;
            let from_node = from_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let from_x = from_node.allocation().x()
                + from_port.allocation().x()
                + from_port.allocation().width();
            let from_y = from_node.allocation().y()
                + from_port.allocation().y()
                + (from_port.allocation().height() / 2);

            let to_port = &nodes.get(&link.node_to)?.0.get_port(link.port_to)?;
            let to_node = to_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let to_x = to_node.allocation().x() + to_port.allocation().x();
            let to_y = to_node.allocation().y()
                + to_port.allocation().y()
                + (to_port.allocation().height() / 2);

            Some((from_x.into(), from_y.into(), to_x.into(), to_y.into()))
        }

        fn set_adjustment(
            &self,
            obj: &super::GraphView,
            adjustment: Option<&gtk::Adjustment>,
            orientation: gtk::Orientation,
        ) {
            match orientation {
                gtk::Orientation::Horizontal => {
                    *self.hadjustment.borrow_mut() = adjustment.cloned()
                }
                gtk::Orientation::Vertical => *self.vadjustment.borrow_mut() = adjustment.cloned(),
                _ => unimplemented!(),
            }

            if let Some(adjustment) = adjustment {
                adjustment
                    .connect_value_changed(clone!(@weak obj => move |_|  obj.queue_allocate() ));
            }
        }

        fn set_adjustment_values(
            &self,
            obj: &super::GraphView,
            adjustment: &gtk::Adjustment,
            orientation: gtk::Orientation,
        ) {
            let size = match orientation {
                gtk::Orientation::Horizontal => obj.width(),
                gtk::Orientation::Vertical => obj.height(),
                _ => unimplemented!(),
            };

            adjustment.configure(
                adjustment.value(),
                -(CANVAS_SIZE / 2.0),
                CANVAS_SIZE / 2.0,
                f64::from(size) * 0.1,
                f64::from(size) * 0.9,
                f64::from(size),
            );
        }
    }
}

glib::wrapper! {
    pub struct GraphView(ObjectSubclass<imp::GraphView>)
        @extends gtk::Widget;
}

impl GraphView {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create GraphView")
    }

    pub fn add_node(&self, id: u32, node: Node, node_type: Option<NodeType>) {
        let private = imp::GraphView::from_instance(self);
        node.set_parent(self);

        // Place widgets in colums of 3, growing down
        let x = if let Some(node_type) = node_type {
            match node_type {
                NodeType::Output => 20.0,
                NodeType::Input => 820.0,
            }
        } else {
            420.0
        };

        let y = private
            .nodes
            .borrow()
            .values()
            .map(|node| {
                // Map nodes to their locations
                self.get_node_position(&node.0.clone().upcast()).unwrap()
            })
            .filter(|(x2, _)| {
                // Only look for other nodes that have a similar x coordinate
                (x - x2).abs() < 50.0
            })
            .max_by(|y1, y2| {
                // Get max in column
                y1.partial_cmp(y2).unwrap_or(Ordering::Equal)
            })
            .map_or(20_f32, |(_x, y)| y + 100.0);

        private
            .nodes
            .borrow_mut()
            .insert(id, (node, Point::new(x, y)));
    }

    pub fn remove_node(&self, id: u32) {
        let private = imp::GraphView::from_instance(self);
        let mut nodes = private.nodes.borrow_mut();
        if let Some((node, _)) = nodes.remove(&id) {
            node.unparent();
        } else {
            warn!("Tried to remove non-existant node (id={}) from graph", id);
        }
    }

    pub fn add_port(&self, node_id: u32, port_id: u32, port: crate::view::port::Port) {
        let private = imp::GraphView::from_instance(self);

        if let Some((node, _)) = private.nodes.borrow_mut().get_mut(&node_id) {
            node.add_port(port_id, port);
        } else {
            error!(
                "Node with id {} not found when trying to add port with id {} to graph",
                node_id, port_id
            );
        }
    }

    pub fn remove_port(&self, id: u32, node_id: u32) {
        let private = imp::GraphView::from_instance(self);
        let nodes = private.nodes.borrow();
        if let Some((node, _)) = nodes.get(&node_id) {
            node.remove_port(id);
        }
    }

    pub fn add_link(&self, link_id: u32, link: crate::PipewireLink, active: bool) {
        let private = imp::GraphView::from_instance(self);
        private.links.borrow_mut().insert(link_id, (link, active));
        self.queue_draw();
    }

    pub fn set_link_state(&self, link_id: u32, active: bool) {
        let private = imp::GraphView::from_instance(self);
        if let Some((_, state)) = private.links.borrow_mut().get_mut(&link_id) {
            *state = active;
            self.queue_draw();
        } else {
            warn!("Link state changed on unknown link (id={})", link_id);
        }
    }

    pub fn remove_link(&self, id: u32) {
        let private = imp::GraphView::from_instance(self);
        let mut links = private.links.borrow_mut();
        links.remove(&id);

        self.queue_draw();
    }

    /// Get the position of the specified node inside the graphview.
    pub(super) fn get_node_position(&self, node: &Node) -> Option<(f32, f32)> {
        self.imp()
            .nodes
            .borrow()
            .get(&node.pipewire_id())
            .map(|(_, point)| (point.x(), point.y()))
    }

    pub(super) fn move_node(&self, widget: &Node, x: f32, y: f32) {
        let mut nodes = self.imp().nodes.borrow_mut();
        let mut node = nodes
            .get_mut(&widget.pipewire_id())
            .expect("Node is not on the graph");

        // Clamp the new position to within the graph, so a node can't be moved outside it and be lost.
        node.1 = Point::new(
            x.clamp(
                -(CANVAS_SIZE / 2.0) as f32,
                (CANVAS_SIZE / 2.0) as f32 - widget.width() as f32,
            ),
            y.clamp(
                -(CANVAS_SIZE / 2.0) as f32,
                (CANVAS_SIZE / 2.0) as f32 - widget.height() as f32,
            ),
        );

        self.queue_allocate();
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}
