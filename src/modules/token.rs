use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth, Token};
use std::path::PathBuf;

pub async fn authcode(
    creds: &Credentials,
    oauth: &OAuth,
    config: &Config,
) -> Result<AuthCodeSpotify, ()> {
    let valid_token = generate_token(creds.clone(), oauth.clone(), config.cache_path.clone()).await;
    let mut spotify = AuthCodeSpotify::from_token(valid_token?);
    spotify.creds = creds.clone();
    spotify.oauth = oauth.clone();
    spotify.config = config.clone();
    Ok(spotify)
}

async fn generate_token(creds: Credentials, oauth: OAuth, path: PathBuf) -> Result<Token, ()> {
    // Doesn't check if valid creds, just if the token exists.
    match Token::from_cache(path.clone()) {
        Ok(token) => Ok(token),
        Err(_) => {
            let spotify = AuthCodeSpotify::new(creds.clone(), oauth.clone());
            let url = spotify.get_authorize_url(false).unwrap();
            // This function requires the `cli` feature enabled.
            spotify
                .prompt_for_token(&url)
                .await
                .expect("couldn't authenticate successfully");

            spotify
                .get_token()
                .lock()
                .await
                .unwrap()
                .clone()
                .unwrap()
                .write_cache(path.clone())
                .expect("couldn't write token to cache");

            Ok(spotify.get_token().lock().await.unwrap().clone().unwrap())
        }
    }
}

pub async fn default_authcode() -> Result<AuthCodeSpotify, ()> {
    let (creds, oauth, config) = obtain_env_details();

    authcode(&creds, &oauth, &config).await
}

pub fn obtain_env_details() -> (Credentials, OAuth, Config) {
    let creds = Credentials::from_env().unwrap();
    let oauth = OAuth::from_env(scopes!("user-follow-read user-follow-modify")).unwrap();
    let path = PathBuf::from("token_cache.json");
    let config = Config {
        cache_path: path.clone(),
        token_refreshing: true,
        token_cached: true,
        ..Default::default()
    };

    (creds, oauth, config)
}
