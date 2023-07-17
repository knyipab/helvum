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
            name = "name", type = String,
            get = |this: &Self| this.label.text().to_string(),
            set = |this: &Self, val| {
                this.label.set_text(val);
                this.label.set_tooltip_text(Some(val));
            }
        )]
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) port_grid: TemplateChild<gtk::Grid>,
        pub(super) ports: RefCell<HashSet<Port>>,
        pub(super) num_ports_in: Cell<i32>,
        pub(super) num_ports_out: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Node {
        const NAME: &'static str = "HelvumNode";
        type Type = super::Node;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();

            klass.bind_template();
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
            self.label
                .set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());
        }

        fn dispose(&self) {
            if let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Node {}
}

glib::wrapper! {
    pub struct Node(ObjectSubclass<imp::Node>)
        @extends gtk::Widget;
}

impl Node {
    pub fn new(name: &str, pipewire_id: u32) -> Self {
        glib::Object::builder()
            .property("name", name)
            .property("pipewire-id", pipewire_id)
            .build()
    }

    pub fn add_port(&self, port: Port) {
        let imp = self.imp();

        match Direction::from_raw(port.direction()) {
            Direction::Input => {
                imp.port_grid
                    .attach(&port, 0, imp.num_ports_in.get() + 1, 1, 1);
                imp.num_ports_in.set(imp.num_ports_in.get() + 1);
            }
            Direction::Output => {
                imp.port_grid
                    .attach(&port, 1, imp.num_ports_out.get() + 1, 1, 1);
                imp.num_ports_out.set(imp.num_ports_out.get() + 1);
            }
            _ => unreachable!(),
        }

        imp.ports.borrow_mut().insert(port);
    }

    pub fn remove_port(&self, port: &Port) {
        let imp = self.imp();
        if imp.ports.borrow_mut().remove(port) {
            match Direction::from_raw(port.direction()) {
                Direction::Input => imp.num_ports_in.set(imp.num_ports_in.get() - 1),
                Direction::Output => imp.num_ports_in.set(imp.num_ports_out.get() - 1),
                _ => unreachable!(),
            }

            port.unparent();
        } else {
            log::warn!("Tried to remove non-existant port widget from node");
        }
    }
}
