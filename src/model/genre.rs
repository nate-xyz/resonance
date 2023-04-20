/* genre.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::glib;

use std::{cell::Cell, cell::RefCell, rc::Rc};
use regex::Regex;

use super::album::Album;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct GenrePriv {
        pub name: RefCell<String>,
        pub sort_name: RefCell<String>,
        pub id: Cell<i64>,
        pub albums: RefCell<Option<Vec<Rc<Album>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenrePriv {
        const NAME: &'static str = "Genre";
        type Type = super::Genre;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for GenrePriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl GenrePriv {}
}

glib::wrapper! {
    pub struct Genre(ObjectSubclass<imp::GenrePriv>);
}

impl Genre {
    pub fn new(name: String, id: i64) -> Genre {
        let genre: Genre = glib::Object::builder::<Genre>().build();
        genre.load(name, id);
        genre
    }

    fn load(&self, name: String, id: i64) {
        let imp = self.imp();

        imp.id.replace(id);

        let re = Regex::new(r"^(the|a|an)\s+").unwrap();
        let lowercase_name = name.to_lowercase();
        let stripped_name: std::borrow::Cow<str> = re.replace(&lowercase_name, "");
        imp.sort_name.replace(format!("{}", stripped_name));

        imp.name.replace(name);
    }
    
    pub fn id(&self) -> i64 {
        self.imp().id.get().clone()
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().to_string().clone()
    }

    pub fn sort_name(&self) -> String {
        self.imp().sort_name.borrow().to_string()
    }

    pub fn add_album(&self, album: Rc<Album>) {
        let imp = self.imp();

        if None == imp.albums.borrow().as_ref() {
            imp.albums.replace(Some(vec![album]));
            return;
        } 

        if let Some(albums)  = imp.albums.borrow_mut().as_mut() {
            albums.push(album);
            return;
        }

    }

    pub fn albums(&self) -> Option<Vec<Rc<Album>>> {
        if let Some(albums) = self.imp().albums.borrow().as_ref() {
            Some(albums.clone())
        } else {
            None
        }
    }

    pub fn n_tracks(&self) -> usize {
        let mut n_tracks = 0;
        if let Some(albums) = self.imp().albums.borrow().as_ref() {
            for album in albums {
                n_tracks += album.tracks().len();
            }
        }
        n_tracks
    }

    pub fn n_albums(&self) -> usize {
        match self.imp().albums.borrow().as_ref() {
            Some(albums) => albums.len(),
            None => 0,
        }
    }
}
