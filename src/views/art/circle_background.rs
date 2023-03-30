/* circle_background.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use gtk::{subclass::prelude::*, gdk, glib, graphene, gsk};

use std::cell::{RefCell, Cell};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct CircleBackground {
        pub color: RefCell<String>,
        pub size: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CircleBackground {
        const NAME: &'static str = "CircleBackground";
        type Type = super::CircleBackground;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for CircleBackground {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for CircleBackground {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = &*self.obj();
            let color = gdk::RGBA::parse(self.color.borrow().clone()).unwrap();

            let rect = graphene::Rect::new(0.0, 0.0, widget.width() as f32, widget.height() as f32);
           
            let rounded_rect = gsk::RoundedRect::from_rect(rect, 90.0);

            snapshot.push_rounded_clip(&rounded_rect);
            snapshot.append_color(&color, &rect);
            snapshot.pop();

        }
    }

    
}

glib::wrapper! {
    pub struct CircleBackground(ObjectSubclass<imp::CircleBackground>)
        @extends gtk::Widget;
}

impl CircleBackground {
    pub fn new(color: &str, size: i32) -> CircleBackground {
        let bg: CircleBackground = glib::Object::builder::<CircleBackground>().build();
        bg.load(color, size);
        bg
    }

    fn load(&self, color: &str, size: i32) {
        self.imp().color.replace(color.to_string());
        self.imp().size.set(size);
    }
}