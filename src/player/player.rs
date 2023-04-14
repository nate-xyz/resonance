/* player.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;

use gst::prelude::ObjectExt;
use gtk::{glib, glib::{clone, Receiver, Sender}};
use gtk_macros::send;

use std::{cell::Cell, cell::RefCell, rc::Rc};
use log::{debug, error};

use crate::model::track::Track;
use crate::web::{
    music_brainz::ResonanceMusicBrainz,
    discord::{ResonanceDiscord, DiscordAction},
    last_fm::{ResonanceLastFM, LastFmAction},
};
use crate::util::{database, settings_manager};

use super::gst_backend::{GstPlayer, BackendPlaybackState};
use super::queue::{Queue, QueueAction, RepeatMode};
use super::state::PlayerState;
use super::mpris_controller::MprisController;

#[derive(Clone, Debug)]
pub enum PlaybackAction {
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    Tick(u64),
    EOS,
    Error,
    PlaybackState(BackendPlaybackState),
    SkipPrevious,
    SkipNext,
    Raise,
    Seek(u64),
    QueueRepeatMode(RepeatMode),
}


#[derive(Debug)]
pub struct Player {
    pub backend: Rc<GstPlayer>,
    pub queue: Queue,
    pub playback_receiver: RefCell<Option<Receiver<PlaybackAction>>>,
    pub queue_receiver: RefCell<Option<Receiver<QueueAction>>>,
    pub state: PlayerState,
    pub committed: Cell<bool>,
    pub commit_threshold: Cell<f64>,
    pub mpris: Rc<MprisController>,
    pub music_brainz: Rc<ResonanceMusicBrainz>,
    pub discord: Rc<ResonanceDiscord>,
    pub discord_enabled: Cell<bool>,
    pub discord_sender: Sender<DiscordAction>,
    pub lastfm: Rc<ResonanceLastFM>,
    pub lastfm_enabled: Cell<bool>,
    pub lastfm_sender: Sender<LastFmAction>,
}

impl Player {
    pub fn new() -> Rc<Self> {
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_HIGH_IDLE);
        let playback_receiver = RefCell::new(Some(r));

        let (queue_sender, r) = glib::MainContext::channel(glib::PRIORITY_HIGH_IDLE);
        let queue_receiver = RefCell::new(Some(r));

        let (sender_mb_mpris, rec_mb_mpris) = glib::MainContext::channel(glib::PRIORITY_LOW);
        let (sender_mb_discord, rec_mb_discord) = glib::MainContext::channel(glib::PRIORITY_LOW);
        
        let (music_brainz_sender, music_brainz_receiver) = glib::MainContext::channel(glib::PRIORITY_LOW);

        let music_brainz = ResonanceMusicBrainz::new(music_brainz_receiver, sender_mb_mpris, sender_mb_discord);
        let state = PlayerState::default();
        let backend =  GstPlayer::new(sender.clone());
        let mpris = MprisController::new(sender.clone(), music_brainz_sender.clone(), rec_mb_mpris);
        let (discord_sender, discord_receiver) = glib::MainContext::channel(glib::PRIORITY_LOW);
        let discord = ResonanceDiscord::new(discord_receiver, music_brainz_sender, rec_mb_discord);

        let (lastfm_sender, lastfm_receiver) = glib::MainContext::channel(glib::PRIORITY_LOW);
        let lastfm = ResonanceLastFM::new(lastfm_receiver);

        let p = Self {
            backend,
            queue: Queue::new(queue_sender),
            playback_receiver,
            queue_receiver,
            state,
            committed: Cell::new(false),
            commit_threshold: Cell::new(0.95),
            mpris,
            music_brainz,
            discord,
            discord_enabled: Cell::new(false),
            discord_sender,
            lastfm,
            lastfm_enabled: Cell::new(false),
            lastfm_sender,
        };
        
        let player = Rc::new(p);
        player.clone().setup();
        player
    }

    fn setup(self: Rc<Self>) {
        self.clone().setup_channels();
        self.clone().state_connections();
        self.clone().connect_settings();
    }

    fn connect_settings(self: Rc<Self>) {
        let settings = settings_manager();

        self.discord_enabled.set(settings.boolean("discord-rich-presence"));
        self.lastfm_enabled.set(settings.boolean("last-fm-enabled"));
        self.commit_threshold.set(settings.double("play-commit-threshold"));
    }

    fn setup_channels(self: Rc<Self>) {
        let receiver = self.playback_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.clone().process_playback_action(action)),
        );

        let receiver = self.queue_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |action| this.clone().process_queue_action(action)),
        );
    }

    fn state_connections(self: Rc<Self>) {
        self.state().connect_notify_local(
            Some("state"),
            clone!(@weak self as this => move |_, _| {
                let state = this.state().playback_state();
                this.mpris().set_playback_state(&state);
            }),
        );

        self.state().connect_notify_local(
            Some("song"),
            clone!(@weak self as this => move |_, _| {
                let track = this.state().current_track();
                this.mpris().set_track(track);
            }),
        );

        self.state().connect_notify_local(
            Some("position"),
            clone!(@weak self as this => move |_, _| {
                let pos = this.state().position();
                this.mpris().set_position(pos);
            }),
        );

        self.state().connect_notify_local(
            Some("repeat-mode"),
            clone!(@weak self as this => move |_, _| {
                let mode = this.state().repeat_mode();
                this.mpris().set_repeat_mode(mode);
            }),
        );
    }


    fn process_playback_action(&self, action: PlaybackAction) -> glib::Continue {
        match action {
            PlaybackAction::Play => self.play(),
            PlaybackAction::Pause => self.pause(),
            PlaybackAction::TogglePlayPause => self.toggle_play_pause(),
            PlaybackAction::Stop => self.stop(),
            PlaybackAction::Tick(tick) => self.update_tick(tick),
            PlaybackAction::EOS => self.next(),
            PlaybackAction::Error => error!("Player error"),
            PlaybackAction::PlaybackState(state) => self.set_state_state(state),
            PlaybackAction::QueueRepeatMode(mode) => {_ = self.process_queue_action(QueueAction::QueueRepeatMode(mode))},
            PlaybackAction::SkipPrevious => self.prev(),
            PlaybackAction::SkipNext => self.next(),
            PlaybackAction::Raise => debug!("raise"),
            PlaybackAction::Seek(pos) => self.set_track_position(pos as f64),
            // _ => debug!("Received action {:?}", action),
        }

        glib::Continue(true)
    }

    fn process_queue_action(&self, action: QueueAction) -> glib::Continue {
        let state = self.state();
        match action {
            QueueAction::QueueUpdate => {
                debug!("player QueueUpdate");
                state.queue_update();
            },
            QueueAction::QueueEmpty => {
                debug!("player QueueEmpty");
                self.stop();
                state.queue_empty();
                send!(self.discord_sender, DiscordAction::Clear);
            },
            QueueAction::QueueNonEmpty => {
                debug!("player QueueNonEmpty");
                state.queue_nonempty();
            },
            QueueAction::QueuePositionUpdate(pos) => {
                state.queue_position_update(pos);
            },
            QueueAction::QueueRepeatMode(mode) => {
                state.queue_repeat_mode_update(mode);
            },
            QueueAction::QueueDuration(duration) => {
                state.set_queue_time_remaining(duration);
            },
            _ => debug!("Received action {:?}", action),
        }
        glib::Continue(true)
    }

    fn update_tick(&self, tick: u64) {
        match self.backend.pipeline_position() {
            Some(p) => {
                self.state.set_position(p);
            },
            None => {
                self.state.set_position(tick);
            }
        }

        let progress = tick as f64 / self.state().duration();
        if progress > self.commit_threshold.get() && !self.committed.get() {
            self.record_play();
        }
    }

    //RESET PLAYLIST AND PLAY TRACK
    pub fn clear_play_track(&self, track: Rc<Track>) {
        self.state().set_queue_title(Some(track.title()));
        self.queue().set_song(track);
        self.play();
    }

    //ADD TRACK TO END OF THE PLAYLIST
    pub fn add_track(&self, track: Rc<Track>) {
        self.queue().add_track(track);
        self.state().set_queue_title(None);
    }

    //RESET QUEUE AND PLAY ALBUM
    pub fn clear_play_album(&self, tracks: Vec<Rc<Track>>, title: Option<String>) {
        self.queue().set_album(tracks);
        self.state().set_queue_title(title);
        self.play();
    }

    //ADD ALBUM TO END OF THE PLAYLIST
    pub fn add_album(&self, tracks: Vec<Rc<Track>>) {
        self.queue().add_tracks(tracks);
        self.state().set_queue_title(None);
    }

    //GO TO POSITION IN THE PLAYLIST
    pub fn go_to_playlist_position(&self, position: u64) {
        debug!("PLAYER go_to_playlist_position");
        let queue = self.queue();
        queue.set_position(position);
        self.play();
    }

    pub fn play(&self) {
        self.committed.set(false);
        if let Some(track) = self.queue().current_track() {
            self.backend.set_state(BackendPlaybackState::Loading);
            self.backend.set_uri(track.uri());
            self.backend.set_state(BackendPlaybackState::Playing);
            self.set_current_track(Some(track.clone()));
        } else  {
            self.stop();
        }
    }

    fn pause(&self) {
        self.backend.set_state(BackendPlaybackState::Paused);
    }

    pub fn stop(&self) {
        self.backend.set_state(BackendPlaybackState::Stopped);
    }

    pub fn prev(&self) {
        self.queue().get_previous();
        self.play();
    }

    pub fn next(&self) {
        self.queue().get_next();
        self.play();
    }

    fn reset_current_track(&self) {
        self.set_current_track(None);
        self.committed.set(false);
    }

    pub fn toggle_play_pause(&self) {
        let queue = self.queue();

        if queue.is_empty() {
            return;
        }

        if !self.state().current_track().is_none() {
            if self.backend.state() == BackendPlaybackState::Playing {
                self.backend.set_state(BackendPlaybackState::Paused)
            } else {
                self.backend.set_state(BackendPlaybackState::Playing)
            }
        } else {
            queue.update_from_first();
            if !queue.current_track().is_none() {
                self.play();
            }
        }
    }

    pub fn set_track_position(&self, position_second: f64) {
        //debug!("player set_track_position");
        let mut seek_second = position_second; 
        if position_second < 0.0 {
            seek_second = 0.0;
        }

        match self.backend.duration() {
            Some(d) => {
                if seek_second <= d {
                    self.backend.seek(seek_second as u64);
                }
            },
            None => {
                let d = self.state().current_track().unwrap().duration();
                if seek_second <= d {
                    self.backend.seek(seek_second as u64);
                }
            }
        }

        send!(self.discord_sender, DiscordAction::Seek(position_second));
    }


    fn record_play(&self) {
        if let Some(track) = self.state().current_track() {
            send!(self.lastfm_sender, LastFmAction::Scrobble(track.clone()));
            match database().add_play(track, chrono::offset::Utc::now()) {
                Ok(_) => self.committed.set(true),
                Err(e) => error!("An error occurred adding track to playlist: {}", e),
            };
        }
    }

    pub fn has_next(&self) -> bool {
        return true;
    }

    pub fn has_previous(&self) -> bool {
        return true;
    }

    pub fn set_current_track(&self, track: Option<Rc<Track>>) {
        self.state.set_current_track(track.clone());
        if self.discord_enabled.get() {
            if let Some(track) = track.clone() {
                send!(self.discord_sender, DiscordAction::SetPlaying(track));
            }
        }
        if self.lastfm_enabled.get() {
            debug!("last fm enabled, sending track");
            if let Some(track) = track {
                send!(self.lastfm_sender, LastFmAction::SetNowPlaying(track));
            }
        } else {
            debug!("last fm disabled");
        }
    }

    //set the backend playback state enum property of the player state object
    fn set_state_state(&self, state: BackendPlaybackState) {
        debug!("PLAYER set_state_state {:?}", state);
        if state == BackendPlaybackState::Stopped {
            self.reset_current_track();
        }

        self.state().set_playback_state(state);
    }

    pub fn set_volume(&self, volume: f64) {
        self.backend.set_volume(volume);
        self.state.set_volume(volume);
    }

    pub fn tracks(&self) -> Vec<Rc<Track>> {
        let queue = self.queue();
        queue.tracks()
    }

    pub fn track_ids(&self) -> Vec<i64> {
        let queue = self.queue();
        queue.track_ids()
    }
    
    pub fn lastfm(&self) -> &ResonanceLastFM {
        &self.lastfm
    }

    pub fn discord(&self) -> &ResonanceDiscord {
        &self.discord
    }

    pub fn mpris(&self) -> &MprisController {
        &self.mpris
    }

    pub fn state(&self) -> &PlayerState {
        &self.state
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
    