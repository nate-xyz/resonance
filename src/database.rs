/* database.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;

use gtk::{gio, glib, glib::{clone, Sender, Receiver}, prelude::*};
use gtk_macros::send;

use std::{
    cell::{Cell, RefCell}, 
    collections::{HashMap, HashSet}, 
    time::Duration, 
    error::Error, 
    path::PathBuf, 
    rc::Rc, 
    env, 
    fs, 
    fmt, 
    thread,
};
use rusqlite::{Connection, Result, Transaction, OptionalExtension, params};
use chrono::{DateTime, Utc};
use directories_next::BaseDirs; 
use log::{debug, error};

use crate::model::{track::Track, model::ModelAction};

use super::importer::{Importer, MapVal};
use super::toasts::{add_error_toast, add_success_toast};
use super::i18n::{i18n, i18n_k};
use super::util;

#[derive(Clone, Debug)]
pub enum DatabaseAction {
    TryLoadingDataBase,
    TryAddMusicFolder(PathBuf),
    ConstructFromTags((String, HashMap<String, HashMap<String, HashMap<String, MapVal>>>, HashMap<String, Vec<u8>>)),
    AddArtistImages(Vec<(i64, Option<(String, Vec<u8>)>)>),
    AddArtistImage((i64, Option<(String, Vec<u8>)>)),
    AddArtistImagesDone(u32),
    CreatePlaylist((String, String, Vec<i64>)),
    DuplicatePlaylist((String, String, Vec<i64>)),
    RenamePlaylist((i64, String, String)),
    RemoveDirectory(String),
    DeletePlaylist(i64),
    ChangePlaylistTitleAndOrDescription((i64, Option<String>, Option<String>)),
    AddTracksToPlaylist((i64, String, Vec<i64>)),
    RemoveTrackFromPlaylist(i64),
    ReorderPlaylist((i64, usize, usize))
}

#[derive(Debug)]
struct DatabaseError(String);
impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database error: {}", self.0)
    }
}
impl Error for DatabaseError {}

mod imp {
    use super::*;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean
    };
    use once_cell::sync::Lazy;

    #[derive(Debug)]
    pub struct DatabasePriv {
        pub settings: gio::Settings,
        pub folders: RefCell<HashSet<String>>,
        pub loaded: Cell<bool>,
        pub conn: RefCell<Option<Connection>>,
        pub importer: Importer,
        pub model_sender: Sender<ModelAction>,
        pub model_receiver: RefCell<Option<Receiver<ModelAction>>>,
        pub db_sender: Sender<DatabaseAction>,
        pub db_receiver: RefCell<Option<Receiver<DatabaseAction>>>,
        pub import_start_time: RefCell<Option<DateTime<Utc>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DatabasePriv {
        const NAME: &'static str = "Database";
        type Type = super::Database;
        type ParentType = glib::Object;

        fn new() -> Self {
            let (model_sender, r) = glib::MainContext::channel(glib::PRIORITY_LOW);
            let model_receiver = RefCell::new(Some(r));

            let (db_sender, r) = glib::MainContext::channel(glib::PRIORITY_LOW);
            let db_receiver = RefCell::new(Some(r));

            Self {
                settings: util::settings_manager(),
                folders: RefCell::new(HashSet::new()),
                loaded: Cell::new(false),
                conn: RefCell::new(None),
                importer: Importer::new(),
                model_sender,
                model_receiver,
                db_sender,
                db_receiver,
                import_start_time: RefCell::new(None),
            }
        }
    }

    impl ObjectImpl for DatabasePriv {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecBoolean::builder("loaded").default_value(false).build()
                    ]
                );
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "loaded" => {
                    let loaded_: bool = value.get().expect("The value needs to be of type `bool`.");
                    debug!("setting loaded: {}", loaded_);
                    self.loaded.replace(loaded_);
                }
                _ => unimplemented!(),
            }
        }
    
        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "loaded" => self.loaded.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Database(ObjectSubclass<imp::DatabasePriv>);
}

impl Default for Database {
    fn default() -> Self {
        glib::Object::builder::<Database>().build()
    }
}

impl Database {
    pub fn new() -> Rc<Database> {
        let db: Database = Self::default();
        
        let path = env::current_dir().ok().unwrap();
        debug!("The current directory is {}", path.display());

        db.initialize();
        let database = Rc::new(db);
        database.clone().setup_channel();
        database
    }


    pub fn setup_channel(self: Rc<Self>) {
        let receiver = self.imp().db_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_action(action)),
        );
    }

    pub fn sender(&self) -> Sender<DatabaseAction> {
        self.imp().db_sender.clone()
    }


    fn process_action(&self, action: DatabaseAction) -> glib::Continue {
        match action {
            DatabaseAction::ConstructFromTags((folder, tags, cover_art_bytes)) => {
                match self.construct_from_tags(folder, tags, cover_art_bytes) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to contrust database from tags: {}", e),
                }
            },
            DatabaseAction::TryLoadingDataBase => {
                self.try_loading_database();
            },
            DatabaseAction::TryAddMusicFolder(folder) => {
                match self.try_add_music_folder(folder.clone()) {
                    Ok(_) => {
                        add_success_toast(&i18n("Added Folder:"), &format!("{:?}", folder));
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to add music folder."));
                    },
                } 
            }
            DatabaseAction::AddArtistImages(payload) => {
                let n_amount = payload.len();
                match self.add_artist_image_bulk(payload) {
                    Ok(_) => {
                        // Translators: do not replace {number_of_artists}
                        add_success_toast(&i18n("Added Images:"), &i18n_k("Added {number_of_artists} artist images.", &[("number_of_artists", &format!("{}", n_amount))]));

                    },
                    Err(e) => error!("Unable to add artist images: {}", e),
                }
            },
            DatabaseAction::CreatePlaylist((playlist_title, playlist_desc, track_ids)) => {
                match self.create_playlist(playlist_title.clone(), playlist_desc, track_ids) {
                    Ok(_) => {
                        // Translators: do not replace {playlist_title}
                        add_success_toast(&i18n("Added Playlist:"), &i18n_k("Playlist «{playlist_title}» has been created.", &[("playlist_title", &playlist_title)]))
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to add playlist."));
                    },
                }
            },
            DatabaseAction::DuplicatePlaylist((playlist_title, playlist_desc, track_ids)) => {
                match self.create_playlist(playlist_title, playlist_desc, track_ids) {
                    Ok(_) => {
                        add_success_toast(&i18n("Duplicated:"), &i18n("Duplicated playlist successfully."))
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to duplicate playlist."));
                    },
                }
            },
            DatabaseAction::RenamePlaylist((playlist_id, old_title, new_title)) => {
                match self.rename_playlist(playlist_id, new_title.clone()) {
                    Ok(_) => {
                        // Translators: do not replace {old_title} or {new_title}
                        add_success_toast(&i18n("Renamed:"), &i18n_k("Playlist has been renamed from {old_title} to {new_title}.", &[("old_title", &old_title), ("new_title", &new_title)]))
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to rename playlist."));
                    },
                }
            },
            DatabaseAction::RemoveDirectory(dir_to_remove) => {
                match self.try_remove_folder(dir_to_remove.clone()) {
                    Ok(_) => {
                        // Translators: do not replace {removed_directory}
                        add_success_toast(&i18n("Removed:"), &i18n_k("{removed_directory} has been removed from the database.", &[("removed_directory", &dir_to_remove)]))
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to remove directory."));
                    },
                }
            },
            DatabaseAction::DeletePlaylist(playlist_id) => {
                match self.delete_playlist(playlist_id) {
                    Ok(_) => {
                        debug!("Deleted playlist {}", playlist_id);
                        add_success_toast(&i18n("Deleted:"), &i18n("Removed playlist successfully."))
                    },
                    Err(e) => {
                        error!("Removing playlist error: {}", e);
                        add_error_toast(i18n("Unable to remove playlist."));
                    },
                }
            },
            DatabaseAction::ChangePlaylistTitleAndOrDescription((playlist_id, title_option, description_option)) => {
                match self.change_playlist_title_and_or_description(playlist_id, title_option, description_option) {
                    Ok(_) => {
                        debug!("Deleted playlist {}",playlist_id);
                        add_success_toast(&i18n("Modified:"), &i18n("Changed playlist successfully."));
                    },
                    Err(e) => {
                        error!("Modifying playlist error: {}", e);
                        add_error_toast(i18n("Unable to modify playlist."));
                    },
                }
            },
            DatabaseAction::AddTracksToPlaylist((playlist_id, playlist_title, tracks)) => {
                match self.add_tracks_to_playlist(playlist_id, tracks) {
                    Ok(_) => {
                        add_success_toast(&i18n("Added:"), &i18n_k("Added tracks to {playlist_title}.", &[("playlist_title", &playlist_title)]))
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to add track to playlist."));
                    },
                }
            },
            DatabaseAction::RemoveTrackFromPlaylist(playlist_entry_id) => {
                match self.remove_track_from_playlist(playlist_entry_id) {
                    Ok(_) => {
                        debug!("removed playlist_entry from playlist");
                        add_success_toast(&i18n("Removed:"), &i18n("Removed track from playlist."));
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to remove track from playlist."));
                    },
                }
            },
            DatabaseAction::ReorderPlaylist((playlist_id, old_position, new_position)) => {
                match self.reorder_playlist(playlist_id, old_position, new_position) {
                    Ok(_) => {
                        debug!("reordered playlist");
                    },
                    Err(e) => {
                        error!("{}", e);
                        add_error_toast(i18n("Unable to reorder playlist."));
                    },
                }
            },
            _ => debug!("Received action {:?}", action),
        }

        glib::Continue(true)
    }

    fn initialize(&self) {
        self.setup_settings();
        self.get_dirs();
    }

    fn setup_settings(&self) {
        let imp = self.imp();

        imp.settings.connect_changed(
            Some("music-folders"),
            glib::clone!(@strong self as this => move |_settings, _name| {
                debug!("database -> music folders has changed");
                let imp = this.imp();
                
                this.get_dirs();

                if !imp.loaded.get() {
                    this.try_loading_database();
                }

                let dirs = imp.settings.strv("music-folders").to_vec();
                let mut set = HashSet::new();
                for s in &dirs {
                    set.insert(s.to_string());
                }

                debug!(" new dirs {:?}", set);

                send!(imp.model_sender, ModelAction::PopulateAll);
            }),
        );
    }

    fn get_dirs(&self) {
        let imp = self.imp();

        let dirs = imp.settings.strv("music-folders").to_vec();
        let mut set = HashSet::new();

        for s in &dirs {
            set.insert(s.to_string());
        }

        debug!("music-folders = {:?}", dirs);

        imp.folders.replace(set);
    }

    pub fn try_loading_database(&self) {
        let imp = self.imp();

        if let Ok(_) = self.open_connection_to_db() {
            let is_folders = imp.folders.borrow().is_empty().clone();
            if !is_folders { // there are folders loaded
                match self.verify_music_folders() {
                    Ok(_) => {
                        //self.emit_by_name::<()>("populate-model", &[]);
                        send!(imp.model_sender, ModelAction::PopulateAll);
                        debug!("Folder verified & loaded");
                        self.set_property("loaded", true.to_value());
                        return
                    },
                    Err(e) => {
                        error!("Unable to load music folders ... {}", e);
                        add_error_toast(i18n("Unable to load music folders."));
                    },
                }

            } else {
                debug!("No folders, but opened connection to db.")
            }
        } else {
            error!("Unable to open connection to database.");
        }
        self.set_property("loaded", false.to_value());

        // if !imp.folders.borrow().is_empty() { // there are folders loaded
        //     if self.open_connection_to_db() {
        //         self.emit_by_name::<()>("populate-model", &[]);
        //         imp.loaded.set(true);
        //     } else {
        //         error!("unable to open connection to database");
        //         imp.loaded.set(false);
        //     }
        // } else {
        //     debug!("no music folders.");
        //     imp.loaded.set(false);
        //     return;
        // }

    }

    fn check_loaded(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Music_Folders LIMIT 1);")?;
        let exists: i32 = stmt.query_row([], |row| row.get(0))?;
        
        if exists == 0 {
            debug!("no folders loaded");
            self.set_property("loaded", false.to_value());
            util::player().stop();
        }

        Ok(())
    }

    fn verify_music_folders(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        let folders = imp.folders.borrow().clone();
        for dir in folders.iter() {
    
            if !self.check_if_folder_exists(&tx,dir.to_string())? {
                self.importer().extract_folder(dir.to_string(), imp.db_sender.clone());
            }
        }

        tx.commit()?;
        
        Ok(())
    }

    fn open_connection_to_db(&self) -> Result<(), Box<dyn Error>> {
        let path = self.database_location();
        debug!("database path: {:?}", path);

        let conn = match Connection::open_with_flags(path.clone(), rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE) {
            Ok(c) => {
                debug!("open database by existing uri");
                c
            },
            Err(_) => {
                debug!("open new database");
                Connection::open(path).unwrap()
            }
        };

        let result = conn.query_row("SELECT sqlite_source_id()", params![], |row| row.get(0));
        
        debug!("sqlite_source_id: {}", result.unwrap_or("Error".to_string()));

        *self.imp().conn.borrow_mut() = Some(conn);
        //self.imp().conn.replace(Some(conn));

        self.setup_db_tables()?;
        self.check_new_artists_and_import()?;
        Ok(())
    }




    // rusqlite::OpenFlags::SQLITE_OPEN_CREATE
    // fn open_connection_to_db(&self) -> bool {
    //     let path = self.database_location();
    //     let mut new = false;
    //     let conn = match Connection::open_with_flags(
    //         path.clone(),
    //         rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
    //     ) {  
    //         Ok(c) => {
    //             debug!("open database by existing uri");
    //             c
    //         }
    //         Err(_) => {
    //             debug!("open NEW database");
    //             new = true;
    //             Connection::open(path).unwrap()
    //         }
    //     };

    //     let result = conn.query_row("SELECT sqlite_source_id()", params![], |row| row.get(0));
    //     debug!("sqlite_source_id: {}", result.unwrap_or("Error".to_string()));

    //     *self.imp().conn.borrow_mut() = Some(conn);
    //     new
    // }

    fn database_location(&self) -> PathBuf {
        if let Some(base_dirs) = BaseDirs::new() {
            let folder = base_dirs.data_dir().join("io.github.nate_xyz.Resonance");
            fs::create_dir_all(folder.clone()).unwrap();
            folder.join("resonance.db")
        } else {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("io.github.nate_xyz.Resonance").unwrap();
            match xdg_dirs.place_data_file("resonance.db") {
                Ok(path) => {
                    let folder = xdg_dirs.get_data_home();
                    fs::create_dir_all(folder).unwrap();
                    path
                },
                Err(_) => {
                    PathBuf::from("resonance.db")
                }
            }
                          
        }
    }

    //Extract tags from music folder with mutagen
    pub fn try_add_music_folder(&self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        let path_str = path.into_os_string().into_string().ok().unwrap();
        imp.import_start_time.replace(Some(chrono::offset::Utc::now()));
        self.importer().extract_folder(path_str, imp.db_sender.clone());
        Ok(())
    }

    //Called after importer extracts tags with mutagen
    fn construct_from_tags(&self, folder: String, tags: HashMap<String, HashMap<String, HashMap<String, MapVal>>>, bytes: HashMap<String, Vec<u8>>)  -> Result<(), Box<dyn Error>>{
        let imp = self.imp();
        let new_artists = {
            let mut conn = imp.conn.borrow_mut();
            let conn = conn.as_mut().ok_or("Connection not established")?;
    
            let tx = conn.transaction()?;
            self.importer().build_database_from_tags(&tx, folder.clone(), tags, bytes)?;
            let new_artists = self.new_artists(&tx)?;
            
            tx.commit()?;

            new_artists
        };

        //UPDATE SETTINGS
        let mut folders = imp.folders.borrow_mut().clone();
        folders.insert(folder);
        let music_folders = folders.into_iter().collect::<Vec<String>>();
        imp.settings.set_strv("music-folders", music_folders.as_slice())?;


        //UPDATE ARTISTS ART
        if !new_artists.is_empty() {
            debug!("Adding new artists ...");

            let sender = imp.db_sender.clone();

            thread::spawn(move || {
                match fetch_artist_images_bulk(new_artists, sender.clone()) {
                    Ok(()) => debug!("Added all artist images!"),
                    Err(e) => error!("Adding artist images error: {}", e),
                }
            });
        }

        if let Some(start) = imp.import_start_time.take() {
            let end = chrono::offset::Utc::now();
            let duration = end.signed_duration_since(start);
            if duration.num_seconds() < 60 {
                debug!("Import time elapsed: {}", duration.num_seconds())
            } else {
                debug!("Import elapsed: {} minutes", duration.num_minutes())
            }   
        }

        Ok(())
    }

    //Checks if new artists exist that have not been fetched yet (for image scrape)
    fn check_new_artists_and_import(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        let mut conn = imp.conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        let new_artists = self.new_artists(&tx)?;
        tx.commit()?;

        //UPDATE ARTISTS ART
        if !new_artists.is_empty() {
            debug!("Adding new artists ...");

            let sender = imp.db_sender.clone();

            thread::spawn(move || {
                match fetch_artist_images_bulk(new_artists, sender.clone()) {
                    Ok(()) => debug!("Added all artist images!"),
                    Err(e) => error!("Adding artist images error: {}", e),
                }
            });
        } else {
            debug!("No new artists");
        }

        Ok(())
    }

    fn new_artists(&self, tx: &Transaction) -> Result<Vec<(i64, String)>, Box<dyn Error>> {
        let mut stmt = tx.prepare("SELECT * FROM Artists WHERE Artists.image_fetched=0 AND NOT EXISTS(SELECT 1 FROM Artist_Discog_Artist_Image_Junction WHERE Artists.id = Artist_Discog_Artist_Image_Junction.artist_id);")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id, name))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }


    /*
    ADDING VALUES TO TABLES
    */

    pub fn add_folder(&self, tx: &Transaction, uri: String) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Music_Folders (uri) VALUES ( ? );")?;
        stmt.execute(params![uri])?;
        Ok(tx.last_insert_rowid())
    }

    pub fn add_genre(&self, tx: &Transaction, name: String) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Genres (name) VALUES ( ? );")?;
        stmt.execute(params![name])?;
        Ok(tx.last_insert_rowid())
    }

    pub fn add_artist(&self, tx: &Transaction, name: String) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Artists ( name, image_fetched ) VALUES ( ?, ? );")?;
        stmt.execute(params![name, 0])?;
        Ok(tx.last_insert_rowid())
    }

    pub fn add_cover_art(&self, tx: &Transaction, data: &[u8]) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Cover_Art (data, thing) VALUES ( ?, ? );")?;
        stmt.execute(params![data, 0])?;
        Ok(tx.last_insert_rowid())
    }

    //ADD ALBUM
    pub fn add_album_full(&self, tx: &Transaction, 
        title: String, artist: String, date: String, genre: String,
        cover_art_id: Option<i64>, album_artist_id: i64, genre_id: Option<i64>) -> Result<i64, Box<dyn Error>> {
        
        let album_id = self.add_album(&tx, title, artist, date, genre)?;
        self.add_album_artist_junction(&tx, album_id, album_artist_id)?;

        if let Some(id) = cover_art_id {
            self.add_album_cover_art_junction(&tx, album_id, id)?;
        }

        if let Some(id) = genre_id {
            self.add_album_genre_junction(&tx, album_id, id)?;
        }

        Ok(album_id)
    }

    fn add_album(&self, tx: &Transaction, 
        title: String, artist: String, date: String, genre: String) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Albums (title, artist, date, genre) VALUES ( ?, ?, ?, ? );")?;
        stmt.execute(params![
            title, artist, date, genre
        ])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_album_artist_junction(&self, tx: &Transaction, 
        album_id: i64, artist_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Album_Artist_Junction (album_id, artist_id) VALUES ( ?, ? );")?;
        stmt.execute(params![album_id, artist_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_album_cover_art_junction(&self, tx: &Transaction, 
        album_id: i64, cover_art_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Album_Cover_Art_Junction (album_id, cover_art_id) VALUES ( ?, ? );")?;
        stmt.execute(params![album_id, cover_art_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_album_genre_junction(&self, tx: &Transaction, 
        album_id: i64, genre_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Album_Genre_Junction (album_id, genre_id) VALUES ( ?, ? );")?;
        stmt.execute(params![
            album_id, genre_id
        ])?;
        Ok(tx.last_insert_rowid())
    }

    //ADD TRACK

    pub fn add_track_full(&self, tx: &Transaction, 
        title: String, filetype: String, album_name: String, album_artist: String, date: String, 
        duration: f32, track_number: u32, disc_number: u32,  
        album_id: i64, album_artist_id: i64, cover_art_id: Option<i64>,
        file_uri: String, last_modified: DateTime<Utc>, folder_id: i64, 
    ) -> Result<i64, Box<dyn Error>> {
        
        let file_id = self.add_file_uri(&tx,file_uri, last_modified, folder_id)?;
        let track_id = self.add_track(&tx, title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_id)?;
        
        self.add_track_album_junction(&tx, track_id, album_id)?;
        self.add_track_artist_junction(&tx, track_id, album_artist_id)?;
        self.add_track_folder_junction(&tx, track_id, folder_id)?;

        if let Some(id) = cover_art_id {
            self.add_track_cover_art_junction(&tx, track_id, id)?;
        }

        Ok(track_id)
    }

    fn add_file_uri(&self, tx: &Transaction, file_uri: String, last_modified: DateTime<Utc>, folder_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO File_URIs (uri, last_modified) VALUES ( ?, ? );")?;
        stmt.execute(params![file_uri, last_modified.timestamp()])?;
        let file_id = tx.last_insert_rowid();

        let mut stmt = tx.prepare("INSERT INTO Folder_File_Junction (folder_id, file_id) VALUES ( ?, ? );")?;
        stmt.execute(params![folder_id, file_id])?;

        Ok(file_id)
    }

    fn add_track(&self, tx: &Transaction, 
        title: String, filetype: String, album_name: String, date: String, duration: f32, 
        track_number: u32, disc_number: u32,  album_artist: String,  file_uri_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Tracks (title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_uri_id) VALUES ( ?, ?, ?, ?, ?, ?, ?, ?, ? );")?;
        stmt.execute(params![title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_uri_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_track_folder_junction(&self, tx: &Transaction, 
        track_id: i64, folder_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Track_Folder_Junction (track_id, folder_id) VALUES ( ?, ? );")?;
        stmt.execute(params![track_id, folder_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_track_album_junction(&self, tx: &Transaction, 
        track_id: i64, album_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Track_Album_Junction (track_id, album_id) VALUES ( ?, ? );")?;
        stmt.execute(params![track_id, album_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_track_artist_junction(&self, tx: &Transaction, 
        track_id: i64, artist_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Track_Artist_Junction (track_id, artist_id) VALUES ( ?, ? );")?;
        stmt.execute(params![track_id, artist_id])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_track_cover_art_junction(&self, tx: &Transaction, 
        track_id: i64, cover_art_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Track_Cover_Art_Junction (track_id, cover_art_id) VALUES ( ?, ? );")?;
        stmt.execute(params![track_id, cover_art_id])?;
        Ok(tx.last_insert_rowid())
    }


    // ADD PLAYLIST
    fn create_playlist(&self, title: String, description: String, tracks: Vec<i64>) -> Result<(), Box<dyn Error>> {       
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;


        let playlist_id  =  self.add_playlist(&tx, title, description)?;

        for (position, track_id) in tracks.iter().enumerate() {
            self.add_track_to_playlist(&tx, playlist_id, position as i64, *track_id)?;
        }

        tx.commit()?;
        send!(self.imp().model_sender, ModelAction::PopulatePlaylists);
        Ok(())
    }

    fn add_playlist(&self, tx: &Transaction, title: String, description: String) -> Result<i64, Box<dyn Error>> {
        debug!("create_playlist");
        let mut stmt = tx.prepare("INSERT INTO PLAYLISTS (title, description, creation_time, modify_time) VALUES ( ?, ?, ?, ? );")?;
        let creation_time = chrono::offset::Utc::now();
        let modify_time = chrono::offset::Utc::now();
        stmt.execute(params![
            title, 
            description,
            creation_time,
            modify_time,
        ])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_track_to_playlist(&self, tx: &Transaction, playlist_id: i64, playlist_position: i64, track_id: i64) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Playlist_Entries (playlist_position) VALUES ( ? );")?;
        stmt.execute(params![playlist_position])?;
        
        let playlist_entry_id = tx.last_insert_rowid();

        let mut stmt = tx.prepare("INSERT INTO Playlist_Entry_Track_Junction (playlist_entry_id, track_id) VALUES ( ?, ? );")?;
        stmt.execute(params![playlist_entry_id, track_id])?;

        let mut stmt = tx.prepare("INSERT INTO Playlist_Entry_Playlist_Junction (playlist_entry_id, playlist_id) VALUES ( ?, ? );")?;
        stmt.execute(params![playlist_entry_id, playlist_id])?;

        Ok(())
    }

    pub fn add_tracks_to_playlist(&self, playlist_id: i64, tracks: Vec<i64>) -> Result<(), Box<dyn Error>> {
        debug!("add_tracks_to_playlist");

        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        if !self.check_if_playlist_exists(&tx, playlist_id)? {
            add_error_toast(i18n("Tried to add track(s) to playlist that does not exist."));
            return Ok(());
        }

        let statements = |tx: &Transaction| -> Result<(), Box<dyn Error>> {
            let mut stmt = tx.prepare("SELECT count( * ) FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = ( ? );")?;
            let number_of_playlist_members: i64 = stmt.query_row([playlist_id], |row| row.get(0))?;
    
            for (i, track_id) in tracks.iter().enumerate() {
                self.add_track_to_playlist(tx, playlist_id, number_of_playlist_members+i as i64, *track_id)?;
            }

            Ok(())
        };

        statements(&tx)?;

        self.update_playlist_modify_time(&tx, playlist_id)?;

        tx.commit()?;
        //self.emit_by_name::<()>("populate-model", &[]);
        Ok(())
    }

    fn add_artist_image_bulk(&self, payload: Vec<(i64, Option<(String, Vec<u8>)>)>) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        let mut conn = imp.conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        for (artist_id, image_option) in payload {
            if let Some((url, data)) = image_option {
                self.add_artist_image_full(&tx, artist_id, url, &data)?;
            } else {
               self.set_artist_image_fetched(&tx, artist_id, true)?;
            }
            //self.add_artist_image_full(&tx, artist_id, image_url, image_data.as_slice())?;
        }
        tx.commit()?;

        send!(self.imp().model_sender, ModelAction::PopulateArtists);

        Ok(())
    }

    //ADD ARTIST IMAGE

    fn add_artist_image_full(&self, tx: &Transaction, artist_id: i64, url: String, data: &[u8]) -> Result<i64, Box<dyn Error>> {
        let image_id = self.add_artist_image(&tx, url, data)?;
        self.add_artist_discog_image_junction(&tx, artist_id, image_id)?;
        Ok(image_id)
    }

    fn set_artist_image_fetched(&self, tx: &Transaction, artist_id: i64, fetched: bool) -> Result<(), Box<dyn Error>> {
        //SET FETCHED 
        let statements = |tx: &Transaction| -> Result<(), Box<dyn Error>> {
            let mut stmt = if fetched {
                tx.prepare("UPDATE Artists SET image_fetched = 1 WHERE id = ( ? );")?
            } else {
                tx.prepare("UPDATE Artists SET image_fetched = 0 WHERE id = ( ? );")?
            };
            // let mut stmt = tx.prepare("UPDATE Artists SET image_fetched = 1 WHERE id = ( ? );")?;
            stmt.execute(params![artist_id])?;
            Ok(())
        };

        statements(&tx)?;
        Ok(())
    }
    
    fn add_artist_image(&self, tx: &Transaction, url: String, data: &[u8]) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Discog_Artist_Image (url, data, thing) VALUES ( ?, ?, ? );")?;
        stmt.execute(params![url, data, 0])?;
        Ok(tx.last_insert_rowid())
    }

    fn add_artist_discog_image_junction(&self, tx: &Transaction, 
        artist_id: i64, image_id: i64) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("INSERT INTO Artist_Discog_Artist_Image_Junction (artist_id, image_id) VALUES ( ?, ? );")?;
        stmt.execute(params![artist_id, image_id])?;
        Ok(tx.last_insert_rowid())
    }

    // fn add_artist_image_full_conn(&self, artist_id: i64, url: String, data: &[u8]) -> Result<i64, Box<dyn Error>> {
    //     let mut conn = self.imp().conn.borrow_mut();
    //     let conn = conn.as_mut().ok_or("Connection not established")?;
    //     let tx = conn.transaction()?;

    //     let image_id = self.add_artist_image(&tx, url, data)?;
    //     self.add_artist_discog_image_junction(&tx, artist_id, image_id)?;
        
    //     //SET FETCHED 
    //     let statements = |tx: &Transaction| -> Result<(), Box<dyn Error>> {
    //         let mut stmt = tx.prepare("UPDATE Artists SET image_fetched = 1 WHERE id = ( ? );")?;
    //         stmt.execute(params![artist_id])?;
    //         Ok(())
    //     };

    //     statements(&tx)?;

    //     tx.commit()?;
    //     Ok(image_id)
    // }

    //ADD PLAY
    pub fn add_play(&self, track: Rc<Track>, datetime_stamp: DateTime<Utc>) -> Result<(), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established add_play")?;
        let mut stmt = conn.prepare("INSERT INTO PLAYS (playtime, track_id, album_id, album_artist_id) VALUES ( ?, ?, ?, ? );")?;
        stmt.execute(params![
            datetime_stamp.timestamp(), 
            track.id(),
            track.album_id(),
            track.artist_id(),
        ])?;
        Ok(())

    }

    /*
    QUERIES
    */

    pub fn query_artist_images(&self) -> Result<Vec<(i64, String, Vec<u8>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        
        let mut stmt = conn.prepare("SELECT id, url, data FROM Discog_Artist_Image;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let url: String = row.get(1)?;
            let data: Vec<u8> = row.get(2)?;
            Ok((id, url, data))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    fn check_if_folder_exists(&self, tx: &Transaction, music_folder: String) -> Result<bool, Box<dyn Error>>{
        let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Music_Folders WHERE uri = (?)  LIMIT 1);")?;
        let exists: bool = match stmt.query_row([music_folder], |row| row.get::<usize, i64>(0)) {
            Ok(count) => {
                count > 0
            },
            Err(e) => return Err(Box::new(e)),
        };

        Ok(exists)
    }

    fn check_if_playlist_exists(&self, tx: &Transaction, playlist_id: i64) -> Result<bool, Box<dyn Error>>{
        let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Playlists WHERE id = (?)  LIMIT 1);")?;
        let exists: bool = match stmt.query_row([playlist_id], |row| row.get::<usize, i64>(0)) {
            Ok(count) => {
                count > 0
            },
            Err(e) => return Err(Box::new(e)),
        };

        Ok(exists)
    }

    // Used in the import process if a track has no track number & no album artist tags, to add to the Unknown Album
    pub fn query_orphan_tracks(&self, tx: &Transaction) -> Result<i64, Box<dyn Error>> {
        let mut stmt = tx.prepare("SELECT count( * ) FROM tracks WHERE album_name = \"Unknown Album\" AND album_artist = \"Unknown Artist\";")?;
        let count = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }



    // MODEL POPULATION QUERIES

    // Used in model population
    pub fn query_playlists(&self) -> Result<Vec<(i64, String, String, DateTime<Utc>, DateTime<Utc>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM playlists;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let creation_time: DateTime<Utc> = row.get(3)?;
            let modify_time: DateTime<Utc> = row.get(4)?;
            Ok((id, title, description, creation_time, modify_time))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn query_playlist_by_id(&self, id: u64) -> Result<(i64, String, String, DateTime<Utc>, DateTime<Utc>), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM playlists WHERE id = (?);")?;
        let rows = stmt.query_map([id], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let creation_time: DateTime<Utc> = row.get(3)?;
            let modify_time: DateTime<Utc> = row.get(4)?;
            Ok((id, title, description, creation_time, modify_time))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        let result = result.remove(0);

        Ok(result)
    }

    // Used in model population
    pub fn query_playlist_entries(&self) -> Result<Vec<(i64, i64, i64, i64)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        
        let mut stmt = conn.prepare("SELECT * FROM playlist_entries;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let playlist_position: i64 = row.get(1)?;
            Ok((id, playlist_position))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (playlist_entry_id, playlist_position) = row?;

            let mut stmt = conn.prepare("SELECT playlist_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = (?);")?;
            let playlist_id: i64 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

            let mut stmt = conn.prepare("SELECT track_id FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = (?);")?;
            let track_id: i64 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

            result.push((playlist_entry_id, playlist_id, playlist_position, track_id));
        }

        Ok(result)
    }

    pub fn query_playlist_entries_by_playlist_id(&self, playlist_id: i64) -> Result<Vec<(i64, i64, i64)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        
        let mut stmt = conn.prepare("SELECT playlist_entry_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?);")?;
        let rows = stmt.query_map([playlist_id], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;

        let mut result = Vec::new();
        for row in rows {
            let playlist_entry_id = row?;

            let mut stmt = conn.prepare("SELECT playlist_position FROM Playlist_Entries WHERE id = (?);")?;
            let playlist_position: i64 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

            let mut stmt = conn.prepare("SELECT track_id FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = (?);")?;
            let track_id: i64 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

            result.push((playlist_entry_id, playlist_position, track_id));
        }

        Ok(result)
    }

     // Used in model population
    pub fn query_art(&self) -> Result<Vec<(i64, Vec<u8>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;

        let mut stmt = conn.prepare("SELECT id, data FROM cover_art;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((id, data))
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn query_art_by_id(&self, id: u64) -> Result<(i64, Vec<u8>), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;

        let mut stmt = conn.prepare("SELECT data FROM cover_art WHERE id = (?);")?;
        let data: Vec<u8> = stmt.query_row([id], |row| row.get(0))?;

        Ok((id as i64, data))
    }

     // Used in model population
    pub fn query_genres(&self) -> Result<Vec<(i64, String, Option<Vec<i64>>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM genres;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id, name))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (genre_id, genre_name) = row?;

            let mut albums = Vec::new();
            let mut stmt = conn.prepare("SELECT album_id from Album_Genre_Junction WHERE genre_id = (?);")?;
            let album_rows = stmt.query_map([genre_id], |row| {
                let album_id: i64 = row.get(0)?;
                Ok(album_id)
            })?;

            for r in album_rows {
                let album_id = r?;
                albums.push(album_id);
            }
            
            if albums.is_empty() {
                result.push((genre_id, genre_name, None));
            } else {
                result.push((genre_id, genre_name, Some(albums)));
            }
        }

        Ok(result)
    }

    pub fn query_genre_by_id(&self, id: u64) -> Result<(i64, String), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;

        let mut stmt = conn.prepare("SELECT name FROM Genres WHERE id = (?);")?;
        let name: String = stmt.query_row([id], |row| row.get(0))?;

        Ok((id as i64, name))
    }

    // Used in model population
    pub fn query_artists(&self) -> Result<Vec<(i64, String, Option<i64>, Option<Vec<i64>>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM Artists;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id, name))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (artist_id, artist_name) = row?;
            let mut stmt = conn.prepare("SELECT image_id FROM Artist_Discog_Artist_Image_Junction WHERE artist_id = (?);")?;
            let image_id: Option<i64> = stmt.query_row([artist_id], |row| row.get(0)).optional()?;
            
            let mut albums = Vec::new();
            let mut stmt = conn.prepare("SELECT album_id from Album_Artist_Junction WHERE artist_id = (?);")?;
            let album_rows = stmt.query_map([artist_id], |row| {
                let album_id: i64 = row.get(0)?;
                Ok(album_id)
            })?;

            for r in album_rows {
                let album_id = r?;
                albums.push(album_id);
            }
            
            if albums.is_empty() {
                result.push((artist_id, artist_name, image_id, None));
            } else {
                result.push((artist_id, artist_name, image_id, Some(albums)));
            }
        }

        Ok(result)
    }

    pub fn query_artist_by_id(&self, id: u64) -> Result<(i64, String), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;

        let mut stmt = conn.prepare("SELECT name FROM Artists WHERE id = (?);")?;
        let name: String = stmt.query_row([id], |row| row.get(0))?;

        Ok((id as i64, name))
    }

     // Used in model population
    pub fn query_albums(&self) -> Result<Vec<(i64, String, String, String, String, Option<i64>, i64)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM albums;")?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let album_artist: String = row.get(2)?;
            let date: String = row.get(3)?;
            let genre: String = row.get(4)?;

            Ok((id, title, album_artist, date, genre))
        })?;


        let mut result = Vec::new();
        for row in rows {
            let (album_id, title, artist, date, genre) = row?;

            let mut stmt = conn.prepare("SELECT artist_id FROM Album_Artist_Junction WHERE album_id = ?;")?;
            let album_artist_id: i64 = stmt.query_row([album_id], |row| row.get(0))?;

            let mut stmt = conn.prepare("SELECT cover_art_id FROM Album_Cover_Art_Junction WHERE album_id = ?;")?;
            let cover_art_id: Option<i64> = stmt.query_row([album_id], |row| row.get(0)).optional()?;
                          
            result.push((album_id, title, artist, date, genre, cover_art_id, album_artist_id));
        }

        Ok(result)
    }

    pub fn query_album_by_id(&self, id: u64) -> Result<(i64, String, String, String, String, Option<i64>, i64, Option<i64>), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM Albums WHERE id = (?);")?;
        let rows = stmt.query_map([id], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let album_artist: String = row.get(2)?;
            let date: String = row.get(3)?;
            let genre: String = row.get(4)?;
            Ok((id, title, album_artist, date, genre))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (album_id, title, artist, date, genre) = row?;

            let mut stmt = conn.prepare("SELECT artist_id FROM Album_Artist_Junction WHERE album_id = ?;")?;
            let album_artist_id: i64 = stmt.query_row([album_id], |row| row.get(0))?;

            let mut stmt = conn.prepare("SELECT cover_art_id FROM Album_Cover_Art_Junction WHERE album_id = ?;")?;
            let cover_art_id: Option<i64> = stmt.query_row([album_id], |row| row.get(0)).optional()?;
            
            let mut stmt = conn.prepare("SELECT genre_id FROM Album_Genre_Junction WHERE album_id = ?;")?;
            let genre_id: Option<i64> = stmt.query_row([album_id], |row| row.get(0)).optional()?;
                
            result.push((album_id, title, artist, date, genre, cover_art_id, album_artist_id, genre_id));
        }

        let result = result.remove(0);

        Ok(result)
    }


    // Used in model population
    pub fn query_tracks(&self) -> Result<Vec<(i64, String, String, String, String, f64, i64, i64, String, String, i64, i64, Option<i64>)>, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        let mut stmt = conn.prepare("SELECT * FROM tracks;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let filetype: String = row.get(2)?;
            let album_name: String = row.get(3)?;
            let date: String = row.get(4)?;
            let duration: f64 = row.get(5)?;
            let track_number: i64 = row.get(6)?;
            let disc_number: i64 = row.get(7)?;
            let album_artist: String = row.get(8)?;
            let file_uri_id: i64 = row.get(9)?;
            Ok((id, title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_uri_id))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (track_id, title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_uri_id) = row?;

            let mut stmt = conn.prepare("SELECT album_id FROM Track_Album_Junction WHERE track_id = ?;")?;
            let album_id: i64 = stmt.query_row([track_id], |row| row.get(0))?;
            
            let mut stmt = conn.prepare("SELECT artist_id FROM Track_Artist_Junction WHERE track_id = ?;")?;
            let artist_id: i64 = stmt.query_row([track_id], |row| row.get(0))?;

            let mut stmt = conn.prepare("SELECT cover_art_id FROM Track_Cover_Art_Junction WHERE track_id = ?;")?;
            let cover_art_id: Option<i64> = stmt.query_row([track_id], |row| row.get(0)).optional()?;

            let mut stmt = conn.prepare("SELECT uri FROM File_URIs WHERE id = ?")?;
            let uri: String = stmt.query_row([file_uri_id], |row| row.get(0))?;

            result.push((track_id, title, filetype, album_name, date, duration, track_number, disc_number, album_artist, uri, artist_id, album_id, cover_art_id));
        }

        Ok(result)
    }

    //Used in naming a new playlist
    pub fn query_n_playlists(&self, playlist_id: Option<i64>) -> Result<i64, Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let conn = conn.as_ref().ok_or("Connection not established")?;
        
        match playlist_id {
            Some(playlist_id) => {
                let mut stmt = conn.prepare("SELECT count( * ) FROM playlists WHERE id = (?);")?;
                let count = stmt.query_row([playlist_id], |row| row.get::<usize, i64>(0))?;
                return Ok(count);
            },
            None => {
                let mut stmt = conn.prepare("SELECT count( * ) FROM playlists;")?;
                let count = stmt.query_row([], |row| row.get::<usize, i64>(0))?;
                return Ok(count);
            },
        }
    }

    //Used in the home page
    // pub fn query_most_recent_tracks(&self) -> Result<Vec<i64>, Box<dyn Error>> {
    //     let conn = self.imp().conn.borrow();
    //     let conn = conn.as_ref().ok_or("Connection not established")?;
    //     let mut stmt = conn.prepare("SELECT * FROM plays ORDER BY playtime;")?;
    //     let rows = stmt.query_map([], |row| {
    //         let track_id: i64 = row.get(2)?;
    //         Ok(track_id)
    //     })?;

    //     let mut result = Vec::new();
    //     for row in rows {
    //         let id = row?;
    //         if !result.contains(&id) {
    //             result.push(id);
    //         }

    //         if result.len() >= 20 {
    //             break;
    //         }
           
    //     }

    //     Ok(result)
    // }

    //Used in the home page
    // fn query_most_recent_albums(&self) -> Result<Vec<i64>, Box<dyn Error>> {
    //     let conn = self.imp().conn.borrow();
    //     let conn = conn.as_ref().ok_or("Connection not established")?;
    //     let mut stmt = conn.prepare("SELECT * FROM plays ORDER BY playtime;")?;
    //     let rows = stmt.query_map([], |row| {
    //         let album_id: i64 = row.get(3)?;
    //         Ok(album_id)
    //     })?;

    //     let mut result = Vec::new();
    //     for row in rows {
    //         let id = row?;
    //         if !result.contains(&id) {
    //             result.push(id);
    //         }

    //         if result.len() >= 20 {
    //             break;
    //         }
           
    //     }
    //     Ok(result)
    // }

    // ######
    // # MODIFY VALUES 
    // ######

    pub fn update_album_art(&self, tx: &Transaction, album_id: i64, album_art: i64,) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("SELECT cover_art_id FROM Album_Cover_Art_Junction WHERE album_id = (?);")?;
        let cover_art_id: Option<i64> = stmt.query_row([album_id], |row| row.get(0)).optional()?;

        if let Some(id) = cover_art_id {
            if id != album_art {
                let mut stmt = tx.prepare("UPDATE Album_Cover_Art_Junction SET cover_art_id = (?) WHERE album_id = (?);")?;
                stmt.execute(params![album_art, album_id])?;
            }
        } else {
            self.add_album_cover_art_junction(&tx, album_id, album_art)?;
        }

        Ok(())
    }

    pub fn change_playlist_title_and_or_description(&self, playlist_id: i64, new_title: Option<String>, new_description: Option<String>) -> Result<(), Box<dyn Error>> {
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        if !new_title.is_none() {
            self.modify_playlist_title(&tx, playlist_id, new_title.unwrap())?;
        }

        if !new_description.is_none() {
            self.modify_playlist_description(&tx, playlist_id, new_description.unwrap())?;
        }
        
        self.update_playlist_modify_time(&tx, playlist_id)?;
        
        tx.commit()?;
        send!(self.imp().model_sender, ModelAction::PopulatePlaylist(playlist_id as u64));
        Ok(())
    }


    pub fn rename_playlist(&self, playlist_id: i64, new_title: String) -> Result<(), Box<dyn Error>> {
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;
        self.modify_playlist_title(&tx, playlist_id, new_title)?;
        self.update_playlist_modify_time(&tx, playlist_id)?;
        tx.commit()?;
        send!(self.imp().model_sender, ModelAction::PopulatePlaylist(playlist_id as u64));
        Ok(())
    }

    fn modify_playlist_title(&self, tx: &Transaction, playlist_id: i64, new_title: String) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("UPDATE Playlists SET title = (?) WHERE id = (?);")?;
        stmt.execute(params![new_title, playlist_id])?;
        Ok(())
    }

    fn modify_playlist_description(&self, tx: &Transaction, playlist_id: i64, new_description: String) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("UPDATE Playlists SET description = (?) WHERE id = (?);")?;
        stmt.execute(params![new_description, playlist_id])?;
        Ok(())
    }

    fn update_playlist_modify_time(&self, tx: &Transaction, playlist_id: i64) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("UPDATE Playlists SET modify_time = (?) WHERE id = (?);")?;
        let modify_time = chrono::offset::Utc::now();
        stmt.execute(params![modify_time, playlist_id])?;
        Ok(())
    }

    fn reorder_playlist(&self, playlist_id: i64, old_position: usize, new_position: usize) -> Result<(), Box<dyn Error>> {
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;

        let mut entries = self.query_ordered_vec_of_playlist_entries(&tx, playlist_id)?;
        let temp = entries.remove(old_position);
        entries.insert(new_position, temp);
        
        for (pos, entry) in entries.iter().enumerate() {
            self.modify_playlist_entry_positition_by_entry_id(&tx, *entry, pos)?;
        }
        
        self.update_playlist_modify_time(&tx, playlist_id)?;
        
        tx.commit()?;
        send!(self.imp().model_sender, ModelAction::PopulatePlaylist(playlist_id as u64));
        Ok(())
    }

    fn query_ordered_vec_of_playlist_entries(&self, tx: &Transaction, playlist_id: i64) -> Result<Vec<i64>, Box<dyn Error>> {
        debug!("SELECT count( * ) FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?)");


        let mut stmt = tx.prepare("SELECT count( * ) FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?)")?;
        let count: usize = stmt.query_row([playlist_id], |row| row.get(0))?;

        let mut result = vec![0; count];

        debug!("SELECT playlist_entry_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?)");

        let mut stmt = tx.prepare("SELECT playlist_entry_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?)")?;
        let rows = stmt.query_map([playlist_id], |row| {
            let playlist_entry_id: i64 = row.get(0)?;
            Ok(playlist_entry_id)
        })?;

        for r in rows {
            let playlist_entry_id = r?;
            
            let mut stmt = tx.prepare("SELECT playlist_position FROM Playlist_Entries WHERE id = (?)")?;
            let playlist_position: usize = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

            _ = std::mem::replace(&mut result[playlist_position], playlist_entry_id);

            //result.insert(playlist_position, playlist_entry_id);
        }

        Ok(result)
    }


    // fn modify_playlist_entry_positition(&self, tx: &Transaction, playlist_entry_id: i64, old_position: usize, new_position: usize) -> Result<(), Box<dyn Error>> {
    //     let mut stmt = tx.prepare("UPDATE Playlist_Entries SET playlist_position = (?) WHERE id = (?) AND playlist_position = (?);")?;
    //     stmt.execute(params![new_position, playlist_entry_id, old_position])?;
    //     Ok(())
    // }

    fn modify_playlist_entry_positition_by_entry_id(&self, tx: &Transaction, playlist_entry_id: i64, new_position: usize) -> Result<(), Box<dyn Error>> {
        let mut stmt = tx.prepare("UPDATE Playlist_Entries SET playlist_position = (?) WHERE id = (?);")?;
        stmt.execute(params![new_position, playlist_entry_id])?;
        Ok(())
    }


    // ######
    // # REMOVE VALUES 
    // ######
    

    //REMOVE MUSIC FOLDER

    pub fn try_remove_folder(&self, path: String) -> Result<(), Box<dyn Error>> {
        {
           
            let mut conn = self.imp().conn.borrow_mut();
            let conn = conn.as_mut().ok_or("Connection not established")?;
            let tx = conn.transaction()?;
            
            self.remove_music_folder(&tx, path.clone())?;
            self.check_loaded(&tx)?;
            tx.commit()?;
        }
        let imp = self.imp();
        let mut folders = imp.folders.borrow_mut().clone();
        folders.remove(&path);
        let music_folders = folders.into_iter().collect::<Vec<String>>();
        imp.settings.set_strv("music-folders", music_folders.as_slice())?;

        Ok(())
    }

    fn remove_music_folder(&self, tx: &Transaction, path: String) -> Result<(), Box<dyn Error>> {        
        debug!("remove_music_folder: {:?}", path);
        
        //GET ID OF FOLDER FROM FOLDER TABLE
        let mut stmt = tx.prepare("SELECT id FROM Music_Folders WHERE uri = (?)")?;
        let folder_id: i64 = stmt.query_row([path], |row| row.get(0))?;

        debug!("folder id: {}", folder_id);

        //GET ALL TRACK IDS WITH FOLDER FROM TRACK URI JUNCTION TABLE Track_Folder_Junction
        let mut stmt = tx.prepare("SELECT track_id FROM Track_Folder_Junction WHERE folder_id = (?);")?;
        let rows = stmt.query_map([folder_id], |row| {
            let track_id: i64 = row.get(0)?;
            Ok(track_id)
        })?;

        //need to avoid foreign key constraint -> delete all junctions before track deletion
        // (Track_Cover_Art_Junction) 
        // Track_Artist_Junction) 
        // (Track_Album_Junction)
        // Playlist_Entry_Track_Junction
        // Plays

        for row in rows {
            let track_id = row?;

            debug!("removing Track_Folder_Junction track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Track_Folder_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            let mut stmt = tx.prepare("DELETE FROM Track_Folder_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            debug!("removing Track_Cover_Art_Junction track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Track_Cover_Art_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            debug!("removing Track_Artist_Junction track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Track_Artist_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            debug!("removing Track_Album_Junction track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Track_Album_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            debug!("removing Playlist_Entry_Track_Junction track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Playlist_Entry_Track_Junction WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

 
            debug!("removing Plays track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Plays WHERE track_id = (?);")?;
            stmt.execute(params![track_id])?;

            debug!("removing Tracks track_id = {}", track_id);
            let mut stmt = tx.prepare("DELETE FROM Tracks WHERE id = (?);")?;
            stmt.execute(params![track_id])?;
        }

        //delete album, artist, playlist, play if none 
        self.prune_all(&tx)?;

        let mut stmt = tx.prepare("DELETE FROM Music_Folders WHERE id = (?);")?;
        stmt.execute(params![folder_id])?;
        debug!("removed folder");

        Ok(())
    }


    //PRUNE ORPHANS

    fn prune_all(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("pruning");
        self.prune_albums(&tx)?;
        self.prune_playlists(&tx)?;
        self.prune_genres(&tx)?;
        self.prune_artists(&tx)?;
        self.prune_cover_art(&tx)?;
        self.prune_artist_image(&tx)?;
        self.prune_file_uri(&tx)?;
        Ok(())
    }

    fn prune_albums(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_albums");

        let mut stmt = tx.prepare("SELECT id FROM Albums;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let album_id = row?;

            //if there are no tracks in the album, it should not exist
            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Track_Album_Junction WHERE album_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([album_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Album_Cover_Art_Junction WHERE album_id = (?);")?;
                stmt.execute(params![album_id])?;

                let mut stmt = tx.prepare("DELETE FROM Album_Artist_Junction WHERE album_id = (?);")?;
                stmt.execute(params![album_id])?;

                let mut stmt = tx.prepare("DELETE FROM Album_Genre_Junction WHERE album_id = (?);")?;
                stmt.execute(params![album_id])?;

                let mut stmt = tx.prepare("DELETE FROM Albums WHERE id = (?);")?;
                stmt.execute(params![album_id])?;
            }
        }
        Ok(())
    }


    fn prune_genres(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_genres");

        let mut stmt = tx.prepare("SELECT id FROM Genres;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let genre_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Album_Genre_Junction WHERE genre_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([genre_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Genres WHERE id = (?);")?;
                stmt.execute(params![genre_id])?;
            }
        }
        Ok(())
    }

    fn prune_file_uri(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_file_uri");

        let mut stmt = tx.prepare("SELECT id FROM File_URIs;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;

        for row in rows {
            let file_uri_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Tracks WHERE file_uri_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([file_uri_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Folder_File_Junction WHERE file_id = (?);")?;
                stmt.execute(params![file_uri_id])?;

                let mut stmt = tx.prepare("DELETE FROM File_URIs WHERE id = (?);")?;
                stmt.execute(params![file_uri_id])?;
            }
        }


        Ok(())
    }

    fn prune_artists(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_artists");

        let mut stmt = tx.prepare("SELECT id FROM Artists;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let artist_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Album_Artist_Junction WHERE artist_id = (?) LIMIT 1) OR EXISTS(SELECT 1 FROM Track_Artist_Junction WHERE artist_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([artist_id, artist_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Artist_Discog_Artist_Image_Junction WHERE artist_id = (?);")?;
                stmt.execute(params![artist_id])?;

                let mut stmt = tx.prepare("DELETE FROM Artists WHERE id = (?);")?;
                stmt.execute(params![artist_id])?;
            }
        }
        Ok(())
    }


    fn prune_cover_art(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_cover_art");

        let mut stmt = tx.prepare("SELECT id FROM Cover_Art;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let cover_art_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Track_Cover_Art_Junction WHERE cover_art_id = (?) LIMIT 1) OR EXISTS(SELECT 1 FROM Album_Cover_Art_Junction WHERE cover_art_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([cover_art_id, cover_art_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Cover_Art WHERE id = (?);")?;
                stmt.execute(params![cover_art_id])?;
            }
        }
        Ok(())
    }

    fn prune_artist_image(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_artist_image");

        let mut stmt = tx.prepare("SELECT id FROM Discog_Artist_Image;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let image_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Artist_Discog_Artist_Image_Junction WHERE image_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([image_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Discog_Artist_Image WHERE id = (?);")?;
                stmt.execute(params![image_id])?;
            }
        }
        Ok(())
    }


    fn prune_playlists(&self, tx: &Transaction) -> Result<(), Box<dyn Error>> {
        debug!("prune_playlists Playlist_Entries");
        let mut stmt = tx.prepare("SELECT id FROM Playlist_Entries;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let playlist_entry_id = row?;
            debug!("prune_playlists Playlist_Entry_Track_Junction");
            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;
            
            if exists == 0 {
                debug!("prune_playlists delete Playlist Entry");
                // if the junction does not exist, remove from playlist and then delete entry.
                // let mut stmt = tx.prepare("SELECT playlist_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = (?);")?;
                // let playlist_id: i64 = stmt.query_row([playlist_entry_id], |row| row.get(0))?;

                self.delete_track_from_playlist(&tx, playlist_entry_id)?;

                let mut stmt = tx.prepare("DELETE FROM Playlist_Entries WHERE id = (?);")?;
                stmt.execute(params![playlist_entry_id])?;
            }
        }
        debug!("prune_playlists Playlists");
        let mut stmt = tx.prepare("SELECT id FROM Playlists;")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        for row in rows {
            let playlist_id = row?;

            let mut stmt = tx.prepare("SELECT EXISTS(SELECT 1 FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?) LIMIT 1);")?;
            let exists: i32 = stmt.query_row([playlist_id], |row| row.get(0))?;
            
            if exists == 0 {
                let mut stmt = tx.prepare("DELETE FROM Playlists WHERE id = (?);")?;
                stmt.execute(params![playlist_id])?;
            }
        }
        Ok(())
    }


    
    //REMOVE PLAYLIST METHODS
    fn delete_playlist(&self, playlist_id: i64) -> Result<(), Box<dyn Error>> {
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;
        
        let statements = |tx: &Transaction| -> Result<(), Box<dyn Error>> {
            debug!("begin removal statements");

            let mut stmt = tx.prepare("DELETE FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = (?);")?;
            stmt.execute(params![playlist_id])?;
            debug!("removed Playlist_Entry_Playlist_Junction");

            let entries = self.query_ordered_vec_of_playlist_entries(&tx, playlist_id)?;

            debug!("removing entries: {:?}", entries);
            for entry_id in entries {

                let mut stmt = tx.prepare("DELETE FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = (?);")?;
                stmt.execute(params![entry_id])?;
                debug!("removed Playlist_Entry_Track_Junction");

                let mut stmt = tx.prepare("DELETE FROM Playlist_Entries WHERE id = (?);")?;
                stmt.execute(params![entry_id])?;
                debug!("removed Playlist_Entries");
            }
            debug!("removed Playlist_Entries & Playlist_Entry_Track_Junction");

            let mut stmt = tx.prepare("DELETE FROM Playlists WHERE id = (?);")?;
            stmt.execute(params![playlist_id])?;
            debug!("removed playlist");

            Ok(())
        };

        statements(&tx)?;
        tx.commit()?;
        send!(self.imp().model_sender, ModelAction::PopulatePlaylists);
        Ok(())
    }


    fn remove_track_from_playlist(&self, playlist_entry_id: i64) -> Result<(), Box<dyn Error>> {
        let mut conn = self.imp().conn.borrow_mut();
        let conn = conn.as_mut().ok_or("Connection not established")?;
        let tx = conn.transaction()?;
        let playlist_id: Option<i64> = self.delete_track_from_playlist(&tx, playlist_entry_id)?;
        tx.commit()?;
        if !playlist_id.is_none() {
            send!(self.imp().model_sender, ModelAction::PopulatePlaylist(playlist_id.unwrap() as u64));
        }
        Ok(())
    }   

    fn delete_track_from_playlist(&self, tx: &Transaction, playlist_entry_id: i64) -> Result<Option<i64>, Box<dyn Error>> {
        debug!("SELECT playlist_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = ({})", playlist_entry_id);

        let mut stmt = tx.prepare("SELECT playlist_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = (?)")?;
        let playlist_id: i64 = match stmt.query_row([playlist_entry_id], |row| row.get(0)).optional()? {
            Some(id) => id,
            None => return Ok(None),
        };

        debug!("SELECT playlist_position FROM Playlist_Entries WHERE id = (?)");

        let mut stmt = tx.prepare("SELECT playlist_position FROM Playlist_Entries WHERE id = (?)")?;
        let playlist_position: usize = stmt.query_row([playlist_entry_id], |row| row.get(0))?;
        
        let mut entries = self.query_ordered_vec_of_playlist_entries(&tx, playlist_id)?;
        _ = entries.remove(playlist_position);
        for (pos, entry) in entries.iter().enumerate() {
            self.modify_playlist_entry_positition_by_entry_id(&tx, *entry, pos)?;
        }

        debug!("begin removal statements");
        let mut stmt = tx.prepare("DELETE FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = (?);")?;
        stmt.execute(params![playlist_entry_id])?;
        debug!("removed Playlist_Entry_Playlist_Junction");

        let mut stmt = tx.prepare("DELETE FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = (?);")?;
        stmt.execute(params![playlist_entry_id])?;
        debug!("removed Playlist_Entry_Track_Junction");

        let mut stmt = tx.prepare("DELETE FROM Playlist_Entries WHERE id = (?);")?;
        stmt.execute(params![playlist_entry_id])?;
        debug!("removed Playlist_Entries");

        self.update_playlist_modify_time(&tx, playlist_id)?;
        Ok(Some(playlist_id))
    }


