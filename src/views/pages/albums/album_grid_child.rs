/* album_grid_child.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gdk, glib, glib::clone, CompositeTemplate};

use std::{cell::{Cell, RefCell}, rc::Rc};
use log::{debug, error};

use crate::model::album::Album;
use crate::views::art::{
    placeholder_art::PlaceHolderArt,
    rounded_album_art::RoundedAlbumArt,
};
use crate::search::SearchMethod;
use crate::sort::SortMethod;
use crate::util::{model, player, seconds_to_string_longform};
use crate::i18n::i18n_k;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecEnum, ParamSpecBoolean,
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_grid_child.ui")]
    pub struct AlbumGridChild {
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

        #[template_child(id = "artist_label")]
        pub artist_label: TemplateChild<gtk::Label>,

        #[template_child(id = "date_label")]
        pub date_label: TemplateChild<gtk::Label>,

        #[template_child(id = "genre_label")]
        pub genre_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_count_label")]
        pub track_count_label: TemplateChild<gtk::Label>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "info_box")]
        pub info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,
        
        pub art: RefCell<Option<RoundedAlbumArt>>,
        pub placeholder_art: RefCell<Option<PlaceHolderArt>>,
        pub art_size: Cell<i32>,
        pub album: RefCell<Option<Rc<Album>>>,
        pub display_labels_default: Cell<bool>,
        pub search_method: Cell<SearchMethod>,
        pub sort_method: Cell<SortMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumGridChild {
        const NAME: &'static str = "AlbumGridChild";
        type Type = super::AlbumGridChild;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AlbumGridChild {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecEnum::builder::<SortMethod>("sort-method").build(),
                    ParamSpecBoolean::builder("display-labels-default").default_value(false).build(),
                    ]
                );
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "display-labels-default" => {
                    let display_labels_default = value.get().expect("The value needs to be of type `bool`.");
                    self.display_labels_default.replace(display_labels_default);
                },
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
                "display-labels-default" => self.display_labels_default.get().to_value(),
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

    impl WidgetImpl for AlbumGridChild {}
    impl FlowBoxChildImpl for AlbumGridChild {}
    impl AlbumGridChild {}
}

glib::wrapper! {
    pub struct AlbumGridChild(ObjectSubclass<imp::AlbumGridChild>)
    @extends gtk::FlowBoxChild, gtk::Widget;
}

impl Default for AlbumGridChild {
    fn default() -> Self {
        glib::Object::builder::<AlbumGridChild>().build()
    }
}

impl AlbumGridChild {
    pub fn new() -> AlbumGridChild {
         Self::default()
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        self.bind_property("display-labels-default", &*imp.title_label, "visible")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        self.bind_property("display-labels-default", &*imp.artist_label, "visible")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        imp.main_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                let id = this.imp().album.borrow().as_ref().unwrap().id().clone();
                this.emit_by_name::<()>("clicked", &[&id]);
            })
        );

        imp.play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let album = this.album();
                player().clear_play_album(album.tracks(), Some(album.title()));
            })
        );

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let tracks = this.album().tracks();
                player().add_album(tracks);
            })
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();
                imp.overlay_box.show();
                imp.main_button.show();
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();
                imp.overlay_box.hide();
                imp.main_button.hide();
            })
        );
        self.add_controller(ctrl);


        let ctrl = gtk::GestureClick::new();
        ctrl.connect_unpaired_release(
            clone!(@strong self as this => move |_gesture_click, _x, _y, button, _sequence| {
                let imp = this.imp();
                debug!("unpaired release");
                if button == gdk::BUTTON_SECONDARY {
                    imp.popover.popup();
                }
            })
        );
        self.add_controller(ctrl);
    }

    pub fn load_album(&self, album: Rc<Album>) {
        self.imp().album.replace(Some(Rc::clone(&album)));
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        match imp.album.borrow().as_ref() {
            Some(album) => {
                imp.main_button.set_tooltip_text(Some(&format!("{} - {}", album.title(), album.artist())));
                imp.title_label.set_label(&album.title());
                imp.artist_label.set_label(&album.artist());
                imp.date_label.set_label(&album.date());
                imp.genre_label.set_label(&album.genre());

                let duration = album.duration();
                if duration > 0.0 {
                    imp.duration_label.set_label(&seconds_to_string_longform(duration));
                }

                imp.track_count_label.set_label(&i18n_k("{number_of_tracks} tracks", &[("number_of_tracks", &format!("{}", album.n_tracks()))]));

                match album.cover_art_id() {
                    Some(id) => match self.load_image(id) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(Some(art));
                            imp.placeholder_art.replace(None);
                        }
                        Err(msg) => {
                            error!("Tried to set art, but: {}", msg);
                            let art = PlaceHolderArt::new(album.title(), album.artist(), 200);
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(None);
                            imp.placeholder_art.replace(Some(art));
                        }
                    },
                    None => {
                        let art = PlaceHolderArt::new(album.title(), album.artist(), 200);
                        imp.art_bin.set_child(Some(&art));
                        imp.art.replace(None);
                        imp.placeholder_art.replace(Some(art));
                    }
                }

                // if album.cover_art_id.is_some() {
                //     self.load_image();
                // } else if album.cover_art_id.is_none() {
                //     self.on_populated(PlaceHolderArt(album.title.clone(), album.album_artist.clone()));
                // }

                //imp.popover.set_menu_model(Some(album.menu_model()))
            }
            None => {
                imp.title_label.set_label("");
                imp.artist_label.set_label("");
                imp.date_label.set_label("");
                imp.genre_label.set_label("");
                imp.duration_label.set_label("");
                imp.track_count_label.set_label("");
                imp.art_bin.set_child(gtk::Widget::NONE);
            }
        }
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let art = RoundedAlbumArt::new(200);
        match model().cover_art(cover_art_id) {
            Ok(cover_art) => {
                match cover_art.pixbuf() {
                    Ok(pixbuf) => {
                        art.load(pixbuf);
                        return Ok(art);
                    }
                    Err(msg) => return Err(msg),
                };
            }
            Err(msg) => return Err(msg),
        };
    }

    fn album(&self ) -> Rc<Album> {
        self.imp().album.borrow().as_ref().unwrap().clone()
    }

    pub fn visible_labels(&self, search: SearchMethod, searching: bool) {
        let imp = self.imp();

        if !searching {
            return;
        } 

        imp.info_box.set_visible(searching);
        match search {
            SearchMethod::Album => {
                imp.title_label.show();

                imp.artist_label.hide();
                imp.date_label.hide();
                imp.genre_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
            SearchMethod::Genre => {
                imp.title_label.show();
                imp.genre_label.show();

                imp.artist_label.hide();
                imp.date_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
            SearchMethod::ReleaseDate => {
                imp.title_label.show();
                imp.date_label.show();

                imp.artist_label.hide();
                imp.genre_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
            _ => {
                imp.title_label.show();
                imp.artist_label.show();

                imp.date_label.hide();
                imp.genre_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
        }


    }

    fn update_sort_ui(&self) {
        let imp = self.imp();
        imp.info_box.show();
        match imp.sort_method.get() {
            SortMethod::Genre => {
                if !imp.display_labels_default.get() {
                    imp.title_label.hide();
                } else {
                    imp.title_label.show();
                }
                imp.genre_label.show();

                imp.artist_label.hide();
                imp.date_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
            SortMethod::ReleaseDate => {
                if !imp.display_labels_default.get() {
                    imp.title_label.hide();
                } else {
                    imp.title_label.show();
                }
                imp.date_label.show();

                imp.genre_label.hide();
                imp.artist_label.hide();
                imp.duration_label.hide();
                imp.track_count_label.hide();
            },
            SortMethod::Duration => {
                if !imp.display_labels_default.get() {
                    imp.title_label.hide();
                } else {
                    imp.title_label.show();
                }
                imp.duration_label.show();

                imp.date_label.hide();
                imp.genre_label.hide();
                imp.artist_label.hide();
                imp.track_count_label.hide();
            },
            SortMethod::TrackCount => {
                if !imp.display_labels_default.get() {
                    imp.title_label.hide();
                } else {
                    imp.title_label.show();
                }
                imp.track_count_label.show();

                imp.duration_label.hide();
                imp.date_label.hide();
                imp.genre_label.hide();
                imp.artist_label.hide();
            },
            _ => {
                if !imp.display_labels_default.get() {
                    imp.info_box.hide();
                } else {
                    imp.title_label.show();
                    imp.artist_label.show();
                    imp.date_label.hide();
                    imp.genre_label.hide();
                    imp.duration_label.hide();
                    imp.track_count_label.hide();
                }
            },
        }
    }
}
