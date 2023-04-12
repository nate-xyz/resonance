/* track_page_row.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, cell::Cell, rc::Rc};
use log::error;

use crate::model::track::Track;
use crate::views::{
    art::rounded_album_art::RoundedAlbumArt,
    art::icon_with_background::IconWithBackground,
};
use crate::search::SearchMethod;
use crate::sort::SortMethod;
use crate::util::{player, model, seconds_to_string};

mod imp {
    use super::*;
    use glib::{
        Value, ParamSpec, ParamSpecEnum, ParamSpecBoolean,
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/track_page_row.ui")]
    pub struct TrackPageRowPriv {
        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "end_box")]
        pub end_box: TemplateChild<gtk::Box>,

        #[template_child(id = "track_title_label")]
        pub track_title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label")]
        pub album_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_artist_label")]
        pub album_artist_label: TemplateChild<gtk::Label>,

        #[template_child(id = "genre_label")]
        pub genre_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "date_label")]
        pub date_label: TemplateChild<gtk::Label>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "play_icon_no_art")]
        pub play_icon_no_art: TemplateChild<gtk::Image>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track: RefCell<Option<Rc<Track>>>,
        pub art: RefCell<Option<RoundedAlbumArt>>,
        pub display_labels_default: Cell<bool>,
        pub search_method: Cell<SearchMethod>,
        pub sort_method: Cell<SortMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrackPageRowPriv {
        const NAME: &'static str = "TrackPageRow";
        type Type = super::TrackPageRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TrackPageRowPriv {
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
    }

    impl WidgetImpl for TrackPageRowPriv {}
    impl BoxImpl for TrackPageRowPriv {}
    impl TrackPageRowPriv {}
}

glib::wrapper! {
    pub struct TrackPageRow(ObjectSubclass<imp::TrackPageRowPriv>)
    @extends gtk::Box, gtk::Widget;
}

impl TrackPageRow {
    pub fn new() -> TrackPageRow {
        let track_row: TrackPageRow = glib::Object::builder::<TrackPageRow>().build();
        track_row
    }

    fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        self.bind_property("display-labels-default", &*imp.genre_label, "visible")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        self.bind_property("display-labels-default", &*imp.date_label, "visible")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
            
        imp.overlay_box.prepend(&IconWithBackground::new("media-playback-start-symbolic", 60, false));

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let track = this.track();
                player().add_track(track);
            })
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();

                imp.duration_label.hide();
                imp.date_label.hide();
                imp.genre_label.hide();
                
                imp.add_button.show();
                
                match imp.art.borrow().as_ref() {
                    Some(_art) => {
                        imp.overlay_box.show();
                    }
                    None => {
                        imp.play_icon_no_art.show();
                    },
                }
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();

                let method = imp.search_method.get();

                if method == SearchMethod::Genre || imp.display_labels_default.get() {
                    imp.genre_label.show();
                }

                if method == SearchMethod::ReleaseDate || imp.display_labels_default.get() {
                    imp.date_label.show();
                }

                imp.duration_label.show();
                imp.add_button.hide();
                
                match imp.art.borrow().as_ref() {
                    Some(_art) => {
                        imp.overlay_box.hide();
                    }
                    None => {
                        imp.play_icon_no_art.hide();
                    },
                }
            })
        );
        self.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, _y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }


    pub fn update_track(&self, track: Rc<Track>) {
        self.imp().track.replace(Some(track.clone()));
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        match imp.track.borrow().as_ref() {
            Some(track) => {
                imp.track_title_label.set_label(&track.title());
                imp.album_name_label.set_label(&track.album());
                imp.album_artist_label.set_label(&track.artist());
                imp.date_label.set_label(&track.date());
                imp.genre_label.set_label(&track.genre());

                let duration = track.duration();
                if duration > 0.0 {
                    imp.duration_label.set_label(&seconds_to_string(duration));
                }

                match track.cover_art_option() {
                    Some(id) => match self.load_image(id) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(Some(art));
                        }
                        Err(msg) => {
                            error!("{}", msg);
                            imp.art_bin.set_child(gtk::Widget::NONE);
                            imp.art.replace(None);
                        }
                    },
                    None => {
                        imp.art_bin.set_child(gtk::Widget::NONE);
                        imp.art.replace(None);
                    }
                }

                //imp.popover.set_menu_model(Some(track.menu_model()));
            }
            None => {
                imp.track_title_label.set_label("");
                imp.album_name_label.set_label("");
                imp.album_artist_label.set_label("");
                imp.date_label.set_label("");
                imp.genre_label.set_label("");
                imp.duration_label.set_label("");
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.art.replace(None);
            }
        }
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let art = RoundedAlbumArt::new(60);
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


    fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }

    pub fn visible_labels(&self, method: SearchMethod, searching: bool) {
        let imp = self.imp();

        if !searching {
            if !imp.display_labels_default.get() {
                imp.genre_label.hide();
                imp.date_label.hide();
            } else {
                imp.genre_label.show();
                imp.date_label.show();
            }
            return;
        }

        match method {
            SearchMethod::Genre => {
                imp.genre_label.show();
                imp.date_label.hide();
            },
            SearchMethod::ReleaseDate => {
                imp.genre_label.hide();
                imp.date_label.show();
            },
            _ => {
                imp.genre_label.hide();
                imp.date_label.hide();
            },
        }

        imp.search_method.set(method);

    }

    fn update_sort_ui(&self) {
        let imp = self.imp();
        match imp.sort_method.get() {
            SortMethod::Genre => {
                if !imp.display_labels_default.get() {
                    imp.genre_label.show();
                    imp.date_label.hide();
                } else {
                    imp.genre_label.show();
                    imp.date_label.show();
                }
            },
            SortMethod::ReleaseDate => {
                if !imp.display_labels_default.get() {
                    imp.genre_label.hide();
                    imp.date_label.show();
                } else {
                    imp.genre_label.show();
                    imp.date_label.show();
                }
            },
            _ => {
                if !imp.display_labels_default.get() {
                    imp.genre_label.hide();
                    imp.date_label.hide();
                } else {
                    imp.genre_label.show();
                    imp.date_label.show();
                }
            },
        }
    }

}
