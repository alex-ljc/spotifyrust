use itertools::Itertools;
use rspotify::{
    model::{FullTrack, PlaylistId},
    prelude::*,
    AuthCodeSpotify,
};

use super::{
    conversion::{id_to_playable_ids, saved_tracks_to_tracks},
    retrieve::{get_all_tracks, recently_added_tracks, recently_liked_tracks},
    storage::LibraryDatabase,
};
use rand::{seq::IteratorRandom, thread_rng};

pub async fn update_recently_added(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    num_songs: u32,
) {
    println!("Updating recently added");
    let recent_tracks =
        saved_tracks_to_tracks(recently_added_tracks(spotify, library, Some(num_songs)).await);

    clear_playlist(spotify, playlist_id).await;

    add_tracks_to_playlist(spotify, playlist_id, recent_tracks).await;
}

pub async fn update_everything(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    num_recent_songs: u32,
    num_total_songs: usize,
) {
    clear_playlist(spotify, playlist_id).await;
    let mut recent_tracks = saved_tracks_to_tracks(
        recently_added_tracks(spotify, library, Some(num_recent_songs)).await,
    );
    let mut rng = thread_rng();
    let mut all_tracks = library
        .retrieve_tracks()
        .into_values()
        .choose_multiple(&mut rng, num_total_songs);
    recent_tracks.append(all_tracks.as_mut());
    add_tracks_to_playlist(spotify, playlist_id, recent_tracks).await;
}

pub async fn update_weekly_sample(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    num_songs: usize,
) {
    clear_playlist(spotify, playlist_id).await;
    let mut rng = thread_rng();
    let all_tracks = library
        .retrieve_tracks()
        .into_values()
        .choose_multiple(&mut rng, num_songs);
    add_tracks_to_playlist(spotify, playlist_id, all_tracks).await;
}

pub async fn clear_playlist(spotify: &AuthCodeSpotify, playlist_id: &str) {
    spotify
        .playlist_replace_items(PlaylistId::from_id(playlist_id).unwrap(), [])
        .await
        .unwrap();
}

pub async fn update_liked(spotify: &AuthCodeSpotify, library: &LibraryDatabase, playlist_id: &str) {
    // I need to think of a more elegant solution than doing 600 hard coded
    let liked_tracks = library.retrieve_tracks().into_values().collect();
    clear_playlist(spotify, playlist_id).await;
    add_tracks_to_playlist(spotify, playlist_id, liked_tracks).await;
}

// Adds tracks to playlist and ensures there are no duplicats in the tracks added
// Is it bad design decision for this function to call unique? Probably but fuck it
pub async fn add_tracks_to_playlist(
    spotify: &AuthCodeSpotify,
    playlist_id: &str,
    recent_tracks: Vec<FullTrack>,
) {
    let recent_tracks = recent_tracks
        .iter()
        .map(|track| track.id.as_ref().unwrap().clone())
        .unique()
        .collect_vec();
    let paginated_tracks = paginate_vec(recent_tracks, 99);

    for tracks in paginated_tracks {
        spotify
            .playlist_add_items(
                PlaylistId::from_id(playlist_id).unwrap(),
                id_to_playable_ids(tracks).into_iter(),
                None,
            )
            .await
            .unwrap();
    }
}

fn paginate_vec<T>(vec: Vec<T>, page_size: usize) -> Vec<Vec<T>> {
    let mut pages = Vec::new();
    let mut page = Vec::new();
    for item in vec {
        page.push(item);
        if page.len() == page_size {
            pages.push(page);
            page = Vec::new();
        }
    }
    if !page.is_empty() {
        pages.push(page);
    }
    pages
}

// fn playlist_filter_songs
