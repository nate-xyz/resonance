/* player.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;

use gst::prelude::ObjectExt;
use gtk::{glib, glib::{clone, Receiver, Sender}};

use std::{cell::Cell, cell::RefCell, rc::Rc};
use log::{debug, error};
use gtk_macros::send;

use crate::model::track::Track;
use crate::web::music_brainz::ResonanceMusicBrainz;
use crate::web::discord::{ResonanceDiscord, DiscordAction};
use crate::web::last_fm::{ResonanceLastFM, LastFmAction};
use crate::util::{database, settings_manager};

use super::gst_backend::GstPlayer;
use super::gst_backend::BackendPlaybackState;
use super::queue::{Queue, QueueAction};
use super::state::PlayerState;
use super::queue::RepeatMode;
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

    //set the backend playback state enum property of the player state object
    fn set_state_state(&self, state: BackendPlaybackState) {
        debug!("PLAYER set_state_state {:?}", state);
        self.state().set_playback_state(state);
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
                //self.reset_current_track();
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

    pub fn tracks(&self) -> Vec<Rc<Track>> {
        let queue = self.queue();
        queue.tracks()
    }

    pub fn track_ids(&self) -> Vec<i64> {
        let queue = self.queue();
        queue.track_ids()
    }

    fn update_tick(&self, tick: u64) {
        //self.state.set_tick(tick);
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



    // #RESET PLAYLIST AND PLAY TRACK
    // def clear_play_track(self, track, title=None):
    //     if title != None:
    //         self.send_toast(title, timeout=1)
    //     self._playlist.set_song(track)
    //     self.play()
    //     self.reset_title()
    //     self.notify_queue_page()

    pub fn clear_play_track(&self, track: Rc<Track>) {
        self.state().set_queue_title(Some(track.title()));
        self.queue().set_song(track);
        self.play();
    }

    // #ADD TRACK TO END OF THE PLAYLIST
    // def add_track(self, track, title=None):
    //     if title != None:
    //         self.send_toast(title, now_playing=False, timeout=1)
    //     self._playlist.add_track(track)
    //     self.reset_title()
    //     self.notify_queue_page()

    pub fn add_track(&self, track: Rc<Track>) {
        self.queue().add_track(track);
        self.state().set_queue_title(None);
    }

    // #RESET PLAYLIST AND PLAY ALBUM
    // def clear_play_album(self, tracks, title="Playlist"):
    //     if title != "Playlist":
    //         self.send_toast(title)
    //     _tracks = deque()
    //     for i in sorted(tracks.keys()):
    //         _tracks.append(tracks[i])
    //     self._playlist.set_album(_tracks)
    //     self.play()

    //     if title != self._playlist_title:
    //         self._playlist_title = title 
    //         self.emit('playlist-title-change', self._playlist_title)

    //     self.notify_queue_page()

    pub fn clear_play_album(&self, tracks: Vec<Rc<Track>>, title: Option<String>) {
        self.queue().set_album(tracks);
        self.state().set_queue_title(title);
        self.play();
    }

    // #ADD ALBUM TO END OF THE PLAYLIST
    // def add_album(self, tracks, title=None, additional_disc=False):
    //     if title != None:
    //         self.send_toast(title, now_playing=False, timeout=1.5)
    //     _tracks = deque()
    //     for i in sorted(tracks.keys()):
    //         _tracks.append(tracks[i])
    //     self._playlist.add_tracks(_tracks)

    //     if not additional_disc:
    //         self.reset_title()

    //     self.notify_queue_page()

    pub fn add_album(&self, tracks: Vec<Rc<Track>>) {
        self.queue().add_tracks(tracks);
        self.state().set_queue_title(None);
    }

    // #GO TO POSITION IN THE PLAYLIST
    // def go_to_playlist_position(self, position):
    //     self._playlist.set_position(position)
    //     self.play()

    pub fn go_to_playlist_position(&self, position: u64) {
        debug!("PLAYER go_to_playlist_position");
        let queue = self.queue();
        queue.set_position(position);
        self.play();
    }

    // def remove_track_from_playlist(self, position):
    //     if position == self._playlist._current_position:
    //         if len(self._playlist.queue) <= 1:
    //             self.next()
    //             return
    //         if self._playlist._repeat == RepeatMode.NORMAL and position == len(self._playlist.queue)-1:
    //             if self.has_previous: self.prev()
    //             else: self.stop()
    //         else:
    //             if self.has_next: self.next()
    //             elif self.has_previous: self.prev()
    //             else: self.stop()
    //     self._playlist.remove_track(position)

    // def play(self):
    //     self._reset_current_track()
    //     #get current track from playlist
    //     track = self._playlist.current_song
    //     if track == None:
    //         self.stop()
    //         self.emit('song-changed')
    //         return
    //     else:            
    //         self._load_uri(track.uri)
    //         self._gst_player._file_duration = track.duration
    //         self._gst_player.props.state = Playback.PLAYING
    //         self._current_track = track
    //         self.emit('song-changed')
    //         return


    pub fn play(&self) {
        self.committed.set(false);
        let queue = self.queue();
        match queue.current_track() {
            Some(track) => {
                self.backend.set_state(BackendPlaybackState::Loading);
                self.backend.set_uri(track.uri());
                self.backend.set_state(BackendPlaybackState::Playing);
                self.set_current_track(Some(track.clone()));
            }, 
            None => {
                self.reset_current_track();
                self.stop();
            }
        };
    }
    // def pause(self):
    //     self._gst_player.props.state = Playback.PAUSED

    fn pause(&self) {
        self.backend.set_state(BackendPlaybackState::Paused);
    }

    // def stop(self):
    //     self._reset_current_track()
    //     self._gst_player.props.state = Playback.STOPPED

    pub fn stop(&self) {
        self.reset_current_track();
        self.backend.set_state(BackendPlaybackState::Stopped);
    }

    // def prev(self):
    //     self._playlist.get_previous()
    //     self.play()

    pub fn prev(&self) {
        self.queue().get_previous();
        self.play();
    }

    // def next(self):
    //     self._playlist.get_next()
    //     self.play()

    pub fn next(&self) {
        self.queue().get_next();
        self.play();
    }

    // def _reset_current_track(self):
    //     self._current_track = None
    //     self._current_track_progress = 0.0
    //     self._commited_already = False

    fn reset_current_track(&self) {
        self.set_current_track(None);
        self.committed.set(false);
    }

    // #Called from play method, loads track uri into GstPlayer
    // def _load_uri(self, uri):
    //     self._gst_player.props.state = Playback.LOADING
    //     self._gst_player.props.url = uri

    // def toggle_play_pause(self):
    //     if len(self._playlist.queue) == 0:
    //         return
    //     if self.props.state == Playback.PLAYING:
    //         self._gst_player.props.state = Playback.PAUSED
    //     else:
    //         self._gst_player.props.state = Playback.PLAYING

    pub fn toggle_play_pause(&self) {
        if self.queue().is_empty() {
            return;
        }

        if self.backend.state() == BackendPlaybackState::Playing {
            self.backend.set_state(BackendPlaybackState::Paused)
        } else {
            self.backend.set_state(BackendPlaybackState::Playing)
        }

    }

    // def get_track_position(self):
    //     return self._gst_player.props.position

    // def set_track_position(self, position_second):
    //     if position_second < 0.0:
    //         position_second = 0.0

    //     duration_second = self._gst_player.props.duration
    //     if position_second <= duration_second:
    //         self._gst_player.seek(position_second)

    pub fn set_track_position(&self, position_second: f64) {
        debug!("player set_track_position");
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

    // def notify_queue_page(self):
    //     active_window = self._window
    //     if not active_window.queue_stack_page.get_needs_attention(): active_window.queue_stack_page.set_needs_attention(True)

    // def send_toast(self, message: str, now_playing=True, timeout=None):
    //     if now_playing:
    //         message = "Now Playing “{}”".format(message)
    //     else:
    //         message = "Added “{}” to Queue".format(message)
    //     if timeout != None:
    //         self._window.add_toast(message, timeout)
    //         return
    //     self._window.add_toast(message)

    // def reset_title(self):
    //     if self._playlist_title != 'Playlist':
    //         self._playlist_title = 'Playlist' 
    //         self.emit('playlist-title-change', self._playlist_title)

    // def _on_error(self, _gst_player):
    //     print('_on_error')
    //     self.reset_gst_player()
    //     self.stop()
    //     self.emit('song-changed')

    //     # if (self.has_next and self.props.repeat != RepeatMode.LOOP_SONG):
    //     #     self.next()

    // def _on_eos(self, _gst_player):
    //     print('_on_eos')
    //     self.next()
    //     # self._playlist.next()

    // def _on_clock_tick(self, _gst_player, tick):
    //     if self.props.state == Playback.PLAYING and self._current_track != None:
    //         self._current_track_progress = tick / self._current_track.duration
    //         if self._current_track_progress > self.play_commit_threshold and not self._commited_already:
    //             self._record_play(self._current_track)
        
    // def _record_play(self, track):
    //     current_time = datetime.datetime.now()
    //     self._database.add_play(current_time, track)
    //     self._commited_already = True

    fn record_play(&self) {
        if let Some(track) = self.state().current_track() {
            send!(self.lastfm_sender, LastFmAction::Scrobble(track.clone()));
            match database().add_play(track, chrono::offset::Utc::now()) {
                Ok(_) => self.committed.set(true),
                Err(e) => error!("An error occurred adding track to playlist: {}", e),
            };
        }
    }

    // @GObject.Property(type=bool, default=False, flags=GObject.ParamFlags.READABLE)
    // def has_next(self):
    //     return True if len(self._playlist.queue) != 0 else False

    pub fn has_next(&self) -> bool {
        return true;
    }
    // @GObject.Property(type=bool, default=False, flags=GObject.ParamFlags.READABLE)
    // def has_previous(self):
    //     return True if len(self._playlist.queue) != 0 else False

    pub fn has_previous(&self) -> bool {
        return true;
    }

    // @GObject.Property(type=Track, default=None, flags=GObject.ParamFlags.READABLE)
    // def current_song(self):
    //     return self._playlist.props.current_song

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

    // @GObject.Property(type=float, default=None, flags=GObject.ParamFlags.READABLE)
    // def volume(self):
    //     return self._gst_player.props.volume

    // def set_volume(self, volume: float):
    //     self._gst_player.props.volume = volume
    //     self.emit('volume-changed', self._gst_player.props.volume)

    pub fn set_volume(&self, volume: f64) {
        self.backend.set_volume(volume);
        self.state.set_volume(volume);
    }

    // def reset_gst_player(self):
    //     del self._gst_player
    //     gc.collect()
    //     self._gst_player = GstPlayer()
    //     self.connect_gst_player()
    //     self.set_volume(self.default_volume)

    // # @GObject.Property(type=object)
    // # def repeat_mode(self) -> RepeatMode:
    // #     return self._repeat

    // # @repeat_mode.setter  # type: ignore
    // # def repeat_mode(self, mode):
    // #     if mode == self._repeat:
    // #         return
    // #     self._repeat = mode


}
    