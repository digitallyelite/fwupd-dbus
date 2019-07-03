use fwupd_dbus::{Client, Signal};
use std::{
    error::Error,
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

fn main() {
    if let Err(why) = main_() {
        let mut error = format!("error: {}", why);
        let mut cause = why.source();
        while let Some(why) = cause {
            error.push_str(&format!("\n    caused by: {}", why));
            cause = why.source();
        }

        eprintln!("{}", error);
        process::exit(1);
    }
}

fn main_() -> Result<(), Box<dyn Error>> {
    // Atomic value used to stop the background thread.
    let cancellable = Arc::new(AtomicBool::new(false));

    // Begin listening to signals in the background
    listen_in_background(cancellable.clone());

    // Create a new dbus client connection.
    let client = &Client::new()?;

    println!("Version: {}", client.daemon_version()?);
    println!("Status: {:?}", client.status()?);
    println!("Tainted: {}", client.tainted()?);
    if let Ok(percent) = client.percentage() {
        println!("Percentage; {}", percent);
    }

    // Fetch a list of supported devices.
    for device in client.devices()? {
        println!("Device: {} {}", device.vendor, device.name);

        if device.is_updateable() {
            if let Ok(upgrades) = client.upgrades(&device) {
                println!("  upgrades found");
                for upgrade in upgrades {
                    println!("{:#?}", upgrade);
                }
            } else {
                println!("  no updates available");
            }

            if let Ok(downgrades) = client.downgrades(&device) {
                println!("  downgrades found");
                for downgrade in downgrades {
                    println!("{:#?}", downgrade);
                }
            }

            if let Ok(releases) = client.releases(&device) {
                println!("   releases found");
                for release in releases {
                    println!("{:#?}", release);
                }
            }
        } else {
            println!("  device not updateable");
        }
    }

    let http_client = &reqwest::Client::new();

    // Fetch a list of remotes, and update them.
    for remote in client.remotes()? {
        println!("{:#?}", remote);

        remote.update_metadata(client, http_client)?;
    }

    // Stop listening to signals in the background.
    cancellable.store(true, Ordering::SeqCst);

    Ok(())
}

fn listen_in_background(cancellable: Arc<AtomicBool>) {
    thread::spawn(move || {
        if let Ok(client) = Client::new() {
            // Listen for signals received by the daemon.
            for signal in client.listen_signals(cancellable) {
                match signal {
                    Signal::Changed => {
                        println!("changed");
                    }
                    Signal::DeviceAdded(device) => {
                        println!("device added: {:?}", device);
                    }
                    Signal::DeviceChanged(device) => {
                        println!("device changed: {:?}", device);
                    }
                    Signal::DeviceRemoved(device) => {
                        println!("device added: {:?}", device);
                    }
                    Signal::PropertiesChanged {
                        interface,
                        changed,
                        invalidated,
                    } => {
                        println!(
                            "Properties of {} changed:\n \
                             changed: {:?}\n \
                             invalidated: {:?}",
                            interface, changed, invalidated
                        );
                    }
                }
            }
        }
    });
}
