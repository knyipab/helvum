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

use adw::{glib, prelude::*, subclass::prelude::*};
use pipewire::spa::param::format::MediaType;

use super::Port;

mod imp {
    use super::*;

    use std::cell::Cell;

    use once_cell::sync::Lazy;

    pub struct Link {
        pub output_port: glib::WeakRef<Port>,
        pub input_port: glib::WeakRef<Port>,
        pub active: Cell<bool>,
        pub media_type: Cell<MediaType>,
    }

    impl Default for Link {
        fn default() -> Self {
            Self {
                output_port: glib::WeakRef::default(),
                input_port: glib::WeakRef::default(),
                active: Cell::default(),
                media_type: Cell::new(MediaType::Unknown),
            }
        }
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
                    glib::ParamSpecUInt::builder("media-type")
                        .default_value(MediaType::Unknown.as_raw())
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
                "media-type" => self.media_type.get().as_raw().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "output-port" => self.output_port.set(value.get().unwrap()),
                "input-port" => self.input_port.set(value.get().unwrap()),
                "active" => self.active.set(value.get().unwrap()),
                "media-type" => self
                    .media_type
                    .set(MediaType::from_raw(value.get().unwrap())),
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

    pub fn media_type(&self) -> MediaType {
        MediaType::from_raw(self.property("media-type"))
    }

    pub fn set_media_type(&self, media_type: MediaType) {
        self.set_property("media-type", media_type.as_raw())
    }
}

impl Default for Link {
    fn default() -> Self {
        Self::new()
    }
}
