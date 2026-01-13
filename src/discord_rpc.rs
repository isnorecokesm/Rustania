use discord_rich_presence::{DiscordIpcClient, DiscordIpc};
use discord_rich_presence::activity::{Activity, Assets};

pub struct RpcManager {
    client: DiscordIpcClient,
}

impl RpcManager {
    pub fn new() -> Self {
        let mut client = DiscordIpcClient::new("CENSORED").unwrap();
        client.connect().unwrap();
        RpcManager { client }
    }

    /// Prikazuje da je korisnik idle / u meniju
    pub fn update_idle(&mut self) {
        self.client.set_activity(
            Activity::new()
                .details("In Menu")
                .state("Idle")
                .assets(
                    Assets::new()
                        .large_image("sLogo")
                        .large_text("Rustania")
                )
        ).unwrap();
    }

    /// Prikazuje da korisnik igra mapu
    pub fn update_playing(&mut self, map_name: &str, difficulty: &str) {
        self.client.set_activity(
            Activity::new()
                .details(&format!("Playing: {}", map_name))
                .state(&format!("Difficulty: {}", difficulty))
                .assets(
                    Assets::new()
                        .large_image("sLogo")
                        .large_text("Rustania")
                )
        ).unwrap();
    }

    /// Prikazuje da je korisnik završio pesmu
    pub fn update_finished(&mut self, map_name: &str) {
        self.client.set_activity(
            Activity::new()
                .details(&format!("Finished: {}", map_name))
                .state("Idle")
                .assets(
                    Assets::new()
                        .large_image("sLogo")
                        .large_text("Rustania")
                )
        ).unwrap();
    }
}
