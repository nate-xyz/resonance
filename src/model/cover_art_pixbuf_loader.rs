/* cover_art_pixbuf_loader.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{gdk_pixbuf::Pixbuf, gdk_pixbuf, gdk, glib};
use gtk::{subclass::prelude::*, prelude::*};
use std::{cell::RefCell, rc::Rc};

use crate::util;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct CoverArtPixbufLoader {
        pub pixbuf: RefCell<Option<Rc<Pixbuf>>>,
        pub palette: RefCell<Option<Vec<gdk::RGBA>>>
        //pub data: RefCell<Option<Vec<u8>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CoverArtPixbufLoader {
        const NAME: &'static str = "CoverArtPixbufLoader";
        type Type = super::CoverArtPixbufLoader;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for CoverArtPixbufLoader {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl CoverArtPixbufLoader {}
}

glib::wrapper! {
    pub struct CoverArtPixbufLoader(ObjectSubclass<imp::CoverArtPixbufLoader>);
}

impl Default for CoverArtPixbufLoader {
    fn default() -> Self {
        glib::Object::builder::<CoverArtPixbufLoader>().build()
    }
}

impl CoverArtPixbufLoader {
    pub fn new() -> CoverArtPixbufLoader {
        let capl: CoverArtPixbufLoader = glib::Object::builder::<CoverArtPixbufLoader>().build();
        capl
    }

    pub fn load(&self, data: Vec<u8>) -> Result<(), glib::Error> {
        let imp = self.imp();
        let loader = gdk_pixbuf::PixbufLoader::new();
        loader.write(&data[..])?;
        loader.close()?;

        let finished_pixbuf = loader.pixbuf().unwrap();
        let palette = util::load_palette(&finished_pixbuf);
        let finished_pixbuf = Rc::new(finished_pixbuf);
        
        imp.pixbuf.replace(Some(finished_pixbuf));
        imp.palette.replace(palette);
        //imp.data.replace(Some(data));
        Ok(())
    }

    pub fn pixbuf(&self) -> Result<Rc<Pixbuf>, String> {
        match self.imp().pixbuf.borrow().as_ref() {
            Some(p) => {
                let pixbuf = Rc::clone(p);
                Ok(pixbuf)
            },
            None => {
                Err("Unable to access pixbuf".to_string())
            }

        }
    }

    pub fn palette(&self) -> Option<Vec<gdk::RGBA>> {
        self.imp().palette.borrow().as_ref().cloned()
    }
}