/* volume_scale.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{gdk, glib, glib::clone, graphene, gsk, prelude::*, subclass::prelude::*};
use std::cell::Cell;

use crate::util::player;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default)]
    pub struct VolumeScalePriv {
        pub natural_width: Cell<i32>,
        pub widget_height: Cell<i32>,
        pub position: Cell<f64>,
        pub white_width: Cell<f32>,
        pub suggest_pos: Cell<f32>,
        pub suggested_visible: Cell<bool>,
        pub init: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeScalePriv {
        const NAME: &'static str = "VolumeScale";
        type Type = super::VolumeScale;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                natural_width: Cell::new(100),
                widget_height: Cell::new(4),
                position: Cell::new(0.0),
                white_width: Cell::new(0.0),
                suggest_pos: Cell::new(0.0),
                suggested_visible: Cell::new(false),
                init: Cell::new(false),
            }
        }

    }

    impl ObjectImpl for VolumeScalePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }


        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("value-changed").param_types([<f32>::static_type()]).build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for VolumeScalePriv {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32,) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                (self.natural_width.get() as i32 / 2 , self.natural_width.get() as i32, -1, -1)
            } else {
                (self.widget_height.get() as i32 / 2, self.widget_height.get() as i32, -1, -1)
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            //let color = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);

            let width = self.obj().width() as f32;
            let height = self.obj().height() as f32;  

            let default_height = self.widget_height.get() as f32;
            let default_y = (height - default_height) / 2.0;

            let selection_color = "#868687";
            let progress_color = "#fefffe";
            let background_color = "#2c2d2d";

            let bg_color = gdk::RGBA::parse(background_color).ok().unwrap();
            
            let rect = graphene::Rect::new(0.0, default_y, width, default_height);
            let rounded_rect = gsk::RoundedRect::from_rect(rect, 5.0);

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
        }
    }

    impl VolumeScalePriv {}
}

glib::wrapper! {
    pub struct VolumeScale(ObjectSubclass<imp::VolumeScalePriv>)
    @extends gtk::Widget;
}

impl VolumeScale {
    pub fn new() -> VolumeScale {
        glib::Object::builder::<VolumeScale>().build()
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        if imp.init.get() {
            return;
        }

        imp.init.set(true);

        self.set_hexpand(true);
        self.set_vexpand(false);
        self.set_margin_start(5);
        self.set_margin_end(5);

        imp.white_width.set(0.0);
        imp.suggest_pos.set(0.0);

        let ctrl_click = gtk::GestureClick::new();

        ctrl_click.connect_released(
            clone!(@strong self as this => move |_gesture, _n_press, x, _y| {
                if  x > 0.0 && x < this.width() as f64 {
                    let new_pos = x / this.width() as f64;
                    this.set_value(new_pos);
                }
            })
        );
        self.add_controller(ctrl_click);

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                this.imp().suggested_visible.set(true);
                this.queue_draw()
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                this.imp().suggested_visible.set(false);
                this.queue_draw()

            })
        );
        ctrl.connect_motion(
            clone!(@strong self as this => move |_controller, x, _y| {
                if x > 0.0 {
                    this.imp().suggest_pos.set(x as f32);
                    this.queue_draw()
                }
            })
        );
        self.add_controller(ctrl);

        let player = player();

        player.state().connect_notify_local(
            Some("volume"),
            clone!(@weak self as this => move |_, _| {
                this.on_volume_change();
            }),
        );

    }


    fn on_volume_change(&self) {
        let player = player();
        let volume = player.state().volume();
        if volume >= 0.0 {
           let display_volume = volume as f32;
           self.emit_by_name::<()>("value-changed", &[&display_volume]);
           self.redraw(display_volume);
        }
       
    }

    fn redraw(&self, volume: f32) {
        let imp = self.imp();
        let mut width = self.width();

        if self.width() <= 0 {
            width = imp.natural_width.get();
        }

        let white_width = width as f32 * volume;
        imp.white_width.replace(white_width);

        self.queue_draw();
    }

    pub fn position(&self) -> f64 {
        self.imp().position.get()
    }

    pub fn set_position(&self, position: f64) {
        let imp = self.imp();
        imp.position.replace(position);
        self.queue_draw();
    }

    pub fn set_value(&self, value: f64) {
        player().set_volume(value)
    }
}
