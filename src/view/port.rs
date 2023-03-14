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

use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use log::{trace, warn};
use pipewire::spa::Direction;

use crate::MediaType;

/// A helper struct for linking a output port to an input port.
/// It carries the output ports id.
#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "HelvumForwardLink")]
struct ForwardLink(u32);

/// A helper struct for linking an input to an output port.
/// It carries the input ports id.
#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "HelvumReversedLink")]
struct ReversedLink(u32);

mod imp {
    use glib::ParamFlags;
    use once_cell::{sync::Lazy, unsync::OnceCell};
    use pipewire::spa::Direction;

    use super::*;

    /// Graphical representation of a pipewire port.
    #[derive(Default)]
    pub struct Port {
        pub(super) pipewire_id: OnceCell<u32>,
        pub(super) label: gtk::Label,
        pub(super) direction: OnceCell<Direction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Port {
        const NAME: &'static str = "HelvumPort";
        type Type = super::Port;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();

            // Make it look like a GTK button.
            klass.set_css_name("button");
        }
    }

    impl ObjectImpl for Port {
        fn constructed(&self) {
            self.parent_constructed();

            self.label.set_parent(&*self.obj());
            self.label.set_wrap(true);
            self.label.set_lines(2);
            self.label.set_max_width_chars(20);
            self.label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        }

        fn dispose(&self) {
            self.label.unparent()
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecUInt::builder("pipewire-id")
                        .flags(ParamFlags::READWRITE | ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecString::builder("name").build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "pipewire-id" => self.pipewire_id.get().unwrap().to_value(),
                "name" => self.label.text().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => {
                    self.label.set_text(value.get().unwrap());
                    self.label.set_tooltip_text(value.get().ok());
                }
                "pipewire-id" => self.pipewire_id.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("port-toggled")
                    // Provide id of output port and input port to signal handler.
                    .param_types([<u32>::static_type(), <u32>::static_type()])
                    .build()]
            });

            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for Port {}
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Widget;
}

impl Port {
    pub fn new(id: u32, name: &str, direction: Direction, media_type: Option<MediaType>) -> Self {
        // Create the widget and initialize needed fields
        let res: Self = glib::Object::builder()
            .property("pipewire-id", &id)
            .property("name", &name)
            .build();

        let imp = res.imp();

        imp.direction
            .set(direction)
            .expect("Port direction already set");

        // Add a drag source and drop target controller with the type depending on direction,
        // they will be responsible for link creation by dragging an output port onto an input port or the other way around.

        // FIXME: We should protect against different media types, e.g. it should not be possible to drop a video port on an audio port.

        // The port will simply provide its pipewire id to the drag target.
        let drag_src = gtk::DragSource::builder()
            .content(&gdk::ContentProvider::for_value(&match direction {
                Direction::Input => ReversedLink(id).to_value(),
                Direction::Output => ForwardLink(id).to_value(),
            }))
            .build();
        drag_src.connect_drag_begin(clone!(@weak res as obj => move |source, _| {
            trace!("Drag started from port {}", id);
            let paintable = gtk::WidgetPaintable::new(Some(&obj));
            source.set_icon(Some(&paintable), 0, 0);
        }));
        drag_src.connect_drag_cancel(move |_, _, _| {
            trace!("Drag from port {} was cancelled", id);
            false
        });
        res.add_controller(drag_src);

        // The drop target will accept either a `ForwardLink` or `ReversedLink` depending in its own direction,
        // and use it to emit its `port-toggled` signal.
        let drop_target = gtk::DropTarget::new(
            match direction {
                Direction::Input => ForwardLink::static_type(),
                Direction::Output => ReversedLink::static_type(),
            },
            gdk::DragAction::COPY,
        );
        match direction {
            Direction::Input => {
                drop_target.connect_drop(
                    clone!(@weak res as this => @default-panic, move |drop_target, val, _, _| {
                        if let Ok(ForwardLink(source_id)) = val.get::<ForwardLink>() {
                            // Get the callback registered in the widget and call it
                            drop_target
                                .widget()
                                .emit_by_name::<()>("port-toggled", &[&source_id, &this.pipewire_id()]);
                        } else {
                            warn!("Invalid type dropped on ingoing port");
                        }

                        true
                    }),
                );
            }
            Direction::Output => {
                drop_target.connect_drop(
                    clone!(@weak res as this => @default-panic, move |drop_target, val, _, _| {
                        if let Ok(ReversedLink(target_id)) = val.get::<ReversedLink>() {
                            // Get the callback registered in the widget and call it
                            drop_target
                                .widget()
                                .emit_by_name::<()>("port-toggled", &[&this.pipewire_id(), &target_id]);
                        } else {
                            warn!("Invalid type dropped on outgoing port");
                        }

                        true
                    }),
                );
            }
        }
        res.add_controller(drop_target);

        // Display a grab cursor when the mouse is over the port so the user knows it can be dragged to another port.
        res.set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

        // Color the port according to its media type.
        match media_type {
            Some(MediaType::Video) => res.add_css_class("video"),
            Some(MediaType::Audio) => res.add_css_class("audio"),
            Some(MediaType::Midi) => res.add_css_class("midi"),
            None => {}
        }

        res
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

    pub fn direction(&self) -> &Direction {
        self.imp()
            .direction
            .get()
            .expect("Port direction is not set")
    }
}
