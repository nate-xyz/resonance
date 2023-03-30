/* delete_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::cell::Cell;
use log::error;

use crate::database::DatabaseAction;
use crate::util::database;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use once_cell::sync::Lazy;
    
    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/delete_playlist_dialog.ui")]
    pub struct DeletePlaylistDialogPriv {
        pub playlist_id: Cell<i64>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeletePlaylistDialogPriv {
        const NAME: &'static str = "DeletePlaylistDialog";
        type Type = super::DeletePlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                playlist_id: Cell::new(-1),
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

    impl ObjectImpl for DeletePlaylistDialogPriv {
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

    impl WidgetImpl for DeletePlaylistDialogPriv {}
    impl WindowImpl for DeletePlaylistDialogPriv {}
    impl MessageDialogImpl for DeletePlaylistDialogPriv {}
    impl DeletePlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct DeletePlaylistDialog(ObjectSubclass<imp::DeletePlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl DeletePlaylistDialog {
    pub fn new(playlist_id: i64) -> DeletePlaylistDialog {
        let dialog: DeletePlaylistDialog = glib::Object::builder::<DeletePlaylistDialog>().build();
        dialog.imp().playlist_id.set(playlist_id);
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
        if response == "delete" {
            let playlist_id = imp.playlist_id.get();
            if playlist_id == -1 {
                return;
            }
            send!(imp.db_sender, DatabaseAction::DeletePlaylist(playlist_id));
        }
        self.emit_by_name::<()>("done", &[]);
    }
}
