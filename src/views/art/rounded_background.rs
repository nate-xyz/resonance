/* rounded_background.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::*;
use gtk::{subclass::prelude::*, gdk, glib, graphene, gsk, gdk_pixbuf};
use std::{cell::Cell, cell::RefCell, rc::Rc};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct RoundedBackground {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub color: RefCell<String>,
        pub size: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RoundedBackground {
        const NAME: &'static str = "RoundedBackground";
        type Type = super::RoundedBackground;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for RoundedBackground {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for RoundedBackground {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(texture) = self.texture.borrow_mut().as_ref() {
                let rect = graphene::Rect::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);
                let rounded_rect = gsk::RoundedRect::from_rect(rect, 180.0);

                snapshot.push_rounded_clip(&rounded_rect);
                snapshot.append_texture(texture, &rect);
                snapshot.pop();
            } else {
                let widget = &*self.obj();
                let color = gdk::RGBA::parse(self.color.borrow().clone()).unwrap();
                let rect = graphene::Rect::new(0.0, 0.0, widget.width() as f32, widget.height() as f32);
                let rounded_rect = gsk::RoundedRect::from_rect(rect, 10.0);
    
                snapshot.push_rounded_clip(&rounded_rect);
                snapshot.append_color(&color, &rect);
                snapshot.pop();
            }
        }
    }    
}

glib::wrapper! {
    pub struct RoundedBackground(ObjectSubclass<imp::RoundedBackground>)
        @extends gtk::Widget;
}

impl RoundedBackground {
    pub fn new(color: &str, size: i32) -> RoundedBackground {
        let bg: RoundedBackground = glib::Object::builder::<RoundedBackground>().build();
        bg.load(color, size);
        bg
    }

    fn load(&self, color: &str, size: i32) {
        let imp = self.imp();
        imp.color.replace(color.to_string());
        imp.size.set(size);
    }

    pub fn load_art(&self, pixbuf: Rc<gdk_pixbuf::Pixbuf>) {
        let imp = self.imp();
        let bg_pixbuf = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, imp.size.get(), imp.size.get()).unwrap();
        bg_pixbuf.fill(0);

        let w = pixbuf.width();
        let h = pixbuf.height();

        if w > h {
            let scaling_factor = imp.size.get() as f32 / w as f32;

            let h = (h as f32 * scaling_factor) as i32;
            let w = (w as f32 * scaling_factor) as i32;

            let offset_x = (imp.size.get() - w).abs() as f64 / 2.0;
            let offset_y = (imp.size.get() - h).abs() as f64 / 2.0;

            if let Some(pixbuf) = pixbuf.scale_simple(w, h, gdk_pixbuf::InterpType::Bilinear) {
                pixbuf.composite(
                    &bg_pixbuf, 0, 0, 
                    imp.size.get(), imp.size.get(),
                    offset_x, offset_y, 
                    1.0, 1.0, gdk_pixbuf::InterpType::Nearest, 255
                );
                let texture = gdk::Texture::for_pixbuf(&bg_pixbuf);
                imp.texture.replace(Some(texture));
            }

        } else {
            let scaling_factor = imp.size.get() as f32 / h as f32;

            let h = (h as f32 * scaling_factor) as i32;
            let w = (w as f32 * scaling_factor) as i32;

            let offset_x = (imp.size.get() - w).abs() as f64 / 2.0;
            let offset_y = (imp.size.get() - h).abs() as f64 / 2.0;

            if let Some(pixbuf) = pixbuf.scale_simple(w, h, gdk_pixbuf::InterpType::Bilinear) {
                pixbuf.composite(
                    &bg_pixbuf, 0, 0, 
                    imp.size.get(), imp.size.get(),
                    offset_x, offset_y, 
                    1.0, 1.0, gdk_pixbuf::InterpType::Nearest, 255
                );
                let texture = gdk::Texture::for_pixbuf(&bg_pixbuf);
                imp.texture.replace(Some(texture));
            }

        }



        // if let Some(pixbuf) = pixbuf.scale_simple(imp.size.get(), imp.size.get(), gdk_pixbuf::InterpType::Bilinear) {
        //     let texture = gdk::Texture::for_pixbuf(&pixbuf);
        //     imp.texture.replace(Some(texture));
        // }
    }
}