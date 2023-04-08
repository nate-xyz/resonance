/* mpris_controller.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 * 
 * Thanks to 2022 Emmanuele Bassi (amberol)
 * 
 */

use gtk::{glib, glib::{clone, Sender, Receiver}};
use gtk_macros::send;

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};
use log::{debug, error};

use mpris_player::{LoopStatus, Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};

use crate::model::track::Track;
use crate::web::music_brainz::MusicBrainzAction;

use super::player::PlaybackAction;
use super::gst_backend::BackendPlaybackState;
use super::queue::RepeatMode;


#[derive(Debug)]
pub struct MprisController {
    sender: Sender<PlaybackAction>,
    mpris: Arc<MprisPlayer>,
    current_track: RefCell<Option<Rc<Track>>>,
    mb_sender: Sender<MusicBrainzAction>,
    mb_receiver: RefCell<Option<Receiver<(i64, String)>>>,
}

impl MprisController {
    pub fn new(sender: Sender<PlaybackAction>, mb_sender: Sender<MusicBrainzAction>, mb_receiver: Receiver<(i64, String)>) -> Rc<Self> {
        let mpris = MprisPlayer::new(
            "io.github.nate_xyz.Resonance".to_string(),
            "Resonance".to_string(),
            "io.github.nate_xyz.Resonance".to_string(),
        );

        mpris.set_can_raise(true);
        mpris.set_can_play(false);
        mpris.set_can_pause(true);
        mpris.set_can_seek(true);
        mpris.set_can_go_next(true);
        mpris.set_can_go_previous(true);
        mpris.set_can_set_fullscreen(false);

        let res = Self {
            sender,
            mpris,
            current_track: RefCell::new(None),
            mb_sender,
            mb_receiver: RefCell::new(Some(mb_receiver)),
        };

        res.setup_signals();

        let rc_res = Rc::new(res);
        rc_res.clone().setup_channel();
        rc_res
    }

    pub fn setup_channel(self: Rc<Self>) {
        let receiver = self.mb_receiver.borrow_mut().take().unwrap();
        receiver.attach(
            None,
            clone!(@strong self as this => move |(album_id, mb_id)| this.receive_id(album_id, mb_id)),
        );
    }

    fn setup_signals(&self) {
        self.mpris.connect_play_pause(
            clone!(@weak self.mpris as mpris, @strong self.sender as sender => move || {
                send!(sender, PlaybackAction::TogglePlayPause)
            }),
        );


        self.mpris.connect_play(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::TogglePlayPause);
            })
        );

        self.mpris.connect_stop(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Stop);
            })
        );

        self.mpris.connect_pause(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Pause);
            })
        );

        self.mpris.connect_previous(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::SkipPrevious);
            })
        );

        self.mpris.connect_next(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::SkipNext);
            }));

        self.mpris.connect_raise(
            clone!(@strong self.sender as sender => move || {
                send!(sender, PlaybackAction::Raise);
            })
        );

        self.mpris.connect_loop_status(clone!(@strong self.sender as sender => move |status| {
                let mode = match status {
                    LoopStatus::None => RepeatMode::Normal,
                    LoopStatus::Track => RepeatMode::LoopSong,
                    LoopStatus::Playlist => RepeatMode::Loop,
                };
                send!(sender, PlaybackAction::QueueRepeatMode(mode));
            })
        );

        self.mpris.connect_seek(clone!(@strong self.sender as sender => move |position| {
                let pos = Duration::from_micros(position as u64).as_secs();
                send!(sender, PlaybackAction::Seek(pos));
            })
        );

        self.mpris.connect_volume(clone!(@strong self.sender as sender => move |volume| {
                debug!("mpris set volume {}", volume);
            })
        );
    }

    fn update_metadata(&self) {
        let mut metadata = Metadata::new();
        if let Some(track) = self.current_track.take() {
            metadata.length = Some(track.duration() as i64);
            metadata.album = Some(track.album());
            metadata.artist = Some(vec![track.artist()]);
            metadata.disc_number = Some(track.disc_number() as i32);
            metadata.genre = Some(vec![track.genre()]);
            metadata.title = Some(track.title());
            metadata.track_number = Some(track.track_number() as i32);
            metadata.art_url = Some("https://upload.wikimedia.org/wikipedia/commons/b/b6/12in-Vinyl-LP-Record-Angle.jpg".to_string());

            //let length = Duration::from_secs().as_micros() as i64;
            self.mpris.set_metadata(metadata);
            self.current_track.replace(Some(track.clone()));
            send!(self.mb_sender, MusicBrainzAction::FindRelease((true, track)));
        }        
    }

    fn receive_id(&self, album_id: i64, art_url: String) -> glib::Continue {
        if let Some(track) = self.current_track.borrow().as_ref() {
            if track.album_id() == album_id {
                let mut metadata = Metadata::new();
                metadata.length = Some(track.duration() as i64);
                metadata.album = Some(track.album());
                metadata.artist = Some(vec![track.artist()]);
                metadata.disc_number = Some(track.disc_number() as i32);
                metadata.genre = Some(vec![track.genre()]);
                metadata.title = Some(track.title());
                metadata.track_number = Some(track.track_number() as i32);
                metadata.art_url = Some(art_url);
                self.mpris.set_metadata(metadata);
            }
        }
        glib::Continue(true)
    }


    pub fn set_playback_state(&self, state: &BackendPlaybackState) {
        self.mpris.set_can_play(true);

        match state {
            BackendPlaybackState::Playing => self.mpris.set_playback_status(PlaybackStatus::Playing),
            BackendPlaybackState::Paused => self.mpris.set_playback_status(PlaybackStatus::Paused),
            _ => self.mpris.set_playback_status(PlaybackStatus::Stopped),
        };
    }

    pub fn set_track(&self, track: Option<Rc<Track>>) {
        self.current_track.replace(track.clone());
        self.update_metadata();
    }

    pub fn set_position(&self, position: u64) {
        let msecs = Duration::from_secs(position).as_micros();
        self.mpris.set_position(msecs as i64);
    }

    pub fn set_repeat_mode(&self, repeat: RepeatMode) {
        match repeat {
            RepeatMode::Normal => self.mpris.set_loop_status(LoopStatus::None),
            RepeatMode::LoopSong => self.mpris.set_loop_status(LoopStatus::Track),
            RepeatMode::Loop => self.mpris.set_loop_status(LoopStatus::Playlist),
            RepeatMode::Shuffle => self.mpris.set_loop_status(LoopStatus::Playlist),
        }
    }
}