use std::{cmp::Reverse, collections::HashMap};

use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use futures_util::pin_mut;
use itertools::Itertools;
use rspotify::{
    model::{
        AlbumId, ArtistId, FullAlbum, FullTrack, PlayableItem, PlaylistId, SavedAlbum, SavedTrack,
        TrackId,
    },
    prelude::*,
    AuthCodeSpotify,
};

use crate::modules::conversion;

use super::storage::LibraryDatabase;

pub async fn recently_added_albums(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    max_songs: Option<u32>,
) -> Vec<SavedAlbum> {
    let stream = spotify.current_user_saved_albums(None);

    pin_mut!(stream);

    let mut recent_albums = Vec::new();
    let mut num_songs = 0;
    let current_albums = library.retrieve_albums();
    // Loops until we have hit the max number of songs or have added all new_songs
    let mut names = Vec::new();
    while let Some(item) = stream.try_next().await.unwrap() {
        let id = item.album.id.id();
        let name = item.album.name.clone();
        if num_songs < max_songs.unwrap_or(0) || !current_albums.contains_key(id) {
            num_songs += item.album.tracks.total;
            recent_albums.push(item);
        } else {
            break;
        }
        names.push(name);
    }

    for album in &recent_albums {
        println!("Album: {:?}", album.album.name);
    }
    // Storage function
    recent_albums
}

pub async fn playlist_items(
    spotify: &AuthCodeSpotify,
    playlist_id: PlaylistId<'_>,
) -> Vec<FullTrack> {
    let stream = spotify.playlist_items(playlist_id, None, None);

    pin_mut!(stream);

    let mut tracks = Vec::new();
    while let Some(item) = stream.try_next().await.unwrap() {
        if let Some(PlayableItem::Track(track)) = item.track {
            tracks.push(track);
        }
    }
    tracks
}

// This function could probs be split up
// This needs to be refactored
pub async fn recently_added_tracks(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    max_songs: Option<u32>,
) -> Vec<SavedTrack> {
    let recent_albums = recently_added_albums(spotify, library, max_songs).await;
    let recent_album_tracks = conversion::albums_to_tracks(
        spotify,
        recent_albums
            .iter()
            .map(|album| album.album.clone())
            .collect_vec(),
    )
    .await;

    let mut album_to_time = HashMap::new();
    for album in recent_albums {
        album_to_time.insert(album.album.id.id().to_owned(), album.added_at);
    }

    let liked_tracks = recently_liked_tracks(spotify, library, album_to_time.values().min()).await;

    let mut recent_album_tracks = recent_album_tracks
        .into_iter()
        .map(|track| SavedTrack {
            track: track.clone(),
            added_at: album_to_time
                .get(track.album.id.unwrap().id())
                .unwrap()
                .to_owned(),
        })
        .collect_vec();

    recent_album_tracks = arrange_recent_tracks(recent_album_tracks);
    // for track in &recent_album_tracks {
    // println!("Track: {:?}", track.track.name);
    // }
    let full_track_list = recent_album_tracks
        .iter()
        .map(|track| track.track.clone())
        .collect_vec();
    for track in liked_tracks {
        if !full_track_list.contains(&track.track) {
            recent_album_tracks.push(track.clone());
        }
    }
    recent_album_tracks.sort_by_key(|saved_track| Reverse(saved_track.added_at));
    recent_album_tracks
}

fn arrange_recent_tracks(tracks: Vec<SavedTrack>) -> Vec<SavedTrack> {
    let mut tracks = tracks;
    tracks.sort_by_key(|saved_track| saved_track.added_at);
    let albums_from_tracks = tracks
        .iter()
        .map(|saved_track| {
            saved_track
                .track
                .album
                .id
                .as_ref()
                .unwrap()
                .id()
                .to_string()
        })
        .unique()
        .rev()
        .collect_vec();

    let mut albums_to_tracks: HashMap<String, Vec<SavedTrack>> = HashMap::new();
    for track in tracks {
        albums_to_tracks
            .entry(track.track.album.id.as_ref().unwrap().id().to_string())
            .or_insert(Vec::new())
            .push(track);
    }

    let mut sorted_tracks = Vec::new();
    for album in albums_from_tracks {
        let tracks = albums_to_tracks.get_mut(&album).unwrap();
        sorted_tracks.append(tracks.as_mut());
    }

    sorted_tracks
}

