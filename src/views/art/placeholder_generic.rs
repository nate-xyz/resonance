/* placeholder_generic.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::glib;

use html_escape;

use crate::util::model;

use super::rounded_background::RoundedBackground;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlaceHolderGeneric  {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaceHolderGeneric {
        const NAME: &'static str = "PlaceHolderGeneric";
        type Type = super::PlaceHolderGeneric;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for PlaceHolderGeneric {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for PlaceHolderGeneric {}
    impl BinImpl for PlaceHolderGeneric {}

}

glib::wrapper! {
    pub struct PlaceHolderGeneric(ObjectSubclass<imp::PlaceHolderGeneric>)
        @extends gtk::Widget, adw::Bin;
}

impl PlaceHolderGeneric {
    pub fn new(name: String, icon_name: &str, size: i32, image_id: Option<i64>) -> PlaceHolderGeneric {
        let object: PlaceHolderGeneric= glib::Object::builder::<PlaceHolderGeneric>().build();
        object.construct(name, icon_name, size, image_id);
        object
    }

    pub fn construct(&self, name: String, icon_name: &str, size: i32, image_id: Option<i64>) {
        let bg = RoundedBackground::new("rgba(0, 0, 0, 0)", size);
        let mut loaded = false;
        if let Some(id) = image_id {
            if let Ok(image) = model().artist_image(id) {
                if let Ok(pixbuf) = image.pixbuf() {
                    bg.load_art(pixbuf);
                    loaded = true;
                }            
            }
        }

        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_.set_hexpand(true);
        box_.set_vexpand(true);
        box_.set_valign(gtk::Align::Center);
        box_.set_halign(gtk::Align::Center);

        let overlay = gtk::Overlay::new(); 

        if !loaded {
            let icon = gtk::Image::from_icon_name(icon_name);
            icon.set_icon_size(gtk::IconSize::Large);

            let label = gtk::Label::new(None);
            label.set_use_markup(true);
            label.set_hexpand(true);
            label.set_wrap(true);
            label.set_label(&format!("<span weight=\"light\" size=\"x-large\">{}</span>", html_escape::encode_text_minimal(name.as_str())));
            
            box_.append(&icon);
            box_.append(&label);
    
            overlay.add_overlay(&box_);
        }
        overlay.set_child(Some(&bg));

        self.set_child(Some(&overlay));
    }
}