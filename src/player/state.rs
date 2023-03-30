/* state.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 * 
 * Thanks to 2022 Emmanuele Bassi (amberol)
 * 
 */

 use gtk::{glib, prelude::*, subclass::prelude::*};

use std::{cell::Cell, cell::RefCell, rc::Rc};
use log::debug;

use crate::model::track::Track;

use super::gst_backend::BackendPlaybackState;
use super::queue::RepeatMode;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        ParamSpec, ParamSpecBoolean, ParamSpecDouble, ParamSpecEnum, ParamSpecObject,
        ParamSpecString, ParamSpecFloat, ParamSpecUInt64,
    };
    use once_cell::sync::Lazy;


    #[derive(Debug)]
    pub struct PlayerState {
        pub repeat_mode: Cell<RepeatMode>,
        pub playback_state: Cell<BackendPlaybackState>,
        pub position: Cell<u64>,
        pub current_track: RefCell<Option<Rc<Track>>>,
        pub volume: Cell<f64>,
        pub empty: Cell<bool>,
        pub queue_title: RefCell<String>,
        pub queue_time_remaining: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerState {
        const NAME: &'static str = "PlayerState";
        type Type = super::PlayerState;
        type ParentType = glib::Object;

        fn new() -> Self {
            Self {
                repeat_mode: Cell::new(RepeatMode::Normal),
                playback_state: Cell::new(BackendPlaybackState::Stopped),
                position: Cell::new(0),
                current_track: RefCell::new(None),
                volume: Cell::new(-1.0),
                empty: Cell::new(true),
                queue_title: RefCell::new(String::new()),
                queue_time_remaining: Cell::new(0.0),
            }
        }
    }

    impl ObjectImpl for PlayerState {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("playing").read_only().explicit_notify().build(),
                    ParamSpecUInt64::builder("position").minimum(0).maximum(u64::MAX).read_only().explicit_notify().build(),
                    ParamSpecObject::builder::<Track>("song").read_only().explicit_notify().build(),
                    ParamSpecString::builder("title").read_only().explicit_notify().build(),
                    ParamSpecString::builder("artist").read_only().explicit_notify().build(),
                    ParamSpecString::builder("album").read_only().explicit_notify().build(),
                    ParamSpecUInt64::builder("duration").minimum(0).maximum(u64::MAX).default_value(0).read_only().explicit_notify().build(),
                    ParamSpecUInt64::builder("cover").minimum(0).maximum(u64::MAX).default_value(0).read_only().explicit_notify().build(),
                    ParamSpecDouble::builder("volume").minimum(0.0).maximum(1.0).default_value(1.0).read_only().explicit_notify().build(),
                    ParamSpecEnum::builder::<BackendPlaybackState>("state").read_only().explicit_notify().build(),
                    ParamSpecEnum::builder::<RepeatMode>("repeat-mode").read_only().explicit_notify().build(),
                    ParamSpecString::builder("queue-title").read_only().explicit_notify().build(),
                    ParamSpecFloat::builder("queue-time-remaining").read_only().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "playing" => obj.playing().to_value(),
                "position" => obj.position().to_value(),
                "song" => self.current_track.borrow().as_ref().unwrap().to_value(),
                "volume" => obj.volume().to_value(),
                "state" => obj.playback_state().to_value(),
                "repeat-mode" => obj.repeat_mode().to_value(),
                "queue-title" => obj.queue_title().to_value(),
                "queue-time-remaining" => obj.queue_time_remaining().to_value(),

                // These are proxies for Rc<Track> properties
                "title" => obj.title().to_value(),
                "artist" => obj.artist().to_value(),
                "album" => obj.album().to_value(),
                "duration" => obj.duration().to_value(),
                "cover" => obj.cover().unwrap().to_value(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("queue-update").build(),
                    Signal::builder("queue-empty").build(),
                    Signal::builder("queue-nonempty").build(),
                    Signal::builder("queue-position")
                        .param_types([<u64>::static_type()])
                        .build(),
                    Signal::builder("queue-repeat-mode")
                        .param_types([<RepeatMode>::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }
}

// PlayerState is a GObject that we can use to bind to
// widgets and other objects; it contains the current
// state of the audio player: song metadata, playback
// position and duration, etc.
glib::wrapper! {
    pub struct PlayerState(ObjectSubclass<imp::PlayerState>);
}

impl Default for PlayerState {
    fn default() -> Self {
        glib::Object::builder::<PlayerState>().build()
    }
}

impl PlayerState {
    pub fn title(&self) -> Option<String> {
        if let Some(song) = &*self.imp().current_track.borrow() {
            return Some(song.title());
        }

        None
    }

    pub fn artist(&self) -> Option<String> {
        if let Some(song) = &*self.imp().current_track.borrow() {
            return Some(song.artist());
        }

        None
    }

    pub fn album(&self) -> Option<String> {
        if let Some(song) = &*self.imp().current_track.borrow() {
            return Some(song.album());
        }

        None
    }

    pub fn duration(&self) -> f64 {
        if let Some(song) = &*self.imp().current_track.borrow() {
            return song.duration();
        }

        0.0
    }

    pub fn cover(&self) -> Option<i64> {
        if let Some(song) = &*self.imp().current_track.borrow() {
            return song.cover_art_option();
        }

        None
    }

    pub fn playing(&self) -> bool {
        let playback_state = self.imp().playback_state.get();
        matches!(playback_state, BackendPlaybackState::Playing)
    }

    pub fn set_playback_state(&self, playback_state: BackendPlaybackState) {
        debug!("STATE set_playback_state {:?}", playback_state);
        let old_state = self.playback_state();
        self.imp().playback_state.set(playback_state);
        if old_state != playback_state {
            self.notify("playing");
            self.notify("state");
        }
    }

    // pub fn set_playback_state(&self, playback_state: &BackendPlaybackState) -> bool {
    //     let old_state = self.imp().playback_state.replace(*playback_state);
    //     if old_state != *playback_state {
    //         self.notify("playing");
    //         self.notify("state");
    //         return true;
    //     }

    //     false
    // }

    pub fn playback_state(&self) -> BackendPlaybackState {
        self.imp().playback_state.get()
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.imp().repeat_mode.get()
    }

    pub fn current_track(&self) -> Option<Rc<Track>> {
        (*self.imp().current_track.borrow()).as_ref().cloned()
    }

    pub fn set_current_track(&self, song: Option<Rc<Track>>) {
        let imp = self.imp();

        imp.current_track.replace(song.clone());
        imp.position.replace(0);
        self.notify("song");
        self.notify("title");
        self.notify("artist");
        self.notify("album");
        self.notify("duration");
        self.notify("cover");
        self.notify("position");
    }

    pub fn position(&self) -> u64 {
        self.imp().position.get()
    }

    pub fn set_position(&self, position: u64) {
        self.imp().position.replace(position);
        self.notify("position");
    }

    pub fn volume(&self) -> f64 {
        self.imp().volume.get()
    }

    pub fn set_volume(&self, volume: f64) {
        let old_volume = self.imp().volume.replace(volume);

        // We only care about two digits of precision, to avoid
        // notification cycles when we update the volume with a
        // similar value coming from the volume control
        let old_rounded = format!("{:.2}", old_volume);
        let new_rounded = format!("{:.2}", volume);
        if old_rounded != new_rounded {
            self.notify("volume");
        }
    }

    pub fn queue_update(&self) {
        debug!("queue_update");
        self.emit_by_name::<()>("queue-update", &[]);
    }

    pub fn queue_empty(&self) {
        debug!("queue_empty");
        self.set_empty(true);
        self.emit_by_name::<()>("queue-empty", &[]);
    }

    pub fn queue_nonempty(&self) {
        debug!("queue_nonempty");
        self.set_empty(false);
        self.emit_by_name::<()>("queue-nonempty", &[]);
    }

    pub fn queue_position_update(&self, position: u64) {
        self.emit_by_name::<()>("queue-position", &[&position]);
    }

    pub fn queue_repeat_mode_update(&self, mode: RepeatMode) {
        self.imp().repeat_mode.set(mode);
        self.emit_by_name::<()>("queue-repeat-mode", &[&mode]);
        self.notify("repeat-mode");
    }


    fn set_empty(&self, empty: bool) {
        self.imp().empty.set(empty)
    }

    pub fn empty(&self) -> bool {
        self.imp().empty.get()
    }

    pub fn set_queue_title(&self, title: Option<String>) {
        let imp = self.imp();
        
        let queue_title = self.queue_title();
        let new_title: String = match title {
            Some(new_tile) => {
                new_tile
            },
            None => {
                "Playlist".to_string()
            },
        };

        if queue_title != new_title {
            imp.queue_title.replace(new_title);
            self.notify("queue-title");
        }
    }

    pub fn queue_title(&self) -> String {
        self.imp().queue_title.borrow().clone()
    }


    pub fn set_queue_time_remaining(&self, time: f64) {
        self.imp().queue_time_remaining.set(time);
        self.notify("queue-time-remaining");
    }


    pub fn queue_time_remaining(&self) -> f32 {
        self.imp().queue_time_remaining.get() as f32
    }

}
