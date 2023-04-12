/* artist_grid_page.rs
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

use crate::model::artist::Artist;
use crate::views::generic_flowbox_child::{GenericFlowboxChild, GenericChild};
use crate::search::{FuzzyFilter, SearchSortObject};
use crate::sort::{FuzzySorter, SortMethod};
use crate::util::model;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean, ParamSpecEnum
    };
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/artist_grid_page.ui")]
    pub struct ArtistGridPagePriv {
        #[template_child(id = "separator")]
        pub separator: TemplateChild<gtk::Separator>,

        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        #[template_child(id = "flow_box")]
        pub flow_box: TemplateChild<gtk::FlowBox>,

        #[template_child(id = "sort-menu")]
        pub sort_menu: TemplateChild<gio::Menu>,

        pub list_store: RefCell<Option<Rc<ListStore>>>,
        pub hidden: Cell<bool>,
        pub sort_method: Cell<SortMethod>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
        pub sorter: RefCell<Option<FuzzySorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArtistGridPagePriv {
        const NAME: &'static str = "ArtistGridPage";
        type Type = super::ArtistGridPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for ArtistGridPagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
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
                "sort-method" => {
                    let sort_method = value.get().expect("The value needs to be of type `enum`.");
                    self.sort_method.replace(sort_method);
                }
                _ => unimplemented!(),
            }
        }
    
        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "hidden" => self.hidden.get().to_value(),
                "sort-method" => self.sort_method.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("artist-selected")
                        .param_types([<i64>::static_type()])
                        .build()
                    ]
            });

            SIGNALS.as_ref()
        }

    }

    impl WidgetImpl for ArtistGridPagePriv {}
    impl BoxImpl for ArtistGridPagePriv {}
    impl ArtistGridPagePriv {}
}

glib::wrapper! {
    pub struct ArtistGridPage(ObjectSubclass<imp::ArtistGridPagePriv>)
    @extends gtk::Box, gtk::Widget;
}

impl ArtistGridPage {
    pub fn new() -> ArtistGridPage {
        let artist_grid: ArtistGridPage = glib::Object::builder::<ArtistGridPage>().build();
        artist_grid
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        model().connect_local("refresh-artists", false, 
        clone!(@weak self as this => @default-return None, move |_args| {
                this.update_view();
                None
            })
        );

        
        let list_store = gio::ListStore::new(Artist::static_type());
        let filter = FuzzyFilter::new(SearchSortObject::Artist);
        let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);
        filter_model.set_model(Some(&list_store));
        filter_model.set_filter(Some(&filter));
        

        let sorter = FuzzySorter::new(SearchSortObject::Artist);
        let sorter_model = gtk::SortListModel::new(None::<gio::ListStore>, None::<FuzzySorter>);
        sorter_model.set_model(Some(&filter_model));
        sorter_model.set_sorter(Some(&sorter));

        let selection = gtk::NoSelection::new(Some(sorter_model));

        imp.flow_box.bind_model(Some(&selection), 
        clone!(@strong self as this => @default-panic, move |obj| {
            let artist = obj.clone().downcast::<Artist>().expect("Artist is of wrong type");
            let generic_grid_child = GenericFlowboxChild::new(GenericChild::Artist, Some(Rc::new(artist)), None);

            generic_grid_child.connect_local("clicked", false, 
            clone!(@strong this => @default-return None, move |value| {
                    let int = value.get(1);
                    match int {
                        Some(int) => {
                            let id = int.get::<i64>().ok().unwrap();
                            this.on_artist_click_with_id(id);
                        }, 
                        None => (),
                    }
                    None
                })
            );

            this.bind_property("sort-method", &generic_grid_child, "sort-method")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            return generic_grid_child.upcast::<gtk::Widget>();
            })
        );

        imp.list_store.replace(Some(Rc::new(list_store)));

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

        imp.scrolled_window.vadjustment().connect_notify_local(
            Some("value"),
            clone!(@weak self as this => move |adj, _| {
                let imp = this.imp();
                if adj.value() > 10.0 {
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

    pub fn update_view(&self) {
        debug!("artist grid update view");
        let list_store = self.list_store();
        list_store.remove_all();

        if let Some(map) = model().artists() {
            if !map.is_empty() {
                for (_id, artists) in map.iter() {
                    list_store.append(artists.as_ref());
                }
                self.set_property("hidden", false.to_value());
                return;
            }
        }
        error!("No artists in model.");
        self.set_property("hidden", true.to_value());
    }

    fn on_artist_click_with_id(&self, id: i64) {
        debug!("{:?}", id);
        self.emit_by_name::<()>("artist-selected", &[&id]);
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
    