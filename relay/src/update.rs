use iced::{time::Duration, Task};
use std::fs::File;
use std::io::prelude::*;

use crate::{message::ToTcpThreadMessage, FromIpcThreadMessage, Message, State};

pub(crate) fn update(state: &mut State, message: Message) -> Task<Message> {
    use Message as M;

    match message {
        M::Update => handle_update(state),
        M::WindowCloseRequest(id) => close_window(state, id),
        M::ConnectionMessage => handle_connect_message(state),
        M::ConnectIpc => ipc_connect_message(state),
        M::DisconnectIpc => ipc_disconnect_message(state),
        M::ConnectTcp => tcp_connect_message(state),
        M::DisconnectTcp => tcp_disconnect_message(state),
        // Toggle messages for GUI XML generator
            M::AltitudeToggle(value) => toggle_state(&mut state.altitude_toggle, value),
            M::AirspeedToggle(value) => toggle_state(&mut state.airspeed_toggle, value),
            M::VerticalAirspeedToggle(value) => toggle_state(&mut state.vertical_airspeed_toggle, value),
            M::HeadingToggle(value) => toggle_state(&mut state.heading_toggle, value),
        M::CreateXMLFile => create_xml_file(state),
        // Card Open/Close messages for GUI pop-up-card window
            M::CardOpen => card_open(state),
            M::CardClose => card_close(state),
        M::TcpAddrFieldUpdate(addr) => tcp_addr_field_update(state, addr),
    }
}

fn handle_update(state: &mut State) -> Task<Message> {
    state.elapsed_time += Duration::from_millis(10);

    // check for messages from IPC thread
    if let Some(ipc_bichannel) = &state.ipc_bichannel {
        for message in ipc_bichannel.received_messages() {
            match message {
                FromIpcThreadMessage::BatonData(data) => {
                    state.tcp_bichannel.as_mut().map(|tcp_bichannel| {
                        tcp_bichannel.send_to_child(ToTcpThreadMessage::Send(data.clone()))
                    });
                    state.latest_baton_send = Some(data);
                    state.active_baton_connection = true;
                }
                FromIpcThreadMessage::BatonShutdown => {
                    let _ = state.tcp_disconnect();
                    state.active_baton_connection = false;
                }
            }
        }
    }

    // check for messages from TCP thread
    if let Some(tcp_bichannel) = &state.tcp_bichannel {
        for message in tcp_bichannel.received_messages() {
            match message {
                _ => (),
            }
        }
    }

    Task::none()
}

fn close_window(state: &mut State, id: iced::window::Id) -> Task<Message> {
    // pre-shutdown operations go here
    if let Some(ref bichannel) = state.ipc_bichannel {
        let _ = bichannel.killswitch_engage();
    }

    if let Some(ref bichannel) = state.tcp_bichannel {
        let _ = bichannel.killswitch_engage();
    }

    // delete socket file
    let socket_file_path = if cfg!(target_os = "macos") {
        "/tmp/baton.sock"
    } else {
        // TODO: add branch for Windows; mac branch is just for testing/building
        panic!(
            "No implementation available for given operating system: {}",
            std::env::consts::OS
        )
    };
    std::fs::remove_file(socket_file_path).unwrap();

    // necessary to actually shut down the window, otherwise the close button will appear to not work
    iced::window::close(id)
}

fn handle_connect_message(state: &mut State) -> Task<Message> {
    if let Some(status) = state
        .tcp_bichannel
        .as_ref()
        .and_then(|bichannel| bichannel.is_conn_to_endpoint().ok())
    {
        state.tcp_connected = status
    } else {
        state.tcp_connected = false
    }
    Task::none()
}

fn ipc_connect_message(state: &mut State) -> Task<Message> {
    if let Err(e) = state.ipc_connect() {
        state.log_event(format!("Error: {e:?}"));
    };
    Task::none()
}

fn ipc_disconnect_message(state: &mut State) -> Task<Message> {
    if let Err(e) = state.ipc_disconnect() {
        state.log_event(format!("Error: {e:?}"));
    };
    Task::none()
}

