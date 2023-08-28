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
    gio,
    glib::{self, clone, Receiver},
    gtk,
    prelude::*,
    subclass::prelude::*,
};
use pipewire::channel::Sender;

use crate::{graph_manager::GraphManager, ui, GtkMessage, PipewireMessage};

static STYLE: &str = include_str!("style.css");

mod imp {
    use super::*;

    use adw::subclass::prelude::AdwApplicationImpl;
    use once_cell::unsync::OnceCell;

    #[derive(Default)]
    pub struct Application {
        pub(super) window: ui::Window,
        pub(super) graph_manager: OnceCell<GraphManager>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "HelvumApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {}
    impl ApplicationImpl for Application {
        fn activate(&self) {
            let app = &*self.obj();

            let graphview = self.window.graph();

            self.window.set_application(Some(app));

            let zoom_set_action =
                gio::SimpleAction::new("set-zoom", Some(&f64::static_variant_type()));
            zoom_set_action.connect_activate(clone!(@weak graphview => move|_, param| {
                let zoom_factor = param.unwrap().get::<f64>().unwrap();
                graphview.set_zoom_factor(zoom_factor, None)
            }));
            self.window.add_action(&zoom_set_action);

            self.window.show();
        }

        fn startup(&self) {
            self.parent_startup();

            self.obj()
                .style_manager()
                .set_color_scheme(adw::ColorScheme::PreferDark);

            // Load CSS from the STYLE variable.
            let provider = gtk::CssProvider::new();
            provider.load_from_data(STYLE);
            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Error initializing gtk css provider."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
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
            .set(GraphManager::new(
                &imp.window.graph(),
                pw_sender,
                gtk_receiver,
            ))
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
