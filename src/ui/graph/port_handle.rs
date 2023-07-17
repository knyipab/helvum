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

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct PortHandle {}

    #[glib::object_subclass]
    impl ObjectSubclass for PortHandle {
        const NAME: &'static str = "HelvumPortHandle";
        type Type = super::PortHandle;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("port-handle");
        }
    }

    impl ObjectImpl for PortHandle {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            obj.set_halign(gtk::Align::Center);
            obj.set_valign(gtk::Align::Center);
        }
    }

    impl WidgetImpl for PortHandle {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::ConstantSize
        }

        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (Self::HANDLE_SIZE, Self::HANDLE_SIZE, -1, -1)
        }
    }

    impl PortHandle {
        pub const HANDLE_SIZE: i32 = 14;
    }
}

glib::wrapper! {
    pub struct PortHandle(ObjectSubclass<imp::PortHandle>)
        @extends gtk::Widget;
}

impl PortHandle {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn get_link_anchor(&self) -> gtk::graphene::Point {
        gtk::graphene::Point::new(
            imp::PortHandle::HANDLE_SIZE as f32 / 2.0,
            imp::PortHandle::HANDLE_SIZE as f32 / 2.0,
        )
    }
}

impl Default for PortHandle {
    fn default() -> Self {
        Self::new()
    }
}
