use itertools::Itertools;
use rspotify::{
    model::{FullAlbum, FullTrack, TrackId},
    prelude::*,
    AuthCodeSpotify,
};
use serde::Deserialize;
use std::{collections::HashMap, fs};

use super::{
    conversion::{saved_albums_to_albums, saved_tracks_to_tracks},
    retrieve,
};

pub struct LibraryDatabase {
    album_path: String,
    track_path: String,
}

impl LibraryDatabase {
    pub fn new(album_path: String, track_path: String) -> LibraryDatabase {
        Self {
            album_path,
            track_path,
        }
    }

    fn store_hashmap<T>(map: &HashMap<String, T>, filename: &str)
    where
        T: serde::Serialize,
    {
        let serialized = serde_json::to_string(map).unwrap();
        fs::write(filename, serialized).unwrap();
    }

    fn load_hashmap<T>(filename: &str) -> HashMap<String, T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let path = std::path::Path::new(filename);
        if !path.exists() {
            return HashMap::new();
        }

        let contents = fs::read_to_string(filename).unwrap();
        let deserialized = serde_json::from_str::<HashMap<String, T>>(&contents).unwrap();
        deserialized
    }

    fn update_tracks(&self, tracks: Vec<FullTrack>) {
        // These should probably be env variables
        let mut current_tracks = self.retrieve_tracks();
        for track in tracks {
            if !track.is_local {
                current_tracks.insert(track.id.clone().unwrap().id().to_string(), track);
            }
        }

        Self::store_hashmap(&current_tracks, &self.track_path)
    }

    fn update_albums(&self, albums: Vec<FullAlbum>) {
        let mut current_albums = self.retrieve_albums();
        for album in albums {
            current_albums.insert(album.id.id().to_string(), album);
        }

        Self::store_hashmap(&current_albums, &self.album_path);
    }

    pub async fn update_all(&self, spotify: &AuthCodeSpotify) {
        self.update_tracks(saved_tracks_to_tracks(
            retrieve::recently_added_tracks(spotify, self, None).await,
        ));
        self.update_albums(saved_albums_to_albums(
            retrieve::recently_added_albums(spotify, self, None).await,
        ));
    }

    pub fn retrieve_albums(&self) -> HashMap<String, FullAlbum> {
        Self::load_hashmap::<FullAlbum>(&self.album_path)
    }

    pub fn retrieve_tracks(&self) -> HashMap<String, FullTrack> {
        Self::load_hashmap::<FullTrack>(&self.track_path)
    }

    pub fn compile_genres(albums: Vec<FullAlbum>) -> HashMap<String, Vec<TrackId<'static>>> {
        let mut genres = HashMap::new();
        for album in albums {
            println!("{:?}", album.genres);
            let track_ids = album
                .tracks
                .items
                .iter()
                .map(|track| track.id.as_ref().unwrap().to_owned())
                .collect_vec();
            for genre in album.genres {
                println!("{}: {:?}", genre, track_ids);
                genres
                    .entry(genre)
                    .or_insert(Vec::new())
                    .append(track_ids.clone().as_mut());
            }
        }
        genres
    }

    pub fn search_songs(&self, query: &str) -> Vec<String> {
        let tracks = Self::load_hashmap::<FullTrack>(&self.track_path);
        let mut results = Vec::new();
        for (id, track) in tracks.into_iter() {
            let string_track = track.name
                + " "
                + &track
                    .artists
                    .into_iter()
                    .map(|artist| artist.name)
                    .join(" ")
                + " "
                + &track.album.name;
            if string_track.to_lowercase().contains(&query.to_lowercase()) {
                results.push(id.clone());
            }
        }
        results
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use rspotify::{
//         model::ArtistId, prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth,
//     };

//     #[actix_rt::test]
//     async fn test_update_albums() {
//         let creds = Credentials::from_env().unwrap();
//         let oauth = OAuth::from_env(scopes!("user-library-read")).unwrap();
//         let config = Config::default();
//         let spotify = authcode(&creds, &oauth, &config).await.unwrap();
//         let recent_albums = recently_added_albums(&spotify, 10).await;
//         assert_eq!(recent_albums.len(), 10);
//     }
// }
