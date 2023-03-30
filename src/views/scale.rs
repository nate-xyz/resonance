/* scale.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{gdk, glib, glib::clone, graphene, gsk, prelude::*, subclass::prelude::*};

use std::{cell::Cell, cell::RefCell};
use std::time::Duration;
use log::error;

use crate::player::gst_backend::BackendPlaybackState;
use crate::util::player;

mod imp {
    use super::*;
    use glib::{Value, ParamSpec, ParamSpecUInt64, ParamSpecFloat};
    use once_cell::sync::Lazy;

    #[derive(Debug, Default)]
    pub struct ScalePriv {
        pub position: Cell<f64>,
        pub time_position: Cell<f64>,
        pub old_scale_value: Cell<f64>,
        pub song_duration: Cell<f64>,
        pub white_width: Cell<f32>,
        pub suggest_pos: Cell<f32>,
        pub suggested_visible: Cell<bool>,
        pub scrub_mode: Cell<bool>,
        pub timeout: RefCell<Option<glib::SourceId>>,

        pub id: RefCell<String>,
        pub init: Cell<bool>,
        pub radius: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScalePriv {
        const NAME: &'static str = "Scale";
        type Type = super::Scale;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                position: Cell::new(0.0),
                time_position: Cell::new(0.0),
                old_scale_value: Cell::new(0.0),
                song_duration: Cell::new(0.0),
                white_width: Cell::new(0.0),
                suggest_pos: Cell::new(0.0),
                suggested_visible: Cell::new(false),
                scrub_mode: Cell::new(false),
                timeout: RefCell::new(None),
                id: RefCell::new("".to_string()),
                init: Cell::new(false),
                radius: Cell::new(46.0),
            }
        }

    }

    impl ObjectImpl for ScalePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecUInt64::builder("position").minimum(0).maximum(u64::MAX).read_only().build(),
                    ParamSpecUInt64::builder("time-position").minimum(0).maximum(u64::MAX).read_only().build(),
                    ParamSpecFloat::builder("radius").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "radius" => {
                    let radius = value.get().expect("The value needs to be of type `f32`.");
                    self.radius.set(radius);
                },
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "position" => obj.position().to_value(),
                "time-position" => obj.time_position().to_value(),
                "radius" => self.radius.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for ScalePriv {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32,) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                (100, 500, -1, -1)
            } else {
                (10, 55, -1, -1)
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            //let color = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);

            let width = self.obj().width() as f32;
            let height = self.obj().height() as f32;  

            let default_height = 9.0;
            let default_y = if height > default_height {
                (height - default_height) / 2.0
            } else {
                default_height / 2.0
            };

            let selection_color = "#868687";
            let progress_color = "#fefffe";
            let background_color = "#2c2d2d";

            let bg_color = gdk::RGBA::parse(background_color).ok().unwrap();
            
            let rect = graphene::Rect::new(0.0, default_y, width, default_height);
            let rounded_rect = gsk::RoundedRect::from_rect(rect, self.radius.get());

            snapshot.push_rounded_clip(&rounded_rect);
            snapshot.append_color(&bg_color, &rect);

            let prog_color = gdk::RGBA::parse(progress_color).ok().unwrap();
            let progress_rect = graphene::Rect::new(0.0, default_y, self.white_width.get(), default_height);
            snapshot.append_color(&prog_color, &progress_rect);

            if self.suggested_visible.get() {
                let select_color = gdk::RGBA::parse(selection_color).ok().unwrap();
                if self.white_width.get() > self.suggest_pos.get() {
                    let diff_rect = graphene::Rect::new(
                        f32::max(self.suggest_pos.get(), 0.0),
                        default_y,
                        self.white_width.get() - f32::max(self.suggest_pos.get(), 0.0),
                        default_height,
                    );
                    snapshot.append_color(&select_color, &diff_rect);
                } else {
                    if self.suggest_pos.get() < width {
                        let diff_rect = graphene::Rect::new(
                            self.white_width.get(), default_y, self.suggest_pos.get()-self.white_width.get(), default_height 
                        );
                        snapshot.append_color(&select_color, &diff_rect);
                    } else {
                        let diff_rect = graphene::Rect::new(
                            self.white_width.get(), default_y, width-self.white_width.get(), default_height
                        );
                        snapshot.append_color(&select_color, &diff_rect);
                    }
                }
            } 
            snapshot.pop();

            // let circle_size = 15.0;
            // let progress_circle = graphene::Rect::new(self.white_width.get() - (circle_size / 2.0), (height - circle_size) / 2.0, circle_size, circle_size);
            // let circle_clip = gsk::RoundedRect::from_rect(progress_circle, 90.0);
            // snapshot.push_rounded_clip(&circle_clip);
            // snapshot.append_color(&prog_color, &progress_circle);
            // snapshot.pop();
        }
    }

    //internal impl can go here with internal methods
    impl ScalePriv {}
}

glib::wrapper! {
    pub struct Scale(ObjectSubclass<imp::ScalePriv>)
    @extends gtk::Widget;
}

impl Scale {
    pub fn new() -> Scale {
        glib::Object::builder::<Scale>().build()
    }

    pub fn initialize(&self) {
        let imp = self.imp();
        
        //debug!("SCALE INITIALIZE {}", imp.id.borrow());

        if imp.init.get() {
            // debug!("SCALE INITIALIZE {} {}", imp.init.get(), imp.id.borrow());
            return;
        }
        imp.init.set(true);

        self.bind_state();
        
        self.set_hexpand(true);
        self.set_vexpand(true);
        self.set_margin_start(5);
        self.set_margin_end(5);

        imp.old_scale_value.set(0.0);
        imp.song_duration.set(-1.0);
        imp.white_width.set(0.0);
        imp.suggest_pos.set(0.0);

        let ctrl_click = gtk::GestureClick::new();
        ctrl_click.connect_pressed(clone!(@strong self as this => move |_gesture, _n_press, _x, _y| {
            this.remove_timeout();
            this.imp().old_scale_value.set(this.imp().white_width.get() as f64);
            this.imp().scrub_mode.set(true);
        }));

        ctrl_click.connect_released(clone!(@strong self as this => move |_gesture, _n_press, x, _y| {
            if  x > 0.0 && x < this.width() as f64 {
                let player = player();
                let scale_ratio = x / this.width() as f64;
                let time_position = player.state().duration() * scale_ratio;
                this.set_position(time_position as f64);
                player.set_track_position(time_position);
                this.imp().old_scale_value.set(x);
                
                // if player.state().playback_state() !=    BackendPlaybackState::Playing {
                //     this.set_position(time_position as f64);
                // }
                
                this.update_timeout();
               
            }
            this.imp().scrub_mode.set(false);
        }));
        self.add_controller(ctrl_click);

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(clone!(@strong self as this => move |_controller, _x, _y| {
            this.imp().suggested_visible.set(true);
            this.queue_draw()
        }));

        ctrl.connect_leave(clone!(@strong self as this => move |_controller| {
            this.imp().suggested_visible.set(false);
            this.queue_draw()

        }));

        ctrl.connect_motion(clone!(@strong self as this => move |_controller, x, _y| {
            if x > 0.0 {
                if this.imp().scrub_mode.get() {
                    this.scrub_time_position(x)
                }
                this.imp().suggest_pos.set(x as f32);
                this.queue_draw()
            }
        }));

        self.add_controller(ctrl);
    }

    // Bind the PlayerState to the UI
    fn bind_state(&self) {
        let player = player();

        player.state().connect_notify_local(
            Some("duration"),
            clone!(@strong self as this => move |_, _| {
                this.on_duration_change();
            }),
        );

        player.state().connect_notify_local(
            Some("position"),
            clone!(@strong self as this => move |_, _| {
                this.update_time_position();
            }),
        );


        player.state().connect_notify_local(
            Some("state"),
            clone!(@strong self as this => move |_, _| {
                this.on_state_change();
            }),
        );
    }

    fn on_duration_change(&self) {
        let player = player();
        let duration = player.state().duration();

        if duration != -1.0 {
            self.imp().song_duration.set(duration);
        }
    }

    fn on_state_change(&self) {
        let player = player();
        let state = player.state().playback_state();
        //debug!("SCALE {:?} {}", state, self.imp().id.borrow());
        match state {
            
            BackendPlaybackState::Stopped | BackendPlaybackState::Loading => {
                self.set_position(0.0);
                self.set_sensitive(false);
                self.remove_timeout();
            }
            BackendPlaybackState::Playing => {
                self.set_sensitive(true);
                
                //self.update_position_callback();

                let timeout_duration = Duration::from_millis(2);
                let _source_id = glib::timeout_add_local(timeout_duration,
                    clone!(@strong self as this => @default-return Continue(false) , move || {
                        // debug!("update timeout from playback state == playing {}", this.imp().id.borrow());
                        this.update_timeout();
                    glib::Continue(false)
                    }),
                );
            }
            BackendPlaybackState::Paused => {
                self.set_sensitive(true);
                self.remove_timeout();
            }
        }
    }


    fn update_timeout(&self) {
        let imp = self.imp();
        if let Some(duration)  = player().backend.duration() {
      
            let mut width = self.allocated_width();
            if width <= 0 {
                width = 500;
            }
            
            let timeout_ms = (1000.0 * duration) / width as f64;
            let timeout_duration = Duration::from_millis(timeout_ms as u64);
            
            if !imp.timeout.borrow().as_ref().is_none() {
                self.remove_timeout();
            }

            imp.timeout.replace(Some(glib::timeout_add_local(timeout_duration,
                clone!(@strong self as this => @default-return glib::Continue(false) , move || {
                    // debug!("timeout -> update_position_from_timeout {}", this.imp().id.borrow());
                    this.update_position_from_timeout()
                }),
            )));
            
        } else {
            error!("no duration yet");
            let timeout_duration = Duration::from_millis(5);
            let _source_id = glib::timeout_add_local(timeout_duration,
                clone!(@strong self as this => @default-return Continue(false) , move || {
                    // debug!("update timeout from playback state == playing {}", this.imp().id.borrow());
                    this.update_timeout();
                glib::Continue(false)
                }),
            );
        }
    }

    fn update_position_from_timeout(&self) -> glib::Continue {
        if let Some(position) = player().backend.pipeline_position_in_nsecs() {
            if position > 0 {
                let position = position as f64 / 1_000_000_000.0;
                self.set_position(position);
            }
        }
        glib::Continue(true)
    }

    fn remove_timeout(&self) {
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            timeout.remove();
        }
    }

    pub fn position(&self) -> f64 {
        self.imp().position.get()
    }

    pub fn set_position(&self, position: f64) {
        let imp = self.imp();

        imp.position.replace(position);
        // let position_ratio = position / self.imp().song_duration.get();
        // let white_width = self.width() as f64 * position_ratio;

        let time_ratio = position / player().state().duration();
        let white_width = self.width() as f64 * time_ratio;


        imp.white_width.replace(white_width as f32);

        self.notify("position");

        self.queue_draw();
    }

    fn update_time_position(&self) {
        if !self.imp().scrub_mode.get() {
            let position = player().state().position() as f64;
            self.set_time_position(position);
        }
    }

    pub fn set_time_position(&self, position: f64) {
        self.imp().time_position.replace(position);
        self.notify("time-position");
        self.queue_draw();
    }

    pub fn time_position(&self) -> f64 {
        self.imp().time_position.get()
    }

    fn scrub_time_position(&self, position: f64) {
        if position <= self.width() as f64 {
            let scale_ratio = position / self.width() as f64;
            let duration = player().state().duration();
            let time_position = duration * scale_ratio;
            self.imp().time_position.replace(time_position);
            self.notify("time-position");
        }
    }

    pub fn set_id(&self, id: &str) {
        self.imp().id.replace(id.to_string());
    }
}
