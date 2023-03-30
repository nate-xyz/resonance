// use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/example_page.ui")]
    pub struct ExamplePagePriv {
        #[template_child(id = "test_label")]
        pub test_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExamplePagePriv {
        const NAME: &'static str = "ExamplePage";
        type Type = super::ExamplePage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for ExamplePagePriv {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("example-signal").build(),
                ]
            });

            SIGNALS.as_ref()
        }

    }

    impl WidgetImpl for ExamplePagePriv {}
    impl BoxImpl for ExamplePagePriv {}

    impl ExamplePagePriv {}
}

glib::wrapper! {
    pub struct ExamplePage(ObjectSubclass<imp::ExamplePagePriv>)
    @extends gtk::Box, gtk::Widget;
}


impl ExamplePage {
    pub fn new() -> ExamplePage {
        let example_subclass: ExamplePage = glib::Object::builder::<ExamplePage>().build();
        example_subclass
    }

}
    