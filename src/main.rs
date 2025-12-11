mod claude;

use claude::ClaudeCredentials;
use image::GenericImageView;
use ksni::{Handle, TrayMethods, menu::*};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::mpsc;

// Commands nums that can be sent to the updater channel
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
    // Indicates whether the user is logged in
    is_login: bool,
    // The Claude API requires credentials to make requests. To get this
    // information is necessary to log in using the Claude console command
    credentials: claude::ClaudeCredentials,
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
                visible: !self.is_login,
                activate: Box::new(|this: &mut Self| match claude::login() {
                    Ok(credentials) => {
                        this.is_login = true;
                        this.credentials = credentials;

                        if let Err(e) = this.updater_channel.send(UpdaterCommand::Start) {
                            eprintln!("Failed to start updater: {}", e);
                        }
                    }
                    Err(err) => eprintln!("Error logging in: {}", err),
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Plan usage limits\nCurrent session ({}/100)",
                    self.five_hour_usage
                ),
                visible: self.is_login,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Weekly usage limits\nAll models ({}/100)",
                    self.seven_day_usage
                ),
                visible: self.is_login,
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
    let mut is_login = false;

    // Try to load credentials from a file. In case of error, assume the user is
    // not logged in to a claude account and will show the login option in the tray menu
    let credentials: claude::ClaudeCredentials = match claude::get_credentials() {
        Ok(credentials) => {
            is_login = true;
            credentials
        }
        Err(err) => {
            println!("Error getting credentials: {}", err);

            // Set default credentials values. Maybe
            // use Option::None instead of an empty object?
            ClaudeCredentials::new_empty()
        }
    };

    let (updater_channel, mut updater_receiver) = mpsc::unbounded_channel::<UpdaterCommand>();

    // Set the values of the tray variables based on the
    // user's login status and the response to the usage request.
    let tray = AppTray {
        is_login,
        credentials,
        five_hour_usage: 0.0,
        seven_day_usage: 0.0,
        updater_channel,
    };

    let handle = tray.spawn().await.unwrap();
    let handle_for_updater = handle.clone();

    tokio::spawn(async move {
        let mut updater_task: Option<tokio::task::JoinHandle<()>> = None;

        while let Some(command) = updater_receiver.recv().await {
            match command {
                UpdaterCommand::Start => {
                    println!("Starting usage updater");

                    // Check if the updater task is already running to not start it again
                    if updater_task.is_none() {
                        updater_task = Some(usage_updater_task(&handle_for_updater));
                    }
                }
                UpdaterCommand::Stop => {
                    print!("Stopping usage updater");

                    if let Some(task) = updater_task.take() {
                        task.abort();
                    }
                }
            }
        }
    });

    handle
        .update(|tray: &mut AppTray| {
            if tray.is_login {
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

                println!("Fetching usage data...");

                // Get the access token from the handle of the tray
                let token = match handle_for_updater
                    .update(|tray: &mut AppTray| {
                        tray.credentials.claude_ai_oauth.access_token.clone()
                    })
                    .await
                {
                    Some(token) => {
                        if let Some(access_token) = token {
                            access_token
                        } else {
                            break Err("No access token found".to_string());
                        }
                    }
                    None => break Err("No access token found".to_string()),
                };

                // Check if the token is empty. If it is not, return an error
                // and end the loop using a break
                if !token.is_empty() {
                    // Get usage data and compare the result using match.
                    // If it returns an error, break the loop and return the error
                    match claude::get_usage(&token).await {
                        Ok(usage) => {
                            // Update usage data
                            handle_for_updater
                                .update(|tray: &mut AppTray| {
                                    tray.five_hour_usage = usage.five_hour.utilization;
                                    tray.seven_day_usage = usage.seven_day.utilization;
                                })
                                .await;

                            println!("Usage data updated!");
                        }
                        Err(err) => {
                            break Err(format!("Failed to fetch usage data: {}", err));
                        }
                    };
                } else {
                    break Err("No access token found".to_string());
                }
            }
        };

        match result {
            Err(err) => {
                println!("Failed to update usage data: {}", err);

                // If something went wrong, set the values of tray
                // like if the user is not logged in and sends a
                // stop command to the updater
                handle_for_updater
                    .update(|tray: &mut AppTray| {
                        tray.is_login = false;
                        tray.credentials = ClaudeCredentials::new_empty();

                        tray.updater_channel
                            .send(UpdaterCommand::Stop)
                            .expect("Failed to send stop command");
                    })
                    .await;
            }
            Ok(_) => (),
        }
    })
}
