/* duplicate_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::{cell::RefCell, rc::Rc};
use log::error;

use crate::model::playlist::Playlist;
use crate::database::DatabaseAction;
use crate::util::database;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/duplicate_playlist_dialog.ui")]
    pub struct DuplicatePlaylistDialogPriv {
        pub playlist: RefCell<Option<Rc<Playlist>>>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DuplicatePlaylistDialogPriv {
        const NAME: &'static str = "DuplicatePlaylistDialog";
        type Type = super::DuplicatePlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                playlist: RefCell::new(None),
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

    impl ObjectImpl for DuplicatePlaylistDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("done").build(),
                ]
            });

            SIGNALS.as_ref()
        }
    }

    impl WidgetImpl for DuplicatePlaylistDialogPriv {}
    impl WindowImpl for DuplicatePlaylistDialogPriv {}
    impl MessageDialogImpl for DuplicatePlaylistDialogPriv {}
    impl DuplicatePlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct DuplicatePlaylistDialog(ObjectSubclass<imp::DuplicatePlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl DuplicatePlaylistDialog {
    pub fn new(playlist: Rc<Playlist>) -> DuplicatePlaylistDialog {
        let dialog: DuplicatePlaylistDialog = glib::Object::builder::<DuplicatePlaylistDialog>().build();
        dialog.imp().playlist.replace(Some(playlist));
        dialog
    }

    pub fn initialize(&self) {
        self.set_destroy_with_parent(true);

        self.connect_response(
            None,
            clone!(@strong self as this => move |_dialog, response| {
                this.dialog_response(response);
            }),
        );
    }

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();

        if response == "duplicate" {
            if let Some(playlist) = imp.playlist.borrow().as_ref() {
                let new_title = format!("{} copy", playlist.title());
                let mut tracks = Vec::new();
                for track in playlist.tracks() {
                    tracks.push(track.id());
                }
                send!(imp.db_sender, DatabaseAction::DuplicatePlaylist((new_title, playlist.description(), tracks)));   
            }
        }
        self.emit_by_name::<()>("done", &[]);
    }

}
