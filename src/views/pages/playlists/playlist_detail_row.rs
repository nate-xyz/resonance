/* playlist_grid_child.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, gdk, glib, glib::{clone, Sender}, CompositeTemplate};
use gtk_macros::send;

use std::{cell::{Cell, RefCell}, rc::Rc};
use log::{error, debug};

use crate::database::DatabaseAction;
use crate::model::track::Track;
use crate::views::art::rounded_album_art::RoundedAlbumArt;
use crate::views::art::icon_with_background::IconWithBackground;
use crate::util::{model, player, database, seconds_to_string};

use super::track_item::PlaylistDetailTrackItem;

mod imp {
    use super::*;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean
    };
    use once_cell::sync::Lazy;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/playlist_detail_row.ui")]
    pub struct PlaylistDetailRowPriv {
        #[template_child(id = "drag_icon_revealer")]
        pub drag_icon_revealer: TemplateChild<gtk::Revealer>,

        #[template_child(id = "delete_button_revealer")]
        pub delete_button_revealer: TemplateChild<gtk::Revealer>,
        
        #[template_child(id = "info_box")]
        pub info_box: TemplateChild<gtk::Box>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "end_box")]
        pub end_box: TemplateChild<gtk::Box>,

        #[template_child(id = "track_title_label")]
        pub track_title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label")]
        pub album_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_artist_label")]
        pub album_artist_label: TemplateChild<gtk::Label>,

        #[template_child(id = "number_label")]
        pub number_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "add_button")]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child(id = "overlay_box")]
        pub overlay_box: TemplateChild<gtk::Box>,

        #[template_child(id = "play_icon_no_art")]
        pub play_icon_no_art: TemplateChild<gtk::Image>,

        #[template_child(id = "drag_icon")]
        pub drag_icon: TemplateChild<gtk::Image>,

        #[template_child(id = "delete_button")]
        pub delete_button: TemplateChild<gtk::Button>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track: RefCell<Option<Rc<Track>>>,
        pub art: RefCell<Option<RoundedAlbumArt>>,
        pub playlist_id: Cell<i64>,
        pub playlist_entry_id: Cell<i64>,
        pub playlist_position: Cell<i64>,
        pub edit_mode: Cell<bool>,
        pub grab_x: Cell<f64>,
        pub grab_y: Cell<f64>,
        pub playlist_detail_track_item: RefCell<Option<PlaylistDetailTrackItem>>,
        pub db_sender: Sender<DatabaseAction>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistDetailRowPriv {
        const NAME: &'static str = "PlaylistDetailRow";
        type Type = super::PlaylistDetailRow;
        type ParentType = gtk::Box;


        fn new() -> Self {
            Self {
                drag_icon_revealer: TemplateChild::default(),
                delete_button_revealer: TemplateChild::default(),
                info_box: TemplateChild::default(),
                art_bin: TemplateChild::default(),
                end_box: TemplateChild::default(),
                track_title_label: TemplateChild::default(),
                album_name_label: TemplateChild::default(),
                album_artist_label: TemplateChild::default(),
                number_label: TemplateChild::default(),
                duration_label: TemplateChild::default(),
                add_button: TemplateChild::default(),
                overlay_box: TemplateChild::default(),
                play_icon_no_art: TemplateChild::default(),
                drag_icon: TemplateChild::default(),
                delete_button: TemplateChild::default(),
                popover: TemplateChild::default(),
                track: RefCell::new(None),
                art: RefCell::new(None),
                playlist_id: Cell::new(-1),
                playlist_entry_id: Cell::new(-1),
                playlist_position: Cell::new(-1),
                edit_mode: Cell::new(false),
                grab_x: Cell::new(0.0),
                grab_y: Cell::new(0.0),
                playlist_detail_track_item: RefCell::new(None),
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

    impl ObjectImpl for PlaylistDetailRowPriv {
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
                    self.obj().update_edit_ui();
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

    impl WidgetImpl for PlaylistDetailRowPriv {}
    impl BoxImpl for PlaylistDetailRowPriv {}
    impl PlaylistDetailRowPriv {}
}

glib::wrapper! {
    pub struct PlaylistDetailRow(ObjectSubclass<imp::PlaylistDetailRowPriv>)
    @extends gtk::Box, gtk::Widget;
}

impl PlaylistDetailRow {
    pub fn new() -> PlaylistDetailRow {
        let track_row: PlaylistDetailRow = glib::Object::builder::<PlaylistDetailRow>().build();
        track_row
    }

    fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        imp.overlay_box.prepend(&IconWithBackground::new("media-playback-start-symbolic", 60, false));

        imp.add_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                let track = this.track();
                player().add_track(track);
            })
        );

        imp.delete_button.connect_clicked(
                clone!(@strong self as this => @default-panic, move |_button| {
                    let imp = this.imp();
                    send!(imp.db_sender, DatabaseAction::RemoveTrackFromPlaylist(imp.playlist_entry_id.get()));
            })
        );

        let ctrl = gtk::EventControllerMotion::new();
        ctrl.connect_enter(
            clone!(@strong self as this => move |_controller, _x, _y| {
                let imp = this.imp();

                if !imp.edit_mode.get() {
                    imp.duration_label.hide();
                    imp.add_button.show();
                    
                    match imp.art.borrow().as_ref() {
                        Some(_art) => {
                            imp.overlay_box.show();
                        }
                        None => {
                            imp.play_icon_no_art.show();
                        },
                    }
                } 

            })
        );

        ctrl.connect_leave(
            clone!(@strong self as this => move |_controller| {
                let imp = this.imp();

                if !imp.edit_mode.get() {
                    imp.duration_label.show();
                    imp.add_button.hide();
                    
                    match imp.art.borrow().as_ref() {
                        Some(_art) => {
                            imp.overlay_box.hide();
                        }
                        None => {
                            imp.play_icon_no_art.hide();
                        },
                    }
                } 

            })
        );
        self.add_controller(ctrl);

        let ctrl = gtk::DragSource::builder()
            .actions(gdk::DragAction::MOVE)
            .build();

        ctrl.connect_prepare(
            clone!(@strong self as this => @default-return None, move |_drag_source: &gtk::DragSource, x, y| {
                debug!("drag_source prepare");
                let imp = this.imp();
                if imp.edit_mode.get() {
                    
                    imp.grab_x.set(x);
                    imp.grab_y.set(y);
     
                    debug!("drag_source Some");
                    Some(gdk::ContentProvider::for_value(&glib::Value::from(this.playlist_position())))
                } else {
                    debug!("drag_source None");
                    None
                }
            })
        );

        ctrl.connect_drag_begin(            
            clone!(@strong self as this => move |drag_source: &gtk::DragSource, _drag_object| {
                let imp = this.imp();
                let paintable = gtk::WidgetPaintable::new(Some(&this));
                drag_source.set_icon(Some(&paintable), imp.grab_x.get() as i32, imp.grab_y.get() as i32)
            })
        );

        ctrl.connect_drag_end(
            clone!(@strong self as this => move |_drag_source: &gtk::DragSource, _drag_object, _delete_data| {
                debug!("drag_source end");
                ()
  
            })
        );

        ctrl.connect_drag_cancel(
            clone!(@strong self as this => move |_drag_source: &gtk::DragSource, _drag_object, _reason| {
                debug!("drag_source cancel");
                let imp = this.imp();
                imp.edit_mode.get()
            })
        );

        self.add_controller(ctrl);

        let drop_target = gtk::DropTargetAsync::builder()
            .actions(gdk::DragAction::MOVE)
            .build();

        drop_target.connect_accept(clone!(@strong self as this => move |_drop_target, _drop_value| {
                let imp = this.imp();
                imp.edit_mode.get()
        }));

        drop_target.connect_drop(
            clone!(@strong self as this => move |_drop_target, drop_value, _x, _y| {
                debug!("drop_target drop");

                drop_value.read_value_async(i64::static_type(), glib::PRIORITY_DEFAULT, None::<&gio::Cancellable>, 
                    clone!(@strong this => move |value| {
                        let imp = this.imp();
                        let old_position = value.unwrap().get::<i64>().ok().unwrap();

                        send!(imp.db_sender, DatabaseAction::ReorderPlaylist((imp.playlist_id.get(), old_position as usize, imp.playlist_position.get() as usize)));                        
                    })
                );
                
                debug!("done drop");
                drop_value.finish(gdk::DragAction::MOVE);
                true 
            }),
        );

        self.add_controller(drop_target);

        // let ctrl = gtk::GestureClick::new();
        // ctrl.connect_unpaired_release(
        //     clone!(@strong self as this => move |_gesture_click, _x, _y, button, _sequence| {
        //         let imp = this.imp();
        //         if button == gdk::BUTTON_SECONDARY {
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }

    pub fn update_track_item(&self, track_item: PlaylistDetailTrackItem) {
        let imp = self.imp();
        imp.playlist_entry_id.set(track_item.playlist_entry_id());
        imp.playlist_position.set(track_item.position());
        imp.playlist_id.set(track_item.playlist_id());
        imp.track.replace(Some(track_item.track()));
        //imp.popover.set_menu_model(Some(track_item.playlist_entry().menu_model()));
        imp.playlist_detail_track_item.replace(Some(track_item));
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        match imp.track.borrow().as_ref() {
            Some(track) => {
                imp.track_title_label.set_label(&track.title());
                imp.album_name_label.set_label(&format!(" - {} - ", track.album()));
                imp.album_artist_label.set_label(&track.artist());
                imp.number_label.set_label(&format!("{:02}", imp.playlist_position.get()+1));

                let duration = track.duration();
                if duration > 0.0 {
                    imp.duration_label.set_label(seconds_to_string(duration).as_str());
                }

                match track.cover_art_option() {
                    Some(id) => match self.load_image(id) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                            imp.art.replace(Some(art));
                        }
                        Err(msg) => {
                            error!("{}", msg);
                            imp.art_bin.set_child(gtk::Widget::NONE);
                            imp.art.replace(None);
                        }
                    },
                    None => {
                        imp.art_bin.set_child(gtk::Widget::NONE);
                        imp.art.replace(None);
                    }
                }
                

            }
            None => {
                imp.track_title_label.set_label("");
                imp.album_name_label.set_label("");
                imp.album_artist_label.set_label("");
                imp.number_label.set_label("");
                imp.art_bin.set_child(gtk::Widget::NONE);
                imp.art.replace(None);
            }
        }
    }

    pub fn update_edit_ui(&self) {
        let imp = self.imp();

        let edit_mode = imp.edit_mode.get();
        
        imp.duration_label.hide();
        imp.add_button.hide();
        imp.drag_icon_revealer.set_reveal_child(edit_mode);
        imp.delete_button_revealer.set_reveal_child(edit_mode);
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let art = RoundedAlbumArt::new(60);
        match model().cover_art(cover_art_id) {
            Ok(cover_art) => {
                match cover_art.pixbuf() {
                    Ok(pixbuf) => {
                        art.load(pixbuf);
                        return Ok(art);
                    }
                    Err(msg) => return Err(msg),
                };
            }
            Err(msg) => return Err(msg),
        };
    }

    pub fn set_edit_mode(&self, edit_mode: bool) {
        self.imp().edit_mode.set(edit_mode);
    }

    pub fn edit_mode(&self) -> bool {
        self.imp().edit_mode.get()
    }

    pub fn set_playlist_position(&self, position: i64) {
        self.imp().playlist_position.set(position);
    }

    pub fn playlist_position(&self) -> i64 {
        self.imp().playlist_position.get()
    }

    fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }
}
