use gtk::{glib, prelude::*, subclass::prelude::*};

use super::GraphView;

mod imp {
    use std::cell::RefCell;

    use super::*;

    use gtk::{gio, glib::clone};
    use once_cell::sync::Lazy;

    #[derive(gtk::CompositeTemplate)]
    #[template(file = "zoomentry.ui")]
    pub struct ZoomEntry {
        pub graphview: RefCell<Option<GraphView>>,
        #[template_child]
        pub zoom_out_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub zoom_in_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        pub popover: gtk::PopoverMenu,
    }

    impl Default for ZoomEntry {
        fn default() -> Self {
            let menu = gio::Menu::new();
            menu.append(Some("30%"), Some("win.set-zoom(0.30)"));
            menu.append(Some("50%"), Some("win.set-zoom(0.50)"));
            menu.append(Some("75%"), Some("win.set-zoom(0.75)"));
            menu.append(Some("100%"), Some("win.set-zoom(1.0)"));
            menu.append(Some("150%"), Some("win.set-zoom(1.5)"));
            menu.append(Some("200%"), Some("win.set-zoom(2.0)"));
            menu.append(Some("300%"), Some("win.set-zoom(3.0)"));
            let popover = gtk::PopoverMenu::from_model(Some(&menu));

            ZoomEntry {
                graphview: Default::default(),
                zoom_out_button: Default::default(),
                zoom_in_button: Default::default(),
                entry: Default::default(),
                popover,
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ZoomEntry {
        const NAME: &'static str = "HelvumZoomEntry";
        type Type = super::ZoomEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ZoomEntry {
        fn constructed(&self) {
            self.parent_constructed();

            self.zoom_out_button
                .connect_clicked(clone!(@weak self as imp => move |_| {
                    let graphview = imp.graphview.borrow();
                    if let Some(ref graphview) = *graphview {
                        graphview.set_zoom_factor(graphview.zoom_factor() - 0.1, None);
                    }
                }));

            self.zoom_in_button
                .connect_clicked(clone!(@weak self as imp => move |_| {
                    let graphview = imp.graphview.borrow();
                    if let Some(ref graphview) = *graphview {
                        graphview.set_zoom_factor(graphview.zoom_factor() + 0.1, None);
                    }
                }));

            self.entry
                .connect_activate(clone!(@weak self as imp => move |entry| {
                    if let Ok(zoom_factor) = entry.text().trim_matches('%').parse::<f64>() {
                        let graphview = imp.graphview.borrow();
                        if let Some(ref graphview) = *graphview {
                            graphview.set_zoom_factor(zoom_factor / 100.0, None);
                        }
                    }
                }));
            self.entry
                .connect_icon_press(clone!(@weak self as imp => move |_, pos| {
                    if pos == gtk::EntryIconPosition::Secondary {
                        imp.popover.show();
                    }
                }));

            self.popover.set_parent(&self.entry.get());
        }

        fn dispose(&self) {
            self.popover.unparent();

            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<GraphView>("zoomed-widget")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT)
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "zoomed-widget" => self.graphview.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "zoomed-widget" => {
                    let widget: GraphView = value.get().unwrap();
                    widget.connect_notify_local(
                        Some("zoom-factor"),
                        clone!(@weak self as imp => move |graphview, _| {
                            imp.update_zoom_factor_text(graphview.zoom_factor());
                        }),
                    );
                    self.update_zoom_factor_text(widget.zoom_factor());
                    *self.graphview.borrow_mut() = Some(widget);
                }
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for ZoomEntry {}
    impl BoxImpl for ZoomEntry {}

    impl ZoomEntry {
        /// Update the text contained in the combobox's entry to reflect the provided zoom factor.
        ///
        /// This does not update the associated [`GraphView`]s zoom level.
        fn update_zoom_factor_text(&self, zoom_factor: f64) {
            self.entry
                .buffer()
                .set_text(&format!("{factor:.0}%", factor = zoom_factor * 100.));
        }
    }
}

glib::wrapper! {
    pub struct ZoomEntry(ObjectSubclass<imp::ZoomEntry>)
        @extends gtk::Box, gtk::Widget;
}

impl ZoomEntry {
    pub fn new(zoomed_widget: &GraphView) -> Self {
        glib::Object::builder()
            .property("zoomed-widget", zoomed_widget)
            .build()
    }
}
