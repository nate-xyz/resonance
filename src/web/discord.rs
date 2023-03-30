/* discord.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use gtk::{gio, glib, glib::clone, glib::Receiver, glib::Sender};
use gtk_macros::send;

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::fmt;
use log::{debug, error};
use chrono::{self, Duration};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

use crate::model::track::Track;
use crate::util::settings_manager;

use super::music_brainz::MusicBrainzAction;

#[derive(Clone, Debug)]
pub enum DiscordAction {
    SetPlaying(Rc<Track>),
    Seek(f64),
    Clear,
    Close,
    Reconnect,
}

pub struct ResonanceDiscord {
    client: RefCell<Option<DiscordIpcClient>>,
    connected: Cell<bool>,
    settings: gio::Settings,
    receiver: RefCell<Option<Receiver<DiscordAction>>>,
    mb_sender: Sender<MusicBrainzAction>,
    mb_receiver: RefCell<Option<Receiver<(i64, Option<String>)>>>,
    current_track: RefCell<Option<Rc<Track>>>,
    current_art_url: RefCell<Option<String>>,
    now: RefCell<Option<chrono::DateTime<chrono::Utc>>>,
}

impl fmt::Debug for ResonanceDiscord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResonanceDiscord")
         .field("client", &self.client.borrow().as_ref().is_none())
         .field("connected", &self.connected.get())
         .finish()
    }
}

impl ResonanceDiscord {
    pub fn new(receiver: Receiver<DiscordAction>, mb_sender: Sender<MusicBrainzAction>, mb_receiver: Receiver<(i64, Option<String>)>) -> Rc<Self> {
        let res_discord = Self {
            client: RefCell::new(None),
            connected: Cell::new(false),
            settings: settings_manager(),
            receiver: RefCell::new(Some(receiver)),
            mb_sender,
            mb_receiver: RefCell::new(Some(mb_receiver)),
            current_track: RefCell::new(None),
            current_art_url: RefCell::new(None),
            now: RefCell::new(None),
        };
        res_discord.setup();

        let res_discord = Rc::new(res_discord);
        res_discord.clone().setup_channels();
        res_discord
    }

    fn setup(&self) {
        let discord_enabled = self.settings.boolean("discord-rich-presence");
        if discord_enabled {
            match self.connect() {
                Ok(_) => debug!("Setup discord client connection"),
                Err(e) => error!("Unable to connect to discord client: {}", e),
            }
        } else {
            debug!("Not connecting to discord by default");
        }
    }

    fn setup_channels(self: Rc<Self>) {
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.process_action(action)),
        );

        let receiver = self.mb_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |(album_id, mb_id)| this.receive_id(album_id, mb_id)),
        );
    }

    fn process_action(&self, action: DiscordAction) -> glib::Continue {
        match action {
            DiscordAction::SetPlaying(track) => {
                match self.set_playing(track) {
                    Ok(_) => (),
                    Err(e) => error!("Could not set track playing: {}", e),
                }
            },
            DiscordAction::Clear => {
                match self.clear_activity() {
                    Ok(_) => debug!("Cleared activity from discord client."),
                    Err(e) => error!("Could not clear activity: {}", e),
                }
            },
            DiscordAction::Seek(position) => {
                match self.seek(position) {
                    Ok(_) => debug!("Updated discord activity position"),
                    Err(e) => error!("Unable to update discord activity position: {}", e),
                }
            },
            DiscordAction::Close => {
                match self.close() {
                    Ok(_) => debug!("Closed discord client connection"),
                    Err(e) => error!("Unable to close discord client connection: {}", e),
                }
            },
            DiscordAction::Reconnect => {
                match self.reconnect() {
                    Ok(_) => debug!("Reconnected discord client connection"),
                    Err(e) => error!("Unable to reconnect discord client connection: {}", e),
                }
            },
            // _ => debug!("Received action {:?}", action),
        }

        glib::Continue(true)
    }

    fn seek(&self, position: f64) -> Result<(), Box<dyn Error>> {
        if self.connected.get() {
            if let Some(track) = self.current_track.borrow().as_ref() {
                let track_duration = (track.duration() - position) as i64;
                let timestamp = (chrono::offset::Utc::now() + Duration::seconds(track_duration)).timestamp_millis();
                let time = discord_rich_presence::activity::Timestamps::new().end(timestamp);
                self.update_activity(time)?;
            }
        }
        Ok(())
    }

    fn reconnect(&self) -> Result<(), Box<dyn Error>> {
        if !self.connected.get() {
            self.connect()?;
        }

        if let Some(track) = self.current_track.borrow().as_ref() {
            let track_duration = track.duration() as i64;
            let now = if let Some(n) = self.now.take() {
                n
            } else {
                chrono::offset::Utc::now()
            };

            let timestamp = (now + Duration::seconds(track_duration)).timestamp_millis();
            let time = discord_rich_presence::activity::Timestamps::new().end(timestamp);
            self.now.replace(Some(now));
            self.update_activity(time)?;
        }
        Ok(())
    }

    fn close(&self) -> Result<(), Box<dyn Error>> {
        if self.connected.get() {
            if let Some(client) = self.client.take().as_mut() {
                client.clear_activity()?;
                client.close()?;
            }
        }
        self.connected.set(false);
        Ok(())
    }

    fn clear_activity(&self) -> Result<(), Box<dyn Error>> {
        if self.connected.get() {
            if let Some(client) = self.client.borrow_mut().as_mut() {
                client.clear_activity()?;
            }
        }
        Ok(())
    }



    fn load_client(&self) -> Result<(), Box<dyn Error>> {
        let client = DiscordIpcClient::new("1089186365738066031")?;
        self.client.replace(Some(client));
        Ok(())
    }

    fn connect(&self) -> Result<(), Box<dyn Error>> {
        if self.client.borrow().is_none() {
            self.load_client()?;
        }
        if let Some(client) = self.client.borrow_mut().as_mut() {
            client.connect()?;
            self.connected.set(true);
        }
        Ok(())
    }

    fn set_playing(&self, track: Rc<Track>) -> Result<(), Box<dyn Error>> {
        if !self.connected.get() {
            return Ok(())
        }
        let now = chrono::offset::Utc::now();
        self.current_track.replace(Some(track.clone()));
        let track_duration = track.duration() as i64;
        let timestamp = (now + Duration::seconds(track_duration)).timestamp_millis();
        let time = discord_rich_presence::activity::Timestamps::new().end(timestamp);
        self.now.replace(Some(now));
        self.update_activity(time)?;
        send!(self.mb_sender, MusicBrainzAction::FindRelease((false, track.clone())));    
        Ok(())
    }

    fn receive_id(&self, album_id: i64, art_url: Option<String>) -> glib::Continue {
        _ = self.set_playing_track(album_id, art_url);
        glib::Continue(true)
    }


    fn set_playing_track(&self, album_id: i64, art_url: Option<String>) -> Result<(), Box<dyn Error>> {
        if !self.connected.get() {
            self.connect()?;
        }

        if let Some(track) = self.current_track.borrow().as_ref() {
            if track.album_id() == album_id {
                let track_duration = track.duration() as i64;
                let now = if let Some(n) = self.now.take() {
                    n
                } else {
                    chrono::offset::Utc::now()
                };

                let timestamp = (now + Duration::seconds(track_duration))
                .timestamp_millis();
                let time = discord_rich_presence::activity::Timestamps::new().end(timestamp);
                self.current_art_url.replace(art_url.clone());
                self.now.replace(Some(now));
                self.update_activity(time)?;
            }
        }
        Ok(())
    }


    fn update_activity(&self, time_stamps: discord_rich_presence::activity::Timestamps) -> Result<(), Box<dyn Error>> {
        if self.connected.get() {
            if let Some(client) = self.client.borrow_mut().as_mut() {
                if let Some(track) = self.current_track.borrow().as_ref() {
                    let track_name = track.title();
                    let album_name = track.album();
                    let artist_name = track.artist();
                    let album_and_artist = format!("{} - {}", album_name, artist_name);
                    if let Some(art_url) = self.current_art_url.borrow().as_ref() {
                        let activity = activity::Activity::new()
                        .details(&track_name)
                        .state(&album_and_artist)
                        .assets(
                            activity::Assets::new().large_image(art_url.as_str()),
                        )
                        .timestamps(time_stamps)
                        .buttons(vec![activity::Button::new(
                            "Resonance",
                            "https://github.com/nate-xyz/resonance",
                        )]);
                        client.set_activity(activity)?;
                    } else {
                        let activity = activity::Activity::new()
                        .details(&track_name)
                        .state(&album_and_artist)
                        .assets(
                            activity::Assets::new().large_image(
                                "https://upload.wikimedia.org/wikipedia/commons/b/b6/12in-Vinyl-LP-Record-Angle.jpg",
                            ),
                        )
                        .timestamps(time_stamps)
                        .buttons(vec![activity::Button::new(
                            "Resonance",
                            "https://github.com/nate-xyz/resonance",
                        )]);
                        client.set_activity(activity)?;
                    }
                }
            }
        }
        Ok(())
    }

}
