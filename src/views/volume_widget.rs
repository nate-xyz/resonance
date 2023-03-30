/* volume_widget.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

// use adw::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use std::cell::Cell;

use super::volume_scale::VolumeScale;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    
    #[derive(Debug)]
    pub struct VolumeWidgetPriv {
        pub muted_img: gtk::Image,
        pub low_img: gtk::Image,
        pub medium_img: gtk::Image,
        pub high_img: gtk::Image,
        pub scale: VolumeScale,
        pub current_volume: Cell<f64>,
        pub revealer: gtk::Revealer,
        pub button: gtk::Button,


    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeWidgetPriv {
        const NAME: &'static str = "VolumeWidget";
        type Type = super::VolumeWidget;
        type ParentType = gtk::Box;

        fn new() -> Self {
            let high_img =  gtk::Image::from_icon_name("audio-volume-high-symbolic");
            let scale = VolumeScale::new();
            let revealer = gtk::Revealer::new();
            revealer.set_transition_type(gtk::RevealerTransitionType::SlideRight);
            revealer.set_reveal_child(false);
            revealer.set_child(Some(&scale));
            revealer.set_visible(false);
            let button = gtk::Button::new();
            button.set_css_classes(&["flat", "circular"]);
            button.set_child(Some(&high_img));

            Self {
                muted_img: gtk::Image::from_icon_name("audio-volume-muted-symbolic"),
                low_img: gtk::Image::from_icon_name("audio-volume-low-symbolic"),
                medium_img: gtk::Image::from_icon_name("audio-volume-medium-symbolic"),
                high_img: high_img,
                scale: scale,
                current_volume: Cell::new(0.0),
                revealer: revealer,
                button: button,
            }
        }
    }

    impl ObjectImpl for VolumeWidgetPriv {
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

    impl WidgetImpl for VolumeWidgetPriv {}
    impl BoxImpl for VolumeWidgetPriv {}
    impl VolumeWidgetPriv {}
}

glib::wrapper! {
    pub struct VolumeWidget(ObjectSubclass<imp::VolumeWidgetPriv>)
    @extends gtk::Box, gtk::Widget;
}


impl VolumeWidget {
    pub fn new() -> VolumeWidget {
        let volume_widget: VolumeWidget = glib::Object::builder::<VolumeWidget>().build();
        volume_widget
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        self.set_spacing(5);
        self.append(&imp.button);
        self.append(&imp.revealer);


        imp.scale.connect_local(
            "value-changed",
            false,
            glib::clone!(@weak self as this => @default-return None, move |value| {
                let int = value.get(1);
                match int {
                    Some(int) => {
                        let int = int.get::<f32>().ok().unwrap();
                        this.on_scale_value_change(int as f64);
                    },
                    None => (),
                }
                None
            }),
        );

        imp.button.connect_clicked(glib::clone!(@strong self as this => @default-panic, move |_button| {
            let imp = this.imp();
            let revealed = !imp.revealer.get_visible();
            imp.revealer.set_visible(revealed);
            imp.revealer.set_reveal_child(revealed);
        }));


        // ctrl = Gtk.EventControllerScroll(flags=Gtk.EventControllerScrollFlags.BOTH_AXES)
        // ctrl.connect("scroll", self._on_scroll)
        // self.add_controller(ctrl)

        let ctrl = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        ctrl.connect_scroll(glib::clone!(@strong self as this => @default-panic, move |_event, _delta_x, delta_y| {
                let imp = this.imp();
                let new_volume = (imp.current_volume.get()  -  (delta_y / 10.0)).clamp(0.0, 1.0);
                imp.scale.set_value(new_volume);
                gtk::Inhibit(false)
            }
        ));

        self.add_controller(ctrl);
    }


    fn on_scale_value_change(&self, display_volume: f64) {
        let imp = self.imp();
        if display_volume <= 0.05 {
            imp.button.set_child(Some(&imp.muted_img));
        } else if display_volume <= 0.33 {
            imp.button.set_child(Some(&imp.low_img));
        } else if display_volume <= 0.66 {
            imp.button.set_child(Some(&imp.medium_img));
        } else {
            imp.button.set_child(Some(&imp.high_img));
        }
        imp.current_volume.set(display_volume);


    }

}
    