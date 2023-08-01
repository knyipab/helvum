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

use gtk::{glib, prelude::*, subclass::prelude::*};
use pipewire::spa::Direction;

use std::collections::HashMap;

use super::Port;

mod imp {
    use super::*;

    use std::cell::{Cell, RefCell};

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::Node)]
    pub struct Node {
        #[property(get, set, construct_only)]
        pub(super) pipewire_id: Cell<u32>,
        pub(super) grid: gtk::Grid,
        #[property(
            name = "name", type = String,
            get = |this: &Self| this.label.text().to_string(),
            set = |this: &Self, val| {
                this.label.set_text(val);
                this.label.set_tooltip_text(Some(val));
            }
        )]
        pub(super) label: gtk::Label,
        pub(super) ports: RefCell<HashMap<u32, Port>>,
        pub(super) num_ports_in: Cell<i32>,
        pub(super) num_ports_out: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Node {
        const NAME: &'static str = "HelvumNode";
        type Type = super::Node;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn new() -> Self {
            let grid = gtk::Grid::new();

            let label = gtk::Label::new(None);
            label.set_wrap(true);
            label.set_lines(2);
            label.set_max_width_chars(20);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);

            grid.attach(&label, 0, 0, 2, 1);

            // Display a grab cursor when the mouse is over the label so the user knows the node can be dragged.
            label.set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

            Self {
                pipewire_id: Cell::new(0),
                grid,
                label,
                ports: RefCell::new(HashMap::new()),
                num_ports_in: Cell::new(0),
                num_ports_out: Cell::new(0),
            }
        }
    }

    impl ObjectImpl for Node {
        fn constructed(&self) {
            self.parent_constructed();
            self.grid.set_parent(&*self.obj());
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            Self::derived_property(self, id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            Self::derived_set_property(self, id, value, pspec)
        }

        fn dispose(&self) {
            self.grid.unparent();
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
            .property("name", &name)
            .property("pipewire-id", &pipewire_id)
            .build()
    }

    pub fn add_port(&mut self, id: u32, port: Port) {
        let imp = self.imp();

        match port.direction() {
            Direction::Input => {
                imp.grid.attach(&port, 0, imp.num_ports_in.get() + 1, 1, 1);
                imp.num_ports_in.set(imp.num_ports_in.get() + 1);
            }
            Direction::Output => {
                imp.grid.attach(&port, 1, imp.num_ports_out.get() + 1, 1, 1);
                imp.num_ports_out.set(imp.num_ports_out.get() + 1);
            }
        }

        imp.ports.borrow_mut().insert(id, port);
    }

    pub fn get_port(&self, id: u32) -> Option<Port> {
        self.imp().ports.borrow_mut().get(&id).cloned()
    }

    pub fn remove_port(&self, id: u32) {
        let imp = self.imp();
        if let Some(port) = imp.ports.borrow_mut().remove(&id) {
            match port.direction() {
                Direction::Input => imp.num_ports_in.set(imp.num_ports_in.get() - 1),
                Direction::Output => imp.num_ports_in.set(imp.num_ports_out.get() - 1),
            }

            port.unparent();
        }
    }
}
