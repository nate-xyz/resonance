/* queue_sidebar.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, gio::ListStore, glib, glib::clone, CompositeTemplate};

use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

use crate::model::track::Track;
use crate::views::dialog::save_playlist_dialog::SavePlaylistDialog;
use crate::search::{FuzzyFilter, SearchSortObject};
use crate::i18n::i18n_k;
use crate::util::{self, win, player, seconds_to_string_longform};

use super::track_item::TrackItem;
use super::queue_track::QueueTrack;

mod imp {
    use super::*;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/queue_sidebar.ui")]
    pub struct QueueSidebarPriv {
        #[template_child(id = "clear_queue_button")]
        pub clear_queue_button: TemplateChild<gtk::Button>,

        #[template_child(id = "top_box")]
        pub top_box: TemplateChild<gtk::Box>,
        
        #[template_child(id = "scrolled_window")]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    
        #[template_child(id = "list_box")]
        pub list_box: TemplateChild<gtk::ListBox>,
    
        #[template_child(id = "time_left_label")]
        pub time_left_label: TemplateChild<gtk::Label>,
        
        #[template_child(id = "toggle_search_button")]
        pub toggle_search_button: TemplateChild<gtk::Button>,
    
        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,
    
        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
    
        #[template_child(id = "toggle_playlist_save_button")]
        pub toggle_playlist_save_button: TemplateChild<gtk::Button>,
    
        #[template_child(id = "playlist_title_label")]
        pub playlist_title_label: TemplateChild<gtk::Label>,
    
        #[template_child(id = "edit_button")]
        pub edit_button: TemplateChild<gtk::ToggleButton>,
    
        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track: RefCell<Option<Rc<Track>>>,
        pub list_store: RefCell<Option<Rc<ListStore>>>,
        pub playlist_position: RefCell<Option<u64>>,
        pub edit_mode: Cell<bool>,
        pub rows: RefCell<Vec<Rc<gtk::Widget>>>,
        pub revealed: Cell<bool>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QueueSidebarPriv {
        const NAME: &'static str = "QueueSidebar";
        type Type = super::QueueSidebar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for QueueSidebarPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecBoolean::builder("edit-mode").default_value(false).explicit_notify().build()]);
            PROPERTIES.as_ref()
        }
    
        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "edit-mode" => {
                    let edit_mode_ = value.get().expect("The value needs to be of type `bool`.");
                    self.edit_mode.replace(edit_mode_);
                }
                _ => unimplemented!(),
            }
        }
    
        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "edit-mode" => self.edit_mode.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    impl WidgetImpl for QueueSidebarPriv {}
    impl BinImpl for QueueSidebarPriv {}
    impl QueueSidebarPriv {}
}

glib::wrapper! {
    pub struct QueueSidebar(ObjectSubclass<imp::QueueSidebarPriv>)
    @extends gtk::Widget, adw::Bin;
}

impl QueueSidebar {
    pub fn new() -> QueueSidebar {
        let queue_sidebar: QueueSidebar = glib::Object::builder::<QueueSidebar>().build();
        queue_sidebar
    }

    fn initialize(&self) {
        let imp = self.imp();

        self.bind_state();
        // self.create_menu();

        let list_store = gio::ListStore::new(TrackItem::static_type());
        let filter = FuzzyFilter::new(SearchSortObject::QueueTrack);
        let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);
        filter_model.set_model(Some(&list_store));
        filter_model.set_filter(Some(&filter));

        let selection = gtk::NoSelection::new(Some(filter_model));

        imp.list_box.bind_model(Some(&selection), 
        clone!(@strong self as this => @default-panic, move |obj| {
            let track_item = obj.clone().downcast::<TrackItem>().expect("TrackItem is of wrong type");
            let queue_track = QueueTrack::new();
        
            this.bind_property("edit-mode", &queue_track, "edit-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
            

            queue_track.load_track_item(track_item);    
            return queue_track.upcast::<gtk::Widget>();
            })
        );

        imp.list_box.set_activate_on_single_click(true);
        imp.list_box.connect_row_activated(
            clone!(@strong self as this => @default-panic, move |_list_box, row| {
                if !this.imp().edit_mode.get() {
                    let position = row.clone().downcast::<QueueTrack>().unwrap().playlist_position();
                    player().go_to_playlist_position(position);
                } 
            })
        ); 


        imp.clear_queue_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().end_queue();
            })
        );

        imp.edit_button.connect_toggled(
            clone!(@strong self as this => move |_button| {
                this.set_edit_mode(!this.imp().edit_mode.get());
            })
        );

        imp.toggle_playlist_save_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let track_ids = player().track_ids();
                let save_dialog = SavePlaylistDialog::new(track_ids);
                save_dialog.set_transient_for(Some(&win(this.upcast_ref())));
                save_dialog.show();
            })
        );

        imp.toggle_search_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                imp.search_bar.set_search_mode(!imp.search_bar.is_search_mode())
            }),
        );


        imp.list_store.replace(Some(Rc::new(list_store)));

        // imp.search_entry.bind_property("text", &filter, "search")
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
                            }
                        }
                        Continue(false)
                    }),
                );
            }),  
        );
        imp.filter.replace(Some(filter));

    }


    // Bind the PlayerState to the UI
    fn bind_state(&self) {
        let player = player();

        player.state().connect_notify_local(
            Some("queue-title"),
            clone!(@strong self as this => move |_, _| {
                let queue_title = util::player().state().queue_title();
                this.imp().playlist_title_label.set_label(&queue_title);
            }),
        );

        player.state().connect_notify_local(
            Some("queue-time-remaining"),
            clone!(@strong self as this => move |_, _| {
                let imp = this.imp();
                let queue_time_remaining = util::player().state().queue_time_remaining();

                if queue_time_remaining > 0.0 {
                    if !imp.time_left_label.is_visible() {
                        imp.time_left_label.show();
                    }
                    imp.time_left_label.set_label(&i18n_k("{time_remaining} remaining", &[("time_remaining", &seconds_to_string_longform(queue_time_remaining as f64))]));
                } else {
                    imp.time_left_label.hide();
                }
            }),
        );

        // Update current track
        player.state().connect_local(
            "queue-update", false,
            clone!(@strong self as this => move |_| {
                this.reload_ui_from_playlist();
                None
            }),
        );

        player.state().connect_local(
            "queue-position", false,
            clone!(@strong self as this => move |value| {
                let int = value.get(1).unwrap().get::<u64>().ok().unwrap();
                this.update_current_position_playing_icon(int);
                None
            }),
        );
    }

    fn set_edit_mode(&self, edit: bool) {
        let imp = self.imp();
        imp.edit_mode.set(edit);
        self.edit_button_mode(edit);
        self.notify("edit-mode");
    }

    fn reload_ui_from_playlist(&self) {
        self.clear_children();
        let tracks = player().tracks();
        for (position, track) in tracks.iter().enumerate() {
            let search_string = format!("{} {} {}", track.title(), track.album(), track.artist());
            let track_item = TrackItem::new(track.clone(), position as u64, search_string);
            self.list_store().append(&track_item);
        }
    }

    fn update_current_position_playing_icon(&self, position: u64) {
        let imp = self.imp();

        let row_children = imp.list_box.observe_children().snapshot();
        let row_children_len = row_children.len();
        if row_children_len <= 0 || position as usize >= row_children_len {
            return;
        }

        if let Some(pos) = imp.playlist_position.borrow().clone() {
            if (pos as usize) < row_children_len {
                let child = &row_children[pos as usize];
                child.clone().downcast_ref::<QueueTrack>().unwrap().is_playing(false);
            }
        }
   
        imp.playlist_position.replace(Some(position));
        let child = &row_children[position as usize];
        child.clone().downcast_ref::<QueueTrack>().unwrap().is_playing(true);
        imp.list_box.select_row(child.clone().downcast_ref::<QueueTrack>());
    }

    fn clear_children(&self) {
        let list_store = self.list_store();
        list_store.remove_all();
    }

    fn list_store(&self) -> Rc<ListStore> {
        self.imp().list_store.borrow().clone().unwrap().clone()
    }

    pub fn edit_button_mode(&self, mode: bool) {
        let imp = self.imp();
        let button: &gtk::Button = self.imp().edit_button.as_ref();
        if mode {
            imp.clear_queue_button.show();
            button.set_css_classes(&[&"opaque", &"suggested-action"]);
        } else {
            imp.clear_queue_button.hide();
            button.set_css_classes(&[&"flat"]);
        }
    }

    #[allow(dead_code)]
    fn create_menu(&self) {
        let imp = self.imp();
    
        let main = gio::Menu::new();
        let menu = gio::Menu::new();
    
        let menu_item = gio::MenuItem::new(Some(&format!("Clear Queue")), None);
        menu_item.set_action_and_target_value(Some("win.clear-queue"), None);
        menu.append_item(&menu_item);

    
        main.append_section(Some("Queue"), &menu);
    
        
        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&format!("Create Playlist from Queue")), None);
        menu_item.set_action_and_target_value(Some("win.save-playlist"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&format!("Add Queue to Playlist")), None);
        menu_item.set_action_and_target_value(Some("win.add-queue-to-playlist"), None);
        menu.append_item(&menu_item);
    
        main.append_section(Some("Playlist"), &menu);

        imp.popover.set_menu_model(Some(&main));
    }
}
    