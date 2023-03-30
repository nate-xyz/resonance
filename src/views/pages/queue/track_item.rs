/* track_item.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

// use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::glib;
use std::{cell::Cell, cell::RefCell, rc::Rc};
use crate::model::track::Track;

mod imp {
    use super::*;
    
    #[derive(Debug, Default)]
    pub struct TrackItemPriv {
        pub track: RefCell<Option<Rc<Track>>>,
        pub playlist_position: Cell<u64>,
        pub search_string: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrackItemPriv {
        const NAME: &'static str = "TrackItem";
        type Type = super::TrackItem;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for TrackItemPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl TrackItemPriv {}
}

glib::wrapper! {
    pub struct TrackItem(ObjectSubclass<imp::TrackItemPriv>);
}


impl TrackItem {
    pub fn new(track: Rc<Track>, position: u64, search_string: String) -> TrackItem {
        let track_item: TrackItem = glib::Object::builder::<TrackItem>().build();
          track_item.load(track, position, search_string);
          track_item
    }

    fn load(&self, track: Rc<Track>, position: u64, search_string: String) {
        let imp = self.imp();
        imp.track.replace(Some(track));
        imp.playlist_position.set(position);
        imp.search_string.replace(search_string);
    }

    pub fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }

    pub fn position(&self) -> u64 {
        self.imp().playlist_position.get()
    }

    pub fn search_string(&self) -> String {
        self.imp().search_string.borrow().clone()
    }

}
    