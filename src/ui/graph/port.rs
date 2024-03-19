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

use adw::{
    gdk,
    glib::{self, subclass::Signal},
    gtk::{self, graphene},
    prelude::*,
    subclass::prelude::*,
};
use pipewire::spa::utils::Direction;

use super::PortHandle;

mod imp {
    use super::*;

    use std::cell::Cell;

    use once_cell::{sync::Lazy, unsync::OnceCell};
    use pipewire::spa::{param::format::MediaType, utils::Direction};

    /// Graphical representation of a pipewire port.
    #[derive(gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::Port)]
    #[template(file = "port.ui")]
    pub struct Port {
        #[property(get, set, construct_only)]
        pub(super) pipewire_id: OnceCell<u32>,
        #[property(
            type = u32,
            get = |_| self.media_type.get().as_raw(),
            set = Self::set_media_type
        )]
        pub(super) media_type: Cell<MediaType>,
        #[property(
            type = u32,
            get = |_| self.direction.get().as_raw(),
            set = Self::set_direction,
            construct_only
        )]
        pub(super) direction: Cell<Direction>,
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
        pub(super) handle: TemplateChild<PortHandle>,
    }

    impl Default for Port {
        fn default() -> Self {
            Self {
                pipewire_id: OnceCell::default(),
                media_type: Cell::new(MediaType::Unknown),
                direction: Cell::new(Direction::Output),
                label: TemplateChild::default(),
                handle: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Port {
        const NAME: &'static str = "HelvumPort";
        type Type = super::Port;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("port");

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Port {
        fn constructed(&self) {
            self.parent_constructed();

            // Force left-to-right direction for the ports grid to avoid messed up UI when defaulting to right-to-left
            self.obj().set_direction(gtk::TextDirection::Ltr);

            // Display a grab cursor when the mouse is over the port so the user knows it can be dragged to another port.
            self.obj()
                .set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

            self.setup_port_drag_and_drop();
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

    impl WidgetImpl for Port {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => {
                    let (min_handle_width, nat_handle_width, _, _) =
                        self.handle.measure(orientation, for_size);
                    let (min_label_width, nat_label_width, _, _) = self
                        .label
                        .measure(orientation, i32::max(for_size - (nat_handle_width / 2), -1));

                    (
                        (min_handle_width / 2) + min_label_width,
                        (nat_handle_width / 2) + nat_label_width,
                        -1,
                        -1,
                    )
                }
                gtk::Orientation::Vertical => {
                    let (min_label_height, nat_label_height, _, _) =
                        self.label.measure(orientation, for_size);
                    let (min_handle_height, nat_handle_height, _, _) =
                        self.handle.measure(orientation, for_size);

                    (
                        i32::max(min_label_height, min_handle_height),
                        i32::max(nat_label_height, nat_handle_height),
                        -1,
                        -1,
                    )
                }
                _ => unimplemented!(),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let (_, nat_handle_height, _, _) =
                self.handle.measure(gtk::Orientation::Vertical, height);
            let (_, nat_handle_width, _, _) =
                self.handle.measure(gtk::Orientation::Horizontal, width);

            match Direction::from_raw(self.obj().direction()) {
                Direction::Input => {
                    let alloc = gtk::Allocation::new(
                        -nat_handle_width / 2,
                        (height - nat_handle_height) / 2,
                        nat_handle_width,
                        nat_handle_height,
                    );
                    self.handle.size_allocate(&alloc, -1);

                    let alloc = gtk::Allocation::new(
                        nat_handle_width / 2,
                        0,
                        width - (nat_handle_width / 2),
                        height,
                    );
                    self.label.size_allocate(&alloc, -1);
                }
                Direction::Output => {
                    let alloc = gtk::Allocation::new(
                        width - (nat_handle_width / 2),
                        (height - nat_handle_height) / 2,
                        nat_handle_width,
                        nat_handle_height,
                    );
                    self.handle.size_allocate(&alloc, -1);

                    let alloc = gtk::Allocation::new(0, 0, width - (nat_handle_width / 2), height);
                    self.label.size_allocate(&alloc, -1);
                }
                _ => unreachable!(),
            }
        }
    }

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

                let (output_port, input_port) = match Direction::from_raw(port.direction()) {
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

        fn set_media_type(&self, media_type: u32) {
            let media_type = MediaType::from_raw(media_type);

            self.media_type.set(media_type);

            for css_class in ["video", "audio", "midi"] {
                self.handle.remove_css_class(css_class)
            }

            // Color the port according to its media type.
            match media_type {
                MediaType::Video => self.handle.add_css_class("video"),
                MediaType::Audio => self.handle.add_css_class("audio"),
                MediaType::Application | MediaType::Stream => self.handle.add_css_class("midi"),
                _ => {}
            }
        }

        fn set_direction(&self, direction: u32) {
            let direction = Direction::from_raw(direction);

            self.direction.set(direction);

            match direction {
                Direction::Input => {
                    self.obj().set_halign(gtk::Align::Start);
                    self.label.set_halign(gtk::Align::Start);
                }
                Direction::Output => {
                    self.obj().set_halign(gtk::Align::End);
                    self.label.set_halign(gtk::Align::End);
                }
                _ => unreachable!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Widget;
}

impl Port {
    pub fn new(id: u32, name: &str, direction: Direction) -> Self {
        glib::Object::builder()
            .property("pipewire-id", id)
            .property("direction", direction.as_raw())
            .property("name", name)
            .build()
    }

    pub fn link_anchor(&self) -> graphene::Point {
        let style_context = self.style_context();
        let padding_right: f32 = style_context.padding().right().into();
        let border_right: f32 = style_context.border().right().into();
        let padding_left: f32 = style_context.padding().left().into();
        let border_left: f32 = style_context.border().left().into();

        let direction = Direction::from_raw(self.direction());
        graphene::Point::new(
            match direction {
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
