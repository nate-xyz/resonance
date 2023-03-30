/* background.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use gtk::{subclass::prelude::*, gdk, glib, graphene};

use std::cell::{RefCell, Cell};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Background {
        pub color: RefCell<String>,
        pub size: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Background {
        const NAME: &'static str = "Background";
        type Type = super::Background;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Background {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for Background {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();
            let color = gdk::RGBA::parse(self.color.borrow().clone()).unwrap();
            let rect = graphene::Rect::new(0.0, 0.0, widget.width() as f32, widget.height() as f32);
            snapshot.append_color(&color, &rect);
        }
    }

    
}

glib::wrapper! {
    pub struct Background(ObjectSubclass<imp::Background>)
        @extends gtk::Widget;
}

impl Background {
    pub fn new(color: &str, size: i32) -> Background {
        let bg: Background = glib::Object::builder::<Background>().build();
        bg.load(color, size);
        bg
    }

    fn load(&self, color: &str, size: i32) {
        self.imp().color.replace(color.to_string());
        self.imp().size.set(size);
    }
}