fn tcp_connect_message(state: &mut State) -> Task<Message> {
    let address = state.tcp_addr_field.clone();
    if let Err(e) = state.tcp_connect(address) {
        state.log_event(format!("Error: {e:?}"));
    };
    Task::none()
}

fn tcp_disconnect_message(state: &mut State) -> Task<Message> {
    if let Err(e) = state.tcp_disconnect() {
        state.log_event(format!("Error: {e:?}"));
    };
    Task::none()
}

fn toggle_state(toggle: &mut bool, value: bool) -> Task<Message> {
    *toggle = value;
    Task::none()
}

// Creates a default XML file when a button is clicked in the GUI
fn create_xml_file(state: &mut State) -> Task<Message> {
    // Get the user's downloads directory
    let mut downloads_path =
        dirs::download_dir().expect("Retrieving the user's Downloads file directory.");
    downloads_path.push("iMotions.xml");

    // Create file in downloads directory. If alr there, will overwrite the existing file.
    let mut file = File::create(&downloads_path).expect("Creating XML File.");

    // Check if all dataref toggles are false. If so, return error message
    if !state.altitude_toggle
        && !state.airspeed_toggle
        && !state.vertical_airspeed_toggle
        && !state.heading_toggle
    {
        state.error_message = Some("Please select at least one dataref toggle".into());
        return Task::none();
    }
    state.error_message = None; // Clear previous error

    // NOTE: This XML formatting was found in the PilotDataSync Slack. Double check this is the correct formatting.
    let mut contents = String::from(
        "<EventSource Version=\"1\" Id=\"PilotDataSync\" Name=\"Positional Flight Data\">\n",
    );
    if state.altitude_toggle {
        let mut altitude_str =
            String::from("\t<Sample Id=\"AltitudeSync\" Name=\"Altitude Synchronization\">\n");

        altitude_str.push_str(
            "\t\t<Field Id=\"FlightModelAltitude\" Range=\"Variable\" Min=\"0\" Max=\"50000\" />\n",
        );
        altitude_str.push_str(
            "\t\t<Field Id=\"PilotAltitude\" Range=\"Variable\" Min=\"0\" Max=\"50000\" />\n",
        );
        altitude_str.push_str("\t</Sample>\n");

        contents.push_str(&altitude_str);
    }
    if state.airspeed_toggle {
        let mut airspeed_str =
            String::from("\t<Sample Id=\"AirspeedSync\" Name=\"Airspeed Synchronization\">\n");

        airspeed_str.push_str(
            "\t\t<Field Id=\"FlightModelAirspeed\" Range=\"Variable\" Min=\"0\" Max=\"600\" />\n",
        );
        airspeed_str.push_str(
            "\t\t<Field Id=\"PilotAirspeed\" Range=\"Variable\" Min=\"0\" Max=\"600\" />\n",
        );
        airspeed_str.push_str("\t</Sample>\n");

        contents.push_str(&airspeed_str);
    }
    if state.vertical_airspeed_toggle {
        let mut vertical_airspeed_str = String::from(
            "\t<Sample Id=\"VerticalVelocitySync\" Name=\"Vertical Velocity Synchronization\">\n",
        );

        vertical_airspeed_str.push_str("\t\t<Field Id=\"FlightModelVerticalVelocity\" Range=\"Variable\" Min=\"-5000\" Max=\"5000\" />\n");
        vertical_airspeed_str.push_str("\t\t<Field Id=\"PilotVerticalVelocity\" Range=\"Variable\" Min=\"-5000\" Max=\"5000\" />\n");
        vertical_airspeed_str.push_str("\t</Sample>\n");

        contents.push_str(&vertical_airspeed_str);
    }
    if state.heading_toggle {
        let mut heading_str =
            String::from("\t<Sample Id=\"HeadingSync\" Name=\"Heading Synchronization\">\n");

        heading_str.push_str(
            "\t\t<Field Id=\"FlightModelHeading\" Range=\"Variable\" Min=\"0\" Max=\"360\" />\n",
        );
        heading_str.push_str(
            "\t\t<Field Id=\"PilotHeading\" Range=\"Variable\" Min=\"0\" Max=\"360\" />\n",
        );
        heading_str.push_str("\t</Sample>\n");

        contents.push_str(&heading_str);
    }
    contents.push_str("</EventSource>");

    // Write XML file
    file.write_all(contents.as_bytes())
        .expect("Writing to XML file");

    Task::none() // Return type that we need for the Update logic
}

