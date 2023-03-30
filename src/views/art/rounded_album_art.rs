/* rounded_album_art.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::subclass::prelude::*;
use gtk::{glib, prelude::*};

use gtk::{gdk, gdk_pixbuf, graphene, gsk};

use std::{cell::Cell, cell::RefCell, rc::Rc};

//simple gtk widget subclass that displays album art from bytes on a gdk pixbuf, with rounded corners

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default)]
    pub struct RoundedAlbumArt {
        pub size: Cell<i32>,
        pub error: Cell<bool>,
        pub pixbuf: RefCell<Option<gdk_pixbuf::Pixbuf>>,
        pub texture: RefCell<Option<gdk::Texture>>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RoundedAlbumArt {
        const NAME: &'static str = "RoundedAlbumArt";
        type Type = super::RoundedAlbumArt;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for RoundedAlbumArt {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("populated").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for RoundedAlbumArt {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if let Some(texture) = self.texture.borrow_mut().as_ref() {
                let rect = graphene::Rect::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);
                let rounded_rect = gsk::RoundedRect::from_rect(rect, 10.0);

                snapshot.push_rounded_clip(&rounded_rect);
                snapshot.append_texture(texture, &rect);
                snapshot.pop();

            }
        }
    }

    impl RoundedAlbumArt {}
}

glib::wrapper! {
    pub struct RoundedAlbumArt(ObjectSubclass<imp::RoundedAlbumArt>)
    @extends gtk::Widget;
}

impl RoundedAlbumArt {
    pub fn new(size: i32) -> RoundedAlbumArt {
        let album_art: RoundedAlbumArt =  glib::Object::builder::<RoundedAlbumArt>().build();
        album_art.imp().size.set(size);
        album_art
    }

    pub fn load(&self, pixbuf: Rc<gdk_pixbuf::Pixbuf>) {
        let imp = self.imp();
        let new_pixbuf = match pixbuf.scale_simple(imp.size.get(), imp.size.get(), gdk_pixbuf::InterpType::Bilinear) {
            Some(pixbuf) => pixbuf,
            None => {
                return;
            }
        };
        self.add_pixbuf(Some(new_pixbuf));
    }

    fn add_pixbuf(&self, pixbuf: Option<gdk_pixbuf::Pixbuf>) {
        let imp = self.imp();
        match pixbuf {
            Some(pixbuf) => {
                let texture = gdk::Texture::for_pixbuf(&pixbuf);
                imp.texture.replace(Some(texture));
                imp.pixbuf.replace(Some(pixbuf));
                imp.error.set(false);
            },
            None => {
                imp.error.set(true);
            }
        }
        self.emit_by_name::<()>("populated", &[]);
    }

}