/* playlist.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use glib::prelude::ToVariant;
use gtk::{glib, gio};
use gtk::subclass::prelude::*;

use std::collections::{HashSet, HashMap};
use std::{cell::Cell, cell::RefCell, rc::Rc};

use super::track::Track;
use super::playlist_entry::PlaylistEntry;

use chrono::{DateTime, Utc};

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlaylistPriv {
        pub id: Cell<i64>,
        pub title: RefCell<String>,
        pub description: RefCell<String>,
        pub creation_time: RefCell<DateTime<Utc>>,
        pub modify_time: RefCell<DateTime<Utc>>,
        pub cover_art_ids: RefCell<HashSet<i64>>,
        pub entries: RefCell<HashMap<i64, Rc<PlaylistEntry>>>,
        pub total_duration: Cell<f64>,
        pub menu: gio::Menu,
    }


    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistPriv {
        const NAME: &'static str = "Playlist";
        type Type = super::Playlist;
        type ParentType = glib::Object;
    }


    impl ObjectImpl for PlaylistPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl PlaylistPriv {}
}

glib::wrapper! {
    pub struct Playlist(ObjectSubclass<imp::PlaylistPriv>);
}

impl Playlist {
    pub fn new(id: i64, title: String, description: String, creation_time: DateTime<Utc>, modify_time: DateTime<Utc>) -> Playlist {
        let playlist: Playlist = glib::Object::builder::<Playlist>().build();
        playlist.load(id, title, description, creation_time, modify_time);
        playlist
    }

    fn load(&self, id: i64, title: String, description: String, creation_time: DateTime<Utc>, modify_time: DateTime<Utc>) {
        let imp = self.imp();
        imp.id.set(id);
        imp.title.replace(title);
        imp.description.replace(description);
        imp.creation_time.replace(creation_time);
        imp.modify_time.replace(modify_time);
        //self.create_menu();
    }

    pub fn id(&self) -> i64 {
        self.imp().id.get().clone()
    }

    pub fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }

    pub fn description(&self) -> String {
        self.imp().description.borrow().clone()
    }

    pub fn creation_time(&self) -> DateTime<Utc> {
        self.imp().creation_time.borrow().clone()
    }

    pub fn modify_time(&self) -> DateTime<Utc> {
        self.imp().modify_time.borrow().clone()
    }

    pub fn duration(&self) -> f64 {
        self.imp().total_duration.get().clone()
    }

    pub fn n_tracks(&self) -> usize {
        self.imp().entries.borrow().len()
    }

    pub fn cover_art_ids(&self) -> Vec<i64> {
        Vec::from_iter(self.imp().cover_art_ids.borrow().clone())
    }

    pub fn entry_map(&self) -> HashMap<i64, Rc<PlaylistEntry>> {
        self.imp().entries.borrow().clone()
    }

    fn track(&self, position: i64) -> Option<Rc<Track>> {
        if let Some(entry_map) = self.imp().entries.borrow().get(&position) {
            Some(entry_map.track())
        } else {
            None
        }
    }

    pub fn tracks(&self) -> Vec<Rc<Track>> {
        let mut v = vec![self.track(0).unwrap(); self.n_tracks()];
        for playlist_entry in self.entry_map().values() {
            let track = playlist_entry.track();
            let p = playlist_entry.position() as usize;
            let _got = std::mem::replace(&mut v[p], track);
        }
        v
    }

    pub fn add_track(&self, id: i64, playlist_position: i64, track: Rc<Track>) {
        // self.tracks[position] = PlaylistEntry(playlist_entry_id, track, position, self.id)
        // self.total_duration += track.duration
        // if track.cover_art_id != None:
        //     self.cover_art_ids.add(track.cover_art_id)

        let imp = self.imp();

        imp.total_duration.set(imp.total_duration.get() + track.duration());
        
        if let Some(art_id) = track.cover_art_option() {
            imp.cover_art_ids.borrow_mut().insert(art_id);
        }

        let entry = PlaylistEntry::new(id, playlist_position, self.id(), track);
        imp.entries.borrow_mut().insert(playlist_position, Rc::new(entry));
    }

    #[allow(dead_code)]
    fn create_menu(&self) {
        let imp = self.imp();
        
        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Play «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.play-playlist"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Queue", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-playlist-to-queue"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Play"), &menu);

        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Go to Playlist «{}» Detail", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.go-to-playlist-detail"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Navigate"), &menu);

        let menu = gio::Menu::new();
        
        let menu_item = gio::MenuItem::new(Some(&format!("Duplicate «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.duplicate-playlist"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Delete «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.delete-playlist"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Playlist"), &menu);
    }

    pub fn menu_model(&self)-> &gio::Menu {
        &self.imp().menu
    }

}
    