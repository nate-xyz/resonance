/* sort.rs
 *
 * Copyright 2023 nate-xyz
 *
 * Thanks to 2022 John Toohey <john_t@mailo.com>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gtk::{glib, prelude::*, subclass::prelude::*};
use log::debug;
use super::search::SearchSortObject;

#[derive(Debug, Clone, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "SortMethod")]
pub enum SortMethod {
    Track,
    Album,
    Artist,
    Genre,
    Playlist,
    ReleaseDate,
    Duration,
    TrackCount,
    AlbumCount,
    LastModified,
}

impl Default for SortMethod {
    fn default() -> Self {
        Self::Album
    }
}

mod imp {
    use super::*;
    use gtk::glib::{self, ParamSpec, ParamSpecString, Value};

    use std::cell::{RefCell, Cell};
    use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
    use once_cell::sync::Lazy;

    use crate::model::{
        album::Album,
        track::Track,
        playlist::Playlist,
        artist::Artist,
        genre::Genre,
    };

    #[derive(Default)]
    pub struct FuzzySorter {
        pub search: RefCell<Option<String>>,
        pub type_: Cell<SearchSortObject>,
        pub method: Cell<SortMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FuzzySorter {
        const NAME: &'static str = "ResonanceFuzzySorter";
        type Type = super::FuzzySorter;
        type ParentType = gtk::Sorter;
    }

    impl ObjectImpl for FuzzySorter {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecString::builder("search").build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "search" => {
                    let p = value
                        .get::<Option<String>>()
                        .expect("Value must be a string");
                    self.obj().set_search(p);
                }
                _ => unimplemented!(),
            }
        }
    }

    impl SorterImpl for FuzzySorter {
        fn compare(&self, item1: &glib::Object, item2: &glib::Object) -> gtk::Ordering {
            match self.method.get() {
                SortMethod::Duration => {
                    let (item1_key, item2_key) = match self.type_.get() {
                        SearchSortObject::Album => {
                            let item1 = item1.downcast_ref::<Album>().unwrap();
                            let item2 = item2.downcast_ref::<Album>().unwrap();
        
                            (item1.duration(), item2.duration())
                        },
                        SearchSortObject::Track => {
                            let item1 = item1.downcast_ref::<Track>().unwrap();
                            let item2 = item2.downcast_ref::<Track>().unwrap();
        
                            (item1.duration(), item2.duration())
                        },
                        SearchSortObject::Playlist => {
                            let item1 = item1.downcast_ref::<Playlist>().unwrap();
                            let item2 = item2.downcast_ref::<Playlist>().unwrap();
        
                            (item1.duration(), item2.duration())
                        },
                        _ => unimplemented!("no sorting for")
        
                    };
        
                    if item1_key < item2_key {
                        gtk::Ordering::Larger
                    } else if item1_key > item2_key  {
                        gtk::Ordering::Smaller
                    } else {
                        gtk::Ordering::Equal
                    }
                },
                SortMethod::TrackCount => {
                    let (item1_key, item2_key) = match self.type_.get() {
                        SearchSortObject::Album => {
                            let item1 = item1.downcast_ref::<Album>().unwrap();
                            let item2 = item2.downcast_ref::<Album>().unwrap();
        
                            (item1.n_tracks(), item2.n_tracks())
                        },
                        SearchSortObject::Playlist => {
                            let item1 = item1.downcast_ref::<Playlist>().unwrap();
                            let item2 = item2.downcast_ref::<Playlist>().unwrap();
        
                            (item1.n_tracks(), item2.n_tracks())
                        },
                        SearchSortObject::Artist => {
                            let item1 = item1.downcast_ref::<Artist>().unwrap();
                            let item2 = item2.downcast_ref::<Artist>().unwrap();
        
                            (item1.n_tracks(), item2.n_tracks())
                        },
                        SearchSortObject::Genre => {
                            let item1 = item1.downcast_ref::<Genre>().unwrap();
                            let item2 = item2.downcast_ref::<Genre>().unwrap();
        
                            (item1.n_tracks(), item2.n_tracks())
                        },
                        _ => unimplemented!("no sorting for")
        
                    };
                
                    if item1_key < item2_key {
                        gtk::Ordering::Larger
                    } else if item1_key > item2_key  {
                        gtk::Ordering::Smaller
                    } else {
                        gtk::Ordering::Equal
                    }
                },
                SortMethod::AlbumCount => {
                    let (item1_key, item2_key) = match self.type_.get() {
                        SearchSortObject::Artist => {
                            let item1 = item1.downcast_ref::<Artist>().unwrap();
                            let item2 = item2.downcast_ref::<Artist>().unwrap();
        
                            (item1.n_albums(), item2.n_albums())
                        },
                        SearchSortObject::Genre => {
                            let item1 = item1.downcast_ref::<Genre>().unwrap();
                            let item2 = item2.downcast_ref::<Genre>().unwrap();
        
                            (item1.n_albums(), item2.n_albums())
                        },
                        _ => unimplemented!("no sorting for")
        
                    };
              
                    if item1_key < item2_key {
                        gtk::Ordering::Larger
                    } else if item1_key > item2_key  {
                        gtk::Ordering::Smaller
                    } else {
                        gtk::Ordering::Equal
                    }
                },
                SortMethod::LastModified => {
                    let (item1_key, item2_key) = match self.type_.get() {
                        SearchSortObject::Playlist => {
                            let item1 = item1.downcast_ref::<Playlist>().unwrap();
                            let item2 = item2.downcast_ref::<Playlist>().unwrap();
        
                            (item1.modify_time(), item2.modify_time())
                        },
                        _ => unimplemented!("no sorting for")
        
                    };
               
                    if item1_key > item2_key {
                        gtk::Ordering::Larger
                    } else if item1_key < item2_key  {
                        gtk::Ordering::Smaller
                    } else {
                        gtk::Ordering::Equal
                    }
                },
                _ => {
                    let (item1_key, item2_key) = match self.type_.get() {
                        SearchSortObject::Album => {
                            let item1 = item1.downcast_ref::<Album>().unwrap();
                            let item2 = item2.downcast_ref::<Album>().unwrap();
        
                            match self.method.get() {
                                SortMethod::Album => (item1.sort_title(), item2.sort_title()),
                                SortMethod::Artist => (item1.sort_artist(), item2.sort_artist()),
                                SortMethod::Genre => (item1.genre(), item2.genre()),
                                SortMethod::ReleaseDate => (item1.date(), item2.date()),
                                _ => (item1.sort_string(), item2.sort_string()),
                            }
                        },
                        SearchSortObject::Track => {
                            let item1 = item1.downcast_ref::<Track>().unwrap();
                            let item2 = item2.downcast_ref::<Track>().unwrap();
        
                            match self.method.get() {
                                SortMethod::Track => (item1.sort_title(), item2.sort_title()),
                                SortMethod::Album => (item1.sort_album(), item2.sort_album()),
                                SortMethod::Artist => (item1.sort_artist(), item2.sort_artist()),
                                SortMethod::Genre => (item1.genre(), item2.genre()),
                                SortMethod::ReleaseDate => (item1.date(), item2.date()),
                                _ => (item1.sort_string(), item2.sort_string()),
                            }
                        },
                        SearchSortObject::Playlist => {
                            let item1 = item1.downcast_ref::<Playlist>().unwrap();
                            let item2 = item2.downcast_ref::<Playlist>().unwrap();
        
                            (item1.title(), item2.title())
                        },
                        SearchSortObject::Artist => {
                            let item1 = item1.downcast_ref::<Artist>().unwrap();
                            let item2 = item2.downcast_ref::<Artist>().unwrap();
        
                            (item1.sort_name(), item2.sort_name())
                        },
                        SearchSortObject::Genre => {
                            let item1 = item1.downcast_ref::<Genre>().unwrap();
                            let item2 = item2.downcast_ref::<Genre>().unwrap();
        
                            (item1.sort_name(), item2.sort_name())
                        },
                        _ => unimplemented!("no sorting for")
        
                    };
        
                    if item1_key.is_empty() {
                        return gtk::Ordering::Smaller;
                    }
        
                    if item2_key.is_empty() {
                        return gtk::Ordering::Larger;
                    }

                    if let Some(search) = self.search.borrow().as_ref() {
                        if !search.is_empty() {
                            debug!("search string: {}", search);
                            let matcher = SkimMatcherV2::default();
                            let item1_score = matcher.fuzzy_match(&item1_key, search);
                            let item2_score = matcher.fuzzy_match(&item2_key, search);
                            let order = item1_score.cmp(&item2_score).reverse();
        
                            if order != std::cmp::Ordering::Equal {
                                return order.into()
                            } 
                        }
                    } 
        
                    if item1_key > item2_key {
                        gtk::Ordering::Larger
                    } else if item1_key < item2_key  {
                        gtk::Ordering::Smaller
                    } else {
                        gtk::Ordering::Equal
                    }
                }
            }

        }

        fn order(&self) -> gtk::SorterOrder {
            gtk::SorterOrder::Partial
        }
    }
}

glib::wrapper! {
    pub struct FuzzySorter(ObjectSubclass<imp::FuzzySorter>)
    @extends gtk::Sorter;
}

impl FuzzySorter {
    pub fn new(type_: SearchSortObject) -> Self {
        let sorter = glib::Object::builder::<FuzzySorter>().build();
        sorter.set_type(type_);
        sorter
    }

    fn set_type(&self, type_: SearchSortObject) {
        self.imp().type_.set(type_);
    }

    pub fn set_method(&self, method: SortMethod) {
        self.imp().method.set(method);
        self.changed(gtk::SorterChange::Different);
    }

    pub fn search(&self) -> Option<String> {
        self.imp().search.borrow().as_ref().map(ToString::to_string)
    }

    pub fn set_search(&self, search: Option<String>) {
        if &*self.imp().search.borrow() != &search {
            *self.imp().search.borrow_mut() = search;
            self.changed(gtk::SorterChange::Different);
        }
    }
}
