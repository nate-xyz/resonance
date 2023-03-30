/* gst_backend.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 * 
 * Rust rewrite of gstplayer.py from GNOME Music (GPLv2)
 * used my own python rewrite of gnome music gstplayer, Noteworthy rewrite, amberol gst backend for reference
 * See https://gitlab.gnome.org/GNOME/gnome-music/-/blob/master/gnomemusic/gstplayer.py
 * See https://github.com/SeaDve/Noteworthy/blob/main/src/core/audio_player.rs
 * See https://gitlab.gnome.org/World/amberol/-/blob/main/src/audio/gst_backend.rs
 * 
 */

use gtk::{glib, glib::clone, glib::Sender};
use gst::{glib::Continue, prelude::*};
use gtk_macros::send;

use std::{cell::Cell, cell::RefCell, rc::Rc};
use std::time::Duration;
use log::{debug, error};

use super::player::PlaybackAction;

#[derive(Debug, Clone, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "GstPlayerPlaybackState")]
pub enum BackendPlaybackState {
    Stopped,
    Loading,
    Paused,
    Playing,
}

impl Default for BackendPlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug)]
pub struct GstPlayer {
    pub sender: Sender<PlaybackAction>,
    pub pipeline: gst::Pipeline,
    pub state: Cell<BackendPlaybackState>,
    pub clock_id: RefCell<Option<gst::PeriodicClockId>>,
    pub clock: RefCell<Option<gst::Clock>>,
    // pub tick: Cell<u64>,
    pub duration: RefCell<Option<f64>>,
    pub volume: Cell<f64>,

}

impl GstPlayer {
    pub fn new(player_sender: Sender<PlaybackAction>) -> Rc<GstPlayer> {
        let pipeline = gst::ElementFactory::make_with_name("playbin3", None)
            .unwrap()
            .downcast::<gst::Pipeline>()
            .unwrap();
        
        let gstplayer = Rc::new(Self {
            sender: player_sender,
            pipeline,
            state: Cell::new(BackendPlaybackState::default()),
            clock_id: RefCell::new(None),
            clock: RefCell::new(None),
            // tick: Cell::new(0),
            duration: RefCell::new(None),
            volume: Cell::new(0.0),

        });

        gstplayer.clone().connect_bus();
        gstplayer
    }

    pub fn pipeline(&self) -> &gst::Pipeline {
        &self.pipeline
    }


    // STATE
    pub fn set_state(&self, state: BackendPlaybackState) {
        let result = self.set_pipeline_state(state);
        match result {
            Ok(()) =>{
                self.state.set(state);
                //send!(self.sender, PlaybackAction::PlaybackState(self.state.get()));
            },
            Err(e) => error!("{}", e),
        }
    }

    pub fn set_pipeline_state(&self, state: BackendPlaybackState) -> Result<(), gst::StateChangeError> {
        match state {
            BackendPlaybackState::Paused => {
                self.pipeline.set_state(gst::State::Paused)?;
            }
            BackendPlaybackState::Stopped => {
                // Changing the state to NULL flushes the pipeline.
                // Thus, the change message never arrives.
                self.pipeline.set_state(gst::State::Null)?;
            }
            BackendPlaybackState::Loading => {
                //debug!("setting ready");
                self.pipeline.set_state(gst::State::Ready)?;
            }
            BackendPlaybackState::Playing => {
                //debug!("setting playing");
                self.pipeline.set_state(gst::State::Playing)?;
            }
        }
        Ok(())
    }

    pub fn state(&self) -> BackendPlaybackState {
        self.state.get()
    }

    // URI
    pub fn set_uri(&self, uri: String) {
        self.pipeline.set_property("uri", format!("file:{}", uri).to_value());
    }

    //VOLUME
    pub fn set_volume(&self, volume: f64) {
        let mut set_volume = volume.clamp(0.0, 1.0);
        if set_volume <= 0.05 {
            set_volume = 0.0;
        }
        
        let linear_volume = gst_audio::StreamVolume::convert_volume(
            gst_audio::StreamVolumeFormat::Cubic,
            gst_audio::StreamVolumeFormat::Linear,
            set_volume,
        );
        self.pipeline.set_property_from_value("volume", &linear_volume.to_value());
        self.volume.set(linear_volume);
    }

    fn set_volume_internal(&self) {
        self.pipeline.set_property_from_value("volume", &self.volume.get().to_value());
    }


    pub fn volume(&self) -> f64 {
        self.pipeline.property("volume")
    }

    //POSITION
    pub fn pipeline_position_in_nsecs(&self) -> Option<u64> {
        let pos: Option<gst::ClockTime> = {
            // Create a new position query and send it to the pipeline.
            // This will traverse all elements in the pipeline, until one feels
            // capable of answering the query.
            let mut q = gst::query::Position::new(gst::Format::Time);
            if self.pipeline.query(&mut q) {
                Some(q.result())
            } else {
                None
            }
        }
        .and_then(|pos| pos.try_into().ok())?;
        match pos {
            Some(d) => {
                Some(d.nseconds()) 
            },
            None => {
                None
            }
        }

    }

    pub fn pipeline_position(&self) -> Option<u64> {
        let pos: Option<gst::ClockTime> = {
            // Create a new position query and send it to the pipeline.
            // This will traverse all elements in the pipeline, until one feels
            // capable of answering the query.
            let mut q = gst::query::Position::new(gst::Format::Time);
            if self.pipeline.query(&mut q) {
                Some(q.result())
            } else {
                None
            }
        }
        .and_then(|pos| pos.try_into().ok())?;
        match pos {
            Some(d) => {
                Some(d.seconds()) 
            },
            None => {
                None
            }
        }

    }

