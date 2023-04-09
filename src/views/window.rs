/* window.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::{subclass::prelude::*, Squeezer, ViewSwitcher, ViewSwitcherTitle};
use gtk::{prelude::*, gdk, gio, glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::cell::{Cell, RefCell};
use rand::{thread_rng, Rng};
use log::{debug, error};

use crate::views::dialog::{save_playlist_dialog::SavePlaylistDialog, add_tracks_to_playlist_dialog::AddToPlaylistDialog};
use crate::util::{model, player, database, get_child_by_index, settings_manager};
use crate::toasts::add_error_toast;
use crate::database::DatabaseAction;
use crate::web::discord::DiscordAction;
use crate::web::last_fm::LastFmAction;
use crate::sort::SortMethod;
use crate::i18n::i18n;
use crate::app::App;

use super::dialog::delete_playlist_dialog::DeletePlaylistDialog;
use super::dialog::duplicate_playlist_dialog::DuplicatePlaylistDialog;
use super::pages::albums::album_detail_page::AlbumDetailPage;
use super::pages::albums::album_grid_page::AlbumGridPage;
use super::pages::artists::artist_detail_page::ArtistDetailPage;
use super::pages::artists::artist_grid_page::ArtistGridPage;
use super::pages::genres::genre_detail_page::GenreDetailPage;
use super::pages::genres::genre_grid_page::GenreGridPage;
use super::pages::playlists::playlist_detail_page::PlaylistDetailPage;
use super::pages::playlists::playlist_grid_page::PlaylistGridPage;
use super::pages::queue::queue_page::QueuePage;
use super::pages::queue::queue_sidebar::QueueSidebar;
use super::pages::tracks::track_page::TrackPage;
use super::control_bar::ControlBar;

#[derive(Debug, Clone, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "WindowPage")]
pub enum WindowPage {
    Queue,
    Albums,
    Artists,
    Tracks,
    Genres,
    Playlists,
}

impl Default for WindowPage {
    fn default() -> Self {
        Self::Albums
    }
}

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/window.ui")]
    pub struct WindowPriv {
        #[template_child(id = "welcome_info_label")]
        pub welcome_info_label: TemplateChild<gtk::Label>,

        #[template_child(id = "welcome_percentage_label")]
        pub welcome_percentage_label: TemplateChild<gtk::Label>,

        #[template_child(id = "switcher-bar")]
        pub view_switcher_bar: TemplateChild<adw::ViewSwitcherBar>,

        #[template_child(id = "welcome_status")]
        pub welcome_status: TemplateChild<adw::StatusPage>,

        #[template_child(id = "spinner")]
        pub spinner: TemplateChild<gtk::Spinner>,

        #[template_child(id = "queue_flap")]
        pub queue_flap: TemplateChild<adw::Flap>,

        #[template_child(id = "full-stack")]
        pub full_stack: TemplateChild<gtk::Box>,

        #[template_child(id = "welcome-page")]
        pub welcome_page: TemplateChild<gtk::Box>,

        #[template_child(id = "meta-stack")]
        pub meta_stack: TemplateChild<gtk::Stack>,

        #[template_child(id = "add_library_button")]
        pub add_library_button: TemplateChild<gtk::Button>,

        #[template_child(id = "toast_overlay")]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,

        #[template_child(id = "queue_sidebar")]
        pub queue_sidebar: TemplateChild<QueueSidebar>,

        #[template_child(id = "title")]
        pub view_switcher_title: TemplateChild<adw::ViewSwitcherTitle>,

        #[template_child(id = "toggle_sort_button")]
        pub toggle_sort_button: TemplateChild<gtk::MenuButton>,

        #[template_child(id = "toggle_search_button")]
        pub toggle_search_button: TemplateChild<gtk::Button>,

        #[template_child(id = "stack")]
        pub stack: TemplateChild<adw::ViewStack>,

        // #[template_child(id = "home_page")]
        // pub home_page: TemplateChild<HomePage>,

        #[template_child(id = "queue_page")]
        pub queue_page: TemplateChild<QueuePage>,

        #[template_child(id = "queue_stack_page")]
        pub queue_stack_page: TemplateChild<adw::ViewStackPage>,

        #[template_child(id = "album-stack")]
        pub album_stack: TemplateChild<gtk::Stack>,

        #[template_child(id = "album_grid_page")]
        pub album_grid_page: TemplateChild<AlbumGridPage>,

        #[template_child(id = "album_detail_page")]
        pub album_detail_page: TemplateChild<AlbumDetailPage>,

        #[template_child(id = "artist-stack")]
        pub artist_stack: TemplateChild<gtk::Stack>,

        #[template_child(id = "artist_grid_page")]
        pub artist_grid_page: TemplateChild<ArtistGridPage>,

        #[template_child(id = "artist_detail_page")]
        pub artist_detail_page: TemplateChild<ArtistDetailPage>,

        #[template_child(id = "track_page")]
        pub track_page: TemplateChild<TrackPage>,

        #[template_child(id = "genre-stack")]
        pub genre_stack: TemplateChild<gtk::Stack>,

        #[template_child(id = "genre_grid_page")]
        pub genre_grid_page: TemplateChild<GenreGridPage>,

        #[template_child(id = "genre_detail_page")]
        pub genre_detail_page: TemplateChild<GenreDetailPage>,

        #[template_child(id = "playlist-stack")]
        pub playlist_stack: TemplateChild<gtk::Stack>,

        #[template_child(id = "playlist_grid_page")]
        pub playlist_grid_page: TemplateChild<PlaylistGridPage>,

        #[template_child(id = "playlist_detail_page")]
        pub playlist_detail_page: TemplateChild<PlaylistDetailPage>,

        #[template_child(id = "control_bar")]
        pub control_bar: TemplateChild<ControlBar>,

        #[template_child(id = "navigate_back_button")]
        pub navigate_back_button: TemplateChild<gtk::Button>,
        
        pub open_queue: Cell<bool>,
        pub show_back_nav_button: Cell<bool>,
        pub current_css_cover_art_id: Cell<i64>,
        pub window_page: Cell<WindowPage>,
        pub provider: gtk::CssProvider,
        pub folder_dialog: RefCell<Option<gtk::FileChooserNative>>,
        pub settings: gio::Settings,
        pub db_sender: RefCell<Option<Sender<DatabaseAction>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WindowPriv {
        const NAME: &'static str = "Window";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                welcome_info_label: TemplateChild::default(),
                welcome_percentage_label: TemplateChild::default(),
                view_switcher_bar: TemplateChild::default(),
                welcome_status: TemplateChild::default(),
                spinner: TemplateChild::default(),
                navigate_back_button: TemplateChild::default(),
                show_back_nav_button: Cell::new(true),
                queue_flap: TemplateChild::default(),
                full_stack: TemplateChild::default(),
                welcome_page: TemplateChild::default(),
                meta_stack: TemplateChild::default(),
                add_library_button: TemplateChild::default(),
                toast_overlay: TemplateChild::default(),
                queue_sidebar: TemplateChild::default(),
                view_switcher_title: TemplateChild::default(),
                toggle_sort_button: TemplateChild::default(),
                toggle_search_button: TemplateChild::default(),
                stack: TemplateChild::default(),
                queue_page: TemplateChild::default(),
                queue_stack_page: TemplateChild::default(),
                album_stack: TemplateChild::default(),
                album_grid_page: TemplateChild::default(),
                album_detail_page: TemplateChild::default(),
                artist_stack: TemplateChild::default(),
                artist_grid_page: TemplateChild::default(),
                artist_detail_page: TemplateChild::default(),
                track_page: TemplateChild::default(),
                genre_stack: TemplateChild::default(),
                genre_grid_page: TemplateChild::default(),
                genre_detail_page: TemplateChild::default(),
                playlist_stack: TemplateChild::default(),
                playlist_grid_page: TemplateChild::default(),
                playlist_detail_page: TemplateChild::default(),
                control_bar: TemplateChild::default(),
                
                open_queue: Cell::new(true),
                db_sender: RefCell::new(None),
                window_page: Cell::new(WindowPage::default()),
                provider: gtk::CssProvider::new(),
                folder_dialog: RefCell::new(None),
                settings: settings_manager(),
                current_css_cover_art_id: Cell::new(-1),
            }
        }
    }

    impl ObjectImpl for WindowPriv {}
    impl WidgetImpl for WindowPriv {}
    impl WindowImpl for WindowPriv {}
    impl ApplicationWindowImpl for WindowPriv {}
    impl AdwApplicationWindowImpl for WindowPriv {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::WindowPriv>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Window {
    pub fn new(app: &App) -> Self {
        let window: Window = glib::Object::builder::<Window>().build();
        window.set_application(Some(app));
        window.setup();
        window
    }

    fn setup(&self) {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

        let imp = self.imp();
        self.setup_gactions();
        self.setup_settings();
        self.setup_database();
        self.setup_provider();
        self.bind_state();
        self.bind_signals();
        self.add_dialog();
        self.setup_view_stack_button_connection();

        let volume = settings_manager().double("default-volume");
        player().set_volume(volume);

        if let Some(sender) = imp.db_sender.borrow().as_ref() {
            send!(sender, DatabaseAction::TryLoadingDataBase)
        }

        self.view_stack_page_visible(WindowPage::Queue, false);
        self.search_sort_visibility(imp.window_page.get());
    }

    fn add_dialog(&self) {
        let imp = self.imp();

        let dialog = gtk::FileChooserNative::builder()
            .accept_label(&i18n("_Add Music"))
            .cancel_label(&i18n("_Cancel"))
            .modal(true)
            .title(&i18n("Select Music Library"))
            .action(gtk::FileChooserAction::SelectFolder)
            .select_multiple(false)
            .transient_for(self)
            .build();

        //let filter = gtk::FileFilter::new();
        // gtk::FileFilter::set_name(&filter, Some(&i18n("Image files")));
        // filter.add_mime_type("image/*");
        // dialog.add_filter(&filter);

        dialog.connect_response(
            clone!(@weak self as this => move |dialog, response| {
                if response == gtk::ResponseType::Accept {
                    let imp = this.imp();

                    let folder = dialog.file().unwrap().path().unwrap();
                    
                    if !folder.is_dir() {
                        add_error_toast("Unable to add music folder, not a valid directory".to_string());
                    }
                    
                    let path_str = folder.clone().into_os_string().into_string().ok().unwrap();

                    imp.spinner.set_visible(true);
                    imp.spinner.start();
                    imp.add_library_button.hide();

                    imp.welcome_status.set_title("Importing Music Folder...");
                    imp.welcome_status.set_description(Some(&path_str));
                    
                    debug!("DIALOG FOLDER RECEIVED: {:?}", path_str);

                    if let Some(sender) = imp.db_sender.borrow().as_ref() {
                        send!(sender, DatabaseAction::TryAddMusicFolder(folder))
                    }
                } else {
                    error!("No folder selected.");
                }
                dialog.hide();
            }),
        );

        imp.folder_dialog.replace(Some(dialog));

        imp.add_library_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                this.imp().folder_dialog.borrow().as_ref().unwrap().show();
            }),
        );        
    }

    fn setup_database(&self) {
        let imp = self.imp();
        let database = database();
        let db_sender = database.sender();
        imp.db_sender.replace(Some(db_sender));

        database.connect_notify_local(Some("loaded"),
            clone!(@strong self as this => move |database, _pspec| {
                debug!("Notified database loaded in window.");
                let imp = this.imp();
                
                imp.spinner.hide();
                imp.spinner.stop();
                imp.add_library_button.show();

                imp.welcome_status.set_title("Welcome to Resonance");
                imp.welcome_status.set_description(Some("Add your music library to get started!"));

                if database.imp().loaded.get() {
                    imp.meta_stack.set_visible_child_full(
                        "main-stack-page",
                        gtk::StackTransitionType::Crossfade,
                    );
                } else {
                    imp.meta_stack.set_visible_child_full(
                        "welcome-stack-page",
                        gtk::StackTransitionType::Crossfade,
                    );                    
                }
                

            })
        );
    }

    // Bind the PlayerState to the UI
    fn bind_state(&self) {
        let player = player();

        // Update the cover, if any is available
        player.state().connect_notify_local(
            Some("cover"),
            clone!(@weak self as win => move |_, _| {
                win.update_colors_from_cover();
            }),
        );

        player.state().connect_local(
            "queue-empty",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.view_stack_page_visible(WindowPage::Queue, false);
                None
            }),
        );

        player.state().connect_local(
            "queue-nonempty",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.view_stack_page_visible(WindowPage::Queue, true);
                None
            }),
        );
    }

    pub fn set_import_message(&self, message: &str) {
        let imp = self.imp();
        if let Some(name) = imp.meta_stack.visible_child_name() {
            if "welcome-stack-page" == name.as_str() {
                if !imp.welcome_info_label.is_visible() {
                    imp.welcome_info_label.show()
                }
                imp.welcome_info_label.set_label(message);
            }
        }
    }

    pub fn set_import_percentage(&self, message: &str) {
        let imp = self.imp();
        if let Some(name) = imp.meta_stack.visible_child_name() {
            if "welcome-stack-page" == name.as_str() {
                if !imp.welcome_percentage_label.is_visible() {
                    imp.welcome_percentage_label.show()
                }
                imp.welcome_percentage_label.set_label(message);
            }
        }
    }

    fn bind_signals(&self) {
        debug!("bind signals");
        let imp = self.imp();

        imp.meta_stack.connect_notify_local(Some("visible-child"),         
        clone!(@strong self as this => move |stack, _pspec| {
                let imp = this.imp();
                if imp.welcome_info_label.is_visible() || imp.welcome_percentage_label.is_visible(){
                    imp.welcome_info_label.hide();
                    imp.welcome_percentage_label.hide();
                }
                if let Some(name) = stack.visible_child_name() {
                    match name.as_str() {
                        "main-stack-page" => {
                            this.set_default_height(1150);
                            this.set_default_width(1400);
                            this.set_resizable(true);
                        },
                        "welcome-stack-page" => {
                            this.set_default_height(600);
                            this.set_default_width(500);
                            this.set_resizable(false);
                        },
                        _ => {
                            this.set_default_height(300);
                            this.set_default_width(300);
                            this.set_resizable(false);
                        },
                    }
                }
            }
        ));

        imp.album_grid_page.connect_notify_local(Some("hidden"), 
        clone!(@strong self as this => move |page, _pspec| {
                this.view_stack_page_visible(WindowPage::Albums, !page.imp().hidden.get());
            }
        ));

        imp.track_page.connect_notify_local(Some("hidden"), 
        clone!(@strong self as this => move |page, _pspec| {
                this.view_stack_page_visible(WindowPage::Tracks, !page.imp().hidden.get());
            }
        ));


        imp.playlist_grid_page.connect_notify_local(Some("hidden"), 
        clone!(@strong self as this => move |page, _pspec| {
                this.view_stack_page_visible(WindowPage::Playlists, !page.imp().hidden.get());
            }
        ));


        imp.genre_grid_page.connect_notify_local(Some("hidden"), 
        clone!(@strong self as this => move |page, _pspec| {
                this.view_stack_page_visible(WindowPage::Genres, !page.imp().hidden.get());
            }
        ));


        imp.artist_grid_page.connect_notify_local(Some("hidden"), 
        clone!(@strong self as this => move |page, _pspec| {
                this.view_stack_page_visible(WindowPage::Artists, !page.imp().hidden.get());
            }
        ));


        imp.stack.connect_notify_local(
            Some("visible-child-name"),
            clone!(@weak self as this => move |stack, _| {
                if let Some(stack_child_name) = stack.visible_child_name() {
                    let stack_enum = match stack_child_name.as_str() {
                        "queue-stack-page" => WindowPage::Queue,
                        "album-stack-page" => WindowPage::Albums,
                        "tracks-stack-page" => WindowPage::Tracks,
                        "playlists-stack-page" => WindowPage::Playlists,
                        "artists-stack-page" => WindowPage::Artists,
                        "genre-stack-page" => WindowPage::Genres,
                        _ => {
                            error!("error retrieving stack page name");
                            WindowPage::Albums
                        },
                    };
                    this.switch_stack_page(stack_enum, false);
                }

            }),
        );

        imp.control_bar.go_to_queue_button().connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                this.switch_stack_page(WindowPage::Queue, true);
            }),
        );

        imp.control_bar.track_info_button().connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                this.switch_stack_page(WindowPage::Queue, true);
            }),
        );

        imp.queue_page.connect_local(
            "done",
            false,
            clone!(@strong self as this => @default-return None, move |_args| {
                if this.window_page() == WindowPage::Queue {
                    this.switch_stack_page(WindowPage::Albums, true);
                }
                None
            }),
        );

        // self.queue_page.show_queue_button.connect('clicked', self.show_queue_sidebar)
        imp.queue_page.show_queue_button().connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                let open = !imp.queue_flap.reveals_flap();
                imp.queue_flap.set_reveal_flap(open);
                imp.open_queue.set(open);
            }),
        );

        // self.album_grid_page.connect('album_selected', self.album_selected_go_to_detail)
        imp.album_grid_page.connect_local(
            "album-selected",
            false,
            clone!(@strong self as this => @default-return None, move |value| {
                // let main_context = glib::MainContext::default();
                let album_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                this.album_selected_go_to_detail(album_id);
                None
            }),
        );

        // self.album_grid_page.album_sidebar.connect('album_selected', self.album_selected_go_to_detail)
        imp.album_grid_page.album_sidebar().connect_local(
            "album-selected",
            false,
            clone!(@strong self as this => @default-return None, move |value| {
                let album_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                debug!("going to album detail: {}", album_id);
                this.album_selected_go_to_detail(album_id);
                None
            }),
        );

        // self.album_detail_page.connect('back', self._go_back_to_albums)
        imp.album_detail_page.connect_local(
            "back",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.go_back_to_albums();
                None
            }),
        );

        // self.artists_grid_page.connect('artist_selected', self.artist_selected_go_to_detail)
        imp.artist_grid_page.connect_local(
            "artist-selected",
            false,
            clone!(@strong self as this => @default-return None, move |value| {

                let artist_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                this.artist_selected_go_to_detail(artist_id);
                None
            }),
        );

        // self.artist_detail_page.connect('back', self._go_back_to_artists)
        imp.artist_detail_page.connect_local(
            "back",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.go_back_to_artists();
                None
            }),
        );

        // self.genre_grid_page.connect('genre_selected', self.genre_selected_go_to_detail)
        imp.genre_grid_page.connect_local(
            "genre-selected",
            false,
            clone!(@strong self as this => @default-return None, move |value| {
                let genre_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                this.genre_selected_go_to_detail(genre_id);
                None
            }),
        );

        // self.genre_detail_page.connect('back', self._go_back_to_genres)
        imp.genre_detail_page.connect_local(
            "back",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.go_back_to_genres();
                None
            }),
        );

        // self.playlists_grid_page.connect('playlist_selected', self.playlist_selected_go_to_detail)
        imp.playlist_grid_page.connect_local(
            "playlist-selected",
            false,
            clone!(@strong self as this => @default-return None, move |value| {
                let playlist_id = value.get(1).unwrap().get::<i64>().ok().unwrap();
                debug!("playlist selected, going to detail");
                this.playlist_selected_go_to_detail(playlist_id);
                None
            }),
        );

        // self.playlist_detail_page.connect('back', self._go_back_to_playlists)
        imp.playlist_detail_page.connect_local(
            "back",
            false,
            clone!(@strong self as this => @default-return None, move |_| {
                this.go_back_to_playlists();
                None
            }),
        );
        // self.toggle_search_button.connect('clicked', self.on_toggle_search_button)

        imp.toggle_search_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                match imp.window_page.get() {
                    WindowPage::Albums => imp.album_grid_page.on_toggle_search_button(),
                    WindowPage::Tracks => imp.track_page.on_toggle_search_button(),
                    WindowPage::Artists => {
                        if imp.artist_stack.visible_child_name().unwrap().as_str() == "artists-grid-stack-page" {
                            imp.artist_grid_page.on_toggle_search_button();
                        } else {
                            imp.artist_detail_page.on_toggle_search_button();
                        }
                    },
                    WindowPage::Genres => {
                        if imp.genre_stack.visible_child_name().unwrap().as_str() == "genre-grid-stack-page" {
                            imp.genre_grid_page.on_toggle_search_button();
                        } else {
                            imp.genre_detail_page.on_toggle_search_button();
                        }
                    },
                    WindowPage::Playlists => {
                        if imp.playlist_stack.visible_child_name().unwrap().as_str() == "playlists-grid-stack-page" {
                            imp.playlist_grid_page.on_toggle_search_button();
                        } else {
                            imp.playlist_detail_page.on_toggle_search_button();
                        }
                    },
                    _ => (),
                }
            }),
        );

        imp.navigate_back_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                match imp.window_page.get() {
                    WindowPage::Albums =>{
                        if imp.artist_stack.visible_child_name().unwrap().as_str() != "album-gridstack-page" {
                            this.go_back_to_albums();
                        }
                    },
                    WindowPage::Artists => {
                        if imp.artist_stack.visible_child_name().unwrap().as_str() != "artists-grid-stack-page" {
                            this.go_back_to_artists();
                        }
                    },
                    WindowPage::Genres => {
                        if imp.genre_stack.visible_child_name().unwrap().as_str() != "genre-grid-stack-page" {
                            this.go_back_to_genres();
                        }
                    },
                    WindowPage::Playlists => {
                        if imp.playlist_stack.visible_child_name().unwrap().as_str() != "playlists-grid-stack-page" {
                            this.go_back_to_playlists();
                        }
                    },
                    _ => (),
                }
            }),
        );
    }


    /*
    NAVIGATION
    */


    fn switch_stack_page(&self, page_option: WindowPage, switch: bool) {
        let imp = self.imp();
        debug!("switching to: {:?}", page_option);

        if switch {
            match page_option {
                WindowPage::Queue => imp.stack.set_visible_child(&*imp.queue_page),
                WindowPage::Albums => imp.stack.set_visible_child(&*imp.album_stack),
                WindowPage::Artists => imp.stack.set_visible_child(&*imp.artist_stack),
                WindowPage::Tracks => imp.stack.set_visible_child(&*imp.track_page),
                WindowPage::Genres => imp.stack.set_visible_child(&*imp.genre_stack),
                WindowPage::Playlists => imp.stack.set_visible_child(&*imp.playlist_stack),
                // _ => (),
            }
        }

        self.set_window_page(page_option);
        self.search_sort_visibility(page_option);

        let is_queue: bool = page_option == WindowPage::Queue;
        if is_queue {
            imp.queue_page.update_view();
        } 

     

        imp.queue_flap.set_reveal_flap(is_queue && imp.open_queue.get());
        imp.control_bar.set_revealed(!is_queue);
    }

    fn search_sort_visibility(&self, page_option: WindowPage) {
        let imp = self.imp();
        //imp.toggle_sort_button.set_menu_model();
        imp.navigate_back_button.hide();
        match page_option {
            WindowPage::Queue => self.set_search_sort_visibility(false),
            WindowPage::Albums => {
                let is_grid = imp.album_stack.visible_child().as_ref() == Some(imp.album_grid_page.upcast_ref());
                self.set_search_sort_visibility(is_grid);
                imp.toggle_sort_button.set_menu_model(Some(imp.album_grid_page.sort_menu()));
                if !is_grid && imp.show_back_nav_button.get() {
                    imp.navigate_back_button.show();
                }
            }
            WindowPage::Artists => {
                let is_grid = imp.artist_stack.visible_child().as_ref() == Some(imp.artist_grid_page.upcast_ref());
                imp.toggle_search_button.set_visible(true);
                imp.toggle_sort_button.set_visible(is_grid);
                if is_grid {
                    imp.toggle_sort_button.set_menu_model(Some(imp.artist_grid_page.sort_menu()));
                } else {
                    if imp.show_back_nav_button.get() {
                        imp.navigate_back_button.show();
                    }
                    imp.toggle_sort_button.set_menu_model(Some(imp.artist_detail_page.sort_menu()));
                }
            }
            WindowPage::Tracks => {
                self.set_search_sort_visibility(true);
                imp.toggle_sort_button.set_menu_model(Some(imp.track_page.sort_menu()));
            },
            WindowPage::Genres => {
                let is_grid = imp.genre_stack.visible_child().as_ref()
                    == Some(imp.genre_grid_page.upcast_ref());
                imp.toggle_search_button.set_visible(true);
                imp.toggle_sort_button.set_visible(is_grid);
                if is_grid {
                    imp.toggle_sort_button.set_menu_model(Some(imp.genre_grid_page.sort_menu()));
                } else {
                    if imp.show_back_nav_button.get() {
                        imp.navigate_back_button.show();
                    }
                    imp.toggle_sort_button.set_menu_model(Some(imp.genre_detail_page.sort_menu()));
                }
            }
            WindowPage::Playlists => {
                let is_grid = imp.playlist_stack.visible_child().as_ref() == Some(imp.playlist_grid_page.upcast_ref());
                imp.toggle_search_button.set_visible(true);
                imp.toggle_sort_button.set_visible(is_grid);
                if is_grid {
                    imp.toggle_sort_button.set_menu_model(Some(imp.playlist_grid_page.sort_menu()));
                } else {
                    if imp.show_back_nav_button.get() {
                        imp.navigate_back_button.show();
                    }
                }

            }
        }
    }

    /*
    GRID to DETAIL & DETAIL to GRID PAGE NAVIGATION
    */

    fn go_back_to_albums(&self) {
        let imp = self.imp();
        imp.album_stack.set_visible_child_full(
            "album-grid-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        self.set_window_page(WindowPage::Albums);
        self.set_search_sort_visibility(true);
        imp.navigate_back_button.hide();
    }

    fn go_back_to_artists(&self) {
        let imp = self.imp();
        imp.artist_stack.set_visible_child_full(
            "artists-grid-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.toggle_sort_button.set_menu_model(Some(imp.artist_grid_page.sort_menu()));
        self.set_window_page(WindowPage::Artists);
        self.set_search_sort_visibility(true);
        imp.navigate_back_button.hide();
    }

    fn go_back_to_genres(&self) {
        let imp = self.imp();
        imp.genre_stack.set_visible_child_full(
            "genre-grid-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.toggle_sort_button.set_menu_model(Some(imp.artist_grid_page.sort_menu()));
        self.set_window_page(WindowPage::Genres);
        self.set_search_sort_visibility(true);
        imp.navigate_back_button.hide();
    }

    fn go_back_to_playlists(&self) {
        let imp = self.imp();
        imp.playlist_stack.set_visible_child_full(
            "playlists-grid-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        self.set_window_page(WindowPage::Playlists);
        self.set_search_sort_visibility(true);
        imp.navigate_back_button.hide();
    }

    fn album_selected_go_to_detail(&self, album_id: i64) {
        let imp = self.imp();

        imp.album_stack.set_visible_child_full(
            "album-detail-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.album_detail_page.load_album(album_id);
        self.set_search_sort_visibility(false);
        if imp.show_back_nav_button.get() {
            imp.navigate_back_button.show();
        }
    }

    fn artist_selected_go_to_detail(&self, artist_id: i64) {
        let imp = self.imp();
        imp.artist_stack.set_visible_child_full(
            "artist-detail-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.toggle_sort_button.set_menu_model(Some(imp.artist_detail_page.sort_menu()));
        imp.artist_detail_page.update_artist(artist_id);
        self.set_search_sort_visibility(true);
        if imp.show_back_nav_button.get() {
            imp.navigate_back_button.show();
        }
    }

    fn genre_selected_go_to_detail(&self, genre_id: i64) {
        let imp = self.imp();
        imp.genre_stack.set_visible_child_full(
            "genre-detail-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.toggle_sort_button.set_menu_model(Some(imp.genre_detail_page.sort_menu()));
        imp.genre_detail_page.update_genre(genre_id);
        self.set_search_sort_visibility(true);
        if imp.show_back_nav_button.get() {
            imp.navigate_back_button.show();
        }
    }

    fn playlist_selected_go_to_detail(&self, playlist_id: i64) {
        let imp = self.imp();
        imp.playlist_stack.set_visible_child_full(
            "playlist-detail-stack-page",
            gtk::StackTransitionType::Crossfade,
        );
        imp.playlist_detail_page.load_playlist(playlist_id);
        imp.toggle_search_button.set_visible(true);
        imp.toggle_sort_button.set_visible(false);
        if imp.show_back_nav_button.get() {
            imp.navigate_back_button.show();
        }
    }

    fn set_window_page(&self, state: WindowPage) {
        let imp = self.imp();
        imp.control_bar.set_window_page(state);
        imp.queue_page.set_window_page(state);
        imp.window_page.set(state);
    }

    pub fn window_page(&self) -> WindowPage {
        self.imp().window_page.get()
    }

    /*
    SEARCH AND SORT UI
    */

    fn set_search_sort_visibility(&self, toggle: bool) {
        let imp = self.imp();
        imp.toggle_search_button.set_visible(toggle);
        imp.toggle_sort_button.set_visible(toggle);
    }

    /*
    CSS
    */

    fn setup_provider(&self) {
        let imp = self.imp();
        
        if let Some(display) = gdk::Display::default() {
            gtk::StyleContext::add_provider_for_display(&display, &imp.provider, 400);
        }
        
        self.update_color_hex();
    }

    fn update_color_hex(&self) {
        let imp = self.imp();
        imp.provider.load_from_data("");
        let colors = vec![
            ["#008B8B", "#AFEEEE", "#AFEEEE"],
            ["@blue_3", "@orange_1", "@orange_1"],
            ["#ffe228", "#ff28cd", "#5a28ff"],
            ["#ff182f", "#18ffd1", "#a3ff18"],
            ["#2cff8e", "#ffae2c", "#ff2cce"],
            ["#d328d6", "#d6282b", "#82d628"],
            ["#b2e263", "#63b2e2", "#e263b2"],
            ["#e2637f", "#7fe263", "#637fe2"],
        ];
        let mut rng = thread_rng();
        let random_interval = rng.gen_range(0..(colors.len() - 1));
        let mut css = String::new();
        for i in 0..3 {
            css.push_str(&format!(
                "@define-color background_color_{} {};",
                i, colors[random_interval][i]
            ));
            css.push_str(&format!(
                "@define-color control_bar_{} {};",
                i,
                colors[random_interval][colors[random_interval].len() - 1 - i]
            ));
        }
        imp.provider.load_from_data(css.as_str());
    }

    fn update_colors_from_cover(&self) {
        let imp = self.imp();
        let player = player();
        let state = player.state();
        let model = model();

        if let Some(cover_art_id) = state.cover() {

            if imp.current_css_cover_art_id.get() == cover_art_id {
                return;
            }

            if let Ok(cover_art) = model.cover_art(cover_art_id)  {
                if let Ok(palette) = cover_art.palette() {
                    if let Some(palette) = palette {
                        self.update_color_rgb(palette);
                        imp.current_css_cover_art_id.set(cover_art_id);
                        return;
                    }
                }
            }
        }

        imp.current_css_cover_art_id.set(-1);
        self.update_color_hex();
    }

    fn update_color_rgb(&self, bg_colors: Vec<gdk::RGBA>) {
        let imp = self.imp();

        let mut css = String::new();

        let n_colors = bg_colors.len();
        for i in 0..n_colors {
            css.push_str(&format!(
                "@define-color background_color_{} {};",
                i, bg_colors[i]
            ));

            css.push_str(&format!(
                "@define-color control_bar_{} {};",
                i,
                bg_colors[n_colors - 1 - i]
            ));
        }

        imp.provider.load_from_data(css.as_str());
    }

    /*
    TOAST
    */

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    /*
    VIEW STACK
    */

    pub fn view_stack_page_visible(&self, page_enum: WindowPage, visible: bool) {
        let imp = self.imp();

        let label = match page_enum {
            WindowPage::Queue => "Queue",
            WindowPage::Albums => "Albums",
            WindowPage::Artists => "Artists",
            WindowPage::Tracks => "Tracks",
            WindowPage::Genres => "Genres",
            WindowPage::Playlists => "Playlists",
        };

        //SET VIEW SWITCHER TITLE BUTTON VISIBLE

        let squeezer = get_child_by_index::<ViewSwitcherTitle, Squeezer>(&imp.view_switcher_title, 0);
        let view_switcher = get_child_by_index::<Squeezer, ViewSwitcher>(&squeezer, 0);


        for child in view_switcher.observe_children().snapshot().iter() {
            let name = child.property::<String>("label");
            if name.as_str() == label {
                child.set_property("visible", visible.to_value());
                break
            }
        }


        //SET VIEW SWITCHER BAR BUTTON VISIBLE

        let action_bar = get_child_by_index::<adw::ViewSwitcherBar, gtk::ActionBar>(&imp.view_switcher_bar, 0);
        let revealer = get_child_by_index::<gtk::ActionBar, gtk::Revealer>(&action_bar, 0);
        let center_box = get_child_by_index::<gtk::Revealer, gtk::CenterBox>(&revealer, 0);
        let view_switcher = get_child_by_index::<gtk::CenterBox, adw::ViewSwitcher>(&center_box, 1);

        for child in view_switcher.observe_children().snapshot().iter() {
            let name = child.property::<String>("label");
            if name.as_str() == label {
                child.set_property("visible", visible.to_value());
                break
            }
        }

        //SET VIEW STACK PAGE VISIBLE

        match page_enum {
            // WindowPage::Home => {
            //     imp.home_page.set_visible(visible);
            // },
            WindowPage::Queue => {
                imp.queue_page.set_visible(visible);
            },
            WindowPage::Albums => {
                imp.album_stack.set_visible(visible);
            },
            WindowPage::Artists => {
                imp.artist_stack.set_visible(visible);
            },
            WindowPage::Tracks => {
                imp.track_page.set_visible(visible);
            },
            WindowPage::Genres => {
                imp.genre_stack.set_visible(visible);
            },
            WindowPage::Playlists => {
                imp.playlist_stack.set_visible(visible);
            }
            // _ => debug!("Not implemented."),
        }
    }

    fn setup_view_stack_button_connection(&self) {
        let imp = self.imp();


        //SET VIEW SWITCHER TITLE BUTTON CONNECTION
        let squeezer = get_child_by_index::<ViewSwitcherTitle, Squeezer>(&imp.view_switcher_title, 0);
        let view_switcher = get_child_by_index::<Squeezer, ViewSwitcher>(&squeezer, 0);

        for child in view_switcher.observe_children().snapshot().iter() {
            let name = child.property::<String>("label");
            if let Some(button) =  child.downcast_ref::<gtk::Button>() {
                self.view_stack_button_connection(name, button);
            }
        }

        //SET VIEW SWITCHER BAR BUTTON CONNECTION
        let action_bar = get_child_by_index::<adw::ViewSwitcherBar, gtk::ActionBar>(&imp.view_switcher_bar, 0);
        let revealer = get_child_by_index::<gtk::ActionBar, gtk::Revealer>(&action_bar, 0);
        let center_box = get_child_by_index::<gtk::Revealer, gtk::CenterBox>(&revealer, 0);
        let view_switcher = get_child_by_index::<gtk::CenterBox, adw::ViewSwitcher>(&center_box, 1);

        for child in view_switcher.observe_children().snapshot().iter() {
            let name = child.property::<String>("label");
            if let Some(button) =  child.downcast_ref::<gtk::Button>() {
                self.view_stack_button_connection(name, button);
            }
        }
    }

    fn view_stack_button_connection(&self, label: String, button: &gtk::Button) {
        let page_enum = match label.as_str() {
            "Queue" => WindowPage::Queue, 
            "Albums" => WindowPage::Albums, 
            "Artists" => WindowPage::Artists, 
            "Tracks" => WindowPage::Tracks, 
            "Genres" => WindowPage::Genres, 
            "Playlists" => WindowPage::Playlists,
            _ => return,
        };

        match page_enum {
            WindowPage::Albums => {
                button.connect_clicked(
                    clone!(@strong self as this => @default-panic, move |_button| {
                        let imp = this.imp();
                        if imp.window_page.get() == WindowPage::Albums && imp.album_stack.visible_child().as_ref() != Some(imp.album_grid_page.upcast_ref()){
                            this.go_back_to_albums();
                        }
                    })
                );
            },
            WindowPage::Artists => {
                button.connect_clicked(
                    clone!(@strong self as this => @default-panic, move |_button| {
                        let imp = this.imp();
                        if imp.window_page.get() == WindowPage::Artists && imp.artist_stack.visible_child().as_ref() != Some(imp.artist_grid_page.upcast_ref()){
                            this.go_back_to_artists();
                        }
                    })
                );
            },
            WindowPage::Genres => {
                button.connect_clicked(
                    clone!(@strong self as this => @default-panic, move |_button| {
                        let imp = this.imp();
                        if imp.window_page.get() == WindowPage::Genres && imp.genre_stack.visible_child().as_ref() != Some(imp.genre_grid_page.upcast_ref()){
                            this.go_back_to_genres();
                        }
                    })
                );
            },
            WindowPage::Playlists => {
                button.connect_clicked(
                    clone!(@strong self as this => @default-panic, move |_button| {
                        let imp = this.imp();
                        if imp.window_page.get() == WindowPage::Playlists && imp.playlist_stack.visible_child().as_ref() != Some(imp.playlist_grid_page.upcast_ref()){
                            this.go_back_to_playlists();
                        }
                    })
                );
            },
            _ => (),
        }
    }

    /*
    GACTIONS
    */

    fn setup_gactions(&self) {
        let imp = self.imp();

        self.add_simple_action("genre-detail-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.genre_detail_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Album,
                        1 => SortMethod::Artist,
                        2 => SortMethod::ReleaseDate,
                        3 => SortMethod::Duration,
                        4 => SortMethod::TrackCount,
                        _ => SortMethod::Album,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("artist-detail-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.artist_detail_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Album,
                        1 => SortMethod::ReleaseDate,
                        2 => SortMethod::Duration,
                        3 => SortMethod::TrackCount,
                        _ => SortMethod::Album,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("genre-grid-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.genre_grid_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Genre,
                        1 => SortMethod::AlbumCount,
                        2 => SortMethod::TrackCount,
                        _ => SortMethod::Genre,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("artist-grid-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.artist_grid_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Artist,
                        1 => SortMethod::AlbumCount,
                        2 => SortMethod::TrackCount,
                        _ => SortMethod::Artist,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("playlist-grid-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.playlist_grid_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Playlist,
                        1 => SortMethod::LastModified,
                        2 => SortMethod::Duration,
                        3 => SortMethod::TrackCount,
                        _ => SortMethod::LastModified,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("track-page-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.track_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Track,
                        1 => SortMethod::Album,
                        2 => SortMethod::Artist,
                        3 => SortMethod::Genre,
                        4 => SortMethod::ReleaseDate,
                        5 => SortMethod::Duration,
                        _ => SortMethod::Track,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("album-grid-sort", Some(glib::VariantTy::UINT16), 
        clone!(@strong imp.album_grid_page as this => @default-panic, move |_, sort_id| {
                if let Some(id) = sort_id.and_then(|u| u.get::<u16>()) {
                    let sort_method = match id {
                        0 => SortMethod::Album,
                        1 => SortMethod::Artist,
                        2 => SortMethod::Genre,
                        3 => SortMethod::ReleaseDate,
                        4 => SortMethod::Duration,
                        5 => SortMethod::TrackCount,
                        _ => SortMethod::Album,
                    };
                    this.set_property("sort-method", sort_method.to_value());
                }
            })
        );

        self.add_simple_action("play-album", Some(glib::VariantTy::INT64), 
        move |_, album_id| {
            if let Some(id) = album_id.and_then(|u| u.get::<i64>()) {
                let album = model().album(id).unwrap();
                player().clear_play_album(album.tracks(), Some(album.title()));
            }
        });

        self.add_simple_action("add-album", Some(glib::VariantTy::INT64), 
        move |_, album_id| {
            if let Some(id) = album_id.and_then(|u| u.get::<i64>()) {
                let album = model().album(id).unwrap();
                player().add_album(album.tracks());
            }
        });

        self.add_simple_action("create-playlist-from-album", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, album_id| {
                if let Some(id) = album_id.and_then(|u| u.get::<i64>()) {
                    let album = model().album(id).unwrap();
                    let save_dialog = SavePlaylistDialog::new(album.track_ids());
                    save_dialog.set_transient_for(Some(&this));
                    save_dialog.show();
                }
            })
        );

        self.add_simple_action("add-album-to-playlist", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, album_id| {
                if let Some(id) = album_id.and_then(|u| u.get::<i64>()) {
                    let album = model().album(id).unwrap();
                    let dialog = AddToPlaylistDialog::new(album.track_ids());
                    dialog.set_transient_for(Some(&this));
                    dialog.show();
                }
            })
        );
        

        self.add_simple_action("go-to-album-detail", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, album_id| {
                if let Some(id) = album_id.and_then(|u| u.get::<i64>()) {
                    this.album_selected_go_to_detail(id);
                }
            })
        );


        self.add_simple_action("go-to-artist-detail", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, artist_id| {
                if let Some(id) = artist_id.and_then(|u| u.get::<i64>()) {
                    this.switch_stack_page(WindowPage::Artists, true);
                    this.artist_selected_go_to_detail(id);
                }
            })
        );

        self.add_simple_action("go-to-playlist-detail", Some(glib::VariantTy::INT64), 
        clone!(@strong self as this => @default-panic, move |_, artist_id| {
            if let Some(id) = artist_id.and_then(|u| u.get::<i64>()) {
                this.switch_stack_page(WindowPage::Playlists, true);
                this.playlist_selected_go_to_detail(id);
            }
        })
    );


        // self.create_action('go-to-queue', self.action_go_to_queue)
        self.add_simple_action("go-to-queue", None, 
            clone!(@strong self as this => @default-panic, move |_, _| {
                this.switch_stack_page(WindowPage::Queue, true);
            })
        );

        // self.create_action_array('play-album-from-track', self.action_play_album_from_track)
        self.add_simple_action("play-album-from-track", Some(glib::VariantType::new_array(glib::VariantTy::INT64).as_ref()), 
            move |_, array| {
                if let Some(array) = array.and_then(|u| u.get::<Vec<i64>>()) {
                    let album_id = array[0];
                    let disc_no = array[1];
                    let track_no = array[2];
                    let tracks = model().album(album_id).unwrap().disc(disc_no).unwrap().values().cloned().collect();
                    let player = player();
                    player.clear_play_album(tracks, None);
                    player.go_to_playlist_position(track_no as u64);
                }
            }
        );

        // self.create_action_parameter('play-track', self.action_play_track)
        self.add_simple_action("play-track", Some(glib::VariantTy::INT64), 
            move |_, track_id| {
                if let Some(id) = track_id.and_then(|u| u.get::<i64>()) {
                    let track = model().track(id).unwrap();
                    player().clear_play_track(track);
                }
            }
        );


        // self.create_action_parameter('add-track-to-queue', self.action_add_track_to_queue)
        self.add_simple_action("add-track-to-queue", Some(glib::VariantTy::INT64), 
            move |_, track_id| {
                if let Some(id) = track_id.and_then(|u| u.get::<i64>()) {
                    let track = model().track(id).unwrap();
                    player().add_track(track);
                }
            }
        );

        // self.create_action_parameter('create-playlist-from-track', self.action_create_playlist_from_track)
        self.add_simple_action("create-playlist-from-track", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, track_id| {
                if let Some(id) = track_id.and_then(|u| u.get::<i64>()) {
                    let save_dialog = SavePlaylistDialog::new(vec![id]);
                    save_dialog.set_transient_for(Some(&this));
                    save_dialog.show();
                }
            })
        );

        // self.create_action_parameter('add-track-to-playlist', self.action_add_track_to_playlist)
        self.add_simple_action("add-track-to-playlist", Some(glib::VariantTy::INT64), 
            clone!(@strong self as this => @default-panic, move |_, track_id| {
                if let Some(id) = track_id.and_then(|u| u.get::<i64>()) {
                    let dialog = AddToPlaylistDialog::new(vec![id]);
                    dialog.set_transient_for(Some(&this));
                    dialog.show();
                }
            })
        );




        // self.create_action_parameter('remove-track-from-queue', self.action_remove_track_from_queue)
        self.add_simple_action("remove-track-from-queue", Some(glib::VariantTy::UINT64), 
            move |_, position| {
                if let Some(position_to_remove) = position.and_then(|u| u.get::<u64>()) {
                    player().queue().remove_track(position_to_remove as usize);
                }
            }
        );

        // self.create_action_parameter('play-playlist', self.action_play_playlist)
        self.add_simple_action("play-playlist", Some(glib::VariantTy::INT64), 
            move |_, playlist_id| {
                if let Some(id) = playlist_id.and_then(|u| u.get::<i64>()) {
                    let playlist = model().playlist(id).unwrap();
                    player().clear_play_album(playlist.tracks(), Some(playlist.title()));
                }
            }
        );
        
        // self.create_action_parameter('add-playlist-to-queue', self.action_add_playlist_to_queue)
        self.add_simple_action("add-playlist-to-queue", Some(glib::VariantTy::INT64), 
            move |_, playlist_id| {
                if let Some(id) = playlist_id.and_then(|u| u.get::<i64>()) {
                    let playlist = model().playlist(id).unwrap();
                    player().add_album(playlist.tracks());
                }
            }
        );

        // self.create_action_parameter('duplicate-playlist', self.action_duplicate_playlist)
        self.add_simple_action("duplicate-playlist", Some(glib::VariantTy::INT64), 
        clone!(@strong self as this => @default-panic, move |_, playlist_id| {
                if let Some(id) = playlist_id.and_then(|u| u.get::<i64>()) {
                    let playlist = model().playlist(id).unwrap();
                    let dialog = DuplicatePlaylistDialog::new(playlist);
                    dialog.set_transient_for(Some(&this));
                    dialog.show();
                }
            })
        );

        // self.create_action_parameter('delete-playlist', self.action_delete_playlist)
        self.add_simple_action("delete-playlist", Some(glib::VariantTy::INT64), 
        clone!(@strong self as this => @default-panic, move |_, playlist_id| {
                if let Some(id) = playlist_id.and_then(|u| u.get::<i64>()) {
                    let dialog = DeletePlaylistDialog::new(id);
                    dialog.set_transient_for(Some(&this));
                    dialog.show();
                }
            })
        );


        // self.create_action_array('play-playlist-from-track', self.action_play_playlist_from_track)
        self.add_simple_action("play-playlist-from-track", Some(glib::VariantType::new_array(glib::VariantTy::INT64).as_ref()), 
            move |_, array| {
                if let Some(array) = array.and_then(|u| u.get::<Vec<i64>>()) {
                    let playlist_id = array[0];
                    let track_no = array[1];
                    let tracks = model().playlist(playlist_id).unwrap().tracks();
                    let player = player();
                    player.clear_play_album(tracks, None);
                    player.go_to_playlist_position(track_no as u64);
                }
            }
        );

        // self.create_action('save-playlist', self.action_create_playlist_from_queue)
        self.add_simple_action("save-playlist", None, 
            clone!(@strong self as this => @default-panic, move |_, _| {
                let save_dialog = SavePlaylistDialog::new(player().queue().track_ids());
                save_dialog.set_transient_for(Some(&this));
                save_dialog.show();
            })
        );

        // self.create_action('add-queue-to-playlist', self.action_add_queue_to_playlist)
        self.add_simple_action("add-queue-to-playlist", None, 
            clone!(@strong self as this => @default-panic, move |_, _| {
                let save_dialog = AddToPlaylistDialog::new(player().queue().track_ids());
                save_dialog.set_transient_for(Some(&this));
                save_dialog.show();
            })
        );

        // self.create_action('clear-queue', self.action_clear_queue)
        self.add_simple_action("end-queue", None, 
            move |_, _| {
                player().queue().end_queue();
            }
        );

        self.add_simple_action("toggle-play-pause", None, 
        move |_, _| {
                player().toggle_play_pause();
            }
        );

        self.add_simple_action("prev", None, 
        move |_, _| {
                player().prev();
            }
        );

        self.add_simple_action("next", None, 
        move |_, _| {
                player().next();
            }
        );

        self.add_simple_action("next", None, 
        move |_, _| {
                player().next();
            }
        );

        
        self.add_simple_action("skip-queue-to-track", Some(glib::VariantTy::UINT64), 
            move |_, playlist_position| {
                if let Some(pos) = playlist_position.and_then(|u| u.get::<u64>()) {
                    player().go_to_playlist_position(pos);
                }
            }
        );

    }


    pub fn add_simple_action<F>(&self, name: &str, param: Option<&glib::VariantTy>, f: F) 
    where
    F: Fn(&gio::SimpleAction, Option<&glib::Variant>) + 'static, {
        let action = gio::SimpleAction::new(name, param);
        action.set_enabled(true);
        action.connect_activate(f);
        self.add_action(&action);
    }

    /*
    GIO SETTINGS
    */

    pub fn setup_settings(&self) {
        let imp = self.imp();


        imp.settings.connect_changed(
            Some("album-grid-sort"),
            clone!(@strong imp.album_grid_page as this => move |settings, _name| {
                let selected = settings.int("album-grid-sort");
                let sort_method = match selected {
                    0 => SortMethod::Album,
                    1 => SortMethod::Artist,
                    2 => SortMethod::Genre,
                    3 => SortMethod::ReleaseDate,
                    4 => SortMethod::Duration,
                    5 => SortMethod::TrackCount,
                    _ => SortMethod::Album,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("album-grid-sort");
        let sort_method = match selected {
            0 => SortMethod::Album,
            1 => SortMethod::Artist,
            2 => SortMethod::Genre,
            3 => SortMethod::ReleaseDate,
            4 => SortMethod::Duration,
            5 => SortMethod::TrackCount,
            _ => SortMethod::Album,
        };
        imp.album_grid_page.set_property("sort-method", sort_method.to_value());

        imp.settings.bind("album-grid-display-labels", &*imp.album_grid_page, "display-labels-default")
            .flags(gio::SettingsBindFlags::GET)
            .build();

        imp.settings.bind("album-grid-disable-flap", &*imp.album_grid_page, "disable-flap")
            .flags(gio::SettingsBindFlags::GET)
            .build();

        // TRACK PAGE
        imp.settings.connect_changed(
            Some("track-page-sort"),
            clone!(@strong imp.track_page as this => move |settings, _name| {
                let selected = settings.int("track-page-sort");
                let sort_method = match selected {
                    0 => SortMethod::Track,
                    1 => SortMethod::Album,
                    2 => SortMethod::Artist,
                    3 => SortMethod::Genre,
                    4 => SortMethod::ReleaseDate,
                    5 => SortMethod::Duration,
                    _ => SortMethod::Track,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("track-page-sort");
        let sort_method = match selected {
            0 => SortMethod::Track,
            1 => SortMethod::Album,
            2 => SortMethod::Artist,
            3 => SortMethod::Genre,
            4 => SortMethod::ReleaseDate,
            5 => SortMethod::Duration,
            _ => SortMethod::Track,
        };
        imp.track_page.set_property("sort-method", sort_method.to_value());

        imp.settings.bind("track-page-display-labels", &*imp.track_page, "display-labels-default")
            .flags(gio::SettingsBindFlags::GET)
            .build();

        imp.settings.bind("track-page-display-search", &*imp.track_page, "search-mode-default")
            .flags(gio::SettingsBindFlags::GET)
            .build();


        // PLAYLIST GRID
        imp.settings.connect_changed(
            Some("playlist-grid-sort"),
            clone!(@strong imp.playlist_grid_page as this => move |settings, _name| {
                let selected = settings.int("playlist-grid-sort");
                let sort_method = match selected {
                    0 => SortMethod::Playlist,
                    1 => SortMethod::LastModified,
                    2 => SortMethod::Duration,
                    3 => SortMethod::TrackCount,
                    _ => SortMethod::LastModified,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("playlist-grid-sort");
        let sort_method = match selected {
            0 => SortMethod::Playlist,
            1 => SortMethod::LastModified,
            2 => SortMethod::Duration,
            3 => SortMethod::TrackCount,
            _ => SortMethod::LastModified,
        };
        imp.playlist_grid_page.set_property("sort-method", sort_method.to_value());

        // ARTIST GRID
        imp.settings.connect_changed(
            Some("artist-grid-sort"),
            clone!(@strong imp.artist_grid_page as this => move |settings, _name| {
                let selected = settings.int("artist-grid-sort");
                let sort_method = match selected {
                    0 => SortMethod::Artist,
                    1 => SortMethod::AlbumCount,
                    2 => SortMethod::TrackCount,
                    _ => SortMethod::Artist,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("artist-grid-sort");
        let sort_method = match selected {
            0 => SortMethod::Artist,
            1 => SortMethod::AlbumCount,
            2 => SortMethod::TrackCount,
            _ => SortMethod::Artist,
        };
        imp.artist_grid_page.set_property("sort-method", sort_method.to_value());

        // GENRE GRID
        imp.settings.connect_changed(
            Some("genre-grid-sort"),
            clone!(@strong imp.genre_grid_page as this => move |settings, _name| {
                let selected = settings.int("genre-grid-sort");
                let sort_method = match selected {
                    0 => SortMethod::Genre,
                    1 => SortMethod::AlbumCount,
                    2 => SortMethod::TrackCount,
                    _ => SortMethod::Genre,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("genre-grid-sort");
        let sort_method = match selected {
            0 => SortMethod::Genre,
            1 => SortMethod::AlbumCount,
            2 => SortMethod::TrackCount,
            _ => SortMethod::Genre,
        };
        imp.genre_grid_page.set_property("sort-method", sort_method.to_value());

        // ARTIST DETAIL
        imp.settings.connect_changed(
            Some("artist-detail-sort"),
            clone!(@strong imp.artist_detail_page as this => move |settings, _name| {
                let selected = settings.int("artist-detail-sort");
                let sort_method = match selected {
                    0 => SortMethod::Album,
                    1 => SortMethod::ReleaseDate,
                    2 => SortMethod::Duration,
                    3 => SortMethod::TrackCount,
                    _ => SortMethod::Album,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("artist-detail-sort");
        let sort_method = match selected {
            0 => SortMethod::Album,
            1 => SortMethod::ReleaseDate,
            2 => SortMethod::Duration,
            3 => SortMethod::TrackCount,
            _ => SortMethod::Album,
        };
        imp.artist_detail_page.set_property("sort-method", sort_method.to_value());

        // GENRE DETAIL
        imp.settings.connect_changed(
            Some("genre-detail-sort"),
            clone!(@strong imp.genre_detail_page as this => move |settings, _name| {
                let selected = settings.int("genre-detail-sort");
                let sort_method = match selected {
                    0 => SortMethod::Album,
                    1 => SortMethod::Artist,
                    2 => SortMethod::ReleaseDate,
                    3 => SortMethod::Duration,
                    4 => SortMethod::TrackCount,
                    _ => SortMethod::Album,
                };
                this.set_property("sort-method", sort_method.to_value());
            }),
        );

        let selected = imp.settings.int("genre-detail-sort");
        let sort_method = match selected {
            0 => SortMethod::Album,
            1 => SortMethod::Artist,
            2 => SortMethod::ReleaseDate,
            3 => SortMethod::Duration,
            4 => SortMethod::TrackCount,
            _ => SortMethod::Album,
        };
        imp.genre_detail_page.set_property("sort-method", sort_method.to_value());

        // QUEUE OPEN BY DEFAULT
        imp.settings.connect_changed(
            Some("queue-open-default"),
            clone!(@strong self as this => move |settings, _name| {
                let queue_open = settings.boolean("queue-open-default");
                this.imp().open_queue.set(queue_open);
            }),
        );

        let queue_open = imp.settings.boolean("queue-open-default");
        imp.open_queue.set(queue_open);

        imp.settings.connect_changed(
            Some("full-page-back-button"),
            clone!(@strong self as this => move |settings, _name| {
                let imp = this.imp();
                let show_back_nav_button = !settings.boolean("full-page-back-button");
                debug!("show_back_nav_button {}", show_back_nav_button);
                imp.show_back_nav_button.set(show_back_nav_button);

                if !show_back_nav_button {
                    imp.navigate_back_button.hide();
                }
            }),
        );
        
        imp.settings.connect_changed(
                Some("default-volume"),
                move |settings, _name| {
                    let volume = settings.double("default-volume");
                    player().set_volume(volume);
                },
        );
    
        imp.settings.connect_changed(
            Some("shuffle-mode-loop"),
            move |settings, _name| {
                debug!("shuffle mode changed");
                let mode = settings.boolean("shuffle-mode-loop");
                player().queue().set_shuffle_mode(mode);
            }
        );

        imp.settings.connect_changed(
            Some("discord-rich-presence"),
            move |settings, _name| {
                let discord_enabled = settings.boolean("discord-rich-presence");
                let player = player();
                player.discord_enabled.set(discord_enabled);
                let sender = player.discord_sender.clone();
                if discord_enabled {
                    send!(sender, DiscordAction::Reconnect);
                } else {
                    send!(sender, DiscordAction::Close);
                }              
            },
        );

        imp.settings.connect_changed(
            Some("last-fm-enabled"),
            move |settings, _name| {
                let enabled = settings.boolean("last-fm-enabled");
                let player = player();
                player.lastfm_enabled.set(enabled);
                let sender = player.lastfm_sender.clone();
                send!(sender, LastFmAction::Enabled(enabled));            
            },
        );

        imp.settings.connect_changed(
            Some("play-commit-threshold"),
            move |settings, _name| {
                let threshold = settings.double("play-commit-threshold");
                let player = player();
                player.commit_threshold.set(threshold);         
            },
        );
    }
}

