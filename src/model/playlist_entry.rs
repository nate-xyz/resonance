/* playlist_entry.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use glib::prelude::ToVariant;
use gtk::{glib, gio};
use gtk::subclass::prelude::*;

use std::{cell::Cell, cell::RefCell, rc::Rc};

use super::track::Track;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlaylistEntryPriv {
        pub id: Cell<i64>,
        pub position: Cell<i64>,
        pub playlist_id: Cell<i64>,
        pub track: RefCell<Option<Rc<Track>>>,
        pub menu: gio::Menu,
    }


    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistEntryPriv {
        const NAME: &'static str = "PlaylistEntry";
        type Type = super::PlaylistEntry;
        type ParentType = glib::Object;
    }


    impl ObjectImpl for PlaylistEntryPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl PlaylistEntryPriv {}
}

glib::wrapper! {
    pub struct PlaylistEntry(ObjectSubclass<imp::PlaylistEntryPriv>);
}

impl PlaylistEntry {
    pub fn new(id: i64, position: i64, playlist_id: i64, track: Rc<Track>) -> PlaylistEntry {
        //debug!("new artist {} {}", name, id);
        let artist: PlaylistEntry = glib::Object::builder::<PlaylistEntry>().build();
        artist.load(id, position, playlist_id, track);
        artist
    }

    fn load(&self, id: i64, position: i64, playlist_id: i64, track: Rc<Track>) {
        let imp = self.imp();
        imp.id.set(id);
        imp.position.set(position);
        imp.playlist_id.set(playlist_id);
        imp.track.replace(Some(track.clone()));
        //self.create_menu(track);
    }

    pub fn id(&self) -> i64 {
        self.imp().id.get().clone()
    }

    pub fn position(&self) -> i64 {
        self.imp().position.get().clone()
    }

    pub fn playlist_id(&self) -> i64 {
        self.imp().playlist_id.get().clone()
    }

    pub fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }

    #[allow(dead_code)]
    fn create_menu(&self, track: Rc<Track>) {
        let imp = self.imp();
        
        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Play «{}»", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.play-track"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Queue", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-track-to-queue"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Play Playlist from «{}»", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.play-playlist-from-track"), Some(&imp.id.get().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Play"), &menu);


        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some("Go to Album Detail"), None);
        menu_item.set_action_and_target_value(Some("win.go-to-album-detail"), Some(&track.album_id().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some("Go to Artist Detail"), None);
        menu_item.set_action_and_target_value(Some("win.go-to-artist-detail"), Some(&track.artist_id().to_variant()));
        menu.append_item(&menu_item);


        imp.menu.append_section(Some("Navigate"), &menu);

        let menu = gio::Menu::new();
        
        let menu_item = gio::MenuItem::new(Some(&format!("Create Playlist from «{}»", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.create-playlist-from-track"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Playlist", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-track-to-playlist"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);

        imp.menu.append_section(Some("Playlist"), &menu);
    }

    pub fn menu_model(&self)-> &gio::Menu {
        &self.imp().menu
    }
}
    