/* cover_art.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{glib, gdk, gdk_pixbuf::Pixbuf};
use gtk::subclass::prelude::*;

use std::{cell::{RefCell, Cell}, rc::Rc};
use log::debug;

use super::cover_art_pixbuf_loader::CoverArtPixbufLoader;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct CoverArt {
        pub id: Cell<i64>,
        pub capl: RefCell<Option<CoverArtPixbufLoader>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CoverArt {
        const NAME: &'static str = "CoverArt";
        type Type = super::CoverArt;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for CoverArt {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl CoverArt {}
}

glib::wrapper! {
    pub struct CoverArt(ObjectSubclass<imp::CoverArt>);
}


impl CoverArt {
    pub fn new(id: i64, data: Vec<u8>)-> CoverArt {
        let cover_art: CoverArt = glib::Object::builder::<CoverArt>().build();
        cover_art.load(id, data);
        cover_art
    }

    pub fn load(&self, id: i64, data: Vec<u8>) {
        let imp = self.imp();
        imp.id.set(id);
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
    