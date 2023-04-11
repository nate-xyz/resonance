/* album_detail_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, glib, glib::clone, CompositeTemplate};

use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc, error::Error};
use log::{debug, error};

use crate::model::{
    album::Album,
    track::Track,
};
use crate::views::{
    disc_button::DiscButton,
    track_entry::TrackEntry,
};
use crate::util::{model, player, seconds_to_string_longform, load_cover_art_pixbuf, settings_manager};
use crate::i18n::{i18n, i18n_k};

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_detail_page.ui")]
    pub struct AlbumDetailPagePriv {
        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        
        #[template_child(id = "art-and-info")]
        pub art_and_info: TemplateChild<gtk::Box>,

        #[template_child(id = "info_box")]
        pub info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "track_bin")]
        pub track_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_label")]
        pub artist_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_amount_label")]
        pub track_amount_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "date_label")]
        pub date_label: TemplateChild<gtk::Label>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "back_button")]
        pub back_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_button_box")]
        pub second_button_box: TemplateChild<gtk::Box>,

        #[template_child(id = "second_play_button")]
        pub second_play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_add_button")]
        pub second_add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track_box: RefCell<Option<gtk::Box>>,
        pub album: RefCell<Option<Rc<Album>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumDetailPagePriv {
        const NAME: &'static str = "AlbumDetailPage";
        type Type = super::AlbumDetailPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumDetailPagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
        
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(
                    || vec![Signal::builder("back").build()],
                );

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for AlbumDetailPagePriv {}
    impl BoxImpl for AlbumDetailPagePriv {}
    impl AlbumDetailPagePriv {}
}

glib::wrapper! {
    pub struct AlbumDetailPage(ObjectSubclass<imp::AlbumDetailPagePriv>)
    @extends gtk::Box, gtk::Widget;
}

impl AlbumDetailPage {
    pub fn new() -> AlbumDetailPage {
        let album_detail: AlbumDetailPage = glib::Object::builder::<AlbumDetailPage>().build();
        album_detail
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        let settings = settings_manager();

        settings.bind("full-page-back-button", &*imp.back_button, "visible")
            .flags(gio::SettingsBindFlags::GET)
            .build();

        imp.popover.set_parent(self);

        imp.back_button.connect_clicked(clone!(@strong self as this => move |_button| {
                this.emit_by_name::<()>("back", &[]);
            })
        );

        imp.play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let album = this.album();
                player().clear_play_album(album.tracks(), Some(album.title()));
            }),
        );

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let tracks = this.album().tracks();
                player().add_album(tracks);
            }),
        );

        imp.second_play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let album = this.album();
                player().clear_play_album(album.tracks(), Some(album.title()));
            }),
        );

        imp.second_add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let tracks = this.album().tracks();
                player().add_album(tracks);
            }),
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();
                imp.overlay_box.show();
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();
                imp.overlay_box.hide();
            })
        );
        imp.overlay.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, x, y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             let height = this.height();
        //             let width = this.width() / 2;
        //             let y = y as i32;
        //             let x = x as i32;
        //             imp.popover.set_offset(x - width, y - height);
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // imp.art_and_info.add_controller(ctrl);
    }

    pub fn load_album(&self, album_id: i64) {
        let imp = self.imp();
        let album = model().album(album_id);
        match album {
            Ok(album) => {
                //imp.popover.set_menu_model(Some(album.menu_model()));
                imp.album.replace(Some(album));
                match self.update_view() {
                    Ok(_) => (),
                    Err(e) => error!("{}", e),
                }
            }
            Err(msg) => {
                debug!("Unable to load genre: {}", msg);
                self.emit_by_name::<()>("back", &[]);
            }
        }
    }

    pub fn update_view(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        // self.clear_track_children();
        if imp.second_button_box.get_visible() {
            imp.second_button_box.set_visible(false);
        }

        match imp.album.borrow().as_ref() {
            Some(album) => {

                if !imp.track_box.borrow().as_ref().is_none() {
                    debug!("removing previous track box");
                    imp.track_bin.set_child(gtk::Widget::NONE);
                    imp.track_box.borrow().as_ref().unwrap().unparent();
                }
                
                let track_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let disc_map = album.discs();
                let num_discs = disc_map.len();
                let mut disc_vec: Vec<(&i64, &HashMap<i64, Rc<Track>>)> = disc_map.iter().collect();
                disc_vec.sort_by(|a, b| a.0.cmp(b.0));
                for (disc_n, disc) in disc_vec {
                    if num_discs > 1 {
                        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                        box_.set_hexpand(false);

                        let disc_button = DiscButton::new(*disc_n, album.clone());
                        // disc_button.set_player(self.obj().player());

                        // play_button.connect_clicked(clone!(@weak self.on_disc_play_button));
                        // add_button.connect_clicked(clone!(@weak self.on_disc_add_button));
                        box_.append(&disc_button);


                        track_box.append(&box_);
                    }

                    let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
                    album_vec.sort_by(|a, b| a.0.cmp(b.0));
                    for (track_n, track) in album_vec {
                        let track = track.clone();
                        let entry = TrackEntry::new(track, *track_n as i32, *disc_n as i32, 300);
                        // entry.set_player(self.player.borrow().as_ref().unwrap().clone());

                        track_box.append(&entry);
                        //self.track_children.add(&entry);
                    }
                }

                imp.track_bin.set_child(Some(&track_box));
                imp.track_box.replace(Some(track_box));
                
                //SET LABELS
                imp.artist_label.set_label(&album.artist());
                imp.title_label.set_label(&album.title());

                let date = album.date();
                if !date.is_empty() {
                    imp.date_label.set_label(&date)
                }

                let n_tracks = album.n_tracks();
                if n_tracks <= 1 {
                    imp.track_amount_label.set_label(&i18n("1 track"));
                } else {
                    imp.track_amount_label.set_label(&i18n_k("{number_of_tracks} tracks", &[("number_of_tracks", &format!("{}", n_tracks))]));
                }

                let duration = album.duration();
                if duration > 0.0 {
                    imp.duration_label
                        .set_label(&seconds_to_string_longform(duration));
                }

                match album.cover_art_id() {
                    Some(id) => match load_cover_art_pixbuf(id, 550) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                        }
                        Err(msg) => {
                            error!("Tried to set art, but: {}", msg);
                            imp.art_bin.set_child(gtk::Widget::NONE);
                            imp.second_button_box.set_visible(true);
                        }
                    },
                    None => {
                        imp.art_bin.set_child(gtk::Widget::NONE);
                        imp.second_button_box.set_visible(true);
                    }
                }
            }
            None => {
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.track_bin.set_child(gtk::Widget::NONE);
                imp.second_button_box.set_visible(true);
                self.emit_by_name::<()>("back", &[]);
            }
        }

        Ok(())
    }

  
    fn album(&self) -> Rc<Album> {
        self.imp().album.borrow().as_ref().unwrap().clone()
    }
}
