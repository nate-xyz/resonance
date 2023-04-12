/* genre_detail_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, glib, glib::clone, CompositeTemplate};

use std::{cell::RefCell, cell::Cell, rc::Rc};
use std::time::Duration;
use html_escape;
use log::debug;

use crate::util::{model, player, settings_manager};
use crate::model::genre::Genre;
use crate::model::album::Album;
use crate::views::album_card::AlbumCard;
use crate::search::{FuzzyFilter, SearchSortObject};
use crate::sort::{FuzzySorter, SortMethod};

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecEnum
    };
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/genre_detail_page.ui")]
    pub struct GenreDetailPagePriv {
        #[template_child(id = "separator")]
        pub separator: TemplateChild<gtk::Separator>,

        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child(id = "name_label")]
        pub name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "back_button")]
        pub back_button: TemplateChild<gtk::Button>,

        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        #[template_child(id = "grid_view")]
        pub grid_view: TemplateChild<gtk::GridView>,
        
        #[template_child(id = "sort-menu")]
        pub sort_menu: TemplateChild<gio::Menu>,
        
        pub genre: RefCell<Option<Rc<Genre>>>,
        pub albums: RefCell<Option<Vec<Rc<Album>>>>,
        pub cards: RefCell<Option<Vec<Rc<AlbumCard>>>>,
        pub sort_method: Cell<SortMethod>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
        pub sorter: RefCell<Option<FuzzySorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GenreDetailPagePriv {
        const NAME: &'static str = "GenreDetailPage";
        type Type = super::GenreDetailPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GenreDetailPagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecEnum::builder::<SortMethod>("sort-method").build()
                    ]
                );
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "sort-method" => {
                    let sort_method = value.get().expect("The value needs to be of type `enum`.");
                    self.sort_method.replace(sort_method);
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
                    Signal::builder("back").build(),
                    Signal::builder("album-click")
                        .param_types([<i64>::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for GenreDetailPagePriv {}
    impl BoxImpl for GenreDetailPagePriv {}
    impl GenreDetailPagePriv {}
}

glib::wrapper! {
    pub struct GenreDetailPage(ObjectSubclass<imp::GenreDetailPagePriv>)
    @extends gtk::Box, gtk::Widget;
}


impl GenreDetailPage {
    pub fn new() -> GenreDetailPage {
        let genre_detail: GenreDetailPage = glib::Object::builder::<GenreDetailPage>().build();
        genre_detail
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        let settings = settings_manager();

        settings.bind("full-page-back-button", &*imp.back_button, "visible")
            .flags(gio::SettingsBindFlags::GET)
            .build();
        
        imp.grid_view.remove_css_class("view");

        imp.back_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                this.emit_by_name::<()>("back", &[]);
            })
        );

        imp.play_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                let albums = this.albums();
                let mut tracks = Vec::new();
                for album in albums {
                    let mut album_tracks = album.tracks();
                    tracks.append(&mut album_tracks);
                }
                player().clear_play_album(tracks.clone(), Some(this.genre().name()));
            })
        );

        imp.add_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                let albums = this.albums();
                let mut tracks = Vec::new();
                for album in albums {
                    let mut album_tracks = album.tracks();
                    tracks.append(&mut album_tracks);
                }
                player().add_album(tracks.clone());                
            })
        );

        let list_item_factory = gtk::SignalListItemFactory::new();
        list_item_factory.connect_setup(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_activatable(false);
                let album_card = AlbumCard::new();
                list_item.set_child(Some(&album_card));
            })
        );

        list_item_factory.connect_bind(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let album_card = list_item.child().unwrap().downcast::<AlbumCard>().expect("AlbumCard is of wrong type");
                let album = list_item.item().unwrap().clone().downcast::<Album>().expect("Album is of wrong type");
                album_card.load_album(Rc::new(album));
            })
        );

        imp.grid_view.set_factory(Some(&list_item_factory));

        imp.scrolled_window.vadjustment().connect_notify_local(
            Some("value"),
            clone!(@weak self as this => move |adj, _| {
                let imp = this.imp();
                if adj.value() > 15.0 {
                    if !imp.separator.is_visible() {
                        imp.separator.show();
                    }
                } else {
                    if imp.separator.is_visible() {
                        imp.separator.hide();
                    }
                }
            }),
        );
    }

    pub fn update_genre(&self, genre_id: i64) {
        let imp = self.imp();
        let genre = model().genre(genre_id);
        let list_store = gio::ListStore::new(Album::static_type());
        match genre {
            Ok(genre) => {
                for album in genre.albums().unwrap().iter() {
                    list_store.append(album.as_ref());
                }
                imp.albums.replace(genre.albums());
                imp.genre.replace(Some(genre));
                self.update_view();
            }
            Err(msg) => {
                debug!("Unable to load genre: {}", msg);
                self.emit_by_name::<()>("back", &[]);
            }
        }

        let filter = FuzzyFilter::new(SearchSortObject::Album);
        let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);
        filter_model.set_model(Some(&list_store));
        filter_model.set_filter(Some(&filter));

        let sorter = FuzzySorter::new(SearchSortObject::Album);
        let sorter_model = gtk::SortListModel::new(None::<gio::ListStore>, None::<FuzzySorter>);
        sorter_model.set_model(Some(&filter_model));
        sorter_model.set_sorter(Some(&sorter));

        let selection_model = gtk::NoSelection::new(Some(sorter_model));
        imp.grid_view.set_model(Some(&selection_model));

        // imp.search_entry.bind_property("text", &filter, "search")
        //     .flags(glib::BindingFlags::SYNC_CREATE)
        //     .build();

        // imp.search_entry.bind_property("text", &sorter, "search")
        //     .flags(glib::BindingFlags::SYNC_CREATE)
        //     .build();

        imp.search_entry.connect_notify_local(
            Some("text"),
            clone!(@strong self as this => move |search_entry, _text| {
                let imp = this.imp();
                let text = search_entry.text().to_string();
                imp.search_string.replace(Some(text.clone()));
                let timeout_duration = Duration::from_millis(50);
                let _source_id = glib::timeout_add_local(timeout_duration,
                    clone!(@strong this as that, @strong text => @default-return Continue(false) , move || {
                        let imp = that.imp();
                        if let Some(current_search) = imp.search_string.take() {
                            if current_search == text {
                                if let Some(filter) = imp.filter.borrow().as_ref() {
                                    filter.set_search(Some(current_search.clone()));
                                }
                                if let Some(sorter) = imp.sorter.borrow().as_ref() {
                                    sorter.set_search(Some(current_search));
                                }
                            }
                        }
                        Continue(false)
                    }),
                );
            }),  
        );
        
        self.connect_notify_local(Some("sort-method"),
            clone!(@strong self as this => move |_, _| {
                let imp = this.imp();
                let sort_method = imp.sort_method.get();
                if let Some(sorter) = imp.sorter.borrow().as_ref() {
                    sorter.set_method(sort_method);
                }
            }),
        );

        imp.filter.replace(Some(filter));
        imp.sorter.replace(Some(sorter));
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        if let Some(genre) = imp.genre.borrow().as_ref() {
            imp.name_label.set_label(&html_escape::encode_text_minimal(&genre.name()).to_string());
        }
    }

    pub fn on_toggle_search_button(&self) {
        let imp = self.imp();
        imp.search_bar.set_search_mode(!imp.search_bar.is_search_mode());
        if !imp.search_bar.is_search_mode() {
            imp.search_bar.grab_focus();
        }
    }

    pub fn sort_menu(&self) -> &gio::Menu {
        &self.imp().sort_menu
    }

    pub fn genre(&self) -> Rc<Genre> {
        self.imp().genre.borrow().as_ref().unwrap().clone()
    }

    pub fn albums(&self) -> Vec<Rc<Album>> {
        self.imp().albums.borrow().as_ref().unwrap().clone()
    }

}
    