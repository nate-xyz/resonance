/* album.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use glib::prelude::ToVariant;
use gtk::subclass::prelude::*;
use gtk::{glib, gio};

use std::collections::{HashSet, HashMap};
use std::{cell::RefCell, cell::Cell};
use std::rc::Rc;

use super::track::Track;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AlbumPriv {
        pub id: Cell<i64>,
        pub title: RefCell<String>,
        pub album_artist: RefCell<String>,
        pub date: RefCell<String>,
        pub genre: RefCell<String>,
        pub search_string: RefCell<String>,
        pub genre_id: Cell<Option<i64>>,
        pub artist_id: Cell<i64>,
        pub cover_art_id: Cell<Option<i64>>,
        pub track_ids: RefCell<HashSet<i64>>,
        pub discs: RefCell<HashMap<i64, HashMap<i64, Rc<Track>>>>,
        pub total_duration: Cell<f64>,
        // genre_parent: RefCell<Option<RefCell<Genre>>>,
        // artist_parent: RefCell<Option<RefCell<Artist>>>,
        pub menu: gio::Menu,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumPriv {
        const NAME: &'static str = "Album";
        type Type = super::Album;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for AlbumPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl AlbumPriv {}
}

glib::wrapper! {
    pub struct Album(ObjectSubclass<imp::AlbumPriv>);
}


impl Album {
    pub fn new(id: i64, title: String, album_artist: String, artist_id: i64, date: String, genre: String) -> Album {
        let album: Album = glib::Object::builder::<Album>().build();
        album.load_info(id, title, album_artist, artist_id, date, genre);
        album
    }

    pub fn load_info(&self, id: i64, title: String, album_artist: String, artist_id: i64, date: String, genre: String) {
        let imp = self.imp();
        imp.search_string.replace(format!("{} {}", title, album_artist));
        imp.id.set(id);
        imp.title.replace(title);
        imp.album_artist.replace(album_artist);
        imp.date.replace(date);
        imp.genre.replace(genre);
        imp.artist_id.set(artist_id);
        //self.create_menu();
    }

    pub fn add_track(&self, track: Rc<Track>) {
        let imp = self.imp();
        if !self.have_track(&track) {
            let disc_number = track.disc_number() - 1;
            let disc_number = std::cmp::max(0, disc_number);
            imp.track_ids.borrow_mut().insert(track.id());
            let duration = imp.total_duration.get() + track.duration();
            imp.total_duration.set(duration);
            //debug!("ADDED {} to {}", track.title(), self.title.borrow());
            imp.discs.borrow_mut().entry(disc_number).or_default().insert(track.track_number(), track.clone());
        }
    }

    pub fn add_genre(&self, genre_id: i64) {
        self.imp().genre_id.set(Some(genre_id));
    }

    pub fn add_artist(&self, artist_id: i64) {
        self.imp().artist_id.set(artist_id);
    }

    pub fn add_cover_art_id(&self, cover_art_option: Option<i64>) {
        self.imp().cover_art_id.set(cover_art_option);
    }

    pub fn cover_art_option(&self) -> Option<i64> {
        self.imp().cover_art_id.get().clone()
    }

    pub fn cover_art_id(&self) -> Option<i64> {
            self.imp().cover_art_id.get()
    }

    pub fn id(&self) -> i64 {
        self.imp().id.get()
    }

    pub fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }

    pub fn artist(&self) -> String {
        self.imp().album_artist.borrow().clone()
    }

    pub fn date(&self) -> String {
        self.imp().date.borrow().clone()
    }

    pub fn genre(&self) -> String {
        self.imp().genre.borrow().clone()
    }

    pub fn search_string(&self) -> String {
        self.imp().search_string.borrow().clone()
    }

    pub fn duration(&self) -> f64 {
        self.imp().total_duration.get().clone()
    }

    pub fn tracks(&self) -> Vec<Rc<Track>> {
        let mut array = Vec::new();
        let disc_map = self.discs();
        let mut disc_vec: Vec<(&i64, &HashMap<i64, Rc<Track>>)> = disc_map.iter().collect();
        disc_vec.sort_by(|a, b| a.0.cmp(b.0));
        for (_disc_n, disc) in disc_vec {
            let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
            album_vec.sort_by(|a, b| a.0.cmp(b.0));
            for (_track_n, track) in album_vec {
                array.push(track.clone());
            }
        }
        array
    }

    pub fn track_ids(&self) -> Vec<i64> {
        let mut ret = Vec::new();
        for track in self.tracks() {
            ret.push(track.id());
        }
        ret
    }

    fn have_track(&self, track: &Track) -> bool {
        self.imp().track_ids.borrow().contains(&track.id())
    }

    pub fn track_index(&self, track_number: i64, disc_number: i64) -> usize {
        let disc_map = self.discs();
        let mut index = 0;
        for i in 0..disc_number-1 {
            index += disc_map[&i].len();
        }
        index += track_number as usize - 1; 
        index
    }

    pub fn n_tracks(&self) -> usize {
        self.imp().track_ids.borrow().len()
    }

    pub fn discs(&self) -> HashMap<i64, HashMap<i64, Rc<Track>>> {
        self.imp().discs.borrow().clone()
    }

    pub fn disc(&self, disc_no: i64) -> Result<HashMap<i64, Rc<Track>>, String> {
        match self.discs().get(&disc_no) {
            Some(disc) => return Ok(disc.clone()),
            None => return Err("disc not in map".to_string()),
        }
    }

    pub fn n_discs(&self) -> usize {
        self.imp().discs.borrow().len()
    }

    #[allow(dead_code)]
    fn create_menu(&self) {
        let imp = self.imp();
        


        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Play «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.play-album"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Queue", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-album"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);


        imp.menu.append_section(Some("Play"), &menu);


        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Go to Album «{}» Detail", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.go-to-album-detail"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Go to Artist {} Detail", self.artist())), None);
        menu_item.set_action_and_target_value(Some("win.go-to-artist-detail"), Some(&imp.artist_id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Navigate"), &menu);

        let menu = gio::Menu::new();
        
        let menu_item = gio::MenuItem::new(Some(&format!("Create Playlist from «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.create-playlist-from-album"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Playlist", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-album-to-playlist"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Playlist"), &menu);
    }

    pub fn menu_model(&self)-> &gio::Menu {
        &self.imp().menu
    }

}
    