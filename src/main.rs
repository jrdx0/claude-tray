mod api;
mod claude;
mod utils;

use claude::Claude;
use image::GenericImageView;
use ksni::{Handle, TrayMethods, menu::*};
use log::{error, info, trace};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::mpsc;

// Commands nums that can be sent to the updater channel
#[derive(Debug)]
enum UpdaterCommand {
    Start,
    Stop,
}

// Loading the icon image that is used in the tray
static CLAUDE_ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let img = image::load_from_memory_with_format(
        include_bytes!("./assets/claude-icon.png"),
        image::ImageFormat::Png,
    )
    .expect("valid image");

    let (width, height) = img.dimensions();
    let mut data = img.into_rgba8().into_vec();

    for pixel in data.chunks_exact_mut(4) {
        pixel.rotate_right(1)
    }

    ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
});

// Tray variables to handle authentication and usage tracking
#[derive(Debug)]
struct AppTray {
    // Claude instance
    claude: Claude,
    // Variables to track usage
    five_hour_usage: f32,
    seven_day_usage: f32,
    // This is the channel used to communicate with the updater controller
    updater_channel: mpsc::UnboundedSender<UpdaterCommand>,
}

// Options to show in the tray menu application
impl ksni::Tray for AppTray {
    // Identifier for the tray
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    // Custom icon for the tray
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![CLAUDE_ICON.clone()]
    }
    // Title for the tray
    fn title(&self) -> String {
        "Claude Tray".into()
    }
    // Menu items for the tray
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            // Login option in case user is not logged in
            StandardItem {
                label: "Iniciar sesión".into(),
                // If login is false, show login option
                visible: self.claude.access_token.is_none(),
                activate: Box::new(|this: &mut Self| {
                    let mut claude = this.claude.clone();
                    let updater_channel = this.updater_channel.clone();

                    tokio::spawn(async move {
                        match claude.login().await {
                            Ok(_) => {
                                trace!("Logged in successfully. Sending start command to updater");

                                if let Err(e) = updater_channel.send(UpdaterCommand::Start) {
                                    error!("Failed to start updater: {}", e);
                                }
                            }
                            Err(err) => error!("Error logging in: {}", err),
                        }
                    });
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Plan usage limits\nCurrent session ({}/100)",
                    self.five_hour_usage
                ),
                visible: self.claude.access_token.is_some(),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Weekly usage limits\nAll models ({}/100)",
                    self.seven_day_usage
                ),
                visible: self.claude.access_token.is_some(),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Option to open ClaudeIA using the browser
            StandardItem {
                label: "Open Claude".into(),
                activate: Box::new(|_| {
                    webbrowser::open("https://claude.ai/new")
                        .expect("Error opening Claude on the browser")
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Option to exit the application
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let (updater_channel, mut updater_receiver) = mpsc::unbounded_channel::<UpdaterCommand>();

    // Initial tray values before executing
    // updater task to update usage information
    let tray = AppTray {
        claude: Claude::new(),
        five_hour_usage: 0.0,
        seven_day_usage: 0.0,
        updater_channel,
    };

    let handle = tray.spawn().await.unwrap();
    let handle_for_updater = handle.clone();

    tokio::spawn(async move {
        let mut updater_task: Option<tokio::task::JoinHandle<()>> = None;

        while let Some(command) = updater_receiver.recv().await {
            trace!("Command sent to handle updater task, value {:?}", command);

            match command {
                UpdaterCommand::Start => {
                    trace!("Starting usage updater");

                    // Check if the updater task is already running to not start it again
                    if updater_task.is_none() {
                        updater_task = Some(usage_updater_task(&handle_for_updater));
                    }
                }
                UpdaterCommand::Stop => {
                    trace!("Stopping usage updater");

                    if let Some(task) = updater_task.take() {
                        task.abort();
                    }
                }
            }
        }
    });

    handle
        .update(|tray: &mut AppTray| {
            if tray.claude.access_token.is_some() {
                tray.updater_channel
                    .send(UpdaterCommand::Start)
                    .expect("Failed to send start command");
            }
        })
        .await;

    std::future::pending().await
}

// Function that returns a tokio task for updating usage data. If access is empty
// or the usage request fails, it will stop the updater by sending a stop command.
fn usage_updater_task(handle: &Handle<AppTray>) -> tokio::task::JoinHandle<()> {
    // Clone the handle for the updater task for scope purposes
    let handle_for_updater = handle.clone();

    // The task that is in charge of updating usage data
    tokio::spawn(async move {
        // By default, the interval is set to 5 minutes
        let mut interval = tokio::time::interval(Duration::from_mins(5));

        // We wait for a Result value type because in case of an error, we can handle
        // it gracefully by doing a match of the response. The only way to break the
        // loop is by getting an error.
        let result: Result<(), String> = {
            loop {
                interval.tick().await;

                trace!("Fetching usage data from updater task");

                let tray_claude = handle_for_updater
                    .update(|tray: &mut AppTray| {
                        if tray.claude.access_token.is_none() {
                            None
                        } else {
                            Some(tray.claude.clone())
                        }
                    })
                    .await;

                let mut tray_claude = match tray_claude {
                    Some(Some(claude)) => claude,
                    Some(None) => break Err("No access token found".to_string()),
                    None => break Err("Failed to access tray".to_string()),
                };

                // Get usage data
                match tray_claude.get_usage().await {
                    Ok(usage) => {
                        // Update usage data
                        handle_for_updater
                            .update(|tray: &mut AppTray| {
                                tray.five_hour_usage = usage.five_hour.utilization;
                                tray.seven_day_usage = usage.seven_day.utilization;
                            })
                            .await;

                        info!("Usage data updated!");
                    }
                    Err(err) => break Err(err),
                }
            }
        };

        if let Err(err) = result {
            error!("{}", err);

            // If something went wrong, clear the token and stop the updater
            // This forces the user to log in again
            handle_for_updater
                .update(|tray: &mut AppTray| {
                    tray.claude.access_token = None;

                    tray.updater_channel
                        .send(UpdaterCommand::Stop)
                        .expect("Failed to send stop command");
                })
                .await;
        }
    })
}
