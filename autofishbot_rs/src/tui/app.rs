use crate::config::Config;
use crate::engine::profile::Profile;

pub struct App {
    pub config: Config,
    pub tabs: Vec<String>,
    pub tab_index: usize,
    pub is_running: bool,
    pub status: String,
    pub logs: Vec<String>,
    pub stats: Stats,
    pub profile: Profile,
    pub last_message: String,
    pub should_quit: bool,
}

pub struct Stats {
    pub fish_caught: u64,
    pub money_earned: u64,
    pub captchas_solved: u64,
    pub runtime: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tabs: vec!["Dashboard".to_string(), "Profile".to_string(), "Logs".to_string(), "Config".to_string()],
            tab_index: 0,
            is_running: false,
            status: "Stopped".to_string(),
            logs: Vec::new(),
            stats: Stats {
                fish_caught: 0,
                money_earned: 0,
                captchas_solved: 0,
                runtime: "00:00:00".to_string(),
            },
            profile: Profile::default(),
            last_message: String::new(),
            should_quit: false,
        }
    }

    pub fn on_tick(&mut self) {
        // Update runtime, etc.
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % self.tabs.len();
    }

    pub fn previous_tab(&mut self) {
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = self.tabs.len() - 1;
        }
    }

    pub fn toggle_bot(&mut self) {
        self.is_running = !self.is_running;
        self.status = if self.is_running { "Running".to_string() } else { "Stopped".to_string() };
        self.add_log(format!("Bot {}", if self.is_running { "Started" } else { "Stopped" }));
    }
}