pub async fn recently_liked_tracks(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    latest_time: Option<&DateTime<Utc>>,
) -> Vec<SavedTrack> {
    let stream = spotify.current_user_saved_tracks(None);
    pin_mut!(stream);

    let current_tracks = library.retrieve_tracks();
    let mut liked_tracks = Vec::new();
    while let Some(item) = stream.try_next().await.unwrap() {
        if item.track.is_local {
            continue;
        } else {
            let id = item.track.id.as_ref().unwrap().id();

            if !current_tracks.contains_key(id.clone())
                || item.added_at > latest_time.unwrap_or(&Utc::now()).to_owned()
            {
                liked_tracks.push(item);
            } else {
                break;
            }
        }
    }

    liked_tracks
}

pub async fn get_all_albums(spotify: &AuthCodeSpotify) -> Vec<FullAlbum> {
    let stream = spotify.current_user_saved_albums(None);
    pin_mut!(stream);

    let mut albums = Vec::new();
    while let Some(item) = stream.try_next().await.unwrap() {
        albums.push(item.album);
    }
    albums
}

pub async fn get_all_tracks(spotify: &AuthCodeSpotify) -> Vec<FullTrack> {
    let stream = spotify.current_user_saved_tracks(None);
    pin_mut!(stream);

    let mut tracks = Vec::new();
    while let Some(item) = stream.try_next().await.unwrap() {
        if item.track.is_local {
            continue;
        } else {
            tracks.push(item.track);
        }
    }

    let mut all_tracks = conversion::albums_to_tracks(spotify, get_all_albums(spotify).await).await;
    tracks.append(all_tracks.as_mut());
    tracks
}

pub async fn print_album(spotify: &AuthCodeSpotify, album: &str) {
    let album = spotify
        .album(AlbumId::from_id(album).unwrap())
        .await
        .unwrap();

    println!("{:?}", album);
}
pub async fn print_artist(spotify: &AuthCodeSpotify, artist: &str) {
    let artist = spotify
        .artist(ArtistId::from_id(artist).unwrap())
        .await
        .unwrap();

    println!("{:?}", artist);
}

pub async fn print_track(spotify: &AuthCodeSpotify, track: &str) {
    let track = spotify
        .track(TrackId::from_id(track).unwrap())
        .await
        .unwrap();

    println!("{:?}", track);
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[actix_rt::test]
//     async fn test_recently_added_albums() {
//         let spotify = authcode(&creds, &oauth, &config).await.unwrap();

//         if spotify.current_user_saved_albums_contains("4aawyAB9vmqN3uQ7FjRGTy") {
//             spotify
//                 .current_user_saved_albums_delete("4aawyAB9vmqN3uQ7FjRGTy")
//                 .await
//                 .unwrap();
//         }

//         spotify
//             .current_user_saved_albums_add("4aawyAB9vmqN3uQ7FjRGTy")
//             .await
//             .unwrap();

//         let recent_albums = recently_added_albums(&spotify, 18).await;

//         if spotify.current_user_saved_albums_contains("4aawyAB9vmqN3uQ7FjRGTy") {
//             spotify
//                 .current_user_saved_albums_delete("4aawyAB9vmqN3uQ7FjRGTy")
//                 .await
//                 .unwrap();
//         }

//         assert(recent_albums.contains_key("4aawyAB9vmqN3uQ7FjRGTy"));

//         let album = recent_albums.get("4aawyAB9vmqN3uQ7FjRGTy").unwrap();
//         assert_eq!(len(album.tracks.items), 18);
//     }

//     #[actix_rt::test]
//     async fn test_recently_added_tracks() {
//         let spotify = authcode(&creds, &oauth, &config).await.unwrap();

//         if spotify.current_user_saved_tracks_contains("0xh2kZQ3rDB47IS8aVNqrf") {
//             spotify
//                 .current_user_saved_tracks_delete("0xh2kZQ3rDB47IS8aVNqrf")
//                 .await
//                 .unwrap();
//         }

//         spotify
//             .current_user_saved_tracks_add("0xh2kZQ3rDB47IS8aVNqrf")
//             .await
//             .unwrap();

//         let recent_tracks = recently_added_tracks(&spotify).await;

//         if spotify.current_user_saved_tracks_contains("0xh2kZQ3rDB47IS8aVNqrf") {
//             spotify
//                 .current_user_saved_tracks_delete("0xh2kZQ3rDB47IS8aVNqrf")
//                 .await
//                 .unwrap();
//         }

//         assert(recent_tracks.contains_key("0xh2kZQ3rDB47IS8aVNqrf"));
//     }
// }
