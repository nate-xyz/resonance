/* disc_button.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */


use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::clone};

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use crate::model::track::Track;
use crate::model::album::Album;

use log::debug;

use crate::util::player;

mod imp {
    use super::*;
    
    #[derive(Debug, Default)]
    pub struct DiscButtonPriv {
        pub play_button: RefCell<gtk::Button>,
        pub add_button: RefCell<gtk::Button>,
        pub play_icon: RefCell<gtk::Image>,
        pub tracks: RefCell<Vec<Rc<Track>>>,
        pub title: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiscButtonPriv {
        const NAME: &'static str = "DiscButton";
        type Type = super::DiscButton;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for DiscButtonPriv {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for DiscButtonPriv {}
    impl BoxImpl for DiscButtonPriv {}
    impl DiscButtonPriv {}
}

glib::wrapper! {
    pub struct DiscButton(ObjectSubclass<imp::DiscButtonPriv>)
    @extends gtk::Box, gtk::Widget;
}

impl DiscButton {
    pub fn new(disc_n: i64, album: Rc<Album>) -> DiscButton {
        let disc_button: DiscButton = glib::Object::builder::<DiscButton>().build();
        disc_button.construct(disc_n, album);
        disc_button
    }

    fn construct(&self, disc_n: i64, album: Rc<Album>) {
        let imp = self.imp();
        let album_title = album.title();

        imp.title.replace(format!("{}, Disc {}", album_title, disc_n+1));

        match album.disc(disc_n) {
            Ok(disc) => {
                self.set_tracks(disc);
            }
            Err(e) => {
                debug!("An error occurred: {}", e)
            },

        }
        
        self.set_margin_start(5);
        self.set_margin_top(5);
        self.set_margin_end(5);
        self.set_hexpand(false);

        let play_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        play_box.set_hexpand(true);
        play_box.set_halign(gtk::Align::Fill);

        let play_button = gtk::Button::new();
        play_button.set_has_frame(false);
        play_button.set_css_classes(&[&"background", &"frame"]);
        play_button.set_tooltip_text(Some(format!("Play Disc {} of {}", disc_n + 1, album_title).as_str()));
        
        let label = gtk::Label::new(None);
        label.set_use_markup(true);
        label.set_label(format!("<span weight=\"ultralight\">Disc {}</span>", disc_n + 1).as_str());
        label.set_halign(gtk::Align::Start);

        let play_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        play_icon.set_visible(false);

        play_box.append(&play_icon);
        play_box.append(&label);
        play_button.set_child(Some(&play_box));

        let add_button = gtk::Button::new();
        add_button.set_visible(false);
        add_button.set_child(Some(&gtk::Image::from_icon_name("plus-symbolic")));
        add_button.set_css_classes(&[&"background", &"frame"]);
   

        play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().clear_play_album(this.tracks(), Some(this.title()));
            }),
        );

        add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().add_album(this.tracks());
            }),
        );


        self.set_css_classes(&[&"linked"]);
        self.append(&play_button);
        self.append(&add_button);

        imp.play_button.replace(play_button);
        imp.add_button.replace(add_button);
        imp.play_icon.replace(play_icon);

        let ctrl = gtk::EventControllerMotion::new();
        
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();
                imp.add_button.borrow().show();
                imp.play_icon.borrow().show();
            })
        );
        
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();
                imp.add_button.borrow().hide();
                imp.play_icon.borrow().hide();
            })
        );
        
        self.add_controller(ctrl);
    }

    fn set_tracks(&self, disc: HashMap<i64, Rc<Track>>) {
        let mut array = Vec::new();
        let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
        album_vec.sort_by(|a, b| a.0.cmp(b.0));
        for (_track_n, track) in album_vec {
            array.push(track.clone());
        }
        self.imp().tracks.replace(array);
    }

    fn tracks(&self) -> Vec<Rc<Track>> {
        self.imp().tracks.borrow().clone()
    }

    fn title(&self) -> String {
        self.imp().title.borrow().clone()
    }
}
    