use futures::stream::TryStreamExt;
use futures_util::pin_mut;

use itertools::Itertools;
use rspotify::{
    model::{FullTrack, PlayableItem, PlaylistId},
    prelude::*,
    AuthCodeSpotify,
};

use super::{
    conversion::{id_to_playable_ids, saved_tracks_to_tracks, track_ids_to_tracks},
    retrieve::recently_added_tracks,
    storage::LibraryDatabase,
};
use rand::{seq::IteratorRandom, thread_rng};

pub async fn update_recently_added(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    num_songs: usize,
) {
    println!("Updating recently added");
    let recent_tracks =
        saved_tracks_to_tracks(recently_added_tracks(spotify, library, Some(num_songs)).await);
    let playlist_tracks = get_playlist_tracks(spotify, &PlaylistId::from_id(playlist_id).unwrap())
        .await
        .into_iter()
        .map(|track| track.id)
        .collect::<Vec<_>>();

    let mut new_tracks_to_add = Vec::new();
    let mut old_tracks_to_add = Vec::new();
    let mut is_new = true;
    for track in recent_tracks {
        if !playlist_tracks.contains(&track.id) && is_new {
            println!(
                "Adding {} - {}",
                track.artists.get(0).unwrap().name,
                track.name
            );
            new_tracks_to_add.push(track);
        } else if !playlist_tracks.contains(&track.id) && !is_new {
            old_tracks_to_add.push(track);
        } else if playlist_tracks.contains(&track.id) && is_new {
            is_new = false;
        }
    }

    add_tracks_to_playlist(spotify, playlist_id, new_tracks_to_add, Some(0)).await;
    add_tracks_to_playlist(spotify, playlist_id, old_tracks_to_add, None).await;
    remove_old_tracks_from_playlist(spotify, playlist_id, num_songs).await;
}

pub async fn add_new_tracks_to_playlist(
    spotify: &AuthCodeSpotify,
    playlist_id: &str,
    tracks_to_add: Vec<FullTrack>,
) {
    let playlist_tracks = get_playlist_tracks(spotify, &PlaylistId::from_id(playlist_id).unwrap())
        .await
        .into_iter()
        .map(|track| track.id)
        .collect::<Vec<_>>();

    let mut new_tracks_to_add = Vec::new();
    let mut old_tracks_to_add = Vec::new();
    let mut is_new = true;
    for track in tracks_to_add {
        if !playlist_tracks.contains(&track.id) && is_new {
            println!(
                "Adding {} - {}",
                track.artists.get(0).unwrap().name,
                track.name
            );
            new_tracks_to_add.push(track);
        } else if !playlist_tracks.contains(&track.id) && !is_new {
            old_tracks_to_add.push(track);
        } else if playlist_tracks.contains(&track.id) && is_new {
            is_new = false;
        }
    }

    add_tracks_to_playlist(spotify, playlist_id, new_tracks_to_add, Some(0)).await;
    add_tracks_to_playlist(spotify, playlist_id, old_tracks_to_add, None).await;
}

pub async fn remove_old_tracks_from_playlist(
    spotify: &AuthCodeSpotify,
    playlist_id: &str,
    num_songs: usize,
) {
    let playlist_id = PlaylistId::from_id(playlist_id).unwrap();
    let playlist_tracks = get_playlist_tracks(spotify, &playlist_id).await;

    if playlist_tracks.len() > num_songs {
        // This is such jank ass code
        let mut to_remove_tracks = playlist_tracks[num_songs - 1..].into_iter();
        let mut index_of_last_valid_song = num_songs - 1;

        let mut prev_track = playlist_tracks.get(index_of_last_valid_song).unwrap();
        while let Some(curr_track) = to_remove_tracks.next() {
            if prev_track.album.id != curr_track.album.id {
                break;
            }
            index_of_last_valid_song += 1;
            prev_track = curr_track;
        }

        let to_remove_tracks = &playlist_tracks[index_of_last_valid_song..];
        let to_remove_id = to_remove_tracks
            .iter()
            .map(|track| track.clone().id.unwrap())
            .collect::<Vec<_>>();
        let to_remove_playable_id = id_to_playable_ids(&to_remove_id);
        let to_remove_tracks = track_ids_to_tracks(&spotify, to_remove_id).await;
        for track in to_remove_tracks {
            println!(
                "Removing {} - {}",
                track.artists.get(0).unwrap().name,
                track.name
            )
        }

        let to_remove_chunks = paginate_vec(to_remove_playable_id, 99);

        for chunk in to_remove_chunks {
            spotify
                .playlist_remove_all_occurrences_of_items(
                    playlist_id.clone(),
                    chunk.into_iter(),
                    None,
                )
                .await
                .unwrap();
        }
    }
}

