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

mod imp {
    use glib::ParamFlags;
    use once_cell::sync::Lazy;

    use super::*;

    use std::cell::{Cell, RefCell};

    pub struct Node {
        pub(super) pipewire_id: Cell<u32>,
        pub(super) grid: gtk::Grid,
        pub(super) label: gtk::Label,
        pub(super) ports: RefCell<HashMap<u32, crate::view::port::Port>>,
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
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.grid.set_parent(obj);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt::new(
                        "pipewire-id",
                        "pipewire-id",
                        "pipewire-id",
                        u32::MIN,
                        u32::MAX,
                        0,
                        ParamFlags::READWRITE | ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new("name", "name", "name", None, ParamFlags::READWRITE),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pipewire-id" => self.pipewire_id.get().to_value(),
                "name" => self.label.text().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "name" => self.label.set_text(value.get().unwrap()),
                "pipewire-id" => self.pipewire_id.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self, _obj: &Self::Type) {
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
        glib::Object::new(&[("name", &name), ("pipewire-id", &pipewire_id)])
            .expect("Failed to create Node")
    }

    pub fn pipewire_id(&self) -> u32 {
        self.property("pipewire-id")
    }

    /// Get the nodes `name` property, which represents the displayed name.
    pub fn name(&self) -> String {
        self.property("name")
    }

    /// Set the nodes `name` property, which represents the displayed name.
    pub fn set_name(&self, name: &str) {
        self.set_property("name", name);
    }

    pub fn add_port(&mut self, id: u32, port: super::port::Port) {
        let private = imp::Node::from_instance(self);

        match port.direction() {
            Direction::Input => {
                private
                    .grid
                    .attach(&port, 0, private.num_ports_in.get() + 1, 1, 1);
                private.num_ports_in.set(private.num_ports_in.get() + 1);
            }
            Direction::Output => {
                private
                    .grid
                    .attach(&port, 1, private.num_ports_out.get() + 1, 1, 1);
                private.num_ports_out.set(private.num_ports_out.get() + 1);
            }
        }

        private.ports.borrow_mut().insert(id, port);
    }

    pub fn get_port(&self, id: u32) -> Option<super::port::Port> {
        let private = imp::Node::from_instance(self);
        private.ports.borrow_mut().get(&id).cloned()
    }

    pub fn remove_port(&self, id: u32) {
        let private = imp::Node::from_instance(self);
        if let Some(port) = private.ports.borrow_mut().remove(&id) {
            match port.direction() {
                Direction::Input => private.num_ports_in.set(private.num_ports_in.get() - 1),
                Direction::Output => private.num_ports_in.set(private.num_ports_out.get() - 1),
            }

            port.unparent();
        }
    }
}
