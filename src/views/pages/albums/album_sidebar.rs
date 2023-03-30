/* album_sidebar.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::{prelude::*, subclass::prelude::*};
use gtk::{glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, rc::Rc};
use std::collections::HashMap;
use log::{debug, error};

use crate::model::{
    album::Album,
    track::Track,
};

use crate::views::{
    art::{rounded_album_art::RoundedAlbumArt, placeholder_art::PlaceHolderArt},
    disc_button::DiscButton,
    track_entry::TrackEntry,
};
use crate::util::{model, player};

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_sidebar.ui")]
    pub struct AlbumFlapPriv {
        #[template_child(id = "main_overlay")]
        pub main_overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        #[template_child(id = "art_and_info_box")]
        pub art_and_info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "track_bin")]
        pub track_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_label")]
        pub artist_label: TemplateChild<gtk::Label>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "info_button")]
        pub info_button: TemplateChild<gtk::Button>,

        #[template_child(id = "back_button")]
        pub back_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_button_box")]
        pub second_button_box: TemplateChild<gtk::Box>,

        #[template_child(id = "second_play_button")]
        pub second_play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_add_button")]
        pub second_add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_info_button")]
        pub second_info_button: TemplateChild<gtk::Button>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub album: RefCell<Option<Rc<Album>>>,
        // pub model: RefCell<Option<Rc<Model>>>,
        // pub player: RefCell<Option<Rc<Player>>>,

    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumFlapPriv {
        const NAME: &'static str = "AlbumFlap";
        type Type = super::AlbumFlap;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumFlapPriv {
        fn constructed(&self) {
            debug!("AlbumFlap constructed");
            self.parent_constructed();
            self.obj().initialize();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("back").build(),
                    Signal::builder("album-selected")
                        .param_types([<i64>::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for AlbumFlapPriv {}
    impl BinImpl for AlbumFlapPriv {}
    impl AlbumFlapPriv {}
}

glib::wrapper! {
    pub struct AlbumFlap(ObjectSubclass<imp::AlbumFlapPriv>)
     @extends adw::Bin, gtk::Widget;
}

impl Default for AlbumFlap {
    fn default() -> Self {
        glib::Object::builder::<AlbumFlap>().build()
    }
}

impl AlbumFlap {
    pub fn new() -> AlbumFlap {
        Self::default()
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);
        
        imp.back_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                this.emit_by_name::<()>("back", &[]);
            }),
        );

        imp.play_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
            let album = this.album();
            player().clear_play_album(album.tracks(), Some(album.title()));
        }));

        imp.add_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
            let tracks = this.album().tracks();
            player().add_album(tracks);
        }));

        imp.info_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                this.emit_by_name::<()>("album-selected", &[&this.album_id()]);
                this.emit_by_name::<()>("back", &[]);
            }),
        );

        imp.second_play_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
            let album = this.album();
            player().clear_play_album(album.tracks(), Some(album.title()));
        }));

        imp.second_add_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
            let tracks = this.album().tracks();
            player().add_album(tracks);
        }));

        imp.second_info_button.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                this.emit_by_name::<()>("album-selected", &[&this.album_id()]);
            }),
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(clone!(@strong self as this => move |_controller, _x, _y| {
            this.imp().overlay_box.show();
        }));
        ctrl.connect_leave(clone!(@strong self as this => move |_controller| {
            this.imp().overlay_box.hide();
        }));
        imp.art_and_info_box.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, y, button, _sequence| {
        //         let imp = this.imp();
        //         debug!("unpaired release");
        //         if button == gdk::BUTTON_SECONDARY {
        //             let height = this.height();
        //             let y = y as i32;
        //             let offset = imp.popover.offset();
        //             debug!("{} {} {:?}", height, y, offset);
        //             // let height = this.height() - (y as i32);
        //             // imp.popover.set_offset(x as i32, height);
        //             // //imp.popover.set_offset((imp.popover.width()*2 + x as i32) - imp.popover.width() / 2, y as i32 - imp.popover.height());

        //             imp.popover.set_offset(0, y - height);

        //             let offset = imp.popover.offset();
        //             debug!("{} {} {:?}", height, y, offset);

        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }

    pub fn load_album(&self, album_id: i64) {
        debug!("Load album, album sidebar");

        let album = model().album(album_id);
        match album {
            Ok(album) => {
                self.imp().album.replace(Some(album));
                debug!("album sidebar update view");
                self.update_view();
                debug!("album sidebar update view done");
            }
            Err(msg) => {
                debug!("Unable to load album: {}", msg);
                self.emit_by_name::<()>("back", &[]);
            }
        }
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        imp.scrolled_window.vadjustment().set_value(0.0);
        // self.clear_track_children();
        if imp.second_button_box.get_visible() {
            imp.second_button_box.set_visible(false);
        }

        match imp.album.borrow().as_ref() {
            Some(album) => {

                debug!("album sidebar make track box");

                imp.track_bin.set_child(gtk::Widget::NONE);
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
                        box_.append(&disc_button);

                        track_box.append(&box_);
                    }
                    
                    let mut album_vec: Vec<(&i64, &Rc<Track>)> = disc.iter().collect();
                    album_vec.sort_by(|a, b| a.0.cmp(b.0));
                    for (track_n, track) in album_vec {
                        let track = track.clone();
                        let entry = TrackEntry::new(track, *track_n as i32, *disc_n as i32, 1000);

                        track_box.append(&entry);
                    }
                }

                debug!("album sidebar set labels");

                imp.title_label.set_label(album.title().as_str());
                imp.artist_label.set_label(album.artist().as_str());
                imp.track_bin.set_child(Some(&track_box));

                debug!("set cover art");

                match album.cover_art_id() {
                    Some(id) => match self.load_image(id) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                        }
                        Err(msg) => {
                            error!("Tried to set art, but: {}", msg);
                            let art = PlaceHolderArt::new(album.title(), album.artist(), 400);
                            imp.art_bin.set_child(Some(&art));
                        }
                    },
                    None => {
                        let art = PlaceHolderArt::new(album.title(), album.artist(), 400);
                        imp.art_bin.set_child(Some(&art));
                    }
                }

                debug!("set popover menu");

                //imp.popover.set_menu_model(Some(album.menu_model()))
            }
            None => {
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.track_bin.set_child(gtk::Widget::NONE);
            },
        }
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let art = RoundedAlbumArt::new(400);
        match model().cover_art(cover_art_id) {
            Ok(cover_art) => {
                match cover_art.pixbuf() {
                    Ok(pixbuf) => {
                        art.load(pixbuf);
                        return Ok(art);
                        //this is where i should add the connection closure if i was multithreading
                    }
                    Err(msg) => return Err(msg),
                };
            }
            Err(msg) => return Err(msg),
            //Err(msg) => error!("error is {}", msg),
        };
    }

    fn album(&self ) -> Rc<Album> {
        self.imp().album.borrow().as_ref().unwrap().clone()
    }

    pub fn album_id(&self) -> i64 {
        self.imp().album.borrow().as_ref().unwrap().id()
    }

}
