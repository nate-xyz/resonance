/* model.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::fmt;
use std::error::Error;
use log::{debug, error};

use crate::database::Database;
use crate::util;

use super::album::Album;
use super::artist::Artist;
use super::cover_art::CoverArt;
use super::genre::Genre;
use super::track::Track;
use super::playlist::Playlist;
use super::artist_image::ArtistImage;

#[derive(Debug)]
struct ModelError(String);
impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "model error: {}", self.0)
    }
}
impl Error for ModelError {}

#[derive(Clone, Debug)]
pub enum ModelAction {
    PopulateAll,
    PopulateArts,
    PopulateTracks,
    PopulateAlbums,
    PopulatePlaylists,
    PopulateArtists,
    PopulateGenres,
    PopulateArt(u64),
    PopulateAlbum(u64),
    PopulateTrack(u64),
    PopulatePlaylist(u64),
    PopulateArtist(u64),
    PopulateGenre(u64),
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug)]
    pub struct ModelPriv {
        pub settings: gio::Settings,
        pub database: RefCell<Option<Rc<Database>>>,
        pub art: RefCell<Option<HashMap<i64, Rc<CoverArt>>>>,
        pub genres: RefCell<Option<HashMap<i64, Rc<Genre>>>>,
        pub artists: RefCell<Option<HashMap<i64, Rc<Artist>>>>,
        pub albums: RefCell<Option<HashMap<i64, Rc<Album>>>>,
        pub tracks: RefCell<Option<HashMap<i64, Rc<Track>>>>,
        pub playlists: RefCell<Option<HashMap<i64, Rc<Playlist>>>>,
        pub artist_images: RefCell<Option<HashMap<i64, Rc<ArtistImage>>>>,
    }
    
    #[glib::object_subclass]
    impl ObjectSubclass for ModelPriv {
        const NAME: &'static str = "Model";
        type Type = super::Model;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                settings: util::settings_manager(),
                database: RefCell::new(None),
                art: RefCell::new(None),
                genres: RefCell::new(None),
                artists: RefCell::new(None),
                albums: RefCell::new(None),
                tracks: RefCell::new(None),
                playlists: RefCell::new(None),
                artist_images: RefCell::new(None),
            }
        }

    }

    impl ObjectImpl for ModelPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("populated").build(),
                    Signal::builder("refresh-genres").build(),
                    Signal::builder("refresh-artists").build(),
                    Signal::builder("refresh-albums").build(),
                    Signal::builder("refresh-tracks").build(),
                    Signal::builder("refresh-playlists").build(),
                    Signal::builder("refresh-plays").build(),
                    Signal::builder("refresh-playlist").param_types([<u64>::static_type()]).build(),
                ]
            });
            
            SIGNALS.as_ref()
        }
    }

    impl ModelPriv {}
}

glib::wrapper! {
    pub struct Model(ObjectSubclass<imp::ModelPriv>);
}

impl Model {
    pub fn new() -> Model {
        let model = glib::Object::builder::<Model>().build();
        model
    }

    fn populate_all(&self) {
        self.reset_all();
        match self.populate() {
            Ok(_) => self.emit_by_name::<()>("populated", &[]),
            Err(e) => {
                error!("Unable to populate model: {}", e);
            },
        }
    }

    fn reset_all(&self) {
        let imp = self.imp();
        imp.art.replace(Some(HashMap::new()));
        imp.artists.replace(Some(HashMap::new()));
        imp.genres.replace(Some(HashMap::new()));
        imp.albums.replace(Some(HashMap::new()));
        imp.tracks.replace(Some(HashMap::new()));
        imp.playlists.replace(Some(HashMap::new()));
        imp.artist_images.replace(Some(HashMap::new()));
    }


    fn populate(&self) -> Result<(), Box<dyn Error>> {
        match self.populate_art() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate art: {}", e),
        }
        
        match self.populate_artists_images() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate artist image: {}", e),
        }

        match self.populate_albums() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate albums: {}", e),
        }
        
        match self.populate_tracks() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate tracks: {}", e),
        }
        
        self.emit_by_name::<()>("refresh-tracks", &[]);

        match self.populate_genres() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate genres: {}", e),
        }
        self.emit_by_name::<()>("refresh-genres", &[]);
        
        match self.populate_artists() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate artists: {}", e),
        }
        self.emit_by_name::<()>("refresh-artists", &[]);

        self.emit_by_name::<()>("refresh-albums", &[]);
    
        match self.populate_playlists() {
            Ok(_) => (),
            Err(e) => error!("Unable to populate playlists: {}", e),
        }
        self.emit_by_name::<()>("refresh-playlists", &[]);

        Ok(())
    }


    fn populate_art(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        
        debug!("populate art");
        let list = self.database().query_art()?;
        if list.is_empty() {
            imp.art.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Art Query Empty".into())));
        }
        let mut art_map = HashMap::new();
        for (id, data) in list {
            let art = Rc::new(CoverArt::new(id, data));
            art_map.insert(id, art);
        }
        imp.art.replace(Some(art_map));
        Ok(())
    }

    fn populate_artists_images(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();

        debug!("populate artist_images");
        let list = self.database().query_artist_images()?;
        if list.is_empty() {
            imp.artists.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Artist Query Empty".into())));
        }
        let mut image_map = HashMap::new();
        for (id, url, data) in list {
            let image = Rc::new(ArtistImage::new(id, url, data));
            image_map.insert(id, image);
        }
        imp.artist_images.replace(Some(image_map));
        Ok(())
    }

    fn populate_genres(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();

        debug!("populate genres");
        let list = self.database().query_genres()?;
        if list.is_empty() {
            imp.genres.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Genre Query Empty".into())));
        }
        let mut genre_map = HashMap::new();
        for (id, name, albums_optional) in list {
            let genre = Rc::new(Genre::new(name.clone(), id.clone()));
            
            if let Some(albums) = albums_optional {
                for album_id in albums {
                    if let Ok(album) = self.album(album_id) {
                        album.add_genre(id);
                        genre.add_album(album.clone());
                    }
                }
            }

            genre_map.insert(id, genre);
        }

        imp.genres.replace(Some(genre_map));
        Ok(())
    }

    fn populate_artists(&self) -> Result<(), Box<dyn Error>> {
        debug!("populate artists");
        let imp = self.imp();

        let list = self.database().query_artists()?;
        if list.is_empty() {
            imp.artists.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Artists Query Empty".into())));
        }
        let mut artist_map = HashMap::new();
        for (id, name, image_optional, albums_optional) in list {
            let artist = Rc::new(Artist::new(name, id, image_optional));

            if let Some(albums) = albums_optional {
                for album_id in albums {
                    if let Ok(album) = self.album(album_id) {
                        album.add_artist(id);
                        artist.add_album(album.clone());
                    }
                }
            }

            artist_map.insert(id, artist);
        }
        
        imp.artists.replace(Some(artist_map));
        Ok(())
    }

    

    fn populate_albums(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        debug!("populate albums");
        let list = self.database().query_albums()?;
        if list.is_empty() {
            imp.albums.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Album Query Empty".into())));
        }
        let mut album_map = HashMap::new();
        for (id, title, album_artist, date, genre, cover_art_option, artist_id) in list
        {
            let album = Rc::new(Album::new(id, title, album_artist, artist_id, date, genre));
            album.add_cover_art_id(cover_art_option);
            album_map.insert(id, album);
        }
        imp.albums.replace(Some(album_map));
        Ok(())
    }

   

    fn populate_tracks(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        debug!("populate tracks");
        let list = self.database().query_tracks()?;
        if list.is_empty() {
            imp.tracks.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Track Query Empty".into())));
        }
        let mut track_map = HashMap::new();
        for (
            id,
            title,
            filetype,
            album_name,
            date,
            duration,
            track_number,
            disc_number,
            album_artist,
            uri,
            artist_id,
            album_id,
            cover_art_option,
        ) in list
        {
            let album = self.album(album_id)?;
            let track = Rc::new(Track::new(
                id,
                title,
                album_name,
                album_artist,
                filetype,
                uri,
                date,
                album.genre(),
                duration,
                track_number,
                disc_number,
            ));
            track.add_artist_id(artist_id);
            track.add_album_id(album_id);

            if cover_art_option.is_none() {
                track.add_cover_art_option(album.cover_art_option());
            } else {
                track.add_cover_art_option(cover_art_option);
            }

            album.add_track(track.clone());
            track_map.insert(id, track);
        }
        imp.tracks.replace(Some(track_map));
        Ok(())
    }

    fn populate_playlists(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        
        debug!("populate playlists");
        let database = self.database();
        
        let list = database.query_playlists()?;
        if list.is_empty() {
            imp.playlists.replace(Some(HashMap::new()));
            return Err(Box::new(ModelError("Playlist Query Empty".into())));
        }

        let mut playlist_map = HashMap::new();
        for (id, title, description, creation_time, modify_time) in list {
            let playlist = Rc::new(Playlist::new(id, title, description, creation_time, modify_time));

            let list = match database.query_playlist_entries_by_playlist_id(id) {
                Ok(list) => list,
                Err(e) => {
                    error!("Error retrieving entries for playlist {}: {}", id, e);
                    continue;
                },
            };
            
            if list.is_empty() {
                debug!("playlist entry list empty for playlist {}", id);
                continue;
            }

            for (id, playlist_position, track_id) in list {
                    let track = self.track(track_id)?;
                    playlist.add_track(id, playlist_position, track);
            }

            playlist_map.insert(id, playlist);
        }
        
        imp.playlists.replace(Some(playlist_map));
        Ok(())
    }
    
    fn populate_playlist_by_id(&self, id: u64) -> Result<(), Box<dyn Error>> {
        debug!("populate playlist {}", id);
        let database = self.database();
        let (playlist_id, title, description, creation_time, modify_time) = database.query_playlist_by_id(id)?;
        let playlist = Rc::new(Playlist::new(playlist_id, title, description, creation_time, modify_time));
        
        let list = match database.query_playlist_entries_by_playlist_id(playlist_id) {
            Ok(list) => list,
            Err(e) => {
                error!("Error retrieving entries for playlist {}: {}", id, e);
                return Err(Box::new(ModelError("Playlist Entries Query Empty".into())))
            },
        };
        
        if !list.is_empty() {
            for (id, playlist_position, track_id) in list {
                let track = self.track(track_id)?;
                playlist.add_track(id, playlist_position, track);
            }
        }

        match self.imp().playlists.borrow_mut().as_mut().unwrap().entry(playlist_id) {
            std::collections::hash_map::Entry::Occupied(o) => {
                *o.into_mut() = playlist;

            },
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(playlist);
            },
        }

        Ok(())
    }



    // fn populate_artists_by_id(&self, id: u64) -> Result<(), Box<dyn Error>> {
    //     debug!("populate artist {}", id);
    //     let (artist_id, name) = self.database().query_artist_by_id(id)?;
    //     let artist = Rc::new(Artist::new(name, artist_id));
    //     match self.imp().artists.borrow_mut().as_mut().unwrap().entry(artist_id) {
    //         std::collections::hash_map::Entry::Occupied(o) => {
    //             *o.into_mut() = artist;

    //         },
    //         std::collections::hash_map::Entry::Vacant(v) => {
    //             v.insert(artist);
    //         },
    //     }

    //     Ok(())
    // }
    fn populate_albums_by_id(&self, id: u64) -> Result<(), Box<dyn Error>> {
        debug!("populate album {}", id);
        let (album_id, title, album_artist, date, genre, cover_art_option, artist_id, genre_option) = self.database().query_album_by_id(id)?;
        let album = Rc::new(Album::new(album_id, title, album_artist, artist_id, date, genre));
        album.add_cover_art_id(cover_art_option);
        if !genre_option.is_none() {
            let genre_id = genre_option.unwrap();
            if let Ok(genre) = self.genre(genre_id) {
                album.add_genre(genre_id);
                genre.add_album(album.clone());
            }
        }
        if let Ok(artist) = self.artist(artist_id) {
            album.add_artist(artist_id);
            artist.add_album(album.clone());
        }
        match self.imp().albums.borrow_mut().as_mut().unwrap().entry(album_id) {
            std::collections::hash_map::Entry::Occupied(o) => {
                *o.into_mut() = album;

            },
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(album);
            },
        }
        Ok(())
    }

    pub fn cover_art(&self, id: i64) -> Result<Rc<CoverArt>, String> {
        match self.imp().art.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(art) => return Ok(art.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn artist_image(&self, id: i64) -> Result<Rc<ArtistImage>, String> {
        match self.imp().artist_images.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(art) => return Ok(art.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn playlist(&self, id: i64) -> Result<Rc<Playlist>, String> {
        match self.imp().playlists.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(playlist) => return Ok(playlist.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn track(&self, id: i64) -> Result<Rc<Track>, String> {
        match self.imp().tracks.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(track) => return Ok(track.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn album(&self, id: i64) -> Result<Rc<Album>, String> {
        match self.imp().albums.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(album) => return Ok(album.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn genre(&self, id: i64) -> Result<Rc<Genre>, String> {
        match self.imp().genres.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(genre) => return Ok(genre.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn artist(&self, id: i64) -> Result<Rc<Artist>, String> {
        match self.imp().artists.borrow().as_ref() {
            Some(map) => match map.get(&id) {
                Some(artist) => return Ok(artist.clone()),
                None => return Err("id not in map".to_string()),
            },
            None => return Err("hashmap does not exist".to_string()),
        }
    }

    pub fn genres(&self) -> Option<HashMap<i64, Rc<Genre>>> {
        self.imp().genres.borrow().as_ref().cloned()
    }

    pub fn artists(&self) -> Option<HashMap<i64, Rc<Artist>>> {
        self.imp().artists.borrow().as_ref().cloned()
    }

    pub fn albums(&self) -> Option<HashMap<i64, Rc<Album>>> {
        self.imp().albums.borrow().as_ref().cloned()
    }

    pub fn tracks(&self) -> Option<HashMap<i64, Rc<Track>>> {
        self.imp().tracks.borrow().as_ref().cloned()
    }

    pub fn art(&self) -> Option<HashMap<i64, Rc<CoverArt>>> {
        self.imp().art.borrow().as_ref().cloned()
    }

    pub fn playlists(&self) -> Option<HashMap<i64, Rc<Playlist>>> {
        self.imp().playlists.borrow().as_ref().cloned()
    }

    pub fn load_database(&self, database: Rc<Database>) {
        self.imp().database.replace(Some(database));
    }

    fn database(&self) -> Rc<Database> {
        self.imp().database.borrow().as_ref().unwrap().clone()
    }

    pub fn process_action(&self, action: ModelAction) -> glib::Continue {
        match action {
            ModelAction::PopulateAll => self.populate_all(),
            ModelAction::PopulateArts => {
                match self.populate_art() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate art: {}", e),
                }
            },
            ModelAction::PopulateTracks => {
                match self.populate_tracks() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate tracks: {}", e),
                }
                self.emit_by_name::<()>("refresh-tracks", &[]);
            },
            ModelAction::PopulateAlbums => {
                match self.populate_albums() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate albums: {}", e),
                }
                self.emit_by_name::<()>("refresh-albums", &[]);
            },
            ModelAction::PopulatePlaylists => {
                match self.populate_playlists() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate playlists: {}", e),
                }
                self.emit_by_name::<()>("refresh-playlists", &[]);
            },
            ModelAction::PopulatePlaylist(id) => {
                match self.populate_playlist_by_id(id) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate playlist: {}", e),
                }
                self.emit_by_name::<()>("refresh-playlist", &[&id]);
            },
            ModelAction::PopulateArtist(id) => {
                match self.populate_albums_by_id(id) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate artist: {}", e),
                }
                //self.emit_by_name::<()>("refresh-artist", &[&id]);
            },
            ModelAction::PopulateArtists => {
                match self.populate_artists_images() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate artists_images: {}", e),
                }
                match self.populate_artists() {
                    Ok(_) => (),
                    Err(e) => error!("Unable to populate artists: {}", e),
                }
                self.emit_by_name::<()>("refresh-artists", &[]);
            },
            _ => debug!("Received action {:?}", action),
        }
        glib::Continue(true)
    }

}
