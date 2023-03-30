/* track_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, gio::ListStore, glib, glib::clone, CompositeTemplate};

use std::{cell::{RefCell, Cell}, rc::Rc};
use std::time::Duration;
use log::{debug, error};

use crate::model::track::Track;
use crate::search::{FuzzyFilter, SearchSortObject, SearchMethod};
use crate::sort::{FuzzySorter, SortMethod};
use crate::util::{player, model};

use super::track_page_row::TrackPageRow;

mod imp {
    use super::*;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean, ParamSpecEnum
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/track_page.ui")]
    pub struct TrackPagePriv {
        #[template_child(id = "list_view")]
        pub list_view: TemplateChild<gtk::ListView>,

        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        #[template_child(id = "drop_down")]
        pub drop_down: TemplateChild<gtk::DropDown>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "scroll_top_button")]
        pub scroll_top_button: TemplateChild<gtk::Button>,

        #[template_child(id = "scroll_bottom_button")]
        pub scroll_bottom_button: TemplateChild<gtk::Button>,

        #[template_child(id = "revealer")]
        pub revealer: TemplateChild<gtk::Revealer>,

        #[template_child(id = "sort-menu")]
        pub sort_menu: TemplateChild<gio::Menu>,
        

        pub last_frame_counter: RefCell<i64>,
        pub list_store: RefCell<Option<Rc<ListStore>>>,
        pub hidden: Cell<bool>,
        pub display_labels_default: Cell<bool>,
        pub search_mode_default: Cell<bool>,
        pub search_method: Cell<SearchMethod>,
        pub sort_method: Cell<SortMethod>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
        pub sorter: RefCell<Option<FuzzySorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrackPagePriv {
        const NAME: &'static str = "TrackPage";
        type Type = super::TrackPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TrackPagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecBoolean::builder("search-mode-default").default_value(true).build(),
                    ParamSpecBoolean::builder("display-labels-default").default_value(false).build(),
                    ParamSpecBoolean::builder("hidden").default_value(false).build(),
                    ParamSpecEnum::builder::<SortMethod>("sort-method").build()
                    ]);
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "hidden" => {
                    let hidden = value.get().expect("The value needs to be of type `bool`.");
                    self.hidden.replace(hidden);
                },
                "search-mode-default" => {
                    let search_mode_default = value.get().expect("The value needs to be of type `bool`.");
                    self.search_mode_default.replace(search_mode_default);
                },
                "display-labels-default" => {
                    let display_labels_default = value.get().expect("The value needs to be of type `bool`.");
                    self.display_labels_default.replace(display_labels_default);
                },
                "sort-method" => {
                    let sort_method = value.get().expect("The value needs to be of type `enum`.");
                    self.sort_method.replace(sort_method);
                }
                _ => {
                    error!("unimplemented property: {}", pspec.name());
                    unimplemented!()
                },
            }
        }
    
        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "hidden" => self.hidden.get().to_value(),
                "display-labels-default" => self.display_labels_default.get().to_value(),
                "search-mode-default" => self.search_mode_default.get().to_value(),
                "sort-method" => self.sort_method.get().to_value(),
                _ => {
                    error!("unimplemented property: {}", pspec.name());
                    unimplemented!()
                },
            }
        }
    }

    impl WidgetImpl for TrackPagePriv {}
    impl BinImpl for TrackPagePriv {}
    impl TrackPagePriv {}
}

glib::wrapper! {
    pub struct TrackPage(ObjectSubclass<imp::TrackPagePriv>)
    @extends adw::Bin, gtk::Widget;
}


impl TrackPage {
    pub fn new() -> TrackPage {
        let track_page: TrackPage = glib::Object::builder::<TrackPage>().build();
        track_page
    }

    fn initialize(&self) {
        let imp = self.imp();

        model().connect_local(
            "refresh-tracks", 
            false, 
        clone!(@weak self as this => @default-return None, move |_args| {
                this.update_view();
                None
            })
        );

        self.bind_property("search-mode-default", &*imp.search_bar, "search-mode-enabled")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        let list_item_factory = gtk::SignalListItemFactory::new();
        list_item_factory.connect_setup(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let track_page_row = TrackPageRow::new();
                
                this.bind_property("sort-method", &track_page_row, "sort-method")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                this.bind_property("display-labels-default", &track_page_row, "display-labels-default")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                list_item.set_child(Some(&track_page_row));
            })
        );

        list_item_factory.connect_bind(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let track_page_row = list_item.child().unwrap().downcast::<TrackPageRow>().expect("TrackPageRow is of wrong type");
                let track = list_item.item().unwrap().clone().downcast::<Track>().expect("Track is of wrong type");
                track_page_row.update_track(Rc::new(track));
                let imp = this.imp();
                track_page_row.visible_labels(imp.search_method.get(), imp.search_bar.is_search_mode());
            })
        );

        let list_store = gio::ListStore::new(Track::static_type());
        
        imp.list_view.set_factory(Some(&list_item_factory));
        imp.list_store.replace(Some(Rc::new(list_store)));

        let controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        
        controller.connect_scroll(
            clone!(@strong self as this => @default-return gtk::Inhibit(false), move |_, _, _| {
                let mut last_frame_counter = this.imp().last_frame_counter.borrow_mut();
                let new_frame_counter = this.imp().scrolled_window.clone().frame_clock().unwrap().frame_counter();
                if *last_frame_counter == new_frame_counter {
                    gtk::Inhibit(true) // Inhibit scroll event to work around bug: https://gitlab.gnome.org/GNOME/gtk/-/issues/2971
                } else {
                    *last_frame_counter = new_frame_counter;
                    gtk::Inhibit(false)
                }
            })
        );
        

        imp.scrolled_window.add_controller(controller);
    }

    pub fn update_view(&self) {
        debug!("track page update view");
        let imp = self.imp();
        let list_store = self.list_store();
        imp.list_view.set_model(None::<&gtk::SingleSelection>);
        list_store.remove_all();

        self.update_list_store();

        let filter = FuzzyFilter::new(SearchSortObject::Track);
        let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);
        filter_model.set_model(Some(&*list_store));
        filter_model.set_filter(Some(&filter));

        let sorter = FuzzySorter::new(SearchSortObject::Track);
        let sorter_model = gtk::SortListModel::new(None::<gio::ListStore>, None::<FuzzySorter>);
        sorter_model.set_model(Some(&filter_model));
        sorter_model.set_sorter(Some(&sorter));

        let selection_model = gtk::SingleSelection::new(Some(sorter_model));

        imp.list_view.set_model(Some(&selection_model));
        imp.list_view.connect_activate(
            clone!(@strong self as this => move |_list_view, _position| {
                let track = selection_model.selected_item().unwrap().clone().downcast::<Track>().expect("Track is of wrong type");
                player().clear_play_track(Rc::new(track));
            })
        );

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

        imp.drop_down.connect_notify_local(Some("selected"),
        clone!(@strong self as this => move |_, _| {
            let imp = this.imp();
            let selected = imp.drop_down.selected();
            let search_method = match selected {
                0 => SearchMethod::Full,
                1 => SearchMethod::Track,
                2 => SearchMethod::Album,
                3 => SearchMethod::Artist,
                4 => SearchMethod::Genre,
                5 => SearchMethod::ReleaseDate,
                _ => SearchMethod::Full,
            };
            if let Some(filter) = imp.filter.borrow().as_ref() {
                filter.set_method(search_method);
            }
            imp.search_method.set(search_method);
        }),
    );

        self.connect_notify_local(
        Some("sort-method"),
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

    fn update_list_store(&self) {
        let list_store = self.list_store();
        if let Some(map) = model().tracks() {
            if !map.is_empty() {
                for (_id, track) in map.iter() {
                    list_store.append(track.as_ref());
                }
                self.set_property("hidden", false.to_value());
                return;
            }
        }
        debug!("No tracks in model.");
        self.set_property("hidden", true.to_value());
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

    fn list_store(&self) -> Rc<ListStore> {
        self.imp().list_store.borrow().clone().unwrap().clone()
    }
}