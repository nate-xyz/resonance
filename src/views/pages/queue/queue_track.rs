/* queue_track.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gio, gdk, glib, glib::clone, CompositeTemplate};

use std::{cell::Cell, cell::RefCell, rc::Rc};
use log::{debug, error};

use crate::model::track::Track;
use crate::views::art::rounded_album_art::RoundedAlbumArt;
use crate::util::{model, player};

use super::track_item::TrackItem;

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use glib::{
        Value, ParamSpec, ParamSpecBoolean
    };
    use once_cell::sync::Lazy;
    
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/queue_sidebar_track_entry.ui")]
    pub struct QueueTrackPriv {
        #[template_child(id = "drag_icon_revealer")]
        pub drag_icon_revealer: TemplateChild<gtk::Revealer>,

        #[template_child(id = "delete_button_revealer")]
        pub delete_button_revealer: TemplateChild<gtk::Revealer>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "drag_icon")]
        pub drag_icon: TemplateChild<gtk::Image>,

        #[template_child(id = "playing_icon")]
        pub playing_icon: TemplateChild<gtk::Image>,

        #[template_child(id = "track_title_label")]
        pub track_title_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label")]
        pub album_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "delete_button")]
        pub delete_button: TemplateChild<gtk::Button>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track: RefCell<Option<Rc<Track>>>,
        pub edit_mode: Cell<bool>,
        pub playlist_position: Cell<u64>,

        pub grab_x: Cell<f64>,
        pub grab_y: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QueueTrackPriv {
        const NAME: &'static str = "QueueTrack";
        type Type = super::QueueTrack;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }

    }

    impl ObjectImpl for QueueTrackPriv {
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

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("example-signal").build(),
                ]
            });

            SIGNALS.as_ref()
        }

    }

    impl WidgetImpl for QueueTrackPriv {}
    impl ListBoxRowImpl for QueueTrackPriv {}
    impl QueueTrackPriv {}
}

glib::wrapper! {
    pub struct QueueTrack(ObjectSubclass<imp::QueueTrackPriv>)
    @extends gtk::Widget, gtk::ListBoxRow;
}


impl QueueTrack {
    pub fn new() -> QueueTrack {
        let queue_track: QueueTrack = glib::Object::builder::<QueueTrack>().build();
        queue_track
    }

    fn initialize(&self) {
        let imp = self.imp();

        imp.popover.set_parent(self);

        imp.delete_button.connect_clicked(
            clone!(@strong self as this => move |_button| {
                player().queue().remove_track(this.playlist_position() as usize);
            })
        );

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

                drop_value.read_value_async(u64::static_type(), glib::PRIORITY_DEFAULT, None::<&gio::Cancellable>, 
                    clone!(@strong this => move |value| {

                        let old_position = value.unwrap().get::<u64>().ok().unwrap();
                        debug!("playlist_position {}", old_position);

                        player().queue().reorder_track(old_position as usize, this.playlist_position() as usize);

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
        //         debug!("unpaired release");
        //         if button == gdk::BUTTON_SECONDARY {
        //             imp.popover.popup();
        //         }
        //     })
        // );
        // self.add_controller(ctrl);
    }

    pub fn load_track_item(&self, track_item: TrackItem) {
        self.set_playlist_position(track_item.position());
        self.imp().track.replace(Some(track_item.track()));
        //self.create_menu(track_item.track());
        self.update_view();
    }


    pub fn update_view(&self) {
        let imp = self.imp();
        match imp.track.borrow().as_ref() {
            Some(track) => {
                self.set_tooltip_text(Some(format!("{} - {}", track.title(), track.artist()).as_str()));

                imp.track_title_label.set_label(track.title().as_str());
                imp.album_name_label.set_label(track.album().as_str());
                
                //LOAD COVER ART
                match track.cover_art_option() {
                    Some(id) => match self.load_image(id) {
                        Ok(art) => {
                            imp.art_bin.set_child(Some(&art));
                        }
                        Err(msg) => {
                            error!("Tried to set art, but: {}", msg);
                            imp.art_bin.set_child(gtk::Widget::NONE);
                        }
                    },
                    None => {
                        imp.art_bin.set_child(gtk::Widget::NONE);
                    }
                }
            }
            None => {
                return;
            },
        }
    }

    pub fn update_edit_ui(&self) {
        let imp = self.imp();

        let edit_mode = imp.edit_mode.get();

        if edit_mode {
            self.add_css_class("frame");
        } else {
            self.remove_css_class("frame");
        }

        imp.drag_icon_revealer.set_reveal_child(edit_mode);
        imp.delete_button_revealer.set_reveal_child(edit_mode);
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let art = RoundedAlbumArt::new(50);
        match model().cover_art(cover_art_id) {
            Ok(cover_art) => {
                match cover_art.pixbuf() {
                    Ok(pixbuf) => {
                        art.load(pixbuf);
                        return Ok(art);
                        //this is where i should add the connection closure if i was multithreading
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


    pub fn set_playlist_position(&self, position: u64) {
        self.imp().playlist_position.set(position);
    }

    pub fn playlist_position(&self) -> u64 {
        self.imp().playlist_position.get()
    }

    pub fn is_playing(&self, playing: bool) {
        if playing {
            self.set_css_classes(&[&"", &"activatable", &"frame"]);
        } else {
            self.set_css_classes(&[&"", &"activatable"]);
        }
        self.imp().playing_icon.set_visible(playing);
    }

    #[allow(dead_code)]
    fn create_menu(&self, track: Rc<Track>) {
        let imp = self.imp();
    
        let main = gio::Menu::new();
        let menu = gio::Menu::new();
    
        let menu_item = gio::MenuItem::new(Some(&format!("Skip to «{}» (pos {})", track.title(), imp.playlist_position.get())), None);
        menu_item.set_action_and_target_value(Some("win.skip-queue-to-track"), Some(&imp.playlist_position.get().to_variant()));
        menu.append_item(&menu_item);
    
        let menu_item = gio::MenuItem::new(Some(&format!("Remove «{}» from Queue (pos {})", track.title(), imp.playlist_position.get())), None);
        menu_item.set_action_and_target_value(Some("win.remove-track-from-queue"), Some(&imp.playlist_position.get().to_variant()));
        menu.append_item(&menu_item);
    
    
        main.append_section(Some("Queue"), &menu);
    
    
        let menu = gio::Menu::new();
    
        let menu_item = gio::MenuItem::new(Some(&format!("Go to Album «{}» Detail", track.album())), None);
        menu_item.set_action_and_target_value(Some("win.go-to-album-detail"), Some(&track.album_id().to_variant()));
        menu.append_item(&menu_item);
    
        let menu_item = gio::MenuItem::new(Some(&format!("Go to Artist {} Detail", track.artist())), None);
        menu_item.set_action_and_target_value(Some("win.go-to-artist-detail"), Some(&track.artist_id().to_variant()));
        menu.append_item(&menu_item);
    
        main.append_section(Some("Navigate"), &menu);
    
        let menu = gio::Menu::new();
        
        let menu_item = gio::MenuItem::new(Some(&format!("Create Playlist from «{}»", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.create-playlist-from-track"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);
    
        let menu_item = gio::MenuItem::new(Some(&format!("Add «{}» to Playlist", track.title())), None);
        menu_item.set_action_and_target_value(Some("win.add-track-to-playlist"), Some(&track.id().to_variant()));
        menu.append_item(&menu_item);
    
        main.append_section(Some("Playlist"), &menu);

        imp.popover.set_menu_model(Some(&main));
    }
}
    