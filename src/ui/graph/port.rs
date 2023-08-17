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
    glib::{self, subclass::Signal},
    graphene,
    prelude::*,
    subclass::prelude::*,
};
use pipewire::spa::Direction;

use crate::MediaType;

mod imp {
    use super::*;

    use once_cell::{sync::Lazy, unsync::OnceCell};
    use pipewire::spa::Direction;

    /// Graphical representation of a pipewire port.
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Port)]
    pub struct Port {
        #[property(get, set, construct_only)]
        pub(super) pipewire_id: OnceCell<u32>,
        #[property(
            name = "name", type = String,
            get = |this: &Self| this.label.text().to_string(),
            set = |this: &Self, val| {
                this.label.set_text(val);
                this.label.set_tooltip_text(Some(val));
            }
        )]
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

    #[glib::derived_properties]
    impl ObjectImpl for Port {
        fn constructed(&self) {
            self.parent_constructed();

            self.label.set_parent(&*self.obj());
            self.label.set_wrap(true);
            self.label.set_lines(2);
            self.label.set_max_width_chars(20);
            self.label.set_ellipsize(gtk::pango::EllipsizeMode::End);

            self.setup_port_drag_and_drop();
        }

        fn dispose(&self) {
            self.label.unparent()
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

    impl Port {
        fn setup_port_drag_and_drop(&self) {
            let obj = &*self.obj();

            // Add a drag source and drop target controller with the type depending on direction,
            // they will be responsible for link creation by dragging an output port onto an input port or the other way around.
            // The port will simply provide its pipewire id to the drag target.
            // The drop target will accept the source port and use it to emit its `port-toggled` signal.

            // FIXME: We should protect against different media types, e.g. it should not be possible to drop a video port on an audio port.

            let drag_src = gtk::DragSource::builder()
                .content(&gdk::ContentProvider::for_value(&obj.to_value()))
                .build();
            // Override the default drag icon with an empty one so that only a grab cursor is shown.
            // The graph will render a link from the source port to the cursor to visualize the drag instead.
            drag_src.set_icon(Some(&gdk::Paintable::new_empty(0, 0)), 0, 0);
            drag_src.connect_drag_begin(|drag_source, _| {
                let port = drag_source
                    .widget()
                    .dynamic_cast::<super::Port>()
                    .expect("Widget should be a Port");

                log::trace!("Drag started from port {}", port.pipewire_id());
            });
            drag_src.connect_drag_cancel(|drag_source, _, _| {
                let port = drag_source
                    .widget()
                    .dynamic_cast::<super::Port>()
                    .expect("Widget should be a Port");

                log::trace!("Drag from port {} was cancelled", port.pipewire_id());

                false
            });
            obj.add_controller(drag_src);

            let drop_target =
                gtk::DropTarget::new(super::Port::static_type(), gdk::DragAction::COPY);
            drop_target.set_preload(true);
            drop_target.connect_value_notify(|drop_target| {
                let port = drop_target
                    .widget()
                    .dynamic_cast::<super::Port>()
                    .expect("Widget should be a Port");

                let Some(value) = drop_target.value() else {
                    return;
                };

                let other_port: super::Port = value.get().expect("Drop value should be a port");

                // Disallow drags between two ports that have the same direction
                if !port.is_linkable_to(&other_port) {
                    // FIXME: For some reason, this prints error:
                    //        "gdk_drop_get_actions: assertion 'GDK_IS_DROP (self)' failed"
                    drop_target.reject();
                }
            });
            drop_target.connect_drop(|drop_target, val, _, _| {
                let port = drop_target
                    .widget()
                    .dynamic_cast::<super::Port>()
                    .expect("Widget should be a Port");
                let other_port = val
                    .get::<super::Port>()
                    .expect("Dropped value should be a Port");

                // Do not accept a drop between imcompatible ports
                if !port.is_linkable_to(&other_port) {
                    log::warn!("Tried to link incompatible ports");
                    return false;
                }

                let (output_port, input_port) = match port.direction() {
                    Direction::Output => (&port, &other_port),
                    Direction::Input => (&other_port, &port),
                    _ => unreachable!(),
                };

                port.emit_by_name::<()>(
                    "port-toggled",
                    &[&output_port.pipewire_id(), &input_port.pipewire_id()],
                );

                true
            });
            obj.add_controller(drop_target);
        }
    }
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Widget;
}

impl Port {
    pub fn new(id: u32, name: &str, direction: Direction, media_type: Option<MediaType>) -> Self {
        // Create the widget and initialize needed fields
        let res: Self = glib::Object::builder()
            .property("pipewire-id", id)
            .property("name", name)
            .build();

        let imp = res.imp();

        imp.direction
            .set(direction)
            .expect("Port direction already set");

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

    pub fn direction(&self) -> Direction {
        *self
            .imp()
            .direction
            .get()
            .expect("Port direction is not set")
    }

    pub fn link_anchor(&self) -> graphene::Point {
        let style_context = self.style_context();
        let padding_right: f32 = style_context.padding().right().into();
        let border_right: f32 = style_context.border().right().into();
        let padding_left: f32 = style_context.padding().left().into();
        let border_left: f32 = style_context.border().left().into();

        graphene::Point::new(
            match self.direction() {
                Direction::Output => self.width() as f32 + padding_right + border_right,
                Direction::Input => 0.0 - padding_left - border_left,
                _ => unreachable!(),
            },
            self.height() as f32 / 2.0,
        )
    }

    pub fn is_linkable_to(&self, other_port: &Self) -> bool {
        self.direction() != other_port.direction()
    }
}
