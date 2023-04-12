/* album_grid_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{self, glib::{self, clone}, gio::{self, ListStore}};
use adw::{prelude::*, subclass::prelude::*};

use std::{cell::{RefCell, Cell}, rc::Rc};
use std::time::Duration;
use log::debug;

use crate::model::album::Album;
use crate::search::{FuzzyFilter, SearchSortObject, SearchMethod};
use crate::sort::{FuzzySorter, SortMethod};
use crate::util::model;

use super::album_sidebar::AlbumFlap;
use super::album_grid_child::AlbumGridChild;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean, ParamSpecEnum
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/album_grid_page.ui")]
    pub struct AlbumGridPagePriv {
        #[template_child(id = "separator")]
        pub separator: TemplateChild<gtk::Separator>,

        #[template_child(id = "flow_box")]
        pub flow_box: TemplateChild<gtk::FlowBox>,

        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child(id = "adwflap")]
        pub adwflap: TemplateChild<adw::Flap>,

        #[template_child(id = "album_sidebar")]
        pub album_sidebar: TemplateChild<AlbumFlap>,

        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        
        #[template_child(id = "drop_down")]
        pub drop_down: TemplateChild<gtk::DropDown>,

        #[template_child(id = "sort-menu")]
        pub sort_menu: TemplateChild<gio::Menu>,

        pub list_store: RefCell<Option<Rc<ListStore>>>,
        pub display_labels_default: Cell<bool>,
        pub disable_flap: Cell<bool>,
        pub hidden: Cell<bool>,
        pub search_method: Cell<SearchMethod>,
        pub sort_method: Cell<SortMethod>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
        pub sorter: RefCell<Option<FuzzySorter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AlbumGridPagePriv {
        const NAME: &'static str = "AlbumGridPage";
        type Type = super::AlbumGridPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for AlbumGridPagePriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![
                    ParamSpecBoolean::builder("hidden").default_value(false).build(),
                    ParamSpecBoolean::builder("disable-flap").default_value(false).build(),
                    ParamSpecBoolean::builder("display-labels-default").default_value(false).build(),
                    ParamSpecEnum::builder::<SortMethod>("sort-method").build()
                    ]
                );
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "hidden" => {
                    let hidden = value.get().expect("The value needs to be of type `bool`.");
                    self.hidden.replace(hidden);
                },
                "disable-flap" => {
                    let disable_flap = value.get().expect("The value needs to be of type `bool`.");
                    self.disable_flap.replace(disable_flap);
                },
                "display-labels-default" => {
                    let display_labels_default = value.get().expect("The value needs to be of type `bool`.");
                    self.display_labels_default.replace(display_labels_default);
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
                "disable-flap" => self.disable_flap.get().to_value(),
                "display-labels-default" => self.display_labels_default.get().to_value(),
                "sort-method" => self.sort_method.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("album-selected")
                        .param_types([<i64>::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for AlbumGridPagePriv {}
    impl BoxImpl for AlbumGridPagePriv {}
    impl AlbumGridPagePriv {}
}

glib::wrapper! {
    pub struct AlbumGridPage(ObjectSubclass<imp::AlbumGridPagePriv>)
    @extends gtk::Box, gtk::Widget;
}

impl Default for AlbumGridPage {
    fn default() -> Self {
        glib::Object::builder::<AlbumGridPage>().build()
    }
}

impl AlbumGridPage {
    pub fn new() -> AlbumGridPage {
        Self::default()
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        self.connect_signals();

        let list_store = gio::ListStore::new(Album::static_type());
        
        let filter = FuzzyFilter::new(SearchSortObject::Album);
        let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);
        filter_model.set_model(Some(&list_store));
        filter_model.set_filter(Some(&filter));

        let sorter = FuzzySorter::new(SearchSortObject::Album);
        let sorter_model = gtk::SortListModel::new(None::<gio::ListStore>, None::<FuzzySorter>);
        sorter_model.set_model(Some(&filter_model));
        sorter_model.set_sorter(Some(&sorter));

        let selection = gtk::NoSelection::new(Some(sorter_model));
        
        imp.flow_box.bind_model(Some(&selection), 
        clone!(@strong self as this => @default-panic, move |obj| {
                let album = obj.clone().downcast::<Album>().expect("Album is of wrong type");
                let album_grid_child = AlbumGridChild::new();
                album_grid_child.connect_local("clicked", true,
                clone!(@strong this => @default-return None, move |value| {
                        let album_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                        debug!("Clicked album grid child, album_id: {}", album_id);
                        this.on_album_click_with_id(album_id);
                        None
                    })
                );
                let imp = this.imp();

                album_grid_child.load_album(Rc::new(album));
                album_grid_child.visible_labels(imp.search_method.get(), imp.search_bar.is_search_mode());

                this.bind_property("sort-method", &album_grid_child, "sort-method")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                this.bind_property("display-labels-default", &album_grid_child, "display-labels-default")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                return album_grid_child.upcast::<gtk::Widget>();
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
                debug!("{}", text);
                let timeout_duration = Duration::from_millis(50);
                let _source_id = glib::timeout_add_local(timeout_duration,
                    clone!(@strong this as that, @strong text => @default-return Continue(false) , move || {
                        let imp = that.imp();
                        if let Some(current_search) = imp.search_string.take() {
                            if current_search == text {
                                debug!("{}", text);
                                if let Some(filter) = imp.filter.borrow().as_ref() {
                                    filter.set_search(Some(current_search.clone()));
                                }
                                if let Some(sorter) = imp.sorter.borrow().as_ref() {
                                    sorter.set_search(Some(current_search));
                                }
                            } else {
                                debug!("Not the same");
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
                    1 => SearchMethod::Album,
                    2 => SearchMethod::Artist,
                    3 => SearchMethod::Genre,
                    4 => SearchMethod::ReleaseDate,
                    _ => SearchMethod::Full,
                };
                if let Some(filter) = imp.filter.borrow().as_ref() {
                    filter.set_method(search_method);
                }
                imp.search_method.set(search_method);
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

    fn connect_signals(&self) {
        let imp = self.imp();

        model().connect_local("refresh-albums", false, 
        clone!(@weak self as this => @default-return None, move |_args| {
                this.update_view();
                None
            })
        );

        imp.album_sidebar.connect_local("back", false, 
        clone!(@strong self as this => @default-return None, move |_| {
                this.imp().adwflap.set_reveal_flap(false);
                None
            })
        );

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

    pub fn update_view(&self) {
        let list_store = self.list_store();
        list_store.remove_all();

        if let Some(map) = model().albums() {
            if !map.is_empty() {
                for (_id, albums) in map.iter() {
                    list_store.append(albums.as_ref());
                }
                self.set_property("hidden", false.to_value());
                return;
            }
        }
        debug!("No albums in model.");
        self.set_property("hidden", true.to_value());
    }

    fn on_album_click_with_id(&self, id: i64) {
        let imp = self.imp();

        if imp.disable_flap.get() {
            self.emit_by_name::<()>("album-selected", &[&id]);
        } else {
            imp.adwflap.set_reveal_flap(true);
            imp.album_sidebar.load_album(id)
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

    pub fn album_sidebar(&self) -> &AlbumFlap {
        &self.imp().album_sidebar
    }

    fn list_store(&self) -> Rc<ListStore> {
        self.imp().list_store.borrow().clone().unwrap().clone()
    }
}
    