pub async fn update_everything(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    num_recent_songs: usize,
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
    add_tracks_to_playlist(spotify, playlist_id, recent_tracks, None).await;
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
    add_tracks_to_playlist(spotify, playlist_id, all_tracks, None).await;
}

pub async fn clear_playlist(spotify: &AuthCodeSpotify, playlist_id: &str) {
    spotify
        .playlist_replace_items(PlaylistId::from_id(playlist_id).unwrap(), [])
        .await
        .unwrap();
}

pub async fn update_liked(spotify: &AuthCodeSpotify, library: &LibraryDatabase, playlist_id: &str) {
    // I need to think of a more elegant solution than doing 600 hard coded
    let liked_tracks = library.retrieve_liked().into_values().collect();
    clear_playlist(spotify, playlist_id).await;
    add_tracks_to_playlist(spotify, playlist_id, liked_tracks, None).await;
}

pub async fn get_playlist_tracks(
    spotify: &AuthCodeSpotify,
    playlist_id: &PlaylistId<'_>,
) -> Vec<FullTrack> {
    let mut tracks = Vec::new();
    let stream = spotify.playlist_items(playlist_id.clone(), None, None);
    pin_mut!(stream);

    while let Some(item) = stream.try_next().await.unwrap() {
        if let Some(track) = item.track {
            if let PlayableItem::Track(track) = track {
                tracks.push(track);
            }
        }
    }

    tracks
}

pub async fn add_searched_tracks(
    spotify: &AuthCodeSpotify,
    library: &LibraryDatabase,
    playlist_id: &str,
    query: &str,
    print_tracks: bool,
) {
    let track_ids = library.search_songs(query);
    let stored_tracks = library.retrieve_tracks();
    let mut filtered_tracks = Vec::new();
    for track in track_ids {
        if print_tracks {
            println!(
                "Adding {} - {}",
                stored_tracks
                    .get(&track)
                    .unwrap()
                    .artists
                    .get(0)
                    .unwrap()
                    .name,
                stored_tracks.get(&track).unwrap().name
            );
        }
        filtered_tracks.push(stored_tracks.get(&track).unwrap().clone());
    }
    clear_playlist(&spotify, playlist_id).await;
    add_tracks_to_playlist(&spotify, playlist_id, filtered_tracks, None).await;
    let _ = spotify
        .playlist_change_detail(
            PlaylistId::from_id(playlist_id).unwrap(),
            Some(query),
            None,
            None,
            None,
        )
        .await;
}

// Adds tracks to playlist and ensures there are no duplicats in the tracks added
// Is it bad design decision for this function to call unique? Probably but fuck it
pub async fn add_tracks_to_playlist(
    spotify: &AuthCodeSpotify,
    playlist_id: &str,
    recent_tracks: Vec<FullTrack>,
    position: Option<i32>,
) {
    let recent_tracks = recent_tracks
        .iter()
        .map(|track| track.id.as_ref().unwrap().clone())
        .unique()
        .collect_vec();

    let paginated_tracks = paginate_vec(recent_tracks, 99);

    let mut count = 0;
    for tracks in paginated_tracks {
        // println!("Iteration {}", count);
        // for track in track_ids_to_tracks(spotify, tracks.clone()).await {
        //     println!("Order of tracks: {}", track.name)
        // }
        spotify
            .playlist_add_items(
                PlaylistId::from_id(playlist_id).unwrap(),
                id_to_playable_ids(&tracks).into_iter(),
                match position {
                    Some(index) => Some(index + count),
                    None => None,
                },
            )
            .await
            .unwrap();
        count += tracks.len() as i32;
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
