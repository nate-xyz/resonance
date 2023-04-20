/* search.rs
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchSortObject {
    Album,
    Track,
    QueueTrack,
    PlaylistTrack,
    Playlist,
    Artist,
    Genre,
}

impl Default for SearchSortObject {
    fn default() -> Self {
        Self::Album
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchMethod {
    Full,
    Track,
    Album,
    Artist,
    Genre,
    ReleaseDate,
}

impl Default for SearchMethod {
    fn default() -> Self {
        Self::Full
    }
}

mod imp {
    use super::*;

    use std::cell::RefCell;
    use std::cell::Cell;

    use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
    use gtk::glib::{self, ParamSpec, ParamSpecString, Value};
    use once_cell::sync::Lazy;

    use crate::model::album::Album;
    use crate::model::track::Track;
    use crate::model::playlist::Playlist;
    use crate::model::artist::Artist;
    use crate::model::genre::Genre;
    use crate::views::pages::queue::track_item::TrackItem;
    use crate::views::pages::playlists::track_item::PlaylistDetailTrackItem;


    #[derive(Default)]
    pub struct FuzzyFilter {
        pub search: RefCell<Option<String>>,
        pub type_: Cell<SearchSortObject>,
        pub method: Cell<SearchMethod>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FuzzyFilter {
        const NAME: &'static str = "ResonanceFuzzyFilter";
        type Type = super::FuzzyFilter;
        type ParentType = gtk::Filter;
    }

    impl ObjectImpl for FuzzyFilter {
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

    impl FilterImpl for FuzzyFilter {
        fn strictness(&self) -> gtk::FilterMatch {
            gtk::FilterMatch::Some
        }

        fn match_(&self, search_obj: &glib::Object) -> bool {
            // let song = song.downcast_ref::<Album>().unwrap();

            let search_key = match self.type_.get() {
                SearchSortObject::Album => {
                    let album = search_obj.downcast_ref::<Album>().unwrap();
                    match self.method.get() {
                        SearchMethod::Full => album.search_string(),
                        SearchMethod::Track => album.title(),
                        SearchMethod::Album => album.title(),
                        SearchMethod::Artist => album.artist(),
                        SearchMethod::Genre => album.genre(),
                        SearchMethod::ReleaseDate => album.date(),
                    }
                },
                SearchSortObject::Track => {
                    let track = search_obj.downcast_ref::<Track>().unwrap();
                    match self.method.get() {
                        SearchMethod::Full => track.search_string(),
                        SearchMethod::Track => track.title(),
                        SearchMethod::Album => track.album(),
                        SearchMethod::Artist => track.artist(),
                        SearchMethod::Genre => track.genre(),
                        SearchMethod::ReleaseDate => track.date(),
                    }
                },
                SearchSortObject::QueueTrack => {
                    let track = search_obj.downcast_ref::<TrackItem>().unwrap();
                    track.search_string()
                },
                SearchSortObject::PlaylistTrack => {
                    let track = search_obj.downcast_ref::<PlaylistDetailTrackItem>().unwrap();
                    track.search_string()
                },
                SearchSortObject::Playlist => {
                    let playlist = search_obj.downcast_ref::<Playlist>().unwrap();
                    playlist.title()
                },
                SearchSortObject::Artist => {
                    let artist = search_obj.downcast_ref::<Artist>().unwrap();
                    artist.name()
                },
                SearchSortObject::Genre => {
                    let genre = search_obj.downcast_ref::<Genre>().unwrap();
                    genre.name()
                },

            };

            if let Some(search) = self.search.borrow().as_ref() {
                let matcher = SkimMatcherV2::default();
                matcher.fuzzy_match(&search_key, search).is_some() || search.is_empty()
            } else {
                true
            }
        }
    }
}

glib::wrapper! {
    pub struct FuzzyFilter(ObjectSubclass<imp::FuzzyFilter>)
    @extends gtk::Filter;
}

impl FuzzyFilter {
    pub fn new(type_: SearchSortObject) -> Self {
        let filter = glib::Object::builder::<FuzzyFilter>().build();
        filter.set_type(type_);
        filter
    }

    fn set_type(&self, type_: SearchSortObject) {
        self.imp().type_.set(type_)
    }

    pub fn set_method(&self, method: SearchMethod) {
        self.imp().method.set(method)
    }

    pub fn search(&self) -> Option<String> {
        self.imp().search.borrow().as_ref().map(ToString::to_string)
    }

    pub fn set_search(&self, search: Option<String>) {
        let imp = self.imp();
        if &*imp.search.borrow() != &search {
            *imp.search.borrow_mut() = search.map(|x| x.to_lowercase());
            self.changed(gtk::FilterChange::Different);
        }
    }
}
