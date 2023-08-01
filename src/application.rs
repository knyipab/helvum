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
    gio,
    glib::{self, clone, Receiver},
    prelude::*,
    subclass::prelude::*,
};
use pipewire::channel::Sender;

use crate::{graph_manager::GraphManager, ui, GtkMessage, PipewireMessage};

static STYLE: &str = include_str!("style.css");

mod imp {
    use super::*;

    use once_cell::unsync::OnceCell;

    #[derive(Default)]
    pub struct Application {
        pub(super) graphview: ui::graph::GraphView,
        pub(super) graph_manager: OnceCell<GraphManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "HelvumApplication";
        type Type = super::Application;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for Application {}
    impl ApplicationImpl for Application {
        fn activate(&self) {
            let app = &*self.obj();

            let scrollwindow = gtk::ScrolledWindow::builder()
                .child(&self.graphview)
                .build();
            let headerbar = gtk::HeaderBar::new();
            let zoomentry = ui::graph::ZoomEntry::new(&self.graphview);
            headerbar.pack_end(&zoomentry);

            let window = gtk::ApplicationWindow::builder()
                .application(app)
                .default_width(1280)
                .default_height(720)
                .title("Helvum - Pipewire Patchbay")
                .child(&scrollwindow)
                .build();
            window
                .settings()
                .set_gtk_application_prefer_dark_theme(true);
            window.set_titlebar(Some(&headerbar));

            let zoom_set_action =
                gio::SimpleAction::new("set-zoom", Some(&f64::static_variant_type()));
            zoom_set_action.connect_activate(
                clone!(@weak self.graphview as graphview => move|_, param| {
                    let zoom_factor = param.unwrap().get::<f64>().unwrap();
                    graphview.set_zoom_factor(zoom_factor, None)
                }),
            );
            window.add_action(&zoom_set_action);

            window.show();
        }

        fn startup(&self) {
            self.parent_startup();

            // Load CSS from the STYLE variable.
            let provider = gtk::CssProvider::new();
            provider.load_from_data(STYLE);
            gtk::StyleContext::add_provider_for_display(
                &gtk::gdk::Display::default().expect("Error initializing gtk css provider."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
    impl GtkApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    /// Create the view.
    /// This will set up the entire user interface and prepare it for being run.
    pub(super) fn new(
        gtk_receiver: Receiver<PipewireMessage>,
        pw_sender: Sender<GtkMessage>,
    ) -> Self {
        let app: Application = glib::Object::builder()
            .property("application-id", "org.pipewire.Helvum")
            .build();

        let imp = app.imp();

        imp.graph_manager
            .set(GraphManager::new(&imp.graphview, pw_sender, gtk_receiver))
            .expect("Should be able to set graph manager");

        // Add <Control-Q> shortcut for quitting the application.
        let quit = gtk::gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(@weak app => move |_, _| {
            app.quit();
        }));
        app.set_accels_for_action("app.quit", &["<Control>Q"]);
        app.add_action(&quit);

        app
    }
}
