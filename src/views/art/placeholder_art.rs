/* placeholder_art.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::glib;

use html_escape;
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

use super::rounded_background::RoundedBackground;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct PlaceHolderArt  {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaceHolderArt {
        const NAME: &'static str = "PlaceHolderArt";
        type Type = super::PlaceHolderArt;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for PlaceHolderArt {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for PlaceHolderArt {}
    impl BinImpl for PlaceHolderArt {}

}

glib::wrapper! {
    pub struct PlaceHolderArt(ObjectSubclass<imp::PlaceHolderArt>)
        @extends gtk::Widget, adw::Bin;
}

impl PlaceHolderArt {
    pub fn new(album: String, artist: String, size: i32) -> PlaceHolderArt {
        let object: PlaceHolderArt= glib::Object::builder::<PlaceHolderArt>().build();
        object.construct(album, artist, size);
        object
    }

    fn construct(&self, album: String, artist: String, size: i32) {
        let bg = RoundedBackground::new("rgba(0, 0, 0, 0.7)", size);
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 0);
        
        box_.set_hexpand(true);
        box_.set_vexpand(true);
        box_.set_valign(gtk::Align::Center);
        box_.set_halign(gtk::Align::Center);

        let album_label = gtk::Label::new(None);
        album_label.set_use_markup(true);
        album_label.set_hexpand(true);
        album_label.set_halign(gtk::Align::Center);
        album_label.set_justify(gtk::Justification::Center);
        album_label.set_wrap(true);
 
        let album_str = album.as_str();
        let width = UnicodeWidthStr::width(album_str);
        let text = if width >= 91 {
            let (truncated, _size) = album_str.unicode_truncate(90);
            let ellipsized = format!("{}…", truncated);
            ellipsized
        } else {
            album
        };        
        album_label.set_label(&format!("<span style=\"oblique\" weight=\"bold\" size=\"large\">{}</span>", html_escape::encode_text_minimal(&text)));

        let artist_label = gtk::Label::new(None);
        artist_label.set_use_markup(true);
        artist_label.set_hexpand(true);
        artist_label.set_halign(gtk::Align::Center);
        artist_label.set_justify(gtk::Justification::Center);
        artist_label.set_wrap(true);

        let artist_str = artist.as_str();
        let width = UnicodeWidthStr::width(artist_str);
        let text = if width >= 61 {
            let (truncated, _size) = artist_str.unicode_truncate(60);
            let ellipsized = format!("{}…", truncated);
            ellipsized
        } else {
            artist
        };
        artist_label.set_label(&format!("<span weight=\"book\" size=\"medium\">{}</span>", html_escape::encode_text_minimal(&text)));


        box_.append(&album_label);
        box_.append(&artist_label);

        box_.set_margin_top(12);
        box_.set_margin_end(12);
        box_.set_margin_start(12);
        box_.set_margin_bottom(12);

        self.set_overflow(gtk::Overflow::Hidden);

        let overlay = gtk::Overlay::new(); 

        overlay.add_overlay(&box_);
        overlay.set_child(Some(&bg));

        self.set_child(Some(&overlay));
    }
}