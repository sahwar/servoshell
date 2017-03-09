#![feature(box_syntax)]

#[macro_use]
extern crate objc;

#[macro_use]
extern crate log;

extern crate simplelog;
extern crate libc;
extern crate cocoa;
extern crate objc_foundation;
extern crate open;

mod app;
mod window;
mod view;
mod servo;
mod platform;
mod commands;

use app::AppEvent;
use window::WindowEvent;
use view::ViewEvent;
use servo::ServoEvent;
use simplelog::{CombinedLogger, Config, LogLevel, LogLevelFilter, TermLogger, WriteLogger};
use std::fs::File;
use std::env::args;
use app::App;
use servo::{Servo, ServoUrl, FollowLinkPolicy};
use commands::{AppCommand, WindowCommand, CommandState};

fn main() {

    // FIXME: can we use NSLog instead of a file?
    let log_file = File::create("/tmp/servoshell.log").unwrap();
    let log_config = Config {
        time: None,
        level: Some(LogLevel::Info),
        target: Some(LogLevel::Info),
        location: Some(LogLevel::Info),
    };

    let _ = match TermLogger::new(LogLevelFilter::Info, log_config) {
        Some(logger) => CombinedLogger::init(vec![
            logger,
            WriteLogger::new(LogLevelFilter::Info, log_config, log_file)
        ]),
        None => WriteLogger::init(LogLevelFilter::Info, log_config, log_file)
    };

    info!("starting");

    platform::init();

    let app = App::load().unwrap();
    let (window, view) = app.create_window().unwrap();

    // Skip first argument (executable), and find the first
    // argument that doesn't start with `-`
    let url = args().skip(1).find(|arg| {
        !arg.starts_with("-")
    }).unwrap_or("http://servo.org".to_owned());

    Servo::configure(&url).unwrap();
    let servo = {
        let geometry = view.get_geometry();
        let riser = window.create_eventloop_riser();
        // let policy = FollowLinkPolicy::FollowOriginalDomain;
        let policy = FollowLinkPolicy::FollowAnyLink;
        Servo::new(geometry, riser, &url, policy)
    };

    info!("Servo version: {}", servo.version());

    window.set_command_state(WindowCommand::OpenLocation, CommandState::Enabled);
    app.set_command_state(AppCommand::ClearHistory, CommandState::Enabled);

    app.run(|| {

        // Loop until no events are available anymore.
        loop {

            let app_events = app.get_events();
            let win_events = window.get_events();
            let view_events = view.get_events();
            let servo_events = servo.get_events();

            if app_events.is_empty() &&
               win_events.is_empty() &&
               view_events.is_empty() &&
               servo_events.is_empty() {
                   break
            }

            // FIXME: it's really annoying we need this
            let mut sync_needed = false;

            for event in app_events {
                match event {
                    AppEvent::DidFinishLaunching => {
                        // FIXME: does this work?
                    }
                    AppEvent::WillTerminate => {
                        // FIXME: does this work?
                    }
                    AppEvent::DidChangeScreenParameters => {
                        // FIXME: does this work?
                        servo.update_geometry(view.get_geometry());
                        view.update_drawable();
                        sync_needed = true;
                    }
                    AppEvent::DoCommand(cmd) => {
                        match cmd {
                            AppCommand::ClearHistory => {
                                // FIXME
                            }
                        }
                    }
                }
            }

            for event in win_events {
                match event {
                    WindowEvent::EventLoopRised => {
                        sync_needed = true;
                    }
                    WindowEvent::GeometryDidChange => {
                        servo.update_geometry(view.get_geometry());
                        view.update_drawable();
                        sync_needed = true;
                    }
                    WindowEvent::DidEnterFullScreen => {
                        // FIXME
                    }
                    WindowEvent::DidExitFullScreen => {
                        // FIXME
                    }
                    WindowEvent::WillClose => {
                        // FIXME
                    }
                    WindowEvent::DoCommand(cmd) => {
                        match cmd {
                            WindowCommand::Stop => {
                                // FIXME
                            }
                            WindowCommand::Reload => {
                                servo.reload();
                                sync_needed = true;
                            }
                            WindowCommand::NavigateBack => {
                                servo.go_back();
                                sync_needed = true;
                            }
                            WindowCommand::NavigateForward => {
                                servo.go_forward();
                                sync_needed = true;
                            }
                            WindowCommand::OpenLocation => {
                                window.focus_urlbar();
                            }
                            WindowCommand::ZoomIn => {
                                // FIXME
                            }
                            WindowCommand::ZoomOut => {
                                // FIXME
                            }
                            WindowCommand::ZoomToActualSize => {
                                // FIXME
                            }
                            WindowCommand::Load(request) => {
                                let url = ServoUrl::parse(&request).or_else(|error| {
                                    // FIXME: weak
                                    if request.ends_with(".com") || request.ends_with(".org") || request.ends_with(".net") {
                                        ServoUrl::parse(&format!("http://{}", request))
                                    } else {
                                        Err(error)
                                    }
                                }).or_else(|_| {
                                    ServoUrl::parse(&format!("https://duckduckgo.com/html/?q={}", request))
                                });
                                match url {
                                    Ok(url) => {
                                        sync_needed = true;
                                        servo.load_url(url)
                                    },
                                    Err(err) => warn!("Can't parse url: {}", err),
                                }
                            }
                        }
                    }
                }
            }

            for event in view_events {
                match event {
                    ViewEvent::MouseWheel(delta, phase) => {
                        let (x, y) = match delta {
                            view::MouseScrollDelta::PixelDelta(x, y) => {
                                (x, y)
                            },
                            _ => (0.0, 0.0),
                        };
                        servo.perform_scroll(0, 0, x, y, phase);
                        sync_needed = true;
                    }
                }
            }

            for event in servo_events {
                match event {
                    ServoEvent::SetWindowInnerSize(..) => {
                        // ignore
                    }
                    ServoEvent::SetWindowPosition(..) => {
                        // ignore
                    }
                    ServoEvent::SetFullScreenState(fullscreen) => {
                        if fullscreen {
                            view.enter_fullscreen();
                        } else {
                            view.exit_fullscreen();
                        }
                    }
                    ServoEvent::Present => {
                        view.swap_buffers();
                    }
                    ServoEvent::TitleChanged(title) => {
                        window.set_title(&title.unwrap_or("No Title".to_owned()));

                    }
                    ServoEvent::UnhandledURL(url) => {
                        open::that(url.as_str()).ok();

                    }
                    ServoEvent::StatusChanged(..) => {
                        // FIXME
                    }
                    ServoEvent::LoadStart(..) => {
                        window.set_command_state(WindowCommand::Reload, CommandState::Disabled);
                        window.set_command_state(WindowCommand::Stop, CommandState::Enabled);
                    }
                    ServoEvent::LoadEnd(..) => {
                        window.set_command_state(WindowCommand::Reload, CommandState::Enabled);
                        window.set_command_state(WindowCommand::Stop, CommandState::Disabled);
                    }
                    ServoEvent::LoadError(..) => {
                        // FIXME
                    }
                    ServoEvent::HeadParsed(url) => {
                        window.set_url(url.as_str());
                    }
                    ServoEvent::CursorChanged(..) => {
                        // FIXME
                    }
                    ServoEvent::FaviconChanged(..) => {
                        // FIXME
                    }
                    ServoEvent::Key(..) => {
                        // FIXME
                    }
                }
            }

            if sync_needed {
                servo.sync();
            }
        }
    });

}
