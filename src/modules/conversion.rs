use itertools::Itertools;
use rspotify::{
    model::{FullAlbum, FullTrack, SavedAlbum, SavedTrack, TrackId},
    prelude::*,
    AuthCodeSpotify,
};

pub async fn albums_to_tracks(
    spotify: &AuthCodeSpotify,
    current_albums: Vec<FullAlbum>,
) -> Vec<FullTrack> {
    let track_ids: Vec<TrackId> = current_albums
        .iter()
        .flat_map(|album| album.tracks.items.iter())
        .map(|track| track.id.as_ref().unwrap().clone())
        .collect();

    let mut tracks = Vec::new();
    for group in track_ids.chunks(50) {
        let mut current_tracks: Vec<FullTrack> = spotify
            .tracks(group.to_vec().into_iter(), None)
            .await
            .unwrap()
            .iter()
            .filter_map(|track| if !track.is_local { Some(track) } else { None })
            .cloned()
            .collect();
        tracks.append(current_tracks.as_mut());
    }
    tracks
}

pub fn tracks_to_ids(tracks: Vec<FullTrack>) -> Vec<TrackId<'static>> {
    let track_ids: Vec<TrackId> = tracks
        .iter()
        .map(|track| track.id.as_ref().unwrap())
        .cloned()
        .collect();
    track_ids
}

pub fn track_to_playable_ids(tracks: Vec<FullTrack>) -> Vec<PlayableId<'static>> {
    tracks_to_ids(tracks)
        .into_iter()
        .map(|track_id| PlayableId::Track(track_id))
        .collect::<Vec<PlayableId>>()
}

pub fn id_to_playable_ids<'a>(ids: Vec<TrackId<'a>>) -> Vec<PlayableId<'a>> {
    ids.into_iter()
        .map(|track_id| PlayableId::Track(track_id))
        .collect::<Vec<PlayableId>>()
}

// Man I hate this code duplication but wat can I do
pub fn saved_tracks_to_tracks(saved: Vec<SavedTrack>) -> Vec<FullTrack> {
    saved
        .into_iter()
        .map(|saved_track| saved_track.track)
        .collect_vec()
}

pub fn saved_albums_to_albums(saved: Vec<SavedAlbum>) -> Vec<FullAlbum> {
    saved
        .into_iter()
        .map(|saved_album| saved_album.album)
        .collect_vec()
}

pub async fn track_ids_to_tracks(
    spotify: &AuthCodeSpotify,
    track_ids: Vec<TrackId<'_>>,
) -> Vec<FullTrack> {
    let tracks = spotify
        .tracks(track_ids.into_iter(), None)
        .await
        .unwrap()
        .iter()
        .filter_map(|track| if !track.is_local { Some(track) } else { None })
        .cloned()
        .collect();
    tracks
}
