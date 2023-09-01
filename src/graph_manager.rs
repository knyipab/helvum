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

use pipewire::channel::Sender as PwSender;

use crate::{ui::graph::GraphView, GtkMessage, PipewireMessage};

mod imp {
    use super::*;

    use std::{cell::RefCell, collections::HashMap};

    use once_cell::unsync::OnceCell;

    use crate::{ui::graph, MediaType, NodeType};

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::GraphManager)]
    pub struct GraphManager {
        #[property(get, set, construct_only)]
        pub graph: OnceCell<crate::ui::graph::GraphView>,

        pub pw_sender: OnceCell<PwSender<crate::GtkMessage>>,
        pub items: RefCell<HashMap<u32, glib::Object>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphManager {
        const NAME: &'static str = "HelvumGraphManager";
        type Type = super::GraphManager;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for GraphManager {}

    impl GraphManager {
        pub fn attach_receiver(&self, receiver: glib::Receiver<crate::PipewireMessage>) {
            receiver.attach(None, glib::clone!(
                @weak self as imp => @default-return glib::ControlFlow::Continue,
                move |msg| {
                    match msg {
                        PipewireMessage::NodeAdded { id, name, node_type } => imp.add_node(id, name.as_str(), node_type),
                        PipewireMessage::PortAdded { id, node_id, name, direction } => imp.add_port(id, name.as_str(), node_id, direction),
                        PipewireMessage::PortFormatChanged { id, media_type } => imp.port_media_type_changed(id, media_type),
                        PipewireMessage::LinkAdded {
                            id, port_from, port_to, active, media_type
                        } => imp.add_link(id, port_from, port_to, active, media_type),
                        PipewireMessage::LinkStateChanged { id, active } => imp.link_state_changed(id, active),
                        PipewireMessage::LinkFormatChanged { id, media_type } => imp.link_format_changed(id, media_type),
                        PipewireMessage::NodeRemoved { id } => imp.remove_node(id),
                        PipewireMessage::PortRemoved { id, node_id } => imp.remove_port(id, node_id),
                        PipewireMessage::LinkRemoved { id } => imp.remove_link(id),
                        PipewireMessage::Disconnected => imp.clear(),
                    };
                    glib::ControlFlow::Continue
                }
            ));
        }

        /// Add a new node to the view.
        fn add_node(&self, id: u32, name: &str, node_type: Option<NodeType>) {
            log::info!("Adding node to graph: id {}", id);

            let node = graph::Node::new(name, id);

            self.items.borrow_mut().insert(id, node.clone().upcast());

            self.obj().graph().add_node(node, node_type);
        }

        /// Remove the node with the specified id from the view.
        fn remove_node(&self, id: u32) {
            log::info!("Removing node from graph: id {}", id);

            let Some(node) = self.items.borrow_mut().remove(&id) else {
                log::warn!("Unknown node (id={id}) removed from graph");
                return;
            };
            let Ok(node) = node.dynamic_cast::<graph::Node>() else {
                log::warn!("Graph Manager item under node id {id} is not a node");
                return;
            };

            self.obj().graph().remove_node(&node);
        }

        /// Add a new port to the view.
        fn add_port(&self, id: u32, name: &str, node_id: u32, direction: pipewire::spa::Direction) {
            log::info!("Adding port to graph: id {}", id);

            let mut items = self.items.borrow_mut();

            let Some(node) = items.get(&node_id) else {
                log::warn!("Node (id: {node_id}) for port (id: {id}) not found in graph manager");
                return;
            };
            let Ok(node) = node.clone().dynamic_cast::<graph::Node>() else {
                log::warn!("Graph Manager item under node id {node_id} is not a node");
                return;
            };

            let port = graph::Port::new(id, name, direction);

            // Create or delete a link if the widget emits the "port-toggled" signal.
            port.connect_local(
                "port_toggled",
                false,
                glib::clone!(@weak self as app => @default-return None, move |args| {
                    // Args always look like this: &[widget, id_port_from, id_port_to]
                    let port_from = args[1].get::<u32>().unwrap();
                    let port_to = args[2].get::<u32>().unwrap();

                    app.toggle_link(port_from, port_to);

                    None
                }),
            );

            items.insert(id, port.clone().upcast());

            node.add_port(port);
        }

        fn port_media_type_changed(&self, id: u32, media_type: MediaType) {
            let items = self.items.borrow();

            let Some(port) = items.get(&id) else {
                log::warn!("Port (id: {id}) for changed media type not found in graph manager");
                return;
            };
            let Some(port) = port.dynamic_cast_ref::<graph::Port>() else {
                log::warn!("Graph Manager item under port id {id} is not a port");
                return;
            };

            port.set_media_type(media_type.as_raw())
        }

        /// Remove the port with the id `id` from the node with the id `node_id`
        /// from the view.
        fn remove_port(&self, id: u32, node_id: u32) {
            log::info!("Removing port from graph: id {}, node_id: {}", id, node_id);

            let mut items = self.items.borrow_mut();

            let Some(node) = items.get(&node_id) else {
                log::warn!("Node (id: {node_id}) for port (id: {id}) not found in graph manager");
                return;
            };
            let Ok(node) = node.clone().dynamic_cast::<graph::Node>() else {
                log::warn!("Graph Manager item under node id {node_id} is not a node");
                return;
            };
            let Some(port) = items.remove(&id) else {
                log::warn!("Unknown Port (id: {id}) removed from graph");
                return;
            };
            let Ok(port) = port.dynamic_cast::<graph::Port>() else {
                log::warn!("Graph Manager item under port id {id} is not a port");
                return;
            };

            node.remove_port(&port);
        }

        /// Add a new link to the view.
        fn add_link(
            &self,
            id: u32,
            output_port_id: u32,
            input_port_id: u32,
            active: bool,
            media_type: MediaType,
        ) {
            log::info!("Adding link to graph: id {}", id);

            let mut items = self.items.borrow_mut();

            let Some(output_port) = items.get(&output_port_id) else {
                log::warn!("Output port (id: {output_port_id}) for link (id: {id}) not found in graph manager");
                return;
            };
            let Ok(output_port) = output_port.clone().dynamic_cast::<graph::Port>() else {
                log::warn!("Graph Manager item under port id {output_port_id} is not a port");
                return;
            };
            let Some(input_port) = items.get(&input_port_id) else {
                log::warn!("Output port (id: {input_port_id}) for link (id: {id}) not found in graph manager");
                return;
            };
            let Ok(input_port) = input_port.clone().dynamic_cast::<graph::Port>() else {
                log::warn!("Graph Manager item under port id {input_port_id} is not a port");
                return;
            };

            let link = graph::Link::new();
            link.set_output_port(Some(&output_port));
            link.set_input_port(Some(&input_port));
            link.set_active(active);
            link.set_media_type(media_type);

            items.insert(id, link.clone().upcast());

            // Update graph to contain the new link.
            self.graph
                .get()
                .expect("graph should be set")
                .add_link(link);
        }

        fn link_state_changed(&self, id: u32, active: bool) {
            log::info!(
                "Link state changed: Link (id={id}) is now {}",
                if active { "active" } else { "inactive" }
            );

            let items = self.items.borrow();

            let Some(link) = items.get(&id) else {
                log::warn!("Link state changed on unknown link (id={id})");
                return;
            };
            let Some(link) = link.dynamic_cast_ref::<graph::Link>() else {
                log::warn!("Graph Manager item under link id {id} is not a link");
                return;
            };

            link.set_active(active);
        }

        fn link_format_changed(&self, id: u32, media_type: pipewire::spa::format::MediaType) {
            let items = self.items.borrow();

            let Some(link) = items.get(&id) else {
                log::warn!("Link (id: {id}) for changed media type not found in graph manager");
                return;
            };
            let Some(link) = link.dynamic_cast_ref::<graph::Link>() else {
                log::warn!("Graph Manager item under link id {id} is not a link");
                return;
            };
            link.set_media_type(media_type);
        }

        // Toggle a link between the two specified ports on the remote pipewire server.
        fn toggle_link(&self, port_from: u32, port_to: u32) {
            let sender = self.pw_sender.get().expect("pw_sender shoud be set");
            sender
                .send(crate::GtkMessage::ToggleLink { port_from, port_to })
                .expect("Failed to send message");
        }

        /// Remove the link with the specified id from the view.
        fn remove_link(&self, id: u32) {
            log::info!("Removing link from graph: id {}", id);

            let Some(link) = self.items.borrow_mut().remove(&id) else {
                log::warn!("Unknown Link (id={id}) removed from graph");
                return;
            };
            let Ok(link) = link.dynamic_cast::<graph::Link>() else {
                log::warn!("Graph Manager item under link id {id} is not a link");
                return;
            };

            self.obj().graph().remove_link(&link);
        }

        fn clear(&self) {
            self.items.borrow_mut().clear();
            self.obj().graph().clear();
        }
    }
}

glib::wrapper! {
    pub struct GraphManager(ObjectSubclass<imp::GraphManager>);
}

impl GraphManager {
    pub fn new(
        graph: &GraphView,
        sender: PwSender<GtkMessage>,
        receiver: glib::Receiver<PipewireMessage>,
    ) -> Self {
        let res: Self = glib::Object::builder().property("graph", graph).build();

        res.imp().attach_receiver(receiver);
        assert!(
            res.imp().pw_sender.set(sender).is_ok(),
            "Should be able to set pw_sender)"
        );

        res
    }
}
