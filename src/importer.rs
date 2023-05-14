/* importer.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{gio, glib::Sender};
use gtk_macros::send;

use std::cell::RefCell;
use std::{collections::HashMap, error::Error, fmt, fs};
use log::{debug, error};
use rusqlite::Transaction;
use pyo3::prelude::*;
use chrono::{DateTime, Utc};
use regex::Regex;

use super::database::DatabaseAction;
use super::util;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct ImportError(String);
impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Import error: {}", self.0)
        
    }
}
impl Error for ImportError {}

#[derive(Debug, FromPyObject, Clone)]
pub enum MapVal {
    Str(String),
    List(Vec<String>),
    Float(f32),
    Int(i32),
    Bytes(Vec<u8>),
}

#[pyclass]
struct LoggingStdout {
    last_update_time: RefCell<Option<Instant>>,
}


#[pymethods]
impl LoggingStdout {
    #[new]
    fn new() -> Self {
        Self {
            last_update_time: RefCell::new(None),
        }
    }

    fn write(&self, data: &str) {
        //println!("stdout from python: {:?}", data);
        if data.contains("ERROR") || data.contains("Exception") {
            error!("{}", data);
        } else if data.contains("DEBUG") {
            debug!("{}", data);
        } else if data.contains("%"){
            if let Some(start) = self.last_update_time.take() {
                if start.elapsed() > Duration::from_secs(3) {
                    if let Some(window) = util::window() {
                        window.set_import_percentage(data);
                    }
                    self.last_update_time.replace(Some(Instant::now()));
                } else {
                    self.last_update_time.replace(Some(start));
                }
            } else {
                if let Some(window) = util::window() {
                    window.set_import_percentage(data);
                }
                self.last_update_time.replace(Some(Instant::now()));
            }
        }
    }
}

#[derive(Debug)]
pub struct Importer {
    pub settings: gio::Settings,
}

impl Importer {
    pub fn new() -> Self {
        Self {
            settings: util::settings_manager(),
        }
    }

    pub fn extract_folder(&self, folder_path: String, sender: Sender<DatabaseAction>) {
        std::thread::spawn(move || {
            let from_python = Python::with_gil(|py| -> Result<(), Box<dyn Error>> {
                let sys = py.import("sys")?;
                sys.setattr("stderr", LoggingStdout::new().into_py(py))?;
                py.run("print('importing')", None, None)?;

                let window = util::window();

                if let Some(w) = window {
                    w.set_import_message("Extracting tags from music files.");
                
                    let code_main = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/python/main.py"));
                    let code_extracting = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/python/extracting.py"));
                    let code_importer = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/python/importer.py"));
                    let code_translate_dicts = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/python/translate_dicts.py"));
                    let code_util = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/python/util.py"));
        
                    PyModule::from_code(py, code_util, "util", "util")?;
                    PyModule::from_code(py, code_translate_dicts,"translate_dicts", "translate_dicts")?;
                    PyModule::from_code(py, code_extracting, "extracting", "extracting")?;
                    PyModule::from_code(py, code_importer, "importer", "importer")?;
        
                    let module = PyModule::from_code(py, code_main, "main", "")?;
                    let args = (&folder_path,);
                    let object: Py<PyAny> = module
                        .getattr("tags_and_cover_art")?
                        .call1(args)
                        .map_err(|e| {
                            e.print_and_set_sys_last_vars(py);
                            e
                        })?
                        .into();
        
                    debug!("retrieving tags & cover art ... ");
        
                    let all: (HashMap<String, HashMap<String, HashMap<String, MapVal>>>, HashMap<String, &[u8]>) = object.extract(py)?;
                    let (tags, bytes_map) = all;
                    if tags.is_empty() {
                        error!("Nothing extracted.");
                        return Err(Box::new(ImportError("Tag Map Empty".into())));
                    }

                    let mut vec_byte_map: HashMap<String, Vec<u8>> = HashMap::new();
                    for (uri, b) in bytes_map.iter() {
                        vec_byte_map.insert(uri.clone(), (*b).to_vec());
                    }

                    send!(sender, DatabaseAction::ConstructFromTags((folder_path, tags, vec_byte_map)));
                } else {
                    error!("Unable to retrieve window.")
                }
                Ok(())
            });
            match from_python {
                Ok(_) => debug!("done extracting folder"),
                Err(e) => error!("Failed to extract tags from music files: {e}"),
            }
        });
    }

    pub fn build_database_from_tags(&self, tx: &Transaction, folder_uri: String, tags: HashMap<String, HashMap<String, HashMap<String, MapVal>>>, cover_art_map: HashMap<String, Vec<u8>>) -> Result<(), Box<dyn Error>> {
        debug!("Building database from tags");
        let window = util::window();
        let database = util::database();

        if let Some(w) = window {
            w.set_import_message("Constructing database from tags.");
    
        }


        let mut added_genres: HashMap<String, i64> = HashMap::new();
        let mut added_artists: HashMap<String, i64> = HashMap::new();
        let mut added_albums: HashMap<String, i64> = HashMap::new();
        let mut added_tracks: HashMap<String, i64> = HashMap::new();
        let mut added_art: HashMap<&[u8], i64> = HashMap::new();
        let mut album_art: HashMap<i64, Vec<i64>> = HashMap::new();

        let mut orphan_tracks = 0;
        let current_folder_id = database.add_folder(&tx, folder_uri)?;

        let re = Regex::new(r"^[^\d]*(\d+)").unwrap();

        for (uri, track_map) in &tags {
            debug!("getting tags from:\n\t -> {:?}", uri);

            let mut title_tag = String::new();
            let mut artist_tag = String::new();
            let mut album_tag = String::new();
            let mut albumartist_tag = String::new();
            let mut date_tag = String::new();
            let mut genre_tag = String::new();
            let mut cover_art = None;
            let mut duration_tag = None;
            let mut disc_number_tag = None;
            let mut track_number_tag = None;
            let mut filetype_tag = String::new();

            if cover_art_map.contains_key(&uri.clone()) {
                cover_art = cover_art_map.get(&uri.clone());
            }
            
            for (type_key, type_map) in track_map {

                match type_key.as_str() {
                    "str_list" => {
                        for (tag, tag_val) in type_map {
                            if let MapVal::List(value) = tag_val {
                                //debug!("\t\ttagging {} {:?} ", tag, value);

                                match tag.as_str() {
                                    "genre" => {
                                        genre_tag = value[0].clone();
                                    },
                                    "album" => {
                                        album_tag = value[0].clone();
                                    },
                                    "title" => {
                                        title_tag = value[0].clone();
                                    },
                                    "albumartist" => {
                                        albumartist_tag = value[0].clone();
                                    },
                                    "artist" => {
                                        artist_tag = value[0].clone();
                                    },
                                    "date" => {
                                        date_tag = value[0].clone();
                                    },
                                    "tracknumber" => {
                                        let tag = re.captures(&value[0]).and_then(|cap| {
                                            cap.get(0).map(|s| s.as_str())
                                        });
                                        track_number_tag = if !tag.is_none() {
                                            match tag.unwrap().parse::<u32>() {
                                                Ok(num) => Some(num),
                                                Err(e) => {
                                                    error!("{}: {}", tag.unwrap(), e);
                                                    None
                                                },
                                            }
                                        } else {
                                            None
                                        };
                                    },
                                    "discnumber" => {
                                        let tag = re.captures(&value[0]).and_then(|cap| {
                                            cap.get(0).map(|s| s.as_str())
                                        });
                                        disc_number_tag = if !tag.is_none() {
                                            match tag.unwrap().parse::<u32>() {
                                                Ok(num) => Some(num),
                                                Err(e) => {
                                                    error!("{}: {}", tag.unwrap(), e);
                                                    None
                                                },
                                            }
                                        } else {
                                            None
                                        };
                                    },
                                    _ => (),
                                }
                            }
                        }
                    }
                    "str" => {
                        for (tag, tag_val) in type_map {
                            if let MapVal::Str(value) = tag_val {
                                match tag.as_str() {
                                    "genre" => {
                                        genre_tag = value.clone();
                                    }
                                    "album" => {
                                        album_tag = value.clone();
                                    }
                                    "title" => {
                                        title_tag = value.clone();
                                    }
                                    "albumartist" => {
                                        albumartist_tag = value.clone();
                                    }
                                    "artist" => {
                                        artist_tag = value.clone();
                                    }
                                    "filetype_" => {
                                        filetype_tag = value.clone();
                                    },
                                    "tracknumber" => {
                                        let tag = re.captures(&value).and_then(|cap| {
                                            cap.get(0).map(|s| s.as_str())
                                        });
                                        debug!("{:?}", tag);
                                        track_number_tag = if !tag.is_none() {
                                            
                                            match tag.unwrap().parse::<u32>() {
                                                Ok(num) => Some(num),
                                                Err(e) => {
                                                    error!("{}: {}", tag.unwrap(), e);
                                                    None
                                                },
                                            }
                                        } else {
                                            None
                                        };
                                    },
                                    "discnumber" => {
                                        let tag = re.captures(&value).and_then(|cap| {
                                            cap.get(0).map(|s| s.as_str())
                                        });
                                        disc_number_tag = if !tag.is_none() {
                                            match tag.unwrap().parse::<u32>() {
                                                Ok(num) => Some(num),
                                                Err(e) => {
                                                    error!("{}: {}", tag.unwrap(), e);
                                                    None
                                                },
                                            }
                                        } else {
                                            None
                                        };
                                    },
                                    _ => (),
                                }
                            }
                        }
                    }
                    "float" => {
                        for (tag, tag_val) in type_map {
                            if let MapVal::Float(value) = tag_val {
                                match tag.as_str() {
                                    "duration" => {
                                        duration_tag = Some(value);
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                    _ => error!("unknown key type"),
                }
            }

            let album_key = format!("{}{}", album_tag, albumartist_tag);
            let song_key = format!("{}{}{}{}", title_tag, album_tag, albumartist_tag, uri);

            if added_tracks.contains_key(&song_key) {
                //debug!("{} already added.", title_tag);
                continue;
            }

            let discnumber = match disc_number_tag {
                Some(number) => number,
                None => 1,
            };

            let tracknumber = match track_number_tag {
                Some(number) => number,
                None => {
                    if album_key == "Unknown AlbumUnknown Artist" {
                        orphan_tracks += 1;
                        let in_db = database.query_orphan_tracks(&tx)? as u32;
                        orphan_tracks+in_db
                    } else {
                        error!("No track number for {}", song_key);
                        1
                    }
                },
            };
   
            // add genre to the database table
            let genre_id = if !genre_tag.is_empty() {
                if !added_genres.contains_key(&genre_tag) {
                    Some(database.add_genre(&tx, genre_tag.clone())?)
                } else {
                    //debug!("Already added genre {}, retrieving from map.", genre_tag);
                    Some(added_genres[&genre_tag])
                }
            } else {
                None
            };

            // add artists to the database table
            let artist_id = if !albumartist_tag.is_empty() {
                if !added_artists.contains_key(&albumartist_tag) {
                    database.add_artist(&tx, albumartist_tag.clone())?
                } else {
                    //debug!("Already added artist {}, retrieving from map.", albumartist_tag);
                    added_artists[&albumartist_tag]
                }
            } else {
                error!("No artist found for track {}", title_tag);
                continue;
            };

            // add cover art to the database table
            let cover_art_id = if let Some(bytes) = cover_art {
                let bytes = bytes.as_slice();
                if added_art.contains_key(&bytes) {
                    //debug!("Already added cover_art, retrieving from map.");
                    added_art.get(&bytes).cloned()
                } else {
                    Some(database.add_cover_art(&tx, bytes)?)
                }
            } else {
                None
            };
            
            // add albums to the database table
            let album_id = if !added_albums.contains_key(&album_key) {
                database.add_album_full(
                    &tx,
                    album_tag.clone(),
                    artist_tag,
                    date_tag.clone(),
                    genre_tag.clone(),
                    cover_art_id,
                    artist_id,
                    genre_id,
                )?
            } else {
                //debug!("Already added {}, retrieving from map.", album_key);
                added_albums[&album_key]
            };
        
            let duration = match duration_tag {
                Some(d) => *d,
                None => {
                    error!("No track duration for {}", song_key);
                    continue;
                },
            };

            

            let metadata = fs::metadata(uri.to_string()).unwrap();
            let modification_time = metadata.modified().unwrap();
            let modification_time_dt = DateTime::<Utc>::from(modification_time);

            // if let Some(w) = window {
            //     w.set_import_message(&format!("Adding: {}", uri.clone()));
            // }

            let track_id =  database.add_track_full(
                &tx,
                title_tag,
                filetype_tag,
                album_tag,
                albumartist_tag.clone(),
                date_tag,
                duration,
                tracknumber,
                discnumber,
                album_id,
                artist_id,
                cover_art_id,
                uri.to_string(),
                modification_time_dt,
                current_folder_id,
            )?;


            if !genre_tag.is_empty() && !genre_id.is_none(){
                added_genres.insert(genre_tag, genre_id.unwrap());
            }
            added_artists.insert(albumartist_tag, artist_id);


            if let Some(data) = cover_art {
                if let Some(art_id) = cover_art_id {
                    added_art.insert(data, art_id);

                    if let Some(ids) = album_art.get_mut(&album_id) {
                        ids.push(art_id);
    
                        if let Some(album_art) = most_frequest_id(ids.clone()) {
                            database.update_album_art(&tx, album_id, album_art)?;
                        }
                    } else {
                        let mut ids = Vec::new();
                        ids.push(art_id);

                        if let Some(album_art) = most_frequest_id(ids.clone()) {
                            database.update_album_art(&tx, album_id, album_art)?;
                        }

                        album_art.insert(album_id, ids);
                    }
                }
            }

            added_albums.insert(album_key, album_id);
            added_tracks.insert(song_key, track_id);

        }

        Ok(())
    }
}

fn most_frequest_id(array: Vec<i64>) -> Option<i64> {
    if array.len() <= 0 {
        return None;
    }

    let mut max_count = 0;
    let mut element_having_max_freq = None;

    for i in 0..array.len() {
        let mut count = 0;
        for j in 0..array.len() {
            if array[i] == array[j] {
                count+=1;
            }
        }
        if count > max_count {
            max_count = count;
            element_having_max_freq = Some(array[i]);
        }
    }

    element_having_max_freq
}