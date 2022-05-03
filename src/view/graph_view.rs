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
    graphene, gsk,
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

use std::{cmp::Ordering, collections::HashMap};

use crate::NodeType;

mod imp {
    use super::*;

    use std::{cell::RefCell, rc::Rc};

    use log::warn;

    #[derive(Default)]
    pub struct GraphView {
        pub(super) nodes: RefCell<HashMap<u32, Node>>,
        /// Stores the link and whether it is currently active.
        pub(super) links: RefCell<HashMap<u32, (crate::PipewireLink, bool)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphView {
        const NAME: &'static str = "GraphView";
        type Type = super::GraphView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.set_layout_manager_type::<gtk::FixedLayout>();
            klass.set_css_name("graphview");
        }
    }

    impl ObjectImpl for GraphView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let drag_state = Rc::new(RefCell::new(None));
            let drag_controller = gtk::GestureDrag::new();

            drag_controller.connect_drag_begin(
                clone!(@strong drag_state => move |drag_controller, x, y| {
                    let mut drag_state = drag_state.borrow_mut();
                    let widget = drag_controller
                        .widget()
                        .dynamic_cast::<Self::Type>()
                        .expect("drag-begin event is not on the GraphView");
                    // pick() should at least return the widget itself.
                    let target = widget.pick(x, y, gtk::PickFlags::DEFAULT).expect("drag-begin pick() did not return a widget");
                    *drag_state = if target.ancestor(Port::static_type()).is_some() {
                        // The user targeted a port, so the dragging should be handled by the Port
                        // component instead of here.
                        None
                    } else if let Some(target) = target.ancestor(Node::static_type()) {
                        // The user targeted a Node without targeting a specific Port.
                        // Drag the Node around the screen.
                        let (x, y) = widget.get_node_position(&target);
                        Some((target, x, y))
                    } else {
                        None
                    }
                }
            ));
            drag_controller.connect_drag_update(
                clone!(@strong drag_state => move |drag_controller, x, y| {
                    let widget = drag_controller
                        .widget()
                        .dynamic_cast::<Self::Type>()
                        .expect("drag-update event is not on the GraphView");
                    let drag_state = drag_state.borrow();
                    if let Some((ref node, x1, y1)) = *drag_state {
                        widget.move_node(node, x1 + x as f32, y1 + y as f32);
                    }
                }
                ),
            );
            obj.add_controller(&drag_controller);
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.nodes
                .borrow()
                .values()
                .for_each(|node| node.unparent())
        }
    }

    impl WidgetImpl for GraphView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            /* FIXME: A lot of hardcoded values in here.
            Try to use relative units (em) and colours from the theme as much as possible. */

            let alloc = widget.allocation();

            // Draw all children
            self.nodes
                .borrow()
                .values()
                .for_each(|node| self.instance().snapshot_child(node, snapshot));

            // Draw all links
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
                .unwrap_or_else(|| gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

            link_cr.set_source_rgba(
                rgba.red().into(),
                rgba.green().into(),
                rgba.blue().into(),
                rgba.alpha().into(),
            );

            for (link, active) in self.links.borrow().values() {
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
    }

    impl GraphView {
        /// Get coordinates for the drawn link to start at and to end at.
        ///
        /// # Returns
        /// `Some((from_x, from_y, to_x, to_y))` if all objects the links refers to exist as widgets.
        fn get_link_coordinates(&self, link: &crate::PipewireLink) -> Option<(f64, f64, f64, f64)> {
            let nodes = self.nodes.borrow();

            // For some reason, gtk4::WidgetExt::translate_coordinates gives me incorrect values,
            // so we manually calculate the needed offsets here.

            let from_port = &nodes.get(&link.node_from)?.get_port(link.port_from)?;
            let from_node = from_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let from_x = from_node.allocation().x()
                + from_port.allocation().x()
                + from_port.allocation().width();
            let from_y = from_node.allocation().y()
                + from_port.allocation().y()
                + (from_port.allocation().height() / 2);

            let to_port = &nodes.get(&link.node_to)?.get_port(link.port_to)?;
            let to_node = to_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let to_x = to_node.allocation().x() + to_port.allocation().x();
            let to_y = to_node.allocation().y()
                + to_port.allocation().y()
                + (to_port.allocation().height() / 2);

            Some((from_x.into(), from_y.into(), to_x.into(), to_y.into()))
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
                self.get_node_position(&node.clone().upcast())
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

        self.move_node(&node.clone().upcast(), x, y);

        private.nodes.borrow_mut().insert(id, node);
    }

    pub fn remove_node(&self, id: u32) {
        let private = imp::GraphView::from_instance(self);
        let mut nodes = private.nodes.borrow_mut();
        if let Some(node) = nodes.remove(&id) {
            node.unparent();
        } else {
            warn!("Tried to remove non-existant node (id={}) from graph", id);
        }
    }

    pub fn add_port(&self, node_id: u32, port_id: u32, port: crate::view::port::Port) {
        let private = imp::GraphView::from_instance(self);

        if let Some(node) = private.nodes.borrow_mut().get_mut(&node_id) {
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
        if let Some(node) = nodes.get(&node_id) {
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
    pub(super) fn get_node_position(&self, node: &gtk::Widget) -> (f32, f32) {
        let layout_manager = self
            .layout_manager()
            .expect("Failed to get layout manager")
            .dynamic_cast::<gtk::FixedLayout>()
            .expect("Failed to cast to FixedLayout");

        let node = layout_manager
            .layout_child(node)
            .dynamic_cast::<gtk::FixedLayoutChild>()
            .expect("Could not cast to FixedLayoutChild");
        node.transform()
            .expect("Failed to obtain transform from layout child")
            .to_translate()
    }

    pub(super) fn move_node(&self, node: &gtk::Widget, x: f32, y: f32) {
        let layout_manager = self
            .layout_manager()
            .expect("Failed to get layout manager")
            .dynamic_cast::<gtk::FixedLayout>()
            .expect("Failed to cast to FixedLayout");

        let transform = gsk::Transform::new()
            // Nodes should not be able to be dragged out of the view, so we use `max(coordinate, 0.0)` to prevent that.
            .translate(&graphene::Point::new(f32::max(x, 0.0), f32::max(y, 0.0)))
            .unwrap();

        layout_manager
            .layout_child(node)
            .dynamic_cast::<gtk::FixedLayoutChild>()
            .expect("Could not cast to FixedLayoutChild")
            .set_transform(&transform);

        // FIXME: If links become proper widgets,
        // we don't need to redraw the full graph everytime.
        self.queue_draw();
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}
