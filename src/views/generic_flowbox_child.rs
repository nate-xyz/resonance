/* generic_flowbox_child.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::clone, CompositeTemplate};
use std::{cell::Cell, rc::Rc};

use crate::model::{
    artist::Artist,
    genre::Genre,
};
use crate::sort::SortMethod;

use super::art::placeholder_generic::PlaceHolderGeneric;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenericChild {
    Artist,
    Genre
}

impl Default for GenericChild {
    fn default() -> Self {
        Self::Artist
    }
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecEnum,
    };
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/generic_flowbox_child.ui")]
    pub struct GenericFlowboxChildPriv {
        #[template_child(id = "main_button")]
        pub main_button: TemplateChild<gtk::Button>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "name_label")]
        pub name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_count_label")]
        pub album_count_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_count_label")]
        pub track_count_label: TemplateChild<gtk::Label>,

        pub id: Cell<i64>,
        pub child_type: Cell<GenericChild>,
        pub sort_method: Cell<SortMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenericFlowboxChildPriv {
        const NAME: &'static str = "GenericFlowboxChild";
        type Type = super::GenericFlowboxChild;
        type ParentType = gtk::FlowBoxChild;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GenericFlowboxChildPriv {
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

    impl WidgetImpl for GenericFlowboxChildPriv {}
    impl FlowBoxChildImpl for GenericFlowboxChildPriv {}
    impl GenericFlowboxChildPriv {}
}

glib::wrapper! {
    pub struct GenericFlowboxChild(ObjectSubclass<imp::GenericFlowboxChildPriv>)
    @extends gtk::FlowBoxChild, gtk::Widget;
}


impl GenericFlowboxChild {
    pub fn new(child_type: GenericChild, artist: Option<Rc<Artist>>, genre: Option<Rc<Genre>>) -> GenericFlowboxChild {
        let generic_flowbox_child: GenericFlowboxChild = glib::Object::builder::<GenericFlowboxChild>().build();
        generic_flowbox_child.construct(child_type, artist, genre);
        generic_flowbox_child
    }

    pub fn construct(&self, child_type: GenericChild, artist: Option<Rc<Artist>>, genre: Option<Rc<Genre>>) {
        let imp = self.imp();



        let bg = match child_type {
            GenericChild::Artist => {
                let artist = artist.unwrap();

                imp.id.set(artist.id());
                imp.album_count_label.set_label(&format!("{} albums", artist.n_albums()));
                imp.track_count_label.set_label(&format!("{} tracks", artist.n_tracks()));

                if !artist.image_id().is_none() {
                    imp.name_label.set_label(&artist.name());

                    let ctrl = gtk::EventControllerMotion::new();
                    ctrl.connect_enter(
                        clone!(@strong self as this => move |_controller, _x, _y| {
                            let imp = this.imp();
                            imp.main_button.show();
                        })
                    );
                    ctrl.connect_leave(
                        clone!(@strong self as this => move |_controller| {
                            let imp = this.imp();
                            imp.main_button.hide();
                        })
                    );
                    self.add_controller(ctrl);

                } else {
                    imp.name_label.hide();
                    imp.main_button.show();
                }
                

                PlaceHolderGeneric::new(artist.name(), "person2-symbolic", 200, artist.image_id())
                
            
            },
            GenericChild::Genre => {
                let genre = genre.unwrap();

                imp.id.set(genre.id());
                imp.album_count_label.set_label(&format!("{} albums", genre.n_albums()));
                imp.track_count_label.set_label(&format!("{} tracks", genre.n_tracks()));
                //imp.name_label.set_label(&genre.name());
                imp.name_label.hide();
                imp.main_button.show();
                PlaceHolderGeneric::new(genre.name(), "music-note-symbolic", 200, None)
            },
        };

        self.imp().art_bin.set_child(Some(&bg));

    }

    pub fn initialize(&self) {
        let imp = self.imp();

        imp.main_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                let id = this.imp().id.get().clone();
                this.emit_by_name::<()>("clicked", &[&id]);
            })
        );
    }

    fn update_sort_ui(&self) {
        let imp = self.imp();
        match imp.sort_method.get() {
            SortMethod::TrackCount => {
                imp.album_count_label.hide();
                imp.track_count_label.show();
            },
            SortMethod::AlbumCount => {
                imp.album_count_label.show();
                imp.track_count_label.hide();
            },
            _ => {
                imp.album_count_label.hide();
                imp.track_count_label.hide();
            },
        }
    }

}
    