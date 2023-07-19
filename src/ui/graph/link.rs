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

use super::Port;

mod imp {
    use super::*;

    use std::cell::Cell;

    use once_cell::sync::Lazy;

    #[derive(Default)]
    pub struct Link {
        pub output_port: glib::WeakRef<Port>,
        pub input_port: glib::WeakRef<Port>,
        pub active: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Link {
        const NAME: &'static str = "HelvumLink";
        type Type = super::Link;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for Link {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<Port>("output-port")
                        .flags(glib::ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecObject::builder::<Port>("input-port")
                        .flags(glib::ParamFlags::READWRITE)
                        .build(),
                    glib::ParamSpecBoolean::builder("active")
                        .default_value(false)
                        .flags(glib::ParamFlags::READWRITE)
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "output-port" => self.output_port.upgrade().to_value(),
                "input-port" => self.input_port.upgrade().to_value(),
                "active" => self.active.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "output-port" => self.output_port.set(value.get().unwrap()),
                "input-port" => self.input_port.set(value.get().unwrap()),
                "active" => self.active.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Link(ObjectSubclass<imp::Link>);
}

impl Link {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn output_port(&self) -> Option<Port> {
        self.property("output-port")
    }

    pub fn set_output_port(&self, port: Option<&Port>) {
        self.set_property("output-port", port);
    }

    pub fn input_port(&self) -> Option<Port> {
        self.property("input-port")
    }

    pub fn set_input_port(&self, port: Option<&Port>) {
        self.set_property("input-port", port);
    }

    pub fn active(&self) -> bool {
        self.property("active")
    }

    pub fn set_active(&self, active: bool) {
        self.set_property("active", active);
    }
}
