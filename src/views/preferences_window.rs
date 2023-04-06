/* preferences_window.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, gio::SettingsBindFlags, glib, glib::{clone, Sender}};
use gtk_macros::send;

use std::cell::RefCell;
use std::error::Error;
use log::{debug, error};

use crate::i18n::i18n;
use crate::util::{self, database};
use crate::database::DatabaseAction;
use crate::views::dialog::remove_directory_dialog::RemoveDirectoryDialog;

mod imp {
    use super::*;

    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/preferences_window.ui")]
    pub struct PreferencesWindow {
        #[template_child(id = "spin_play_threshold")]
        pub spin_play_threshold: TemplateChild<gtk::SpinButton>,

        #[template_child(id = "action_row_lastfm")]
        pub action_row_lastfm: TemplateChild<adw::ActionRow>,

        #[template_child(id = "switch_enable_lastfm")]
        pub switch_enable_lastfm: TemplateChild<gtk::Switch>,

        #[template_child(id = "switch_discord")]
        pub switch_discord: TemplateChild<gtk::Switch>,

        #[template_child(id = "switch_full_page_back")]
        pub switch_full_page_back: TemplateChild<gtk::Switch>,

        #[template_child(id = "switch_queue_open_default")]
        pub switch_queue_open_default: TemplateChild<gtk::Switch>,

        #[template_child(id = "album_grid_sort")]
        pub album_grid_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "switch_display_labels")]
        pub switch_display_labels: TemplateChild<gtk::Switch>,

        #[template_child(id = "switch_disable_album_flap")]
        pub switch_disable_album_flap: TemplateChild<gtk::Switch>,

        #[template_child(id = "track_page_sort")]
        pub track_page_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "switch_disable_track_page_searchbar")]
        pub switch_disable_track_page_searchbar: TemplateChild<gtk::Switch>,

        #[template_child(id = "switch_track_page_additional_labels")]
        pub switch_track_page_additional_labels: TemplateChild<gtk::Switch>,

        #[template_child(id = "playlist_grid_sort")]
        pub playlist_grid_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "artist_grid_sort")]
        pub artist_grid_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "artist_detail_sort")]
        pub artist_detail_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "genre_grid_sort")]
        pub genre_grid_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "genre_detail_sort")]
        pub genre_detail_sort: TemplateChild<gtk::ComboBoxText>,

        #[template_child(id = "spin_volume_default")]
        pub spin_volume_default: TemplateChild<gtk::SpinButton>,

        #[template_child(id = "switch_loop_shuffle")]
        pub switch_loop_shuffle: TemplateChild<gtk::Switch>,

        #[template_child(id = "add_folder_button")]
        pub add_folder_button: TemplateChild<gtk::Button>,

        #[template_child(id = "dir-list")]
        pub dir_list: TemplateChild<adw::PreferencesGroup>,

        #[template_child(id = "play_threshold_adjustment")]
        pub play_threshold_adjustment: TemplateChild<gtk::Adjustment>,

        #[template_child(id = "volume_adjustment")]
        pub volume_adjustment: TemplateChild<gtk::Adjustment>,

        #[template_child(id = "reset_default_all")]
        pub reset_default_all: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_queue")]
        pub reset_default_queue: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_album")]
        pub reset_default_album: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_track")]
        pub reset_default_track: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_playlists")]
        pub reset_default_playlists: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_artists")]
        pub reset_default_artists: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_genres")]
        pub reset_default_genres: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_playback")]
        pub reset_default_playback: TemplateChild<gtk::Button>,

        #[template_child(id = "reset_default_discord")]
        pub reset_default_discord: TemplateChild<gtk::Button>,

        pub folder_dialog: RefCell<Option<gtk::FileChooserNative>>,
        pub dir_rows: RefCell<Option<Vec<adw::ActionRow>>>,
        pub settings: gio::Settings,
        pub db_sender: Sender<DatabaseAction>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "PreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

        fn new() -> Self {
            Self {
                spin_play_threshold: TemplateChild::default(),
                action_row_lastfm: TemplateChild::default(),
                switch_enable_lastfm: TemplateChild::default(),
                switch_discord: TemplateChild::default(),
                switch_queue_open_default: TemplateChild::default(),
                playlist_grid_sort: TemplateChild::default(),
                artist_grid_sort: TemplateChild::default(),
                genre_grid_sort: TemplateChild::default(),
                switch_full_page_back: TemplateChild::default(),
                album_grid_sort: TemplateChild::default(),
                switch_display_labels: TemplateChild::default(),
                switch_disable_album_flap: TemplateChild::default(),
                track_page_sort: TemplateChild::default(),
                switch_disable_track_page_searchbar: TemplateChild::default(),
                switch_track_page_additional_labels: TemplateChild::default(),
                artist_detail_sort: TemplateChild::default(),
                genre_detail_sort: TemplateChild::default(),
                spin_volume_default: TemplateChild::default(),
                switch_loop_shuffle: TemplateChild::default(),
                add_folder_button: TemplateChild::default(),
                dir_list: TemplateChild::default(),
                play_threshold_adjustment: TemplateChild::default(),
                volume_adjustment: TemplateChild::default(),
                reset_default_all: TemplateChild::default(),
                reset_default_queue: TemplateChild::default(),
                reset_default_album: TemplateChild::default(),
                reset_default_track: TemplateChild::default(),
                reset_default_playlists: TemplateChild::default(),
                reset_default_artists: TemplateChild::default(),
                reset_default_genres: TemplateChild::default(),
                reset_default_playback: TemplateChild::default(),
                reset_default_discord: TemplateChild::default(),
                dir_rows: RefCell::new(None),
                folder_dialog: RefCell::new(None),
                settings: util::settings_manager(),
                db_sender: database().sender(),
            }
        }
    }

    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            match obj.setup_settings() {
                Ok(_) => (),
                Err(e) => error!("unable to load settings: {}", e),
            }
        }
    }

    impl WidgetImpl for PreferencesWindow {}
    impl WindowImpl for PreferencesWindow {}
    impl AdwWindowImpl for PreferencesWindow {}
    impl PreferencesWindowImpl for PreferencesWindow {}
}

glib::wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
    @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow,
    @implements gtk::Accessible;
}

impl PreferencesWindow {
    pub fn new() -> PreferencesWindow {
        let prefences: PreferencesWindow = glib::Object::builder::<PreferencesWindow>().build();
        prefences
    }

    fn setup_settings(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();

        debug!("pref window -> setup");

        self.load_folders();
        self.add_dialog();
        self.setup_button_connections();

        imp.settings.connect_changed(
            Some("music-folders"),
            clone!(@strong self as this => move |_settings, _name| {
                this.load_folders();
            }),
        );

        //LASTFM

        imp.settings
        .bind(
            "last-fm-enabled",
            &*imp.switch_enable_lastfm,
            "active",
        )
        .flags(SettingsBindFlags::DEFAULT)
        .build();

        imp.settings
        .bind(
            "play-commit-threshold",
            &imp.spin_play_threshold.adjustment(),
            "value",
        )
        .flags(SettingsBindFlags::DEFAULT)
        .build();

        imp.settings.bind("discord-rich-presence",&*imp.switch_discord, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "queue-open-default",
                &*imp.switch_queue_open_default,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("playlist-grid-sort", &*imp.playlist_grid_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "full-page-back-button",
                &*imp.switch_full_page_back,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("album-grid-sort", &*imp.album_grid_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "album-grid-display-labels",
                &*imp.switch_display_labels,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "album-grid-disable-flap",
                &*imp.switch_disable_album_flap,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();


        imp.settings
            .bind("artist-detail-sort", &*imp.artist_detail_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("artist-grid-sort", &*imp.artist_grid_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("track-page-sort", &*imp.track_page_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();


        imp.settings
            .bind(
                "track-page-display-search",
                &*imp.switch_disable_track_page_searchbar,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "track-page-display-labels",
                &*imp.switch_track_page_additional_labels,
                "active",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();


        imp.settings
            .bind("genre-detail-sort", &*imp.genre_detail_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("genre-grid-sort", &*imp.genre_grid_sort, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind(
                "play-commit-threshold",
                &*imp.play_threshold_adjustment,
                "value",
            )
            .flags(SettingsBindFlags::DEFAULT)
            .build();


        imp.settings
            .bind("default-volume", &*imp.volume_adjustment, "value")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        imp.settings
            .bind("shuffle-mode-loop", &*imp.switch_loop_shuffle, "active")
            .flags(SettingsBindFlags::DEFAULT)
            .build();

        Ok(())
    }

    fn load_folders(&self) {
        let imp = self.imp();
        
        if let Some(dir_rows) = imp.dir_rows.take() {
            for row in dir_rows {
                imp.dir_list.remove(&row);
            }
        } 
        let dirs = imp.settings.strv("music-folders").to_vec();
        let mut rows = Vec::new();
        for (i, s) in dirs.iter().enumerate() {
            let folder_name = s.to_string();

            let row = adw::ActionRow::new();
            row.set_title(&folder_name.clone());
            row.add_css_class("darken-mas-mas");
            

            let button = gtk::Button::new();
            button.set_icon_name("cross-filled-symbolic");
            button.add_css_class("destructive-action");
            button.add_css_class("opaque");
            button.add_css_class("circular");

            button.set_valign(gtk::Align::Center);
            button.set_halign(gtk::Align::End);

            button.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let imp = this.imp();
                    let folder = imp.settings.strv("music-folders").to_vec()[i].to_string();

                    let dialog = RemoveDirectoryDialog::new(folder);
                    dialog.set_transient_for(Some(&this));
                    dialog.connect_local(
                        "done", 
                        false, 
                        clone!(@strong this as that => move |_| {
                            that.close();
                            None
                        })
                    );
                    dialog.show();

                    
                })
            );

            row.add_suffix(&button);
            imp.dir_list.add(&row);
            rows.push(row);
        }

        imp.dir_rows.replace(Some(rows));
        
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

        dialog.connect_response(clone!(@weak self as this => move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let folder = dialog.file().unwrap().path().unwrap();
                debug!("DIALOG FOLDER RECEIVED: {:?}", folder);
                send!(this.imp().db_sender, DatabaseAction::TryAddMusicFolder(folder));
            } else {
                debug!("No folder selected.");
            }
        }));

        imp.folder_dialog.replace(Some(dialog));

        imp.add_folder_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                this.imp().folder_dialog.borrow().as_ref().unwrap().show();
            }),
        );
    }

    fn setup_button_connections(&self) {
        let imp = self.imp();

        imp.reset_default_all.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "full-page-back-button") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("full-page-back-button", value);
                    }
                }
            })
        );

        imp.reset_default_queue.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "queue-open-default") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("queue-open-default", value);
                    }
                }
            })
        );

        imp.reset_default_album.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "album-grid-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("album-grid-sort", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "album-grid-display-labels") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("album-grid-display-labels", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "album-grid-disable-flap") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("album-grid-disable-flap", value);
                    }
                }
            })
        );

        imp.reset_default_track.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "track-page-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("track-page-sort", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "track-page-display-labels") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("track-page-display-labels", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "track-page-display-search") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("track-page-display-search", value);
                    }
                }
            })
        );

        imp.reset_default_playlists.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "playlist-grid-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("playlist-grid-sort", value);
                    }
                }
            })
        );

        imp.reset_default_artists.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "artist-grid-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("artist-grid-sort", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "artist-detail-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("artist-detail-sort", value);
                    }
                }
            })
        );

        imp.reset_default_genres.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "genre-grid-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("genre-grid-sort", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "genre-detail-sort") {
                    if let Some(value) = variant.get::<i32>() {
                        _ = imp.settings.set_int("genre-detail-sort", value);
                    }
                }
            })
        );

        imp.reset_default_playback.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "default-volume") {
                    if let Some(value) = variant.get::<f64>() {
                        _ = imp.settings.set_double("default-volume", value);
                    }
                }
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "shuffle-mode-loop") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("shuffle-mode-loop", value);
                    }
                }
            })
        );

        imp.reset_default_discord.connect_clicked(clone!(@strong self as this => @default-panic, move |_button| {
                let imp = this.imp();
                if let Some(variant) = gio::Settings::default_value(&imp.settings, "discord-rich-presence") {
                    if let Some(value) = variant.get::<bool>() {
                        _ = imp.settings.set_boolean("discord-rich-presence", value);
                    }
                }
            })
        );
    }
}
