/* icon_with_background.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use super::circle_background::CircleBackground;
use super::rounded_background::RoundedBackground;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct IconWithBackground  {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IconWithBackground {
        const NAME: &'static str = "IconWithBackground";
        type Type = super::IconWithBackground;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for IconWithBackground {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for IconWithBackground {}
    impl BinImpl for IconWithBackground {}

}

glib::wrapper! {
    pub struct IconWithBackground(ObjectSubclass<imp::IconWithBackground>)
        @extends gtk::Widget, adw::Bin;
}

impl IconWithBackground {
    pub fn new(icon_name: &str, size: i32, circle: bool) -> IconWithBackground {
        let object: IconWithBackground= glib::Object::builder::<IconWithBackground>().build();
        object.construct(icon_name, size, circle);
        object
    }

    fn construct(&self, icon_name: &str, size: i32, circle: bool) {
        if circle {
            let bg = CircleBackground::new("rgba(0, 0, 0, 0.7)", size);
            let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
            box_.set_hexpand(true);
            box_.set_vexpand(true);
            box_.set_valign(gtk::Align::Center);
            box_.set_halign(gtk::Align::Center);
    
    
            let icon = gtk::Image::from_icon_name(icon_name);
            box_.append(&icon);


            let overlay = gtk::Overlay::new(); 
    
            overlay.add_overlay(&box_);
            overlay.set_child(Some(&bg));
    
            self.set_child(Some(&overlay));
        } else {
            let bg = RoundedBackground::new("rgba(0, 0, 0, 0.7)", size);
            let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
            box_.set_hexpand(true);
            box_.set_vexpand(true);
            box_.set_valign(gtk::Align::Center);
            box_.set_halign(gtk::Align::Center);
    
            let icon = gtk::Image::from_icon_name(icon_name);
            box_.append(&icon);

            let overlay = gtk::Overlay::new(); 
    
            overlay.add_overlay(&box_);
            overlay.set_child(Some(&bg));
    
            self.set_child(Some(&overlay));
        }

    }
}