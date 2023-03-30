/* save_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::cell::RefCell;
use log::error;

use crate::database::DatabaseAction;
use crate::util::database;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/save_playlist_dialog.ui")]
    pub struct SavePlaylistDialogPriv {
        #[template_child(id = "title_adw_entry")]
        pub title_adw_entry: TemplateChild<adw::EntryRow>,

        #[template_child(id = "desc_adw_entry")]
        pub desc_adw_entry: TemplateChild<adw::EntryRow>,

        pub name: RefCell<String>,
        pub track_ids: RefCell<Option<Vec<i64>>>,

        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SavePlaylistDialogPriv {
        const NAME: &'static str = "SavePlaylistDialog";
        type Type = super::SavePlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                title_adw_entry: TemplateChild::default(),
                desc_adw_entry: TemplateChild::default(),
                name: RefCell::new("Playlist".to_string()),
                track_ids: RefCell::new(None),
                db_sender: database().sender(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SavePlaylistDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for SavePlaylistDialogPriv {}
    impl WindowImpl for SavePlaylistDialogPriv {}
    impl MessageDialogImpl for SavePlaylistDialogPriv {}
    impl SavePlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct SavePlaylistDialog(ObjectSubclass<imp::SavePlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SavePlaylistDialog {
    pub fn new(track_ids: Vec<i64>) -> SavePlaylistDialog {
        let dialog: SavePlaylistDialog = glib::Object::builder::<SavePlaylistDialog>().build();
        dialog.load(track_ids);
        dialog
    }

    fn load(&self, track_ids: Vec<i64>) {
        let imp = self.imp();
        if track_ids.len() == 0 {
            imp.track_ids.replace(None);
        } else {
            imp.track_ids.replace(Some(track_ids));
        }
    }

    pub fn initialize(&self) {
        self.set_destroy_with_parent(true);

        self.connect_response(
            None,
            clone!(@strong self as this => move |_dialog, response| {
                this.dialog_response(response);
            }),
        );

        match database().query_n_playlists(None) {
            Ok(count) => {
                let id = count+1;
                let name = format!("Playlist #{id}");
                self.reset_name(name);
            }
            Err(e) => error!("An error occurred: {}", e),
        }
    }

    fn reset_name(&self, name: String) {
        let imp = self.imp();
        imp.title_adw_entry.set_text(name.as_str());
        imp.name.replace(name);
    }

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();
        if response == "save" {
            if let Some(track_ids) = imp.track_ids.borrow().clone() {
                let mut playlist_title = imp.title_adw_entry.text().to_string();
                if playlist_title == "" {
                    playlist_title = imp.name.borrow().clone();
                } 
                let playlist_desc = imp.desc_adw_entry.text().to_string();
                //let database =  database();

                send!(imp.db_sender, DatabaseAction::CreatePlaylist((playlist_title.clone(), playlist_desc, track_ids)));

                // match database.create_playlist(playlist_title.clone(), playlist_desc, track_ids) {
                //     Ok(_) => {
                //         add_success_toast("Added", &format!("Playlist «{}» has been created!", playlist_title))
                //     },
                //     Err(e) => {
                //         error!("{}", e);
                //         add_error_toast("Unable to add playlist.".to_string());
                //     },
                // }
            }
        }
    }
}
