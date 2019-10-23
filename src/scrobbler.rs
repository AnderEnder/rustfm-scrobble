use crate::client::LastFm;
use crate::models::metadata::{Scrobble, ScrobbleBatch};
use crate::models::responses::{
    BatchScrobbleResponse, NowPlayingResponse, ScrobbleResponse, SessionResponse,
};

use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::result;
use std::time::{SystemTimeError, UNIX_EPOCH};

type Result<T> = result::Result<T, Error>;

/// Submits song-play tracking information to Last.fm
pub struct Scrobbler {
    client: LastFm,
}

impl Scrobbler {
    /// Creates a new Scrobbler with the given Last.fm API Key and API Secret
    pub fn new(api_key: &str, api_secret: &str) -> Self {
        let client = LastFm::new(api_key, api_secret);

        Self { client }
    }

    pub fn authenticate_with_password(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<SessionResponse> {
        self.client.set_user_credentials(username, password);
        Ok(self.client.authenticate_with_password()?)
    }

    pub fn authenticate_with_token(&mut self, token: &str) -> Result<SessionResponse> {
        self.client.set_user_token(token);
        Ok(self.client.authenticate_with_token()?)
    }

    pub fn authenticate_with_session_key(&mut self, session_key: &str) {
        self.client.authenticate_with_session_key(session_key)
    }

    /// Registers the given track by the given artist as the currently authenticated user's
    /// "now playing" track.
    pub fn now_playing(&self, scrobble: &Scrobble) -> Result<NowPlayingResponse> {
        let params = scrobble.as_map();

        Ok(self.client.send_now_playing(&params)?)
    }

    /// Registers a scrobble (play) of the track with the given title by the given artist in
    /// the account of the currently authenticated user at the current time.
    pub fn scrobble(&self, scrobble: &Scrobble) -> Result<ScrobbleResponse> {
        let mut params = scrobble.as_map();
        let current_time = UNIX_EPOCH.elapsed()?;

        params
            .entry("timestamp".to_string())
            .or_insert_with(|| format!("{}", current_time.as_secs()));

        Ok(self.client.send_scrobble(&params)?)
    }

    pub fn scrobble_batch(&self, batch: &ScrobbleBatch) -> Result<BatchScrobbleResponse> {
        let mut params = HashMap::new();

        let batch_count = batch.len();
        if batch_count > 50 {
            return Err(Error::new(
                "Scrobble batch too large (must be 50 or fewer scrobbles)".to_owned(),
            ));
        } else if batch_count == 0 {
            return Err(Error::new("Scrobble batch is empty".to_owned()));
        }

        for (i, scrobble) in batch.iter().enumerate() {
            let mut scrobble_params = scrobble.as_map();
            let current_time = UNIX_EPOCH.elapsed()?;
            scrobble_params
                .entry("timestamp".to_string())
                .or_insert_with(|| format!("{}", current_time.as_secs()));

            for (key, val) in &scrobble_params {
                // batched parameters need array notation suffix ie.
                // "artist[1]"" = "Artist 1", "artist[2]" = "Artist 2"
                params.insert(format!("{}[{}]", key, i), val.clone());
            }
        }

        Ok(self.client.send_batch_scrobbles(&params)?)
    }

    /// Gets the session key the client is currently authenticated with. Returns
    /// `None` if not authenticated. Valid session keys can be stored and used
    /// to authenticate with `authenticate_with_session_key`.
    pub fn session_key(&self) -> Option<&str> {
        self.client.session_key()
    }
}


// TODO(v1): Consider moving this to error.rs? It's getting somewhat involved
#[derive(Debug)]
pub struct Error {
    err_msg: String,
}

impl Error {
    pub fn new(err_msg: String) -> Self {
        Self { err_msg }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.err_msg)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.err_msg.as_str()
    }

    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

impl From<SystemTimeError> for Error {
    fn from(error: SystemTimeError) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::new(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    #[test]
    fn make_scrobbler_pass_auth() {
        let _m = mock("POST", mockito::Matcher::Any).create();

        let mut scrobbler = Scrobbler::new("api_key", "api_secret");
        let resp = scrobbler.authenticate_with_password("user", "pass");
        assert!(resp.is_err());

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
                {   
                    "session": {
                        "key": "key",
                        "subscriber": 1337,
                        "name": "foo floyd"
                    }
                }
            "#,
            )
            .create();

        let resp = scrobbler.authenticate_with_password("user", "pass");
        assert!(resp.is_ok());
    }

    #[test]
    fn make_scrobbler_token_auth() {
        let _m = mock("POST", mockito::Matcher::Any).create();

        let mut scrobbler = Scrobbler::new("api_key", "api_secret");
        let resp = scrobbler.authenticate_with_token("some_token");
        assert!(resp.is_err());

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
                {   
                    "session": {
                        "key": "key",
                        "subscriber": 1337,
                        "name": "foo floyd"
                    }
                }
            "#,
            )
            .create();

        let resp = scrobbler.authenticate_with_token("some_token");
        assert!(resp.is_ok());
    }

    #[test]
    fn check_scrobbler_error() {
        let err = Error::new("test_error".into());
        let fmt = format!("{}", err);
        assert_eq!("test_error", fmt);

        let desc = err.description();
        assert_eq!("test_error", desc);

        assert!(err.source().is_none());
    }

    #[test]
    fn check_scrobbler_now_playing() {
        let mut scrobbler = Scrobbler::new("api_key", "api_secret");

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
                {   
                    "session": {
                        "key": "key",
                        "subscriber": 1337,
                        "name": "foo floyd"
                    }
                }
            "#,
            )
            .create();

        let resp = scrobbler.authenticate_with_token("some_token");
        assert!(resp.is_ok());

        let mut scrobble = crate::models::metadata::Scrobble::new(
            "foo floyd and the fruit flies",
            "old bananas",
            "old bananas",
        );
        scrobble.with_timestamp(1337);

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
            { 
                "nowplaying": {
                            "artist": [ "0", "foo floyd and the fruit flies" ],
                            "album": [ "1", "old bananas" ], 
                            "albumArtist": [ "0", "foo floyd"],
                            "track": [ "1", "old bananas"], 
                            "timestamp": "2019-10-04 13:23:40" 
                        }
            }
            "#,
            )
            .create();

        let resp = scrobbler.now_playing(&scrobble);
        assert!(resp.is_ok());
    }

    #[test]
    fn check_scrobbler_scrobble() {
        let mut scrobbler = Scrobbler::new("api_key", "api_secret");

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
                {   
                    "session": {
                        "key": "key",
                        "subscriber": 1337,
                        "name": "foo floyd"
                    }
                }
            "#,
            )
            .create();

        let resp = scrobbler.authenticate_with_token("some_token");
        assert!(resp.is_ok());

        let mut scrobble = crate::models::metadata::Scrobble::new(
            "foo floyd and the fruit flies",
            "old bananas",
            "old bananas",
        );
        scrobble.with_timestamp(1337);

        let _m = mock("POST", mockito::Matcher::Any)
            .with_body(
                r#"
            { 
                "scrobbles": [{
                        "artist": [ "0", "foo floyd and the fruit flies" ],
                        "album": [ "1", "old bananas" ], 
                        "albumArtist": [ "0", "foo floyd"],
                        "track": [ "1", "old bananas"], 
                        "timestamp": "2019-10-04 13:23:40" 
                }]
            }
            "#,
            )
            .create();

        let resp = scrobbler.scrobble(&scrobble);
        assert!(resp.is_ok());
    }
}
