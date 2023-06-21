use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth, Token};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::PathBuf,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthDetails {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

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

pub fn get_auth_details(rspot_dir: &PathBuf) -> Result<AuthDetails, ()> {
    let auth_path = rspot_dir.join("auth.json");

    if !auth_path.exists() {
        println!("Enter the rspotify_client_id");
        let mut rspotify_client_id = String::new();
        io::stdin()
            .read_line(&mut rspotify_client_id)
            .expect("Failed to read line");

        println!("Enter the rspotify_client_secret");
        let mut rspotify_client_secret = String::new();
        io::stdin()
            .read_line(&mut rspotify_client_secret)
            .expect("Failed to read line");

        println!("Enter the rspotify_redirect_uri");
        let mut rspotify_redirect_uri = String::new();
        io::stdin()
            .read_line(&mut rspotify_redirect_uri)
            .expect("Failed to read line");

        let mut auth_json = json!({});
        auth_json["RSPOTIFY_CLIENT_ID"] = json!(rspotify_client_id.trim());
        auth_json["RSPOTIFY_CLIENT_SECRET"] = json!(rspotify_client_secret.trim());
        auth_json["RSPOTIFY_REDIRECT_URI"] = json!(rspotify_redirect_uri.trim());

        let auth_json = serde_json::to_string_pretty(&auth_json).unwrap();
        fs::write(&auth_path, auth_json).expect("Failed to write to file");
    }

    let mut file = File::open(auth_path).expect("Failed to open the file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read the file");

    // Parse the JSON string into a serde_json::Value
    let auth_details: AuthDetails = serde_json::from_str(&contents).expect("Failed to parse JSON");
    Ok(auth_details)
}

pub async fn default_authcode(rspot_dir: &PathBuf) -> Result<AuthCodeSpotify, ()> {
    // This also needs to be requested from user. Maybe set global env variables? Or I could use config_dir
    let auth_details = get_auth_details(rspot_dir).unwrap();
    let creds = Credentials::new(&auth_details.client_id, &auth_details.client_secret);
    let oauth = OAuth {
        redirect_uri: auth_details.redirect_uri,
        scopes: scopes!("user-follow-read user-follow-modify"),
        ..Default::default()
    };

    let path = rspot_dir.join("token_cache.json");
    let config = Config {
        cache_path: path.clone(),
        token_refreshing: true,
        token_cached: true,
        ..Default::default()
    };

    let auth_code = authcode(&creds, &oauth, &config).await.unwrap();
    Ok(auth_code)
}

pub fn obtain_env_details() -> (Credentials, OAuth, Config) {
    let creds = Credentials::from_env().unwrap();
    let oauth = OAuth::from_env(scopes!("user-follow-read user-follow-modify")).unwrap();
    print!("oauth: {:?}", oauth);
    let path = PathBuf::from("token_cache.json");
    let config = Config {
        cache_path: path.clone(),
        token_refreshing: true,
        token_cached: true,
        ..Default::default()
    };

    (creds, oauth, config)
}
