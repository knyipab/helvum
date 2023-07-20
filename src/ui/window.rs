use adw::{gio, gtk, prelude::*, subclass::prelude::*};

use super::graph;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::Window)]
    #[template(file = "window.ui")]
    pub struct Window {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        #[property(type = graph::GraphView, get = |_| self.graph.clone())]
        pub graph: TemplateChild<graph::GraphView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "HelvumWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            // Ensure custom types are registered
            graph::GraphView::ensure_type();
            graph::ZoomEntry::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Window {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for Window {
    fn default() -> Self {
        Self::new()
    }
}
