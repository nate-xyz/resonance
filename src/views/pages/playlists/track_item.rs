/* track_item.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::glib;

use std::{cell::Cell, cell::RefCell, rc::Rc};

use crate::model::playlist_entry::PlaylistEntry;
use crate::model::track::Track;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlaylistDetailTrackItemPriv {
        pub playlist_id: Cell<i64>,
        pub playlist_entry_id: Cell<i64>,
        pub original_playlist_position: Cell<i64>,
        pub playlist_position: Cell<i64>,
        pub search_string: RefCell<String>,
        pub playlist_entry: RefCell<Option<Rc<PlaylistEntry>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistDetailTrackItemPriv {
        const NAME: &'static str = "PlaylistDetailTrackItem";
        type Type = super::PlaylistDetailTrackItem;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for PlaylistDetailTrackItemPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl PlaylistDetailTrackItemPriv {}
}

glib::wrapper! {
    pub struct PlaylistDetailTrackItem(ObjectSubclass<imp::PlaylistDetailTrackItemPriv>);
}


impl PlaylistDetailTrackItem {
    pub fn new(playlist_id: i64, playlist_entry: Rc<PlaylistEntry>) -> PlaylistDetailTrackItem {
        let track_item: PlaylistDetailTrackItem = glib::Object::builder::<PlaylistDetailTrackItem>().build();
        track_item.load(playlist_entry, playlist_id);
        track_item
    }

    fn load(&self, playlist_entry: Rc<PlaylistEntry>, id: i64) {
        let imp = self.imp();
        imp.playlist_id.set(id);
        imp.playlist_entry_id.set(playlist_entry.id());
        imp.playlist_position.set(playlist_entry.position());
        imp.original_playlist_position.set(playlist_entry.position());
        let track = playlist_entry.track();
        imp.search_string.replace(format!("{} {} {}", track.title(), track.album(), track.artist()));
        imp.playlist_entry.replace(Some(playlist_entry));
    }

    pub fn playlist_entry(&self) -> Rc<PlaylistEntry> {
        self.imp().playlist_entry.borrow().as_ref().unwrap().clone()
    }

    pub fn track(&self) -> Rc<Track> {
        self.playlist_entry().track()
    }
    
    pub fn playlist_entry_id(&self) -> i64 {
        self.imp().playlist_entry_id.get()
    }

    pub fn playlist_id(&self) -> i64 {
        self.imp().playlist_id.get()
    }

    pub fn set_position(&self, position: i64) {
        self.imp().playlist_position.set(position);
    }

    pub fn reset_position(&self) {
        self.imp().playlist_position.set(self.imp().original_playlist_position.get())
    }

    pub fn position(&self) -> i64 {
        self.imp().playlist_position.get()
    }

    pub fn search_string(&self) -> String {
        self.imp().search_string.borrow().clone()
    }

}
    