fn card_open(state: &mut State) -> Task<Message> {
    state.card_open = true;
    Task::none()
}

fn card_close(state: &mut State) -> Task<Message> {
    state.card_open = false;
    Task::none()
}

fn tcp_addr_field_update(state: &mut State, addr: String) -> Task<Message> {
    // Update the TCP address text input in the GUI
    let is_chars_valid = addr.chars().all(|c| c.is_numeric() || c == '.' || c == ':');
    let dot_count = addr.chars().filter(|&c| c == '.').count();
    let colon_count = addr.chars().filter(|&c| c == ':').count();
    if is_chars_valid && dot_count <= 3 && colon_count <= 1 {
        state.tcp_addr_field = addr;
    }
    Task::none()
}



// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::ChannelMessage;
//     use std::sync::mpsc;
//     use std::time::Duration;

//     #[test]
//     fn test_handle_baton_msg_data() {
//         let (txx, rxx) = mpsc::channel();
//         txx.send(IpcThreadMessage::BatonData("test".into()))
//             .unwrap();

//         let mut state = State {
//             elapsed_time: Duration::ZERO,
//             ipc_conn_thread_handle: None,
//             tx_kill: None,
//             rx_baton: Some(rxx),
//             latest_baton_send: None,
//             recv: None,
//             connection_status: None,
//             active_baton_connection: false,
//         };

//         let _ = handle_baton_msg(&mut state);

//         assert_eq!(state.latest_baton_send, Some("test".into()));
//         assert!(state.active_baton_connection);
//     }

//     #[test]
//     fn test_handle_baton_msg_shutdown() {
//         let (txx, rxx) = mpsc::channel();
//         txx.send(IpcThreadMessage::BatonShutdown).unwrap();

//         let mut state = State {
//             elapsed_time: Duration::ZERO,
//             ipc_conn_thread_handle: None,
//             tx_kill: None,
//             rx_baton: Some(rxx),
//             latest_baton_send: None,
//             recv: None,
//             connection_status: None,
//             active_baton_connection: true,
//         };

//         let _ = handle_baton_msg(&mut state);

//         assert!(!state.active_baton_connection);
//     }

//     #[test]
//     fn test_update_connection_status_connect() {
//         let (send, recv) = std::sync::mpsc::channel::<ChannelMessage>();
//         let _ = send.send(ChannelMessage::Connect);

//         let mut state = State {
//             elapsed_time: Duration::ZERO,
//             ipc_conn_thread_handle: None,
//             tx_kill: None,
//             rx_baton: None,
//             latest_baton_send: None,
//             recv: Some(recv),
//             connection_status: None,
//             active_baton_connection: false,
//         };

//         let _ = update_connection_status(&mut state);

//         // If you add the `PartialEq` trait to the `ChannelMessage` enum, you can directly assert_eq!(val, enum).
//         match state.connection_status {
//             Some(ChannelMessage::Connect) => assert!(true),
//             _ => assert!(false),
//         };
//     }

//     #[test]
//     fn test_update_connection_status_disconnected() {
//         let (send, recv) = std::sync::mpsc::channel::<ChannelMessage>();
//         let _ = send.send(ChannelMessage::Disconnected);

//         let mut state = State {
//             elapsed_time: Duration::ZERO,
//             ipc_conn_thread_handle: None,
//             tx_kill: None,
//             rx_baton: None,
//             latest_baton_send: None,
//             recv: Some(recv),
//             connection_status: None,
//             active_baton_connection: false,
//         };

//         let _ = update_connection_status(&mut state);

//         match state.connection_status {
//             Some(ChannelMessage::Disconnected) => assert!(true),
//             _ => assert!(false),
//         };
//     }

//     // Unable to test close_window() due to how the ICED gui closes the window.
// }
