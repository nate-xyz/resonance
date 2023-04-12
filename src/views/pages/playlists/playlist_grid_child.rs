/* playlist_grid_child.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{ glib, glib::clone, CompositeTemplate};

use std::{cell::{Cell, RefCell}, rc::Rc};
use log::debug;

use crate::model::playlist::Playlist;
use crate::views::art::{
    grid_art::GridArt,
    placeholder_art::PlaceHolderArt,
};
use crate::sort::SortMethod;
use crate::util::{model, player, seconds_to_string_longform};
use crate::i18n::i18n_k;

mod imp {

    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecEnum,
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/playlist_grid_child.ui")]
    pub struct PlaylistGridChild {
        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "main_button")]
        pub main_button: TemplateChild<gtk::Button>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "modify_time_label")]
        pub modify_time_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_count_label")]
        pub track_count_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "info_box")]
        pub info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub playlist: RefCell<Option<Rc<Playlist>>>,

        pub art: RefCell<Option<GridArt>>,
        pub placeholder_art: RefCell<Option<PlaceHolderArt>>,
        pub art_size: Cell<i32>,

        pub sort_method: Cell<SortMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistGridChild {
        const NAME: &'static str = "PlaylistGridChild";
        type Type = super::PlaylistGridChild;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistGridChild {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecEnum::builder::<SortMethod>("sort-method").build(),
                    ]
                );
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "sort-method" => {
                    let sort_method = value.get().expect("The value needs to be of type `enum`.");
                    self.sort_method.replace(sort_method);
                    self.obj().update_sort_ui();
                },
                _ => unimplemented!(),
            }
        }
    
        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "sort-method" => self.sort_method.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("clicked")
                        .param_types([<i64>::static_type()])
                        .build()
                    ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for PlaylistGridChild {}
    impl FlowBoxChildImpl for PlaylistGridChild {}
    impl PlaylistGridChild {}
}

glib::wrapper! {
    pub struct PlaylistGridChild(ObjectSubclass<imp::PlaylistGridChild>)
    @extends gtk::FlowBoxChild, gtk::Widget;
}

impl Default for PlaylistGridChild {
    fn default() -> Self {
        glib::Object::builder::<PlaylistGridChild>().build()
    }
}

impl PlaylistGridChild {
    pub fn new() -> PlaylistGridChild {
         Self::default()
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        imp.main_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                let id = this.imp().playlist.borrow().as_ref().unwrap().id().clone();
                this.emit_by_name::<()>("clicked", &[&id]);
            })
        );

        imp.play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let playlist = this.playlist();
                let tracks = playlist.tracks();
                player().clear_play_album(tracks, Some(playlist.title()));
            })
        );

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let tracks = this.playlist().tracks();
                player().add_album(tracks);
            })
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(clone!(@strong self as this => move |_controller, _x, _y| {
            let imp = this.imp();
            imp.overlay_box.show();
            imp.main_button.show();
        }));
        ctrl.connect_leave(clone!(@strong self as this => move |_controller| {
            let imp = this.imp();
            imp.overlay_box.hide();
            imp.main_button.hide();
        }));
        self.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, _y, button, _sequence| {
        //         let imp = this.imp();
        //         debug!("unpaired release");
        //         if button == gdk::BUTTON_SECONDARY {
        //             //imp.popover.set_offset((imp.popover.width()*2 + x as i32) - imp.popover.width() / 2, y as i32 - imp.popover.height());
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }

    pub fn load_playlist(&self, playlist: Rc<Playlist>) {
        self.imp().playlist.replace(Some(playlist.clone()));
        self.update_view();
    }

    fn playlist(&self ) -> Rc<Playlist> {
        self.imp().playlist.borrow().as_ref().unwrap().clone()
    }


    pub fn update_view(&self) {
        let imp = self.imp();
        match imp.playlist.borrow().as_ref() {
            Some(playlist) => {
                imp.main_button.set_tooltip_text(Some(&playlist.title()));
                
                imp.title_label.set_label(&playlist.title());
                imp.modify_time_label.set_label(&format!("{}", playlist.modify_time()));
                imp.track_count_label.set_label(&i18n_k("{number_of_tracks} tracks", &[("number_of_tracks", &format!("{}", playlist.n_tracks()))]));
                imp.duration_label.set_label(&seconds_to_string_longform(playlist.duration()));


                let cover_art_ids= playlist.cover_art_ids();
                if cover_art_ids.len() > 0 {
                    match self.load_image(cover_art_ids) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(Some(art));
                            imp.placeholder_art.replace(None);
                        },
                        Err(_) => {
                            let art = PlaceHolderArt::new(playlist.title(), "".to_string(), 200);
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(None);
                            imp.placeholder_art.replace(Some(art));
                        }
                    }
                } else {
                    let art = PlaceHolderArt::new(playlist.title(), "".to_string(), 200);
                    imp.art_bin.set_child(Some(&art));
                    imp.art.replace(None);
                    imp.placeholder_art.replace(Some(art));
                }

                //imp.popover.set_menu_model(Some(playlist.menu_model()))
            }
            None => {
                imp.title_label.set_label("");
                imp.modify_time_label.set_label("");
                imp.track_count_label.set_label("");
                imp.duration_label.set_label("");
                imp.art_bin.set_child(gtk::Widget::NONE);
            }
        }
    }

    fn load_image(&self, cover_art_id: Vec<i64>) -> Result<GridArt, String> {
        debug!("loading image, playlist grid child");
        let art = GridArt::new(200);

        let mut cover_art_pixbufs = Vec::new();
        for id in cover_art_id {
            match model().cover_art(id) {
                Ok(cover_art) => {
                    match cover_art.pixbuf() {
                        Ok(pixbuf) => {
                            cover_art_pixbufs.push(pixbuf);
                        }
                        Err(msg) => return Err(msg),
                    };
                }
                Err(msg) => return Err(msg),
            };
        }

        art.load(cover_art_pixbufs);
        return Ok(art);
    }


    fn update_sort_ui(&self) {
        let imp = self.imp();
        imp.info_box.show();
        match imp.sort_method.get() {
            SortMethod::LastModified => {
                imp.title_label.show();
                imp.modify_time_label.show();
                imp.track_count_label.hide();
                imp.duration_label.hide();
            },
            SortMethod::Playlist => {
                imp.title_label.show();
                imp.modify_time_label.hide();
                imp.track_count_label.hide();
                imp.duration_label.hide();
            },
            SortMethod::Duration => {
                imp.title_label.show();
                imp.modify_time_label.hide();
                imp.track_count_label.hide();
                imp.duration_label.show();
            },
            SortMethod::TrackCount => {
                imp.title_label.show();
                imp.modify_time_label.hide();
                imp.track_count_label.show();
                imp.duration_label.hide();
            },
            _ => (),
        }
    }
}
