/* add_tracks_to_playlist_dialog.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::{rc::Rc, cell::RefCell, collections::HashMap};
use log::error;

use crate::database::DatabaseAction;
use crate::model::playlist::Playlist;
use crate::util::{model, database};
use crate::toasts::add_error_toast;

mod imp {
    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/add_tracks_to_playlist_dialog.ui")]
    pub struct AddToPlaylistDialogPriv {
        #[template_child(id = "list_box")]
        pub list_box: TemplateChild<gtk::ListBox>,

        pub row_map: RefCell<HashMap<gtk::Box, Rc<Playlist>>>,
        pub selected_playlist: RefCell<Option<Rc<Playlist>>>,
        pub track_ids: RefCell<Option<Vec<i64>>>,
        pub db_sender: Sender<DatabaseAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddToPlaylistDialogPriv {
        const NAME: &'static str = "AddToPlaylistDialog";
        type Type = super::AddToPlaylistDialog;
        type ParentType = adw::MessageDialog;

        fn new() -> Self {
            Self {
                list_box: TemplateChild::default(),
                row_map: RefCell::new(HashMap::new()),
                selected_playlist: RefCell::new(None),
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

    impl ObjectImpl for AddToPlaylistDialogPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for AddToPlaylistDialogPriv {}
    impl WindowImpl for AddToPlaylistDialogPriv {}
    impl MessageDialogImpl for AddToPlaylistDialogPriv {}
    impl AddToPlaylistDialogPriv {}
}

glib::wrapper! {
    pub struct AddToPlaylistDialog(ObjectSubclass<imp::AddToPlaylistDialogPriv>)
    @extends gtk::Widget, gtk::Window, adw::MessageDialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AddToPlaylistDialog {
    pub fn new(track_ids: Vec<i64>) -> AddToPlaylistDialog {
        let dialog: AddToPlaylistDialog = glib::Object::builder::<AddToPlaylistDialog>().build();
        dialog.imp().track_ids.replace(Some(track_ids));
        dialog
    }

    pub fn initialize(&self) {
        let imp = self.imp();
        self.set_destroy_with_parent(true);

        match model().playlists() {
            Some(map) => {
                for (_i, playlist) in map {
                    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                    let label = gtk::Label::new(Some(&playlist.title()));
                    box_.append(&label);
                    imp.list_box.append(&box_);
                    imp.row_map.borrow_mut().insert(box_, playlist);
                }
            },
            None => {
                error!("No playlists to add to");
            },
        }

        imp.list_box.set_activate_on_single_click(true);
        imp.list_box.connect_row_activated(
            clone!(@strong self as this => move |_list_box, row| {
                let imp = this.imp();
                let box_ = row.child().unwrap().downcast::<gtk::Box>().unwrap();
                imp.selected_playlist.replace(Some(imp.row_map.borrow()[&box_].clone()));
            }),
        );

        self.connect_response(
            None,
            clone!(@strong self as this => move |_dialog, response| {
                this.dialog_response(response);
            }),
        );
    }

    fn dialog_response(&self, response: &str) {
        let imp = self.imp();
        if response == "add" {
            if let Some(playlist) = imp.selected_playlist.borrow().as_ref() {
                if let Some(track_ids) = imp.track_ids.borrow().as_ref() {
                    send!(imp.db_sender, DatabaseAction::AddTracksToPlaylist((playlist.id(), playlist.title(), track_ids.clone())))
                }
            } else {
                error!("Unable to add tracks to playlist, no playlist selected");
                add_error_toast("Unable to add tracks to playlist, no playlist selected".to_string());
            }
        }
    }
}
