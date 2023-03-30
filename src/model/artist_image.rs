/* artist_image.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{subclass::prelude::*, glib};
use gtk::{gdk, gdk_pixbuf::Pixbuf};

use std::{cell::{RefCell, Cell}, rc::Rc};
use log::debug;

use super::cover_art_pixbuf_loader::CoverArtPixbufLoader;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ArtistImage {
        pub id: Cell<i64>,
        pub url: RefCell<String>,
        pub capl: RefCell<Option<CoverArtPixbufLoader>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistImage {
        const NAME: &'static str = "ArtistImage";
        type Type = super::ArtistImage;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for ArtistImage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl ArtistImage {}
}

glib::wrapper! {
    pub struct ArtistImage(ObjectSubclass<imp::ArtistImage>);
}


impl ArtistImage {
    pub fn new(id: i64, url: String, data: Vec<u8>)-> ArtistImage {
        let cover_art: ArtistImage = glib::Object::builder::<ArtistImage>().build();
        cover_art.load(id, url, data);
        cover_art
    }

    pub fn load(&self, id: i64, url: String, data: Vec<u8>) {
        let imp = self.imp();
        imp.id.set(id);
        imp.url.replace(url);
        let capl = CoverArtPixbufLoader::new();
        let result = capl.load(data);
        match result {
            Ok(()) => {
                imp.capl.replace(Some(capl));
            },
            Err(err) => debug!("{}", err),
        }
    }


    pub fn pixbuf(&self) ->  Result<Rc<Pixbuf>, String> {
        match self.imp().capl.borrow().as_ref() {
            Some(capl) => {
                capl.pixbuf()
            },
            None => Err("No Pixbuf".to_string())
        }
    }

    pub fn palette(&self) ->  Result<Option<Vec<gdk::RGBA>>, String> {
        match self.imp().capl.borrow().as_ref() {
            Some(capl) => {
                Ok(capl.palette())
            },
            None => Err("No Palette".to_string())
        }
    }
}
    