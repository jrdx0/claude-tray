mod claude;
mod utils;

use image::GenericImageView;
use ksni::{Handle, TrayMethods, menu::*};
use std::{sync::LazyLock, time::Duration};
use tokio::sync::mpsc;

use crate::claude::ClaudeCredentials;

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

enum TrayMessage {
    Login,
    StartUsageTracking,
    StopUsageTracking,
}

// Tray variables to handle authentication and usage tracking
#[derive(Debug)]
struct AppTray {
    // Visible items status
    is_login_visible: bool,
    is_usage_visible: bool,
    // Access token for authentication
    access_token: Option<ClaudeCredentials>,
    // Variables to track usage
    five_hour_usage: f32,
    seven_day_usage: f32,
    // Channel to communicate tray actions with actions that
    // need to be performed asynchronously
    notifier: mpsc::Sender<TrayMessage>,
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
                // For some reason, using Cosmic, this element cannot be hidden
                visible: self.is_login_visible,
                activate: Box::new(|this: &mut Self| {
                    let _ = this
                        .notifier
                        .try_send(TrayMessage::Login)
                        .map_err(|e| log::error!("{}", e));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Plan usage limits\nCurrent session ({}/100)",
                    self.five_hour_usage
                ),
                visible: self.is_usage_visible,
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: format!(
                    "Weekly usage limits\nAll models ({}/100)",
                    self.seven_day_usage
                ),
                visible: self.is_usage_visible,
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            // Option to open ClaudeIA using the browser
            StandardItem {
                label: "Open Claude".into(),
                activate: Box::new(|_| {
                    webbrowser::open("https://claude.ai/new")
                        .expect("error opening claude on the browser")
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

    let (notifier, mut tray_msgs) = mpsc::channel::<TrayMessage>(1);

    // Initial tray values before executing
    // updater task to update usage information
    let tray = AppTray {
        is_login_visible: true,
        is_usage_visible: false,
        access_token: None,
        five_hour_usage: 0.0,
        seven_day_usage: 0.0,
        notifier,
    };
    let handle = tray
        .spawn()
        .await
        .expect("tray handler error while spawning");

    match claude::get_local_credentials() {
        Ok(access_token) => {
            handle
                .update(|tray: &mut AppTray| {
                    tray.access_token = Some(access_token);

                    tray.is_login_visible = false;
                    tray.is_usage_visible = true;

                    log::trace!("credentials loaded from locally source. Hidding login button");

                    let _ = tray.notifier.try_send(TrayMessage::StartUsageTracking);
                })
                .await;
        }
        Err(e) => {
            log::error!("{}", e);
        }
    }

    let mut tracking_task: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            Some(msg) = tray_msgs.recv() => {
                match msg {
                    // This code is executed when the login button is clicked
                    TrayMessage::Login => {
                        let claude_credentials = match claude::open_oauth_login().await {
                            Ok(credentials) => credentials,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };
                        let access_token = match claude::save_credentials_locally(&claude_credentials) {
                            Ok(credentials) => credentials,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        handle
                            .update(|tray: &mut AppTray| {
                                tray.access_token = Some(access_token);

                                tray.is_login_visible = false;
                                tray.is_usage_visible = true;

                                let _ = tray.notifier.try_send(TrayMessage::StartUsageTracking)
                                    .map_err(|e| log::error!("{}", e));
                            })
                            .await;
                    }

                    TrayMessage::StartUsageTracking => {
                        if tracking_task.is_none() {
                            if let Ok(task) = usage_tracking_task(&handle).await {
                                tracking_task = Some(task);
                            } else {
                                log::error!("failed to start usage tracking");
                            }
                        }
                    }

                    TrayMessage::StopUsageTracking => {
                        log::trace!("stopping usage tracking");
                        if let Some(task) = tracking_task.take() {
                            task.abort();
                        }
                    }
                }
            }
        }
    }
}

async fn usage_tracking_task(
    handle: &Handle<AppTray>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let handle_tracking = handle.clone();

    let Some(credentials) = handle
        .update(|tray: &mut AppTray| {
            tray.access_token
                .as_ref()
                .map(|token| token.access_token.clone())
        })
        .await
        .flatten()
    else {
        return Err("no credentials available".into());
    };

    let tracking_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_mins(5));

        let tracking_result: Result<(), String> = {
            loop {
                interval.tick().await;

                log::trace!("getting usage data from claude api");

                if let Ok(usage) = claude::get_usage(&credentials).await {
                    handle_tracking
                        .update(|tray: &mut AppTray| {
                            tray.five_hour_usage = usage.five_hour.utilization;
                            tray.seven_day_usage = usage.seven_day.utilization;
                        })
                        .await;
                } else {
                    break Err("failed to get usage data".into());
                }
            }
        };

        if let Err(error) = tracking_result {
            log::error!("usage tracking failed: {}", error);

            handle_tracking
                .update(|tray: &mut AppTray| {
                    tray.is_login_visible = true;
                    tray.is_usage_visible = false;

                    tray.five_hour_usage = 0.0;
                    tray.seven_day_usage = 0.0;

                    let _ = tray.notifier.try_send(TrayMessage::StopUsageTracking);
                })
                .await;
        }
    });

    Ok(tracking_task)
}
