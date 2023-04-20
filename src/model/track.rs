/* track.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::{glib, gio};
use glib::prelude::ToVariant;

use std::{cell::Cell, cell::RefCell};
use regex::Regex;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct TrackPriv {
        pub id: Cell<i64>,
        pub title: RefCell<String>,
        pub album_name: RefCell<String>,
        pub album_artist: RefCell<String>,
        pub search_string: RefCell<String>,
        pub sort_string: RefCell<String>,
        pub sort_title: RefCell<String>,
        pub sort_album: RefCell<String>,
        pub sort_artist: RefCell<String>,
        pub filetype: RefCell<String>,
        pub uri: RefCell<String>,
        pub date: RefCell<String>,
        pub genre: RefCell<String>,
        pub duration: Cell<f64>,
        pub track_number: Cell<i64>,
        pub disc_number: Cell<i64>,
        // artist_parent: RefCell<Option<Artist>>,
        // album_parent: RefCell<Option<Album>>,
        pub genre_id: Cell<Option<i64>>,
        pub album_id: Cell<i64>,
        pub artist_id: Cell<i64>,
        // img_data: RefCell<Option<Vec<u8>>>,
        pub cover_art_id: Cell<Option<i64>>,
        pub menu: gio::Menu,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrackPriv {
        const NAME: &'static str = "Track";
        type Type = super::Track;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for TrackPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl TrackPriv {}
}

glib::wrapper! {
    pub struct Track(ObjectSubclass<imp::TrackPriv>);
}

impl Default for Track {
    fn default() -> Self {
        glib::Object::builder::<Track>().build()
    }
}

impl Track {
    pub fn new(id: i64, title: String, album_name: String, album_artist: String, filetype: String, uri: String, date: String, genre: String, duration: f64, track_number: i64, disc_number: i64) -> Track {
        let track: Track = Self::default();
        track.load_info(id, title, album_name, album_artist, filetype, uri, date, genre, duration, track_number, disc_number);
        track
    }

    pub fn load_info(&self, id: i64, title: String, album_name: String, album_artist: String, filetype: String, uri: String, date: String, genre: String, duration: f64, track_number: i64, disc_number: i64) {
        let imp = self.imp();
        
        let re = Regex::new(r"^(the|a|an)\s+").unwrap();
        
        let lowercase_title = title.to_lowercase();
        let stripped_title = re.replace(&lowercase_title, "");

        let lowercase_artist= album_artist.to_lowercase();
        let stripped_artist = re.replace(&lowercase_artist, "");

        let lowercase_album= album_name.to_lowercase();
        let stripped_album = re.replace(&lowercase_album, "");
        
        imp.sort_title.replace(format!("{}", stripped_title));
        imp.sort_artist.replace(format!("{}", stripped_artist));
        imp.sort_album.replace(format!("{}", stripped_album));

        imp.sort_string.replace(format!("{} {} {}", stripped_title, stripped_album, stripped_artist));
        imp.search_string.replace(format!("{} {} {}", title, album_name, album_artist));

        imp.id.set(id);
        imp.title.replace(title);
        imp.album_name.replace(album_name);
        imp.album_artist.replace(album_artist);
        imp.filetype.replace(filetype);
        imp.uri.replace(uri);
        imp.date.replace(date);
        imp.genre.replace(genre);
        imp.duration.set(duration);
        imp.track_number.set(track_number);
        imp.disc_number.set(disc_number);
        //self.create_menu();
    }
    pub fn add_genre_id(&self, genre_id: i64) {
        self.imp().genre_id.set(Some(genre_id));
    }

    pub fn add_artist_id(&self, artist_id: i64) {
        self.imp().artist_id.set(artist_id);
    }

    pub fn add_album_id(&self, album_id: i64) {
        self.imp().album_id.set(album_id);
    }


    pub fn add_cover_art_option(&self, cover_art_option: Option<i64>) {
        self.imp().cover_art_id.set(cover_art_option);
    }

    pub fn cover_art_option(&self) -> Option<i64> {
        self.imp().cover_art_id.get()
    }

    pub fn id(&self) -> i64 {
        self.imp().id.get().clone()
    }

    pub fn album_id(&self) -> i64 {
        self.imp().album_id.get().clone()
    }

    pub fn artist_id(&self) -> i64 {
        self.imp().artist_id.get().clone()
    }

    pub fn disc_number(&self) -> i64 {
        self.imp().disc_number.get().clone()
    }

    pub fn track_number(&self) -> i64 {
        self.imp().track_number.get().clone()
    }

    pub fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }

    pub fn sort_title(&self) -> String {
        self.imp().sort_title.borrow().clone()
    }

    pub fn uri(&self) -> String {
        self.imp().uri.borrow().clone()
    }

    pub fn date(&self) -> String {
        self.imp().date.borrow().clone()
    }

    pub fn genre(&self) -> String {
        self.imp().genre.borrow().clone()
    }

    pub fn artist(&self) -> String {
        self.imp().album_artist.borrow().clone()
    }

    pub fn sort_artist(&self) -> String {
        self.imp().sort_artist.borrow().clone()
    }

    pub fn album(&self) -> String {
        self.imp().album_name.borrow().clone()
    }

    pub fn sort_album(&self) -> String {
        self.imp().sort_album.borrow().clone()
    }

    pub fn duration(&self) -> f64 {
        self.imp().duration.get().clone()
    }

    pub fn search_string(&self) -> String {
        self.imp().search_string.borrow().clone()
    }

    pub fn sort_string(&self) -> String {
        self.imp().sort_string.borrow().clone()
    }
    
    #[allow(dead_code)]
    fn create_menu(&self) {
        let imp = self.imp();
    
        let menu = gio::Menu::new();
    
        let menu_item = gio::MenuItem::new(Some(&format!("Play «{}»", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.play-track"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);
    
        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Queue", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-track"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);
    
        // let menu_item = gio::MenuItem::new(Some(&format!("Play «{}» from «{}»", self.album(), self.title())), None);
        // menu_item.set_action_and_target_value(Some("win.play-album-from-track"), Some(&imp.id.get().to_variant()));
        // menu.append_item(&menu_item);

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
        menu_item.set_action_and_target_value(Some("win.create-playlist-from-track"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);
    
        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Playlist", self.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-track-to-playlist"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);
    
        imp.menu.append_section(Some("Playlist"), &menu);
    }
    
    pub fn menu_model(&self)-> &gio::Menu {
        &self.imp().menu
    }
}