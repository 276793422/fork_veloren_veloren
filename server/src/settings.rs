mod editable;

pub use editable::EditableSetting;

use authc::Uuid;
use hashbrown::HashMap;
use portpicker::pick_unused_port;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::{error, warn};
use world::sim::FileOpts;

const DEFAULT_WORLD_SEED: u32 = 59686;
//const CONFIG_DIR_ENV: &'static str = "VELOREN_SERVER_CONFIG";
const /*DEFAULT_*/CONFIG_DIR: &'static str = "server_config";
const SETTINGS_FILENAME: &'static str = "settings.ron";
const WHITELIST_FILENAME: &'static str = "whitelist.ron";
const BANLIST_FILENAME: &'static str = "banlist.ron";
const SERVER_DESCRIPTION_FILENAME: &'static str = "description.ron";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerSettings {
    pub gameserver_address: SocketAddr,
    pub metrics_address: SocketAddr,
    pub auth_server_address: Option<String>,
    pub max_players: usize,
    pub world_seed: u32,
    //pub pvp_enabled: bool,
    pub server_name: String,
    pub start_time: f64,
    pub admins: Vec<String>,
    /// When set to None, loads the default map file (if available); otherwise,
    /// uses the value of the file options to decide how to proceed.
    pub map_file: Option<FileOpts>,
    /// Relative paths are relative to the server data dir
    pub persistence_db_dir: String,
    pub max_view_distance: Option<u32>,
    pub banned_words_files: Vec<PathBuf>,
    pub max_player_group_size: u32,
    pub client_timeout: Duration,
    pub update_shutdown_grace_period_secs: u64,
    pub update_shutdown_message: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            gameserver_address: SocketAddr::from(([0; 4], 14004)),
            metrics_address: SocketAddr::from(([0; 4], 14005)),
            auth_server_address: Some("https://auth.veloren.net".into()),
            world_seed: DEFAULT_WORLD_SEED,
            server_name: "Veloren Alpha".into(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            map_file: None,
            admins: Vec::new(),
            persistence_db_dir: "saves".into(),
            max_view_distance: Some(30),
            banned_words_files: Vec::new(),
            max_player_group_size: 6,
            client_timeout: Duration::from_secs(40),
            update_shutdown_grace_period_secs: 120,
            update_shutdown_message: "The server is restarting for an update".to_owned(),
        }
    }
}

impl ServerSettings {
    /// path: Directory that contains the server config directory
    pub fn load(path: &Path) -> Self {
        let path = Self::get_settings_path(path);

        if let Ok(file) = fs::File::open(&path) {
            match ron::de::from_reader(file) {
                Ok(x) => x,
                Err(e) => {
                    warn!(
                        ?e,
                        "Failed to parse setting file! Falling back to default settings and \
                         creating a template file for you to migrate your current settings file"
                    );
                    let default_settings = Self::default();
                    let template_path = path.with_extension("template.ron");
                    if let Err(e) = default_settings.save_to_file(&template_path) {
                        error!(?e, "Failed to create template settings file")
                    }
                    default_settings
                },
            }
        } else {
            let default_settings = Self::default();

            if let Err(e) = default_settings.save_to_file(&path) {
                error!(?e, "Failed to create default settings file!");
            }
            default_settings
        }
    }

    fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        // Create dir if it doesn't exist
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let ron = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("Failed serialize settings.");

        fs::write(path, ron.as_bytes())?;

        Ok(())
    }

    /// path: Directory that contains the server config directory
    pub fn singleplayer(path: &Path) -> Self {
        let load = Self::load(&path);
        Self {
            //BUG: theoretically another process can grab the port between here and server
            // creation, however the timewindow is quite small
            gameserver_address: SocketAddr::from((
                [127, 0, 0, 1],
                pick_unused_port().expect("Failed to find unused port!"),
            )),
            metrics_address: SocketAddr::from((
                [127, 0, 0, 1],
                pick_unused_port().expect("Failed to find unused port!"),
            )),
            auth_server_address: None,
            // If loading the default map file, make sure the seed is also default.
            world_seed: if load.map_file.is_some() {
                load.world_seed
            } else {
                DEFAULT_WORLD_SEED
            },
            server_name: "Singleplayer".to_owned(),
            //server_description: "Who needs friends anyway?".to_owned(),
            max_players: 100,
            start_time: 9.0 * 3600.0,
            admins: vec!["singleplayer".to_string()], /* TODO: Let the player choose if they want
                                                       * to use admin commands or not */
            max_view_distance: None,
            client_timeout: Duration::from_secs(180),
            ..load // Fill in remaining fields from server_settings.ron.
        }
    }

    fn get_settings_path(path: &Path) -> PathBuf {
        let mut path = with_config_dir(path);
        path.push(SETTINGS_FILENAME);
        path
    }
}

fn with_config_dir(path: &Path) -> PathBuf {
    let mut path = PathBuf::from(path);
    //if let Some(path) = std::env::var_os(CONFIG_DIR_ENV) {
    //    let config_dir = PathBuf::from(path);
    //    if config_dir.exists() {
    //        return config_dir;
    //    }
    //    warn!(?path, "VELROREN_SERVER_CONFIG points to invalid path.");
    //}
    path.push(/* DEFAULT_ */ CONFIG_DIR);
    //PathBuf::from(DEFAULT_CONFIG_DIR)
    path
}

#[derive(Deserialize, Serialize, Default)]
#[serde(transparent)]
pub struct Whitelist(Vec<String>);
#[derive(Deserialize, Serialize, Default)]
#[serde(transparent)]
pub struct Banlist(HashMap<Uuid, (String, String)>);
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ServerDescription(String);

impl Default for ServerDescription {
    fn default() -> Self { Self("This is the best Veloren server".into()) }
}

impl EditableSetting for Whitelist {
    const FILENAME: &'static str = WHITELIST_FILENAME;
}

impl EditableSetting for Banlist {
    const FILENAME: &'static str = BANLIST_FILENAME;
}

impl EditableSetting for ServerDescription {
    const FILENAME: &'static str = SERVER_DESCRIPTION_FILENAME;
}

impl Deref for Whitelist {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Whitelist {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for Banlist {
    type Target = HashMap<Uuid, (String, String)>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Banlist {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Deref for ServerDescription {
    type Target = String;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ServerDescription {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
