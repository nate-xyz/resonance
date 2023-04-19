/* album_card.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, rc::Rc};
use std::collections::HashMap;
use log::error;

use crate::model::{
    album::Album,
    track::Track,
};
use crate::util::{model, player, seconds_to_string_longform};
use crate::i18n::{i18n, i18n_k};

use super::art::rounded_album_art::RoundedAlbumArt;
use super::disc_button::DiscButton;
use super::track_entry::TrackEntry;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_card.ui")]
    pub struct AlbumCardPriv  {
        #[template_child(id = "art_and_info_box")]
        pub art_and_info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "overlay_play_button")]
        pub overlay_play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "overlay_add_button")]
        pub overlay_add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

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

        #[template_child(id = "no_art_button_box")]
        pub no_art_button_box: TemplateChild<gtk::Box>,

        #[template_child(id = "no_art_play_button")]
        pub no_art_play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "no_art_add_button")]
        pub no_art_add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "track_box")]
        pub track_box: TemplateChild<gtk::Box>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub album: RefCell<Option<Rc<Album>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumCardPriv {
        const NAME: &'static str = "AlbumCard";
        type Type = super::AlbumCard;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for AlbumCardPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for AlbumCardPriv {}
    impl BinImpl for AlbumCardPriv {}
    impl AlbumCardPriv {}
}

glib::wrapper! {
    pub struct AlbumCard(ObjectSubclass<imp::AlbumCardPriv>)
        @extends gtk::Widget, adw::Bin;
}

impl AlbumCard {
    pub fn new() -> AlbumCard {
        let album_card: AlbumCard = glib::Object::builder::<AlbumCard>().build();
        album_card
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        let play_buttons = [&imp.overlay_play_button, &imp.no_art_play_button];
        for play in play_buttons {
            play.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let album = this.album();
                    player().clear_play_album(album.tracks(), Some(album.title()));
                })
            );
        }

        let add_buttons = [&imp.overlay_add_button, &imp.no_art_add_button];
        for add in add_buttons {
            add.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let tracks = this.album().tracks();
                    player().add_album(tracks);
                })
            );
        }

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                this.imp().overlay_box.show();
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                this.imp().overlay_box.hide();
            })
        );
        imp.overlay.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             let height = this.height();
        //             // let width = this.width() / 2;
        //             let y = y as i32 + 30;
        //             // let x = x as i32;
        //             // let offset = imp.popover.offset();
        //             imp.popover.set_offset(0, y - height);
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // imp.art_and_info_box.add_controller(ctrl);
    }

    pub fn load_album(&self, album: Rc<Album>) {
        self.imp().album.replace(Some(album.clone()));
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();

        match imp.album.borrow().as_ref() {
            Some(album) => {

                // CONSTRUCT UI
                // let track_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                let disc_map = album.discs();
                let num_discs = disc_map.len();
                let mut disc_vec: Vec<(&i64, &HashMap<i64, Rc<Track>>)> = disc_map.iter().collect();
                disc_vec.sort_by(|a, b| a.0.cmp(b.0));
                for (disc_n, disc) in disc_vec {
                    let disc_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                    // disc_box.set_homogeneous(true);
                    disc_box.set_valign(gtk::Align::Fill);
                    disc_box.set_halign(gtk::Align::Fill);
                    disc_box.set_hexpand(true);
                    

                    if num_discs > 1 {
                        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                        box_.set_hexpand(false);

                        let disc_button = DiscButton::new(*disc_n, album.clone());

                        box_.append(&disc_button);

                        // self.track_children.add(&box_);
                        imp.track_box.append(&box_);
                    }



                    let first_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    first_box.set_valign(gtk::Align::Fill);
                    first_box.set_halign(gtk::Align::Fill);
                    first_box.set_hexpand(true);
                    

                    let total_tracks = disc.len();
                    if total_tracks > 3 {
                        let grid = gtk::FlowBox::new();
                        grid.set_valign(gtk::Align::Fill);
                        grid.set_halign(gtk::Align::Fill);
                        grid.set_hexpand(true);
                        grid.set_min_children_per_line(1);
                        grid.set_max_children_per_line(2);
                        grid.set_column_spacing(0);
                        grid.set_row_spacing(0);
                        grid.set_selection_mode(gtk::SelectionMode::None);
                        grid.set_homogeneous(true);
                                                    
                    
                        let second_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
                        second_box.set_valign(gtk::Align::Fill);
                        second_box.set_halign(gtk::Align::Fill);
                        second_box.set_hexpand(true);

                        let second_box_amount = total_tracks/2;
                        let first_box_amount = total_tracks - second_box_amount;

                        grid.append(&first_box);
                        grid.append(&second_box);

                        let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
                        album_vec.sort_by(|a, b| a.0.cmp(b.0));
                        for (track_n, track) in album_vec {
                            let track = track.clone();
                            let entry = TrackEntry::new(album.clone(), track, *track_n as i32, *disc_n as i32, 27);
                            // entry.set_player(self.player.borrow().as_ref().unwrap().clone());

                            if *track_n <= first_box_amount as i64  {
                                first_box.append(&entry);
                            } else {
                                second_box.append(&entry);
                            }
                            //self.track_children.add(&entry);
                        }

                        disc_box.append(&grid);

                    } else {
                        disc_box.append(&first_box);
                        let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
                        album_vec.sort_by(|a, b| a.0.cmp(b.0));
                        for (track_n, track) in album_vec {
                            let track = track.clone();
                            let entry = TrackEntry::new(album.clone(), track, *track_n as i32, *disc_n as i32, 27);                            
                            first_box.append(&entry);
                            //self.track_children.add(&entry);
                        }
                    }

                    imp.track_box.append(&disc_box);

                }

                //SET LABELS
                imp.title_label.set_label(&album.title());
                imp.artist_label.set_label(&album.artist());

                let date = album.date();
                if !date.is_empty() {
                    imp.date_label.set_label(&date);
                } else {
                    imp.date_label.hide();
                }

                let n_tracks = album.n_tracks();
                if n_tracks <= 1 {
                    imp.track_amount_label.set_label(&i18n("1 track"));
                } else {
                    imp.track_amount_label.set_label(&i18n_k("{number_of_tracks} tracks", &[("number_of_tracks", &format!("{}", n_tracks))]));
                }

                let duration = album.duration();
                if duration > 0.0 {
                    imp.duration_label.set_label(&seconds_to_string_longform(duration));
                } else {
                    imp.duration_label.hide();
                }
        
        
                //LOAD ART
                self.load_art(album.cover_art_id());

                //imp.popover.set_menu_model(Some(album.menu_model()));
            }
            None => {
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.no_art_button_box.show();
            },
        }
    }

    fn load_art(&self, art: Option<i64>) {
        let imp = self.imp();
        match art {
            Some(id) => match self.add_art(id, 300) {
                Ok(art) => {
                    imp.art_bin.set_child(Some(&art));
                }
                Err(msg) => {
                    error!("Tried to set art, but: {}", msg);
                    imp.art_bin.set_child(gtk::Widget::NONE);
                    imp.no_art_button_box.show();
                }
            },
            None => {
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.no_art_button_box.show();
            }
        }
    }

    fn add_art(&self, cover_art_id: i64, size: i32) -> Result<RoundedAlbumArt, String> {
        let cover_art = model().cover_art(cover_art_id)?;
        let pixbuf = cover_art.pixbuf()?;
        let art = RoundedAlbumArt::new(size);
        art.load(pixbuf);
        Ok(art)
    }
    
    fn album(&self ) -> Rc<Album> {
        self.imp().album.borrow().as_ref().unwrap().clone()
    }
}