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

use adw::{glib, gtk, prelude::*, subclass::prelude::*};
use pipewire::spa::Direction;

use super::Port;

mod imp {
    use super::*;

    use std::{
        cell::{Cell, RefCell},
        collections::HashSet,
    };

    #[derive(glib::Properties, gtk::CompositeTemplate, Default)]
    #[properties(wrapper_type = super::Node)]
    #[template(file = "node.ui")]
    pub struct Node {
        #[property(get, set, construct_only)]
        pub(super) pipewire_id: Cell<u32>,
        #[property(
            name = "node-name", type = String,
            get = |this: &Self| this.node_name.text().to_string(),
            set = |this: &Self, val| {
                this.node_name.set_text(val);
                this.node_name.set_tooltip_text(Some(val));
            }
        )]
        #[template_child]
        pub(super) node_name: TemplateChild<gtk::Label>,
        #[property(
            name = "media-name", type = String,
            get = |this: &Self| this.media_name.text().to_string(),
            set = |this: &Self, val| {
                this.media_name.set_text(val);
                this.media_name.set_tooltip_text(Some(val));
                this.media_name.set_visible(!val.is_empty());
            }
        )]
        #[template_child]
        pub(super) media_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) separator: TemplateChild<gtk::Separator>,
        #[template_child]
        pub(super) port_grid: TemplateChild<gtk::Grid>,
        pub(super) ports: RefCell<HashSet<Port>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Node {
        const NAME: &'static str = "HelvumNode";
        type Type = super::Node;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();

            klass.bind_template();

            klass.set_css_name("node");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Node {
        fn constructed(&self) {
            self.parent_constructed();

            // Display a grab cursor when the mouse is over the label so the user knows the node can be dragged.
            self.node_name
                .set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());
        }

        fn dispose(&self) {
            if let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Node {}

    impl Node {
        /// Update the internal ports grid to reflect the ports stored in the ports set.
        pub fn update_ports(&self) {
            // We first remove all ports from the grid, then re-add them all, so that
            // ports that have been removed do not leave gaps in the grid.

            while let Some(ref child) = self.port_grid.first_child() {
                self.port_grid.remove(child);
            }

            let ports = self.ports.borrow();

            let mut ports_out = Vec::new();
            let mut ports_in = Vec::new();

            ports
                .iter()
                .for_each(|port| match Direction::from_raw(port.direction()) {
                    Direction::Output => {
                        ports_out.push(port);
                    }
                    Direction::Input => {
                        ports_in.push(port);
                    }
                    _ => unreachable!(),
                });

            ports_out.sort_unstable_by_key(|port| port.name());
            ports_in.sort_unstable_by_key(|port| port.name());

            // In case no ports have been added to the port, hide the seperator as it is not needed
            self.separator
                .set_visible(!ports_out.is_empty() || !ports_in.is_empty());

            for (i, port) in ports_in.into_iter().enumerate() {
                self.port_grid.attach(port, 0, i.try_into().unwrap(), 1, 1);
            }

            for (i, port) in ports_out.into_iter().enumerate() {
                self.port_grid.attach(port, 1, i.try_into().unwrap(), 1, 1);
            }
        }
    }
}

glib::wrapper! {
    pub struct Node(ObjectSubclass<imp::Node>)
        @extends gtk::Widget;
}

impl Node {
    pub fn new(name: &str, pipewire_id: u32) -> Self {
        glib::Object::builder()
            .property("node-name", name)
            .property("pipewire-id", pipewire_id)
            .build()
    }

    pub fn add_port(&self, port: Port) {
        let imp = self.imp();
        imp.ports.borrow_mut().insert(port);
        imp.update_ports();
    }

    pub fn remove_port(&self, port: &Port) {
        let imp = self.imp();
        if imp.ports.borrow_mut().remove(port) {
            imp.update_ports();
        } else {
            log::warn!("Tried to remove non-existant port widget from node");
        }
    }
}
