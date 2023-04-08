/* queue.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::prelude::SettingsExt;
use gtk::{glib, glib::Sender};
use std::{cell::Cell, cell::RefCell, rc::Rc};
use gtk_macros::send;
use log::error;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::model::track::Track;
use crate::util::settings_manager;

use log::debug;

#[derive(Clone, Debug)]
pub enum QueueAction {
    QueueUpdate,
    QueuePositionUpdate(u64),
    QueueTimeRemaining(u64),
    QueueEmpty,
    QueueNonEmpty,
    QueueRepeatMode(RepeatMode),
    QueueDuration(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "RepeatMode")]
pub enum RepeatMode {
    Normal,
    Loop,
    LoopSong,
    Shuffle
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug)]
pub struct Queue {
    pub sender: Sender<QueueAction>,
    pub queue: RefCell<Vec<Rc<Track>>>,
    pub sequential_queue: RefCell<Vec<Rc<Track>>>,
    pub current_position: Cell<u64>,
    pub current_track: RefCell<Option<Rc<Track>>>,
    pub repeat_mode: Cell<RepeatMode>,
    pub shuffle_loop: Cell<bool>,
}

impl Queue {
    pub fn new(queue_sender: Sender<QueueAction>) -> Queue {
        let settings = settings_manager();
        let shuffle_loop = settings.boolean("shuffle-mode-loop");


        let queue = Self {
            sender: queue_sender,
            queue: RefCell::new(Vec::new()),
            sequential_queue: RefCell::new(Vec::new()),
            current_position: Cell::new(0),
            current_track: RefCell::new(None),
            repeat_mode: Cell::new(RepeatMode::default()),
            shuffle_loop: Cell::new(shuffle_loop),
        };
        queue
    }

    pub fn set_shuffle_mode(&self, mode: bool) {
        self.shuffle_loop.set(mode);
    }

    pub fn current_track(&self) -> Option<Rc<Track>> {
        self.current_track.borrow().clone()
    }

    pub fn set_song(&self, track: Rc<Track>) {
        self.current_position.set(0);
        self.clear_queue();
        self.add_track(track.clone());
        send!(self.sender, QueueAction::QueuePositionUpdate(0));
        //self.current_track.replace(Some(track));
    }

    fn clear_queue(&self) {
        self.queue.replace(Vec::new());
        self.sequential_queue.replace(Vec::new());
    }

    pub fn add_track(&self, track: Rc<Track>) {
        self.queue.borrow_mut().push(track.clone());
        self.sequential_queue.borrow_mut().push(track);
        send!(self.sender, QueueAction::QueueNonEmpty);
        send!(self.sender, QueueAction::QueueUpdate);
        self.current_song_update();

    }

    fn current_song_update(&self) {
        let queue =  self.queue.borrow();
        let track = queue[self.current_position.get() as usize].clone();
        
        self.current_track.replace(Some(track));
        send!(self.sender, QueueAction::QueuePositionUpdate(self.position()));
        self.calculate_time_remaining();
    }

    pub fn set_album(&self, tracks: Vec<Rc<Track>>) {
        self.current_position.set(0);
        self.clear_queue();
        self.add_tracks(tracks);
        send!(self.sender, QueueAction::QueuePositionUpdate(0));
    }

    pub fn add_tracks(&self, tracks: Vec<Rc<Track>>) {
        self.sequential_queue.borrow_mut().append(tracks.clone().as_mut());
        self.queue.borrow_mut().append(tracks.clone().as_mut());
        self.current_song_update();
        if self.repeat_mode.get() == RepeatMode::Shuffle {
            self.shuffle_tracks();
        }
        send!(self.sender, QueueAction::QueueNonEmpty);
        send!(self.sender, QueueAction::QueueUpdate);
    }

    pub fn set_position(&self, position: u64) {
        let queue_length = self.queue.borrow().len() as u64;
        if queue_length == 0 {
            self.current_track.replace(None);
            return;
        }
        if self.position() >= queue_length {
            self.current_track.replace(None);
            self.end_queue();
            //self.clear_queue();
            return;
        }
        if position >= queue_length {
            return;
        }

        self.current_position.set(position);
        self.current_song_update();

    }


    pub fn remove_track(&self, position_to_remove: usize) {
        let q_len = self.queue.borrow().len();

        if q_len <= 1 {
            self.current_position.set(0);
            self.current_track.replace(None);
            self.end_queue();
            return;
        }

        let mut current_position = self.position() as usize;

        if current_position < position_to_remove {
            if current_position == 0 {
                current_position = q_len - 2;
            } else {
                current_position -= 1;
            }
        } else {
            if current_position >= (q_len - 1) {
                current_position = 0;
            }
        }

        self.queue.borrow_mut().remove(position_to_remove as usize);
        self.current_position.set(current_position as u64);
        send!(self.sender, QueueAction::QueueUpdate);
        self.current_song_update();
    }

    pub fn reorder_track(&self, old_position: usize, new_position: usize) {
        self.queue.borrow_mut().swap(old_position, new_position);
        self.current_position.set(new_position as u64);
        send!(self.sender, QueueAction::QueueUpdate);
        self.current_song_update();

    }

    pub fn on_repeat_change(&self, mode: RepeatMode) {
        let mode = if mode == self.repeat_mode.get() {
            RepeatMode::Normal
        } else {
            mode
        };
        match mode {
            RepeatMode::Shuffle => {
                self.shuffle_tracks();
                // debug!("Shuffling songs");
                // let remaining_songs = self.queue_len() as i64 - self.current_position.get() as i64 - 1;
                // if remaining_songs <= 0 {
                //     debug!("Nothing left to shuffle");
                //     return;
                // }
                // let mut remaining_songs: Vec<Rc<Track>> = self.queue.borrow_mut().drain((self.current_position.get()+1) as usize..).collect();
                // let mut rng = thread_rng();
                // remaining_songs.shuffle(&mut rng);
                // self.queue.borrow_mut().append(&mut remaining_songs);
            },
            _ => {
                debug!("Restoring queue from sequential");
                self.queue.replace(self.sequential_queue.borrow().clone());
            },
        }
        self.repeat_mode.set(mode);
        if mode == RepeatMode::Shuffle {
            send!(self.sender, QueueAction::QueueUpdate);
        }
        //self.current_song_update();
        send!(self.sender, QueueAction::QueueRepeatMode(mode));
    }

    fn shuffle_tracks(&self) {
        debug!("Shuffling songs");
        let remaining_songs = self.queue_len() as i64 - self.current_position.get() as i64 - 1;
        if remaining_songs <= 0 {
            debug!("Nothing left to shuffle");
            return;
        }
        let mut remaining_songs: Vec<Rc<Track>> = self.queue.borrow_mut().drain((self.current_position.get()+1) as usize..).collect();
        let mut rng = thread_rng();
        remaining_songs.shuffle(&mut rng);
        self.queue.borrow_mut().append(&mut remaining_songs);
    }

    pub fn get_previous(&self) {
        if self.is_empty() {
            self.current_track.replace(None);
            send!(self.sender, QueueAction::QueueEmpty);
            return;
        }
        let queue_length = self.queue.borrow().len() as u64;
        let position = self.current_position.get();
        match self.repeat_mode.get() {
            RepeatMode::Normal => {
                if position <= 0 {
                    self.current_position.set(0);
                } else {
                    self.current_position.set(position-1);
                }
                self.current_song_update();

            },
            RepeatMode::Loop => {
                if position <= 0 {
                    self.current_position.set(queue_length-1);
                } else {
                    self.current_position.set(position-1);
                }
                self.current_song_update();
            },
            RepeatMode::LoopSong => {
                send!(self.sender, QueueAction::QueuePositionUpdate(position));
                self.calculate_time_remaining();
            },
            RepeatMode::Shuffle => {
                if position <= 0 {
                    self.current_position.set(queue_length-1);
                } else {
                    self.current_position.set(position-1);
                }
                self.current_song_update();
            },
        }
    }

    pub fn get_next(&self) {
        if self.is_empty() {
            self.current_track.replace(None);
            send!(self.sender, QueueAction::QueueEmpty);
            return;
        }
        let queue_length = self.queue.borrow().len() as u64;
        let position = self.current_position.get()+1;
        
        match self.repeat_mode.get() {
            RepeatMode::Normal => {
                if position >= queue_length {
                    self.current_track.replace(None);
                    self.end_queue();
                    // send!(self.sender, QueueAction::QueueEmpty);
                    return;
                } else {
                    self.current_position.set(position);
                    self.current_song_update();
                }
            },
            RepeatMode::Loop => {
                if position >= queue_length {
                    self.current_position.set(0);
                } else {
                    self.current_position.set(position);
                }
                self.current_song_update();
            },
            RepeatMode::LoopSong => {
                send!(self.sender, QueueAction::QueuePositionUpdate(position-1));
                self.calculate_time_remaining();
            },
            RepeatMode::Shuffle => {
                if self.shuffle_loop.get() {
                    if position >= queue_length {
                        self.current_position.set(0);
                    } else {
                        self.current_position.set(position);
                    }
                } else {
                    if position >= queue_length {
                        self.current_track.replace(None);
                        self.end_queue();
                        
                        return;
                    } else {
                        self.current_position.set(position);
                    }
                }
                self.current_song_update();
            },
        }
    }

    // def end_queue(self):
    //     self.queue.clear()
    //     self.emit('empty')
    //     self.emit('playlist-update')

    pub fn end_queue(&self) {
        self.clear_queue();
        send!(self.sender, QueueAction::QueueEmpty);
    }
    // def calculate_time_remaining(self, playlist, position):
    //     total_seconds = 0.0
    //     for track in list(self.queue)[position:]:
    //         total_seconds += track.duration
    //     self.emit('time-remaining', int(total_seconds//60))

    fn calculate_time_remaining(&self) {
        let pos = self.position() as usize;
        let tracks = &self.tracks()[pos..];
        let mut total_duration = 0.0;
        for track in tracks.iter() {
            total_duration += track.duration()
        }
        send!(self.sender, QueueAction::QueueDuration(total_duration));
    }

    // def get_all_current_tracks(self) -> list:
    //     return [track.id for track in self.queue]

    pub fn tracks(&self) -> Vec<Rc<Track>> {
        self.queue.borrow().clone()
    }

    pub fn track_ids(&self) -> Vec<i64> {
        let mut ret = Vec::new();
        for track in self.tracks() {
            ret.push(track.id());
        }
        ret
    }


    // @GObject.Property(type=int, default=0, flags=GObject.ParamFlags.READABLE)
    // def position(self):
    //     return self._current_position

    fn position(&self) -> u64 {
        self.current_position.get()
    }

    // @GObject.Property(type=Track, default=None, flags=GObject.ParamFlags.READABLE)
    // def current_song(self):
    //     return self._current_song

    // # REPEAT MODE PROPERTY
    // @GObject.Property(type=int, flags=GObject.ParamFlags.READWRITE)
    // def repeat(self):
    //     return self._repeat

    // @repeat.setter  # type: ignore
    // def repeat(self, _repeat):
    //     self._repeat = _repeat

    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode.get()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.borrow().len() == 0
    }

    fn queue_len(&self) -> usize {
        self.queue.borrow().len()
    }




}
    