// TABLE SETUP 

    fn setup_db_tables(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.imp().conn.borrow();
        let connection = conn.as_ref().ok_or("Connection not established")?;

        connection
            .execute("PRAGMA foreign_keys = ON", params![])
            .unwrap();

        // make directories table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Music_Folders
        (
            id  INTEGER PRIMARY KEY,
            uri TEXT NOT NULL
        );", params![],).unwrap();

        // make files table
        connection.execute("CREATE TABLE IF NOT EXISTS
        File_URIs
        (
            id  INTEGER PRIMARY KEY,
            uri TEXT NOT NULL,
            last_modified TIMESTAMP
        );", params![],).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Folder_File_Junction
        (
            id  INTEGER PRIMARY KEY,
            folder_id INTEGER NOT NULL,
            file_id INTEGER NOT NULL,
            FOREIGN KEY (folder_id) REFERENCES Music_Folders(id),
            FOREIGN KEY (file_id) REFERENCES File_URIs(id)
        );", params![],).unwrap();

        // make genre table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Genres
        (
            id  INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );", params![],).unwrap();

        // make artist table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Artists
        (
            id  INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            image_fetched INTEGER NOT NULL
        );",params![],).unwrap();

        // make album table
        // foreign key -> albumartist_parent
        connection.execute("CREATE TABLE IF NOT EXISTS
        Albums
        (
            id  INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            date TEXT NOT NULL,
            genre TEXT NOT NULL
        );", params![],).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Cover_Art
        (
            id  INTEGER PRIMARY KEY,
            data BLOB NOT NULL,
            thing INTEGER
        );",params![],).unwrap();

        // make tracks table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Tracks
        (
            id  INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            filetype TEXT NOT NULL,
            album_name TEXT NOT NULL,
            date TEXT NOT NULL,
            duration REAL NOT NULL,
            track_number INTEGER NOT NULL,
            disc_number INTEGER NOT NULL,
            album_artist TEXT NOT NULL,
            file_uri_id INTEGER NOT NULL,
            FOREIGN KEY (file_uri_id) REFERENCES File_URIs(id)
        );", params![]).unwrap();

        // make plays table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Plays
        (
            id  INTEGER PRIMARY KEY,
            playtime TIMESTAMP,
            track_id INTEGER NOT NULL,
            album_id INTEGER NOT NULL,
            album_artist_id INTEGER NOT NULL,
            FOREIGN KEY (track_id) REFERENCES tracks(id),
            FOREIGN KEY (album_id) REFERENCES albums(id),
            FOREIGN KEY (album_artist_id) REFERENCES artists(id)
        );",  params![],).unwrap();

        // make playlists table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Playlists
        (
            id  INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            creation_time TIMESTAMP,
            modify_time TIMESTAMP
        );", params![],).unwrap();

        // make playlist entries table
        connection.execute("CREATE TABLE IF NOT EXISTS
        Playlist_Entries
        (
            id  INTEGER PRIMARY KEY,
            playlist_position INTEGER NOT NULL
        );", params![],).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Playlist_Entry_Track_Junction
        (
            id  INTEGER PRIMARY KEY,
            playlist_entry_id INTEGER NOT NULL,
            track_id INTEGER NOT NULL,
            FOREIGN KEY (playlist_entry_id) REFERENCES Playlist_Entries(id),
            FOREIGN KEY (track_id) REFERENCES Tracks(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Playlist_Entry_Playlist_Junction
        (
            id  INTEGER PRIMARY KEY,
            playlist_entry_id INTEGER NOT NULL,
            playlist_id INTEGER NOT NULL,
            FOREIGN KEY (playlist_entry_id) REFERENCES Playlist_Entries(id),
            FOREIGN KEY (playlist_id) REFERENCES Playlists(id)
        );", params![]).unwrap();


        // let mut stmt = connection.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        // let mut rows = stmt.query(rusqlite::params![])?;

        // while let Some(row) = rows.next()? {
        //     let table_name: String = row.get(0)?;
        //     debug!("Table name: {}", table_name);
        // }

        connection.execute("CREATE TABLE IF NOT EXISTS
        Track_Folder_Junction
        (
            id  INTEGER PRIMARY KEY,
            track_id INTEGER NOT NULL,
            folder_id INTEGER NOT NULL,
            FOREIGN KEY (track_id) REFERENCES Tracks(id),
            FOREIGN KEY (folder_id) REFERENCES Music_Folders(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Track_Album_Junction
        (
            id  INTEGER PRIMARY KEY,
            track_id INTEGER NOT NULL,
            album_id INTEGER NOT NULL,
            FOREIGN KEY (track_id) REFERENCES Tracks(id),
            FOREIGN KEY (album_id) REFERENCES Albums(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Track_Artist_Junction
        (
            id  INTEGER PRIMARY KEY,
            track_id INTEGER NOT NULL,
            artist_id INTEGER NOT NULL,
            FOREIGN KEY (track_id) REFERENCES Tracks(id),
            FOREIGN KEY (artist_id) REFERENCES Artists(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Track_Cover_Art_Junction
        (
            id  INTEGER PRIMARY KEY,
            track_id INTEGER NOT NULL,
            cover_art_id INTEGER NOT NULL,
            FOREIGN KEY (track_id) REFERENCES Tracks(id),
            FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Album_Cover_Art_Junction
        (
            id  INTEGER PRIMARY KEY,
            album_id INTEGER NOT NULL,
            cover_art_id INTEGER NOT NULL,
            FOREIGN KEY (album_id) REFERENCES Albums(id),
            FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Album_Artist_Junction
        (
            id  INTEGER PRIMARY KEY,
            album_id INTEGER NOT NULL,
            artist_id INTEGER NOT NULL,
            FOREIGN KEY (album_id) REFERENCES Albums(id),
            FOREIGN KEY (artist_id) REFERENCES Artists(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Album_Genre_Junction
        (
            id  INTEGER PRIMARY KEY,
            album_id INTEGER NOT NULL,
            genre_id INTEGER NOT NULL,
            FOREIGN KEY (album_id) REFERENCES Albums(id),
            FOREIGN KEY (genre_id) REFERENCES Genres(id)
        );", params![]).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Discog_Artist_Image
        (
            id  INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            data BLOB NOT NULL,
            thing INTEGER
        );",params![],).unwrap();

        connection.execute("CREATE TABLE IF NOT EXISTS
        Artist_Discog_Artist_Image_Junction
        (
            id  INTEGER PRIMARY KEY,
            artist_id INTEGER NOT NULL,
            image_id INTEGER NOT NULL,
            FOREIGN KEY (artist_id) REFERENCES Artists(id),
            FOREIGN KEY (image_id) REFERENCES Discog_Artist_Image(id)
        );", params![]).unwrap();

        Ok(())
    }

    pub fn loaded(&self) -> bool {
        self.imp().loaded.get()
    }

    fn importer(&self) -> &Importer {
        &self.imp().importer
    }

}

fn fetch_artist_images_bulk(artists: Vec<(i64, String)>, sender: Sender<DatabaseAction>) -> Result<(), Box<dyn Error>> {
    let dur = Duration::from_millis(2525);
    //let n_artists = artists.len() as u32;

    let mut payload = Vec::new();

    for (id, name) in artists {
        if let Some((url, data)) = util::fetch_artist_image_discog(&name) {
            payload.push((id, Some((url, data))));
            //send!(sender, DatabaseAction::AddArtistImage( ) );
        } else {
            payload.push((id, None));
            //send!(sender, DatabaseAction::AddArtistImage((id, None)));
        }
        
        std::thread::sleep(dur);
    }

    //send!(sender, DatabaseAction::AddArtistImagesDone(n_artists));
    send!(sender, DatabaseAction::AddArtistImages(payload));

    Ok(())
}
