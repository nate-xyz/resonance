/* last_fm.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use gtk::{gio, glib, glib::clone, glib::Receiver, glib::Sender};
use gtk_macros::send;

use std::{cell::{Cell, RefCell}, error::Error, fmt, rc::Rc, thread, time, collections::HashMap};
use serde::{Deserialize, Serialize};
use log::{debug, error};
use chrono;
use reqwest;

use crate::model::track::Track;
use crate::util::settings_manager;

#[derive(Serialize, Deserialize, Debug)]
struct RequestTokenResult {
    token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SessionKeyResultB {
    name: String,
    key: String,
    subscriber: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct SessionKeyResult {
    session: SessionKeyResultB,
}


#[derive(Clone, Debug)]
pub enum LastFmAction {
    Enabled(bool),
    SetNowPlaying(Rc<Track>),
    Scrobble(Rc<Track>),
}

#[derive(Clone, Debug)]
pub enum ThreadedLastFmAction {
    RequestToken(String),
    SessionKey(String),
    HandleErrorResponse(reqwest::StatusCode),
}

pub struct ResonanceLastFM {
    enabled: Cell<bool>,
    settings: gio::Settings,
    tx: Sender<ThreadedLastFmAction>,
    threaded_receiver: RefCell<Option<Receiver<ThreadedLastFmAction>>>,
    receiver: RefCell<Option<Receiver<LastFmAction>>>,
    session_key: RefCell<Option<String>>,
    current_track: RefCell<Option<Rc<Track>>>,
    start_playing: RefCell<Option<chrono::DateTime<chrono::Utc>>>,
}

impl fmt::Debug for ResonanceLastFM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResonanceLastFM")
            .field("enabled", &self.enabled.get())
            .finish()
    }
}

const LASTFM_API_KEY: &str = "0050bff002a877c2fd1c6001a32ab514";
const LASTFM_API_SECRET: &str = "ac6bbca2fee7f6fa28524526d0d49cca";
const LASTFM_API_ROOT: &str = "https://ws.audioscrobbler.com/2.0/?format=json";
const APP_USER_AGENT: &str = concat!("io.github.nate_xyz.Resonance", "/", env!("CARGO_PKG_VERSION"));

impl ResonanceLastFM {
    pub fn new(receiver: Receiver<LastFmAction>) -> Rc<Self> {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_LOW);

        let res_lastfm = Self {
            enabled: Cell::new(false),
            settings: settings_manager(),
            tx,
            threaded_receiver: RefCell::new(Some(rx)),
            receiver: RefCell::new(Some(receiver)),
            current_track: RefCell::new(None),
            start_playing: RefCell::new(None),
            session_key: RefCell::new(None),
        };
        res_lastfm.setup();

        let res_lastfm = Rc::new(res_lastfm);
        res_lastfm.clone().setup_channels();
        res_lastfm
    }

    fn setup(&self) {
        let last_fm_enabled = self.settings.boolean("last-fm-enabled");
        self.set_enabled(last_fm_enabled);
    }

    fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
        if enabled {
            match self.retrieve_request_token() {
                Ok(_) => debug!("Retrieving Last.FM request Token"),
                Err(e) => error!("Unable to retrieve Last.FM API token: {}", e),
            }
        } else {
            debug!("Last.FM disabled.");
        }
    }

    fn setup_channels(self: Rc<Self>) {
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_lastfm_action(action)),
        );

        let receiver = self.threaded_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_internal_lastfm_action(action)),
        );
    }

    fn process_lastfm_action(&self, action: LastFmAction) -> glib::Continue {
        match action {
            LastFmAction::Enabled(enabled) => self.set_enabled(enabled),
            LastFmAction::SetNowPlaying(track) => {
                if !self.enabled.get() {
                    return glib::Continue(true);
                }
                match self.set_now_playing(track) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to set track playing: {}", e),
                }
            },
            LastFmAction::Scrobble(track) => {
                if !self.enabled.get() {
                    return glib::Continue(true);
                }
                match self.scrobble(track) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to scrobble track: {}", e),
                }
            },
        }
        glib::Continue(true)
    }

    fn process_internal_lastfm_action(&self, action: ThreadedLastFmAction) -> glib::Continue {
        match action {
            ThreadedLastFmAction::RequestToken(token) => {
                match self.set_request_token(token) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to set last fm request token: {}", e),
                }
            },
            ThreadedLastFmAction::SessionKey(token) => {
                match self.set_session_key(token) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to set last fm session key: {}", e),
                }
            },
            ThreadedLastFmAction::HandleErrorResponse(code) => {
                match self.handle_error_response(code) {
                    Ok(_) => (),
                    Err(e) => error!("Unable to handle error response: {}", e),
                }
            }
            // _ => debug!("Received action {:?}", action),
        }
        glib::Continue(true)
    }

    fn retrieve_request_token(&self) -> Result<(), Box<dyn Error>> {
        let session_key = self.settings.string("last-fm-session-key").to_string();

        if session_key.is_empty() {
            let sender = self.tx.clone();

            debug!("No session key found, retrieving Last.FM request token");
            thread::spawn(move || {
                match get_request_token() {
                    Ok(token) => {
                        if let Some(t) = token {
                            send!(sender, ThreadedLastFmAction::RequestToken(t));
                        }
                    },
                    Err(e) => error!("Unable to retrieve request token: {}", e),
                }
            });

        } else {
            debug!("Already have Last.FM session key: {}", session_key);
            self.session_key.replace(Some(session_key));
        }

        Ok(())
    }

    fn set_request_token(&self, token: String) -> Result<(), Box<dyn Error>> {
        let url = format!("http://www.last.fm/api/auth/?api_key={}&token={}", LASTFM_API_KEY, token.clone());
        open::that(&url)?;

        let sender = self.tx.clone();
        thread::spawn(move || {
            let sleep_time = time::Duration::from_secs(5);
            for _ in 0..12 {
                thread::sleep(sleep_time);
                match get_session_key(token.clone()) {
                    Ok(token) => {
                        if let Some(t) = token {
                            debug!("retrieved token!");
                            send!(sender, ThreadedLastFmAction::SessionKey(t));
                            break;
                        } else {
                            error!("Unable to retrieve session key");
                        }
                    },
                    Err(e) => {
                        error!("Unable to retrieve session key: {}", e);
                        break;
                    },
                }
                debug!("Did not retrieve token, polling again ...");
            }

        });

        Ok(())
    }

    fn set_session_key(&self, session_key: String) -> Result<(), Box<dyn Error>> {
        debug!("Setting session key");

        self.settings.set_string("last-fm-session-key", &session_key)?;
        self.session_key.replace(Some(session_key));
        Ok(())
    }

    fn set_now_playing(&self, track: Rc<Track>) -> Result<(), Box<dyn Error>> {
        if let Some(session_key) = self.session_key.borrow().clone() {
            self.start_playing.replace(Some(chrono::offset::Utc::now()));
            let artist = track.artist();
            let album = track.album();
            let track_name = track.title();

            let sender = self.tx.clone();

            thread::spawn(move || {
                match update_now_playing(sender, session_key, artist, track_name, album) {
                    Ok(_) => debug!("Updated now playing"),
                    Err(e) => error!("Unable to set now playing: {}", e),
                }
            });

            self.current_track.replace(Some(track));
        }
        Ok(())
    }

    fn scrobble(&self, track: Rc<Track>) -> Result<(), Box<dyn Error>> {
        if let Some(session_key) = self.session_key.borrow().clone() {
            if let Some(current_track) = self.current_track.borrow().as_ref() {
                if track != *current_track {
                    return Ok(());
                }
            }
            if let Some(started) = self.start_playing.borrow().clone() {
                let artist = track.artist();
                let album = track.album();
                let track_name = track.title();
    
                let sender = self.tx.clone();

                thread::spawn(move || {
                    match scrobble(sender, session_key, artist, track_name, album, started) {
                        Ok(_) => debug!("Scrobbled"),
                        Err(e) => error!("Unable to scrobble: {}", e),
                    }
                });
            }
        }
        Ok(())
    }

    fn handle_error_response(&self, code: reqwest::StatusCode) -> Result<(), Box<dyn Error>> {
        if code == 9 { //invalid session key -> need to re-authenticate
            self.settings.set_string("last-fm-session-key", "")?;
            self.session_key.replace(None);
            self.retrieve_request_token()?;
        }

        Ok(())
    }

}

fn get_request_token() -> Result<Option<String>, Box<dyn Error>> {    
    let params = [
        ("api_key", LASTFM_API_KEY),
        ("method", "auth.gettoken"),
    ];

    let url = reqwest::Url::parse_with_params(LASTFM_API_ROOT, &params)?;

    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;

    let response = client.get(url)
        .header("Accept", "application/json")
        .send();

    if let Ok(response) = response {
        if response.status() != 200 {
            error!("Status, {}", response.status());
            return Ok(None);
        }

        let token = response.json::<RequestTokenResult>()?;

        debug!("Retrieved request token: {:?}", token.token);

        return Ok(Some(token.token));
    }

    Ok(None)
}


fn get_session_key(token: String) -> Result<Option<String>, Box<dyn Error>> {
    let mut params = HashMap::new();
    
    params.insert("api_key", LASTFM_API_KEY);
    params.insert("method", "auth.getSession");
    params.insert("token", token.as_str());
    
    let api_signature = get_signature(&params);

    params.insert("api_sig", api_signature.as_str());

    let url = reqwest::Url::parse_with_params(LASTFM_API_ROOT, &params)?;

    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;

    let response = client.get(url)
        .header("Accept", "application/json")
        .send();

    if let Ok(response) = response {
        if response.status() != 200 {
            error!("get_session_key: Status, {}", response.status());
            return Ok(None);
        }

        let token = response.json::<SessionKeyResult>()?;    
        debug!("Retrieved session key: {:?}", token.session.key);
        return Ok(Some(token.session.key));
    }
    Ok(None)
}


// artist (Required) : The artist name.
// track (Required) : The track name.
// album (Optional) : The album name.
// trackNumber (Optional) : The track number of the track on the album.
// context (Optional) : Sub-client version (not public, only enabled for certain API keys)
// mbid (Optional) : The MusicBrainz Track ID.
// duration (Optional) : The length of the track in seconds.
// albumArtist (Optional) : The album artist - if this differs from the track artist.
// api_key (Required) : A Last.fm API key.
// api_sig (Required) : A Last.fm method signature. See authentication for more information.
// sk (Required) : A session key generated by authenticating a user via the authentication protocol.

fn update_now_playing(sender: Sender<ThreadedLastFmAction>, session_key: String, artist: String, track: String, album: String) -> Result<(), Box<dyn Error>> {    
    let mut params = HashMap::new();
    
    params.insert("api_key", LASTFM_API_KEY);
    params.insert("sk", session_key.as_str());
    params.insert("method", "track.updateNowPlaying");
    params.insert("album", album.as_str());
    params.insert("artist", artist.as_str());
    params.insert("track", track.as_str());

    let api_signature = get_signature(&params);
    params.insert("api_sig", api_signature.as_str());

    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;

    let response = client.post(LASTFM_API_ROOT)
        .form(&params)
        .send();

    if let Ok(response) = response {
        let status = response.status();
        if status != 200 {
            if status == 9 { //only handle invalid session key for now
                send!(sender, ThreadedLastFmAction::HandleErrorResponse(status));
            }
            error!("get_session_key: Status, {}", response.status());
            let body = response.text()?;    
            error!("Now playing: {:?}", body);
        } else {
            let body = response.text()?;    
            debug!("Now playing: {:?}", body);
        }
    }
    Ok(())
}


// artist[i] (Required) : The artist name.
// track[i] (Required) : The track name.
// timestamp[i] (Required) : The time the track started playing, in UNIX timestamp format (integer number of seconds since 00:00:00, January 1st 1970 UTC). This must be in the UTC time zone.
// album[i] (Optional) : The album name.
// context[i] (Optional) : Sub-client version (not public, only enabled for certain API keys)
// streamId[i] (Optional) : The stream id for this track received from the radio.getPlaylist service, if scrobbling Last.fm radio
// chosenByUser[i] (Optional) : Set to 1 if the user chose this song, or 0 if the song was chosen by someone else (such as a radio station or recommendation service). Assumes 1 if not specified
// trackNumber[i] (Optional) : The track number of the track on the album.
// mbid[i] (Optional) : The MusicBrainz Track ID.
// albumArtist[i] (Optional) : The album artist - if this differs from the track artist.
// duration[i] (Optional) : The length of the track in seconds.
// api_key (Required) : A Last.fm API key.
// api_sig (Required) : A Last.fm method signature. See authentication for more information.
// sk (Required) : A session key generated by authenticating a user via the authentication protocol.

fn scrobble(sender: Sender<ThreadedLastFmAction>, session_key: String, artist: String, track: String, album: String, started_playing: chrono::DateTime<chrono::Utc>) -> Result<(), Box<dyn Error>> {
    let time_stamp = format!("{}", started_playing.timestamp());

    let mut params = HashMap::new();
    
    params.insert("api_key", LASTFM_API_KEY);
    params.insert("artist", artist.as_str());
    params.insert("track", track.as_str());
    params.insert("album", album.as_str());
    params.insert("method", "track.scrobble");
    params.insert("timestamp", &time_stamp);
    params.insert("sk", session_key.as_str());
    
    let api_signature = get_signature(&params);

    params.insert("api_sig", api_signature.as_str());
        
    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;

    let response = client.post(LASTFM_API_ROOT)
        .form(&params)
        .send();

    if let Ok(response) = response {
        let status = response.status();
        if status != 200 {
            if status == 9 { //only handle invalid session key for now
                send!(sender, ThreadedLastFmAction::HandleErrorResponse(status));
            }
            error!("get_session_key: Status, {}", response.status());
            let body = response.text()?;    
            error!("Scrobble error: {:?}", body);
        } else {
            let body = response.text()?;    
            debug!("Scrobble: {:?}", body);
        }
    }
    Ok(())
}

fn get_signature(params: &HashMap<&str, &str>) -> String {
    let sig_params = params.clone();
    
    let mut keys = Vec::new();
    for k in sig_params.keys() {
        keys.push(k);
    }

    keys.sort();

    let mut sig = String::new();
    for k in keys {
        sig.push_str((k.to_string() + sig_params[k]).as_str())
    }

    sig.push_str(LASTFM_API_SECRET);

    format!("{:x}", md5::compute(sig.as_bytes()))
}


