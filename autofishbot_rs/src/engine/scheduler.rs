use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::Instant;
use crate::config::Config;
use crate::discord::client::DiscordClient;
use log::info;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    Fish,
    Daily,
    BuyBait,
    Sell,
    Profile,
    Boost(String), // e.g. "fish5m"
    Custom(String, Option<serde_json::Value>),
}

#[derive(Debug, Clone)]
pub struct Task {
    pub task_type: TaskType,
    pub next_run: Instant,
    pub interval: Option<Duration>, // None for one-off
    pub manual: bool,
}

pub struct Scheduler {
    config: Config,
    queue: VecDeque<Task>,
    last_action: Instant,
    global_cooldown: Duration,
}

impl Scheduler {
    pub fn new(config: Config) -> Self {
        let mut scheduler = Self {
            config: config.clone(),
            queue: VecDeque::new(),
            last_action: Instant::now(),
            global_cooldown: Duration::from_secs(5), // Base global cooldown
        };
        scheduler.setup();
        scheduler
    }

    fn setup(&mut self) {
        let now = Instant::now();

        // Auto Daily
        if self.config.automation.auto_daily {
            self.add_task(Task {
                task_type: TaskType::Daily,
                next_run: now + Duration::from_secs(10), // Start soon
                interval: Some(Duration::from_secs(24 * 60 * 60)),
                manual: false,
            });
        }

        // Boosts
        let boost_len = self.config.automation.boosts_length;
        let boost_interval = Duration::from_secs(boost_len * 60);

        if self.config.automation.more_fish {
             self.add_task(Task {
                task_type: TaskType::Boost(format!("fish{}m", boost_len)),
                next_run: now + Duration::from_secs(20),
                interval: Some(boost_interval),
                manual: false,
            });
        }

         if self.config.automation.more_treasures {
             self.add_task(Task {
                task_type: TaskType::Boost(format!("treasure{}m", boost_len)),
                next_run: now + Duration::from_secs(25),
                interval: Some(boost_interval),
                manual: false,
            });
        }

        // Auto Sell
        if self.config.automation.auto_sell {
             self.add_task(Task {
                task_type: TaskType::Sell,
                next_run: now + Duration::from_secs(60),
                interval: Some(Duration::from_secs(8 * 60)), // Every 8 mins
                manual: false,
            });
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.queue.push_back(task);
        // Sort queue by next_run? Or just check all?
        // Priority queue would be better.
    }

    pub async fn process(&mut self, client: &DiscordClient) {
        // Sort queue
        // Since VecDeque isn't contiguous, we convert to Vec, sort, and back?
        // Or just iterate and find ready tasks.

        if self.last_action.elapsed() < self.global_cooldown {
            return;
        }

        let now = Instant::now();
        let mut tasks_to_run = Vec::new();
        let mut tasks_to_keep = Vec::new();

        while let Some(task) = self.queue.pop_front() {
            if task.next_run <= now {
                tasks_to_run.push(task);
            } else {
                tasks_to_keep.push(task);
            }
        }

        // Put back non-ready tasks
        for task in tasks_to_keep {
            self.queue.push_back(task);
        }

        if let Some(mut task) = tasks_to_run.pop() {
            // Execute task
            info!("Executing task: {:?}", task.task_type);

            // Use hardcoded IDs for "Virtual Fisher" on this specific server as a fallback,
            // or ideally fetch dynamically. But `process` is synchronous in structure (except for the send part).
            // We need to make sure we have the correct IDs.
            // Since we know the IDs from the discovery step:
            // daily: 912432961033482262
            // sell: 912432960643416116
            // buy: 912432961134166090

            let guild_id = "1273750160022835301"; // Hardcoded for test
            let channel_id = &self.config.system.channel_id.to_string();
            let app_id = "574652751745777665".to_string();

            match &task.task_type {
                TaskType::Daily => {
                    // Fetch command dynamically
                    if let Ok(Some(cmd)) = client.get_command(guild_id, "daily").await {
                         let _ = client.send_command(guild_id, channel_id, &cmd, None).await;
                    } else {
                        // Fallback to construct with known ID if fetch fails
                         let _ = client.send_command(guild_id, channel_id,
                            &crate::discord::types::ApplicationCommand {
                                id: "912432961033482262".to_string(),
                                application_id: app_id.clone(),
                                version: "0".to_string(),
                                default_permission: None,
                                default_member_permissions: None,
                                r#type: 1,
                                name: "daily".to_string(),
                                description: "".to_string(),
                                guild_id: None
                            }, None).await;
                    }
                },
                TaskType::Sell => {
                    if let Ok(Some(cmd)) = client.get_command(guild_id, "sell").await {
                         let _ = client.send_command(guild_id, channel_id, &cmd, Some(vec![serde_json::json!({
                            "type": 3,
                            "name": "amount",
                            "value": "all"
                        })])).await;
                    } else {
                         let _ = client.send_command(guild_id, channel_id,
                            &crate::discord::types::ApplicationCommand {
                                id: "912432960643416116".to_string(),
                                application_id: app_id.clone(),
                                version: "0".to_string(),
                                default_permission: None,
                                default_member_permissions: None,
                                r#type: 1,
                                name: "sell".to_string(),
                                description: "".to_string(),
                                guild_id: None
                            }, Some(vec![serde_json::json!({
                                "type": 3,
                                "name": "amount",
                                "value": "all"
                            })])).await;
                    }
                },
                TaskType::Boost(name) => {
                    if let Ok(Some(cmd)) = client.get_command(guild_id, "buy").await {
                         let _ = client.send_command(guild_id, channel_id, &cmd, Some(vec![serde_json::json!({
                            "type": 3,
                            "name": "item",
                            "value": name
                        })])).await;
                    } else {
                         let _ = client.send_command(guild_id, channel_id,
                            &crate::discord::types::ApplicationCommand {
                                id: "912432961134166090".to_string(),
                                application_id: app_id.clone(),
                                version: "0".to_string(),
                                default_permission: None,
                                default_member_permissions: None,
                                r#type: 1,
                                name: "buy".to_string(),
                                description: "".to_string(),
                                guild_id: None
                            }, Some(vec![serde_json::json!({
                                "type": 3,
                                "name": "item",
                                "value": name
                            })])).await;
                    }
                },
                _ => {}
            }

            // Reschedule if interval
            if let Some(interval) = task.interval {
                task.next_run = now + interval;
                self.queue.push_back(task);
            }

            self.last_action = Instant::now();
            // Add some jitter to global cooldown
            let jitter = rand::random::<u64>() % 3000;
            self.global_cooldown = Duration::from_millis(3000 + jitter);
        } else {
            // No tasks ready
        }

        // Put remaining ready tasks back (we only run one at a time to respect rate limits/human behavior)
        for task in tasks_to_run {
            self.queue.push_back(task);
        }
    }
}