    //DURATION
    pub fn pipeline_duration(&self) -> Option<f64> {
        let dur: Option<gst::ClockTime> = {
            // Create a new duration query and send it to the pipeline.
            // This will traverse all elements in the pipeline, until one feels
            // capable of answering the query.
            let mut q = gst::query::Duration::new(gst::Format::Time);
            if self.pipeline.query(&mut q) {
                Some(q.result())
            } else {
                None
            }
        }
        .and_then(|dur| dur.try_into().ok())?;
        match dur {
            Some(d) => {
                Some(d.seconds() as f64) 
            },
            None => {
                None
            }
        }
    }

    fn query_duration(&self) -> glib::Continue {
        match self.pipeline_duration() {
            Some(duration) => {
                self.duration.replace(Some(duration));
                glib::Continue(false)
            },
            None => {
                self.duration.replace(None);
                glib::Continue(true)
            }
        } 
    }

    pub fn duration(&self) -> Option<f64> {
        self.duration.borrow().clone()
    }

    //BUS SETUP
    fn connect_bus(self: Rc<Self>) {
        let bus = self.pipeline.bus().unwrap();
        bus.add_watch_local(
            clone!(@strong self as this => @default-return Continue(false), move |_, message| {
                let backend = this.clone();
                backend.handle_bus_message(message)
            }),
        )
        .unwrap();
        //debug!("connect bus")
    }

    fn handle_bus_message(self: Rc<Self>, message: &gst::Message) -> Continue {
        use gst::MessageView;

        match message.view() {
            MessageView::Error(ref message) => self.on_bus_error(message),
            MessageView::Eos(_) => self.on_bus_eos(),
            MessageView::StateChanged(ref message) => self.on_state_changed(message),
            MessageView::NewClock(ref message) => self.on_new_clock(message),
            // MessageView::Element(ref message) => self.on_bus_element(message),
            MessageView::StreamStart(_) => self.on_stream_start(),
            _ => (),
        }

        Continue(true)
    }

    fn on_stream_start(self: Rc<Self>) {
        //debug!("BACKEND on_stream_start");
        let timeout_duration = Duration::from_millis(1);
        let _source_id = glib::timeout_add_local(timeout_duration,
            clone!(@strong self as this => @default-return Continue(false) , move || {
                this.query_duration()
            }),
        );
    }

    //CLOCK STUFF

    fn on_new_clock(&self, message: &gst::message::NewClock) {
        //debug!("on new clock");
        self.clock_id.replace(None);
        self.clock.replace(message.clock());
        let clock = self.clock.borrow();
        match clock.as_ref() {
            Some(clock) => {
                let clock_id = clock.new_periodic_id(clock.time().unwrap(), gst::ClockTime::from_seconds(1));
                
                self.clock_id.replace(Some(clock_id));

                //let player_sender = Rc::new(self.player_sender.borrow().as_ref().unwrap());
                self.clock_id.borrow().as_ref().unwrap().wait_async(
                    clone!(@strong self.sender as sender => move |_clock, time, _id| {
                        if let Some(time) = time {
                            let sec = time.seconds();
                            match sender.send(PlaybackAction::Tick(sec)) {
                                Ok(()) => (),
                                Err(err) => error!("{}", err)
                            }
                            //send!(sender, PlaybackAction::Tick(time.seconds()));
                        }          
                    }),
                ).expect("Failed to wait async");

                
            }
            None => {
                return;
            }
        }
    }


    fn on_bus_error(&self, message: &gst::message::Error) {
        let error = message.error();
        let debug = message.debug();

        error!("Error from element `{}`: {:?}", message.src().unwrap().name(), error);

        if let Some(debug) = debug {
            debug!("Debug info: {}", debug);
        }

        ////debug!("Error while playing audio with uri `{}`", self.imp().uri());

        self.set_state(BackendPlaybackState::Stopped);
        send!(self.sender, PlaybackAction::Error);
        //self.emit_by_name::<()>("error", &[]);
    }

    fn on_bus_eos(&self) {
        self.set_state(BackendPlaybackState::Stopped);
        send!(self.sender, PlaybackAction::EOS);
        //self.emit_by_name::<()>("eos", &[]);
    }

    fn on_state_changed(&self, message: &gst::message::StateChanged) {
        if message.src() != Some(self.pipeline.upcast_ref::<gst::Object>()) {
            return;
        }


        let new_state = message.current();

        debug!("BACKEND state changed: `{:?}` -> `{:?}`", message.old(), new_state);

        let state = match new_state {
            gst::State::Null => BackendPlaybackState::Stopped,
            gst::State::Ready => BackendPlaybackState::Loading,
            gst::State::Paused => BackendPlaybackState::Paused,
            gst::State::Playing => BackendPlaybackState::Playing,
            _ => return,
        };

        self.state.set(state);

        //pipeline will change volume sometimes?
        if state == BackendPlaybackState::Playing && self.volume() != self.volume.get() {
            debug!("RESET VOLUME {:?}, {:?}", self.volume.get(), self.volume());
            self.set_volume_internal();
            //self.set_volume(self.volume.get())
        }

        send!(self.sender, PlaybackAction::PlaybackState(self.state.get()));

    }

    pub fn seek(&self, seconds: u64) {
        //self._seek = self.pipeline.seek_simple(Gst.Format.TIME, Gst.SeekFlags.FLUSH | Gst.SeekFlags.KEY_UNIT, seconds * Gst.SECOND)

        match self.pipeline.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, seconds * gst::ClockTime::SECOND) {
            Ok(_) => {
                debug!("seek success");
            },
            Err(_) => ()
        }
    }
}
