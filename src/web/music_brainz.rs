/* music_brainz.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

 use gtk::{
    glib,
    glib::{clone, Receiver, Sender},
};
use gtk_macros::send;

use std::{cell::RefCell, thread};
use std::{collections::{HashSet, HashMap}, rc::Rc};
use log::error;
use reqwest;
use serde::{Deserialize, Serialize};

use crate::model::track::Track;


#[derive(Clone, Debug)]
pub enum MusicBrainzAction {
    FindRelease((bool, Rc<Track>)),
}

#[derive(Clone, Debug)]
pub enum ThreadedMusicBrainzAction {
    MusicBrainzId((bool, i64, Option<Vec<String>>)),
    ValidArtUrl((bool, u32, i64, Vec<String>, Option<String>)),
}

#[derive(Serialize, Deserialize, Debug)]
struct SearchResult {
    releases: Vec<Release>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Release {
    id: String,
}

#[derive(Debug)]
pub struct ResonanceMusicBrainz {
    id_cache: RefCell<HashMap<i64, Vec<String>>>,
    url_cache: RefCell<HashMap<(Vec<String>, u32), String>>,
    mb_receiver: RefCell<Option<Receiver<MusicBrainzAction>>>,
    tx: Sender<ThreadedMusicBrainzAction>,
    id_receiver: RefCell<Option<Receiver<ThreadedMusicBrainzAction>>>,
    sender_mb_mpris: Sender<(i64, String)>,
    sender_mb_discord: Sender<(i64, Option<String>)>,
    thread_spawned_already: RefCell<HashSet<i64>>,
}

impl ResonanceMusicBrainz {
    pub fn new(
        receiver: Receiver<MusicBrainzAction>,
        sender_mb_mpris: Sender<(i64, String)>,
        sender_mb_discord: Sender<(i64, Option<String>)>) -> Rc<Self> {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_LOW);
        let mbz = Self {
            id_cache: RefCell::new(HashMap::new()),
            url_cache: RefCell::new(HashMap::new()),
            mb_receiver: RefCell::new(Some(receiver)),
            tx,
            id_receiver: RefCell::new(Some(rx)),
            sender_mb_mpris,
            sender_mb_discord,
            thread_spawned_already: RefCell::new(HashSet::new()),
        };

        // mbz.receiver.replace(Some(receiver));

        let mb = Rc::new(mbz);
        mb.clone().setup_channels();
        mb
    }

    pub fn setup_channels(self: Rc<Self>) {
        let receiver = self.mb_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_mb_action(action)),
        );

        let receiver = self.id_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_internal_mb_action(action)),
        );
    }

    fn process_mb_action(&self, action: MusicBrainzAction) -> glib::Continue {
        match action {
            MusicBrainzAction::FindRelease((is_mpris, track)) => {
                self.find_release(track, is_mpris);
            }
        }

        glib::Continue(true)
    }

    fn process_internal_mb_action(&self, action: ThreadedMusicBrainzAction) -> glib::Continue {
        match action {
            ThreadedMusicBrainzAction::MusicBrainzId((is_mpris, album_id, mb_ids)) => {
                self.receive_id(is_mpris, album_id, mb_ids)
            }
            ThreadedMusicBrainzAction::ValidArtUrl((is_mpris, size, album_id, mb_ids, art_url)) => {
                if is_mpris {
                    if let Some(url) = art_url {
                        send!(self.sender_mb_mpris, (album_id, url.clone()));
                        self.url_cache.borrow_mut().insert((mb_ids.clone(), size), url);
                    }
                } else {
                    if let Some(url) = art_url {
                        send!(self.sender_mb_discord, (album_id, Some(url.clone())));
                        self.url_cache.borrow_mut().insert((mb_ids.clone(), size), url);
                    } else {
                        send!(self.sender_mb_discord, (album_id, None))
                    }
                }
                self.thread_spawned_already.borrow_mut().remove(&album_id);
            }
        }

        glib::Continue(true)
    }

    // Searches for a release on MusicBrainz
    // Returns its ID if one is found.
    pub fn find_release(&self, track: Rc<Track>, is_mpris: bool) {
        let album_id = track.album_id();
        let artist = track.artist();
        let album = track.album();

        if self.id_cache.borrow().contains_key(&album_id) {
            //debug!("{} already in cache", album_id);
            let art_id = self.id_cache.borrow().get(&album_id).cloned();
            if let Some(music_brainz_ids) = art_id {
                if is_mpris {
                    let key = &(music_brainz_ids.clone(), 500);

                    if self.url_cache.borrow().contains_key(key) {
                        //debug!("{} already in url cache", album_id);
                        let art_url = self.url_cache.borrow().get(key).cloned();
                        if let Some(url) = art_url {
                            send!(self.sender_mb_mpris, (album_id, url))
                        }
                        return;
                    }

                    if !self.thread_spawned_already.borrow().contains(&album_id) {
                        self.thread_spawned_already.borrow_mut().insert(album_id);

                        let sender = self.tx.clone();

                        thread::spawn(move || {
                            let art_url = first_valid_art_url(music_brainz_ids.clone(), 500);
                            send!(sender, ThreadedMusicBrainzAction::ValidArtUrl((is_mpris, 500, album_id, music_brainz_ids, art_url)));
                        });

                    }
                    // if let Some(url) = first_valid_art_url(music_brainz_ids, 500) {
                    //     send!(self.sender_mb_mpris, (album_id, url));
                    // }
                    return;
                } else {
                    let key = &(music_brainz_ids.clone(), 250);

                    if self.url_cache.borrow().contains_key(key) {
                        //debug!("{} already in url cache", album_id);
                        let art_url = self.url_cache.borrow().get(key).cloned();
                        send!(self.sender_mb_discord, (album_id, art_url));
                        return;
                    }

                    if !self.thread_spawned_already.borrow().contains(&album_id) {
                        self.thread_spawned_already.borrow_mut().insert(album_id);

                        let sender = self.tx.clone();

                        thread::spawn(move || {
                            let art_url = first_valid_art_url(music_brainz_ids.clone(), 250);
                            send!(sender, ThreadedMusicBrainzAction::ValidArtUrl((is_mpris, 250, album_id, music_brainz_ids, art_url)));
                        });
                    }
                    return;
                }
            }
        }

        let sender = self.tx.clone();

        thread::spawn(move || {
            let music_brainz_id = music_brainz_id(artist, album);
            send!(sender, ThreadedMusicBrainzAction::MusicBrainzId((is_mpris, album_id, music_brainz_id)));
        });
    }

    fn receive_id(&self, is_mpris: bool, album_id: i64, music_brainz_id: Option<Vec<String>>) {
        if let Some(id) = music_brainz_id.clone() {
            self.id_cache.borrow_mut().insert(album_id, id);
        }
        let sender = self.tx.clone();

        if is_mpris {
            if let Some(ids) = music_brainz_id {
                if !self.thread_spawned_already.borrow().contains(&album_id) {
                    self.thread_spawned_already.borrow_mut().insert(album_id);
                    thread::spawn(move || {
                        let art_url = first_valid_art_url(ids.clone(), 500);
                        send!(sender, ThreadedMusicBrainzAction::ValidArtUrl((is_mpris, 500, album_id, ids, art_url)));
                    });
                }
            }
        } else {
            if let Some(ids) = music_brainz_id {
                if !self.thread_spawned_already.borrow().contains(&album_id) {
                    self.thread_spawned_already.borrow_mut().insert(album_id);
                    thread::spawn(move || {
                        let art_url = first_valid_art_url(ids.clone(), 250);
                        send!(sender, ThreadedMusicBrainzAction::ValidArtUrl((is_mpris, 250, album_id, ids, art_url)));
                    });
                }
            } else {
                send!(self.sender_mb_discord, (album_id, None))
            }
        }
    }
}

fn music_brainz_id(artist: String, album: String) -> Option<Vec<String>> {
    //static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    static APP_USER_AGENT: &str = concat!(
        "io.github.nate_xyz.Resonance", "/", env!("CARGO_PKG_VERSION"));

    let query = format!("trackartist:{} AND release:{}", &artist, &album);

    let url = format!("https://musicbrainz.org/ws/2/release/?query={}&limit=2", query);

    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .expect("Failed to create HTTP client");

    let response = client.get(&url).header("Accept", "application/json").send();

    if let Ok(response) = response {
        if response.status() != 200 {
            error!("Status, {}", response.status());
            return None;
        }

        let response = response
            .json::<SearchResult>()
            .expect("Received response from MusicBrainz in unexpected format");

        let mut all_responses = Vec::new();
        for r in response.releases {
            all_responses.push(r.id.clone())
        }

        if !all_responses.is_empty() {
            return Some(all_responses);
        } else {
            return None;
        }
    }

    None
}

fn first_valid_art_url(release_ids: Vec<String>, size: u32) -> Option<String> {
    for release_id in release_ids {
        let art_url = get_album_art_url(release_id, size);
        let response = reqwest::blocking::get(art_url.clone()).unwrap();
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            continue;
        } else {
            return Some(art_url);
        }
    }
    None
}

fn get_album_art_url(release_id: String, size: u32) -> String {
    format!(
        "https://coverartarchive.org/release/{}/front-{}",
        release_id, size
    )
}
