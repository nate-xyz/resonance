/* playlist_detail_page.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use adw::prelude::*;

use gtk::{gio, gio::ListStore, glib, glib::clone, CompositeTemplate};
use std::{cell::RefCell, cell::Cell, rc::Rc};
use std::time::Duration;
use log::{debug, error};

use crate::views::dialog::{
    delete_playlist_dialog::DeletePlaylistDialog,
    duplicate_playlist_dialog::DuplicatePlaylistDialog,
    confirm_rename_playlist_dialog::ConfirmRenamePlaylistDialog,
};
use crate::model::{
    playlist::Playlist,
    playlist_entry::PlaylistEntry,
};
use crate::views::art::{
    grid_art::GridArt,
    placeholder_art::PlaceHolderArt,
};
use crate::util::{model, player, seconds_to_string_longform, win, settings_manager};
use crate::search::{FuzzyFilter, SearchSortObject};

use super::track_item::PlaylistDetailTrackItem;
use super::playlist_detail_row::PlaylistDetailRow;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean
    };
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/playlist_detail_page.ui")]
    pub struct PlaylistDetailPagePriv {
        #[template_child(id = "art-and-info")]
        pub art_and_info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "list_view")]
        pub list_view: TemplateChild<gtk::ListView>,

        #[template_child(id = "search_bar")]
        pub search_bar: TemplateChild<gtk::SearchBar>,

        #[template_child(id = "search_entry")]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child(id = "info_box")]
        pub info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<gtk::Label>,
        
        #[template_child(id = "desc_label")]
        pub desc_label: TemplateChild<gtk::Label>,

        #[template_child(id = "desc_entry")]
        pub desc_entry: TemplateChild<adw::EntryRow>,

        #[template_child(id = "track_amount_label")]
        pub track_amount_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "creation_date_label")]
        pub creation_date_label: TemplateChild<gtk::Label>,

        #[template_child(id = "modified_date_label")]
        pub modified_date_label: TemplateChild<gtk::Label>,

        #[template_child(id = "overlay")]
        pub overlay: TemplateChild<gtk::Overlay>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "back_button")]
        pub back_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_button_box")]
        pub second_button_box: TemplateChild<gtk::Box>,

        #[template_child(id = "second_play_button")]
        pub second_play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "second_add_button")]
        pub second_add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "adw_entry")]
        pub adw_entry: TemplateChild<adw::EntryRow>,

        #[template_child(id = "list_title_label")]
        pub list_title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "edit_button")]
        pub edit_button: TemplateChild<gtk::ToggleButton>,

        #[template_child(id = "edit_icon")]
        pub edit_icon: TemplateChild<gtk::Image>,

        #[template_child(id = "delete_button")]
        pub delete_button: TemplateChild<gtk::Button>,

        #[template_child(id = "duplicate_button")]
        pub duplicate_button: TemplateChild<gtk::Button>,


        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub art: RefCell<Option<GridArt>>,
        pub placeholder_art: RefCell<Option<PlaceHolderArt>>,
        
        pub current_title: RefCell<String>,
        pub current_description: RefCell<String>,

        pub playlist: RefCell<Option<Rc<Playlist>>>,

        pub list_store: RefCell<Option<Rc<ListStore>>>,
        pub selection_model: RefCell<Option<Rc<gtk::SingleSelection>>>,
        pub edit_mode: Cell<bool>,
        pub queued_for_removal: RefCell<Vec<Rc<PlaylistEntry>>>,
        pub sorter: RefCell<Option<gtk::Sorter>>,
        pub search_string: RefCell<Option<String>>,
        pub filter: RefCell<Option<FuzzyFilter>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistDetailPagePriv {
        const NAME: &'static str = "PlaylistDetailPage";
        type Type = super::PlaylistDetailPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistDetailPagePriv {
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

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("back").build(),
                ]
            });

            SIGNALS.as_ref()
        }

    }

    impl WidgetImpl for PlaylistDetailPagePriv {}
    impl BoxImpl for PlaylistDetailPagePriv {}
    impl PlaylistDetailPagePriv {}
}

glib::wrapper! {
    pub struct PlaylistDetailPage(ObjectSubclass<imp::PlaylistDetailPagePriv>)
    @extends gtk::Box, gtk::Widget;
}


impl PlaylistDetailPage {
    pub fn new() -> PlaylistDetailPage {
        let playlist_detail: PlaylistDetailPage = glib::Object::builder::<PlaylistDetailPage>().build();
        playlist_detail
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        let settings = settings_manager();

        settings.bind("full-page-back-button", &*imp.back_button, "visible")
            .flags(gio::SettingsBindFlags::GET)
            .build();

        imp.popover.set_parent(self);

        self.connect_local(
            "back",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.set_edit_mode(false);
                None
            }),
        );

        model().connect_local(
            "refresh-playlist",
            false,
            clone!(@strong self as this => @default-return None, move |value| {
                let id = value.get(1).unwrap().get::<u64>().ok().unwrap() as i64;
                if id == this.playlist().id() {
                    debug!("refreshing detail page.");
                    this.load_playlist(id);
                }
                None
            }),
        );


        let list_item_factory = gtk::SignalListItemFactory::new();
        list_item_factory.connect_setup(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let playlist_detail_row = PlaylistDetailRow::new();

                this.bind_property("edit-mode", &playlist_detail_row, "edit-mode")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                list_item.set_child(Some(&playlist_detail_row));
            })
        );

        list_item_factory.connect_bind(
            clone!(@strong self as this => move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                let playlist_detail_row = list_item.child().unwrap().downcast::<PlaylistDetailRow>().expect("PlaylistDetailRow is of wrong type");
                let track_item = list_item.item().unwrap().clone().downcast::<PlaylistDetailTrackItem>().expect("PlaylistDetailTrackItem is of wrong type");
                playlist_detail_row.update_track_item(track_item); 
            })
        );

        let list_views = [&imp.list_view];
        for list_view in list_views {
            list_view.set_factory(Some(&list_item_factory));
            list_view.connect_activate(
                clone!(@strong self as this => move |_list_view, _position| {
                    if !this.imp().edit_mode.get() {
                        let track_item = this.selection_model().selected_item().unwrap().clone().downcast::<PlaylistDetailTrackItem>().expect("PlaylistDetailTrackItem is of wrong type");
                        player().clear_play_track(track_item.track());
                    }
                })
            );
        }

        let list_store = gio::ListStore::new(PlaylistDetailTrackItem::static_type());
        imp.list_store.replace(Some(Rc::new(list_store)));

        imp.back_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                this.emit_by_name::<()>("back", &[]);
            })
        );

        let play_buttons = [&imp.play_button, &imp.second_play_button];
        for play in play_buttons {
            play.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let playlist = this.playlist();
                    let tracks = playlist.tracks();
                    player().clear_play_album(tracks, Some(playlist.title()));
                }),
            );
        }

        let add_buttons = [&imp.add_button, &imp.second_add_button];
        for add in add_buttons {
            add.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let tracks = this.playlist().tracks();
                    player().add_album(tracks);
                }),
            );
        }

        let edit_buttons = [&imp.edit_button];
        for edit in edit_buttons {
            edit.connect_toggled(
                clone!(@strong self as this => move |_button| {
                    let imp = this.imp();
                    let edit_mode = imp.edit_mode.get();

                    if edit_mode {
                        let playlist = this.playlist();
                        let new_title = imp.adw_entry.text().to_string();

                        let mut title_option = None;
                        if new_title != playlist.title() {
                            title_option = Some(new_title);
                        }

                        let description = imp.desc_entry.text().to_string();
                        let mut desc_option = None;
                        if description != playlist.description() {
                            desc_option = Some(description);
                        }

                        if !title_option.is_none() || !desc_option.is_none() {
                            let dialog = ConfirmRenamePlaylistDialog::new(playlist.id(), title_option, desc_option);
                            dialog.set_transient_for(Some(&win(this.upcast_ref())));
                            dialog.connect_local(
                                "done", 
                                false, 
                                clone!(@strong this as that => move |_| {
                                    that.set_edit_mode(false);
                                    None
                                })
                            );
                            dialog.show();
                        }



                    }

                    this.set_edit_mode(!edit_mode);



                })
            );
        }


        let duplicate_buttons = [&imp.duplicate_button];
        for duplicate in duplicate_buttons {
            duplicate.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let dialog = DuplicatePlaylistDialog::new(this.playlist());
                    dialog.set_transient_for(Some(&win(this.upcast_ref())));
                    dialog.connect_local(
                        "done", 
                        false, 
                        clone!(@strong this as that => move |_| {
                            that.set_edit_mode(false);
                            that.emit_by_name::<()>("back", &[]);
                            None
                        })
                    );
                    dialog.show();
                }),
            );
        }

        let delete_playlist_buttons = [&imp.delete_button];
        for delete in delete_playlist_buttons {
            delete.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let dialog = DeletePlaylistDialog::new(this.playlist().id());
                    dialog.set_transient_for(Some(&win(this.upcast_ref())));
                    dialog.connect_local(
                        "done", 
                        false,
                        clone!(@strong this as that => move |_| {
                            that.set_edit_mode(false);
                            that.emit_by_name::<()>("back", &[]);
                            None
                        })
                    );
                    dialog.show();
                }),
            );
        }

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();
                imp.overlay_box.show();
            })
        );
        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();
                imp.overlay_box.hide();
            })
        );
        imp.overlay.add_controller(ctrl);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, x, y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             let height = this.height();
        //             let width = this.width() / 2;
        //             let y = y as i32;
        //             let x = x as i32;

        //             let offset = imp.popover.offset();
        //             debug!("{} {} {:?}", width, x, offset);

        //             imp.popover.set_offset(x - width, y - height);

        //             let offset = imp.popover.offset();
        //             debug!("{} {} {:?}", width, x, offset);

        //             imp.popover.popup();
        //         }
        //     })
        // );
        // imp.art_and_info_box.add_controller(ctrl);
    }

    pub fn load_playlist(&self, playlist_id: i64) {
        let model = model();
        let playlist = model.playlist(playlist_id);
        match playlist {
            Ok(playlist) => {
                self.imp().playlist.replace(Some(playlist));
                self.update_view();
            }
            Err(msg) => {
                error!("Unable to load playlist: {}", msg);
                self.emit_by_name::<()>("back", &[]);
            }
        }
    }
    
    pub fn update_view(&self) {
        let imp = self.imp();
        
        debug!("playlist detail: update_view");

        if imp.second_button_box.get_visible() {
            imp.second_button_box.set_visible(false);
        }
        
        match imp.playlist.borrow().as_ref() {
            Some(playlist) => {                    
                let list_store = self.list_store();
                list_store.remove_all();
                imp.list_view.set_model(None::<&gtk::SingleSelection>);

                //let list_store = gio::ListStore::new(PlaylistDetailTrackItem::static_type());

                for (_, entry) in playlist.entry_map() {
                    
                    let track_item = PlaylistDetailTrackItem::new(
                        playlist.id(),
                        entry.clone(), 
                    );
                    list_store.append(&track_item);
                }

                let filter = FuzzyFilter::new(SearchSortObject::PlaylistTrack);
                let filter_model = gtk::FilterListModel::new(None::<gio::ListStore>, None::<FuzzyFilter>);        
                filter_model.set_model(Some(&*list_store));
                filter_model.set_filter(Some(&filter));

                let sorter: gtk::Sorter =  {
                    gtk::CustomSorter::new(move |a, b| {
                        let a = a.downcast_ref::<PlaylistDetailTrackItem>().unwrap();
                        let b = b.downcast_ref::<PlaylistDetailTrackItem>().unwrap();
                
                        let item1_key = a.position();
                        let item2_key = b.position();
                    
                        if item1_key > item2_key {
                            gtk::Ordering::Larger
                        } else if item1_key < item2_key  {
                            gtk::Ordering::Smaller
                        } else {
                            gtk::Ordering::Equal
                        }
                        
                    })
                    .upcast()
                };

                let sorter_model = gtk::SortListModel::new(None::<gio::ListStore>, None::<gtk::Sorter>);
                sorter_model.set_sorter(Some(&sorter));
                sorter_model.set_model(Some(&filter_model));

                imp.sorter.replace(Some(sorter));

                let selection_model = gtk::SingleSelection::new(Some(sorter_model));

                let list_views = [&imp.list_view];
                for list_view in list_views {
                    list_view.set_model(Some(&selection_model));
                }

                imp.selection_model.replace(Some(Rc::new(selection_model)));

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

                //imp.list_store.replace(Some(Rc::new(list_store)));
                imp.filter.replace(Some(filter));

                self.construct_ui(playlist);
            }
            None => {
                imp.art_bin.set_child(gtk::Widget::NONE);
                self.emit_by_name::<()>("back", &[]);
            }
        }
    }


    fn construct_ui(&self, playlist: &Rc<Playlist>) {
        let imp = self.imp();

        let title_labels = [&imp.title_label, &imp.list_title_label];
        for label in title_labels {
            label.set_text(playlist.title().as_str());
        }

        imp.current_title.replace(playlist.title());

        if !playlist.description().is_empty() {
            imp.current_description.replace(playlist.description());
            imp.desc_label.set_text(playlist.description().as_str());
            imp.desc_label.show();
            
     

        } else {
            imp.desc_label.hide();
        }

        let n_tracks = playlist.n_tracks();
        if n_tracks <= 1 {
            imp.track_amount_label.set_label("1 track");
        } else {
            imp.track_amount_label.set_label(&format!("{} tracks", n_tracks));
        }


        let duration = playlist.duration();
        if duration > 0.0 {
            imp.duration_label.set_label(&seconds_to_string_longform(duration));
        }

        let cover_art_ids= playlist.cover_art_ids();
        if cover_art_ids.len() > 0 {
            match self.load_image(cover_art_ids) {
                Ok(art) => {
                    imp.art_bin.set_child(Some(&art));
                    imp.art.replace(Some(art));
                    imp.placeholder_art.replace(None);
                },
                Err(_) => {
                    let art = PlaceHolderArt::new(playlist.title(), "".to_string(), 500);
                    imp.art_bin.set_child(Some(&art));
                    imp.art.replace(None);
                    imp.placeholder_art.replace(Some(art));
                }
            }
        } else {
            let art = PlaceHolderArt::new(playlist.title(), "".to_string(), 500);
            imp.art_bin.set_child(Some(&art));
            imp.art.replace(None);
            imp.placeholder_art.replace(Some(art));
        }

        //imp.popover.set_menu_model(Some(playlist.menu_model()));
    }


    fn load_image(&self, cover_art_id: Vec<i64>) -> Result<GridArt, String> {
        let art = GridArt::new(500);

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



    fn set_edit_mode(&self, edit: bool) {
        let imp = self.imp();
        imp.edit_mode.set(edit);
        self.edit_button_mode(edit);
        self.notify("edit-mode");

        if edit {

            imp.desc_label.hide();
            imp.adw_entry.set_text(self.playlist().title().as_str());
            imp.desc_entry.set_text(self.playlist().description().as_str());
            imp.desc_entry.show();
            
            for label in [&imp.list_title_label] {
                label.hide()
            }

            for entry in [&imp.adw_entry] {
                entry.show();
            }

            for b in [&imp.delete_button, &imp.duplicate_button] {
                b.show();
            }


        } else {
            imp.desc_entry.hide();
            if !imp.current_description.borrow().is_empty() {
                imp.desc_label.show()
            }
            imp.list_title_label.show();

            for label in [&imp.list_title_label] {
                label.show()
            }

            for entry in [&imp.adw_entry] {
                entry.hide();
            }

            for b in [&imp.delete_button, &imp.duplicate_button] {
                b.hide();
            }
        }


    }

    fn edit_button_mode(&self, mode: bool) {
        let imp = self.imp();
        let button: &gtk::Button = imp.edit_button.as_ref();
   
        if !mode {
            for b in [button] {
                b.remove_css_class("opaque");
                b.remove_css_class("suggested-action");
                b.add_css_class("flat");
                b.set_tooltip_markup(Some("Edit Playlist"));
            }

            for i in [&imp.edit_icon] {
                i.set_icon_name(Some("edit-symbolic"));
            }

        } else {
            for b in [button] {
                b.remove_css_class("flat");
                b.add_css_class("opaque");
                b.add_css_class("suggested-action");
                b.set_tooltip_markup(Some("Finish Edit Playlist"));
            }

            for i in [&imp.edit_icon] {
                i.set_icon_name(Some("check-round-outline-symbolic"));
            }
        }
    }

    pub fn on_toggle_search_button(&self) {
        let imp = self.imp();
        imp.search_bar.set_search_mode(!imp.search_bar.is_search_mode());
        if !imp.search_bar.is_search_mode() {
            imp.search_bar.grab_focus();
        }
    }

    fn list_store(&self) -> Rc<ListStore> {
        self.imp().list_store.borrow().clone().unwrap().clone()
    }

    fn selection_model(&self) -> Rc<gtk::SingleSelection> {
        self.imp().selection_model.borrow().clone().unwrap().clone()
    }

    fn playlist(&self) -> Rc<Playlist> {
        self.imp().playlist.borrow().as_ref().unwrap().clone()
    }
}