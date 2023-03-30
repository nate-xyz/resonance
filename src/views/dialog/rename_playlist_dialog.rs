/* rename_playlist_dialog.rs
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
use crate::toasts::add_error_toast;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/rename_playlist_dialog.ui")]
    pub struct RenamePlaylistDialogPriv {
        #[template_child(id = "adw_entry_row")]
        pub adw_entry_row: TemplateChild<adw::EntryRow>,
        pub playlist: RefCell<Option<Rc<Playlist>>>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RenamePlaylistDialogPriv {
        const NAME: &'static str = "RenamePlaylistDialog";
        type Type = super::RenamePlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                adw_entry_row: TemplateChild::default(),
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

    impl ObjectImpl for RenamePlaylistDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for RenamePlaylistDialogPriv {}
    impl WindowImpl for RenamePlaylistDialogPriv {}
    impl MessageDialogImpl for RenamePlaylistDialogPriv {}
    impl RenamePlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct RenamePlaylistDialog(ObjectSubclass<imp::RenamePlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RenamePlaylistDialog {
    pub fn new(playlist: Rc<Playlist>) -> RenamePlaylistDialog {
        let dialog: RenamePlaylistDialog = glib::Object::builder::<RenamePlaylistDialog>().build();
        dialog.reset_name(playlist.title());
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

    fn reset_name(&self, name: String) {
        let imp = self.imp();
        imp.adw_entry_row.set_text(name.as_str());
    }

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();
        let playlist = imp.playlist.borrow().as_ref().unwrap().clone();
        if response == "rename" {
            let new_title = imp.adw_entry_row.text().to_string();
            if new_title.is_empty() {
                add_error_toast("Cannot rename, no name entered".to_string());
                return;
            }
            send!(imp.db_sender, DatabaseAction::RenamePlaylist((playlist.id(), playlist.title(), new_title)));
        }
    }
}
