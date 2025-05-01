use iced::{time::Duration, Task};

use crate::{IpcThreadMessage, Message, State};

pub(crate) fn update(state: &mut State, message: Message) -> Task<Message> {
    use Message as M;

    #[allow(unreachable_patterns)]
    match message {
        M::Update => {
            state.elapsed_time += Duration::from_secs(1);
            Task::none()
        }
        M::WindowCloseRequest(id) => {
            // pre-shutdown operations go here
            if let Some(ref tx) = state.tx_kill {
                let _ = tx.send(());
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
        M::BatonMessage => {
            // if we get a message from the ipc_connection_thread, do something with it
            match state.rx_baton.as_ref().and_then(|rx| rx.try_recv().ok()) {
                Some(IpcThreadMessage::BatonData(data)) => {
                    state.latest_baton_send = Some(data);
                    state.active_baton_connection = true;
                }
                Some(IpcThreadMessage::BatonShutdown) => state.active_baton_connection = false,
                None => { /* do nothing */ }
            };
            Task::none()
        }

      M::ConnectionMessage => {
        println!("Check Connection Status");
        if let Some(status) = state.recv.as_ref().and_then(|recv| recv.try_recv().ok()){
            state.connection_status = Some(status)
        }                                                                                           
        Task::none()
            
        },
        _ => Task::none(),
    }
}



// Attempt to create unit tests for the update.rs file
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChannelMessage;
    use std::sync::mpsc;
    use std::time::Duration;

    // Control test if Update function handles actual Baton message data correctly
    #[test]
    fn control_test_handle_baton_msg_data() {
        let (txx, rxx) = mpsc::channel();
        txx.send(IpcThreadMessage::BatonData("test".into())).unwrap();

        let mut state = State {
            elapsed_time: Duration::ZERO,
            ipc_conn_thread_handle: None,
            tx_kill: None,
            rx_baton: Some(rxx),
            latest_baton_send: None,
            recv: None,
            connection_status: None,
            active_baton_connection: false,
        };

        let _ = update(&mut state, Message::BatonMessage);

        assert_eq!(state.latest_baton_send, Some("test".into()));
        assert!(state.active_baton_connection);
    }

    // Control test if update function sets Baton connection correctly to false value
    #[test]
    fn control_test_handle_baton_msg_shutdown() {
        let (txx, rxx) = mpsc::channel();
        txx.send(IpcThreadMessage::BatonShutdown).unwrap();

        let mut state = State {
            elapsed_time: Duration::ZERO,
            ipc_conn_thread_handle: None,
            tx_kill: None,
            rx_baton: Some(rxx),
            latest_baton_send: None,
            recv: None,
            connection_status: None,
            active_baton_connection: true,
        };

        let _ = update(&mut state, Message::BatonMessage);

        assert!(!state.active_baton_connection);
    }

    // Control test if update function updates the state's connection status correctly to Connected
    #[test]
    fn control_test_update_connection_status_connect() {
        let (send, recv) = std::sync::mpsc::channel::<ChannelMessage>();
        let _ = send.send(ChannelMessage::Connect);

        let mut state = State {
            elapsed_time: Duration::ZERO,
            ipc_conn_thread_handle: None,
            tx_kill: None,
            rx_baton: None,
            latest_baton_send: None,
            recv: Some(recv),
            connection_status: None,
            active_baton_connection: false,
        };

        let _ = update(&mut state, Message::ConnectionMessage);

        // If you add the `PartialEq` trait to the `ChannelMessage` enum, you can directly assert_eq!(val, enum).
        match state.connection_status {
            Some(ChannelMessage::Connect) => assert!(true),
            _ => assert!(false),
        };
    }

    // Control test for if the update function updates the state's connection status correctly to Disconnected
    #[test]
    fn control_test_update_connection_status_disconnected() {
        let (send, recv) = std::sync::mpsc::channel::<ChannelMessage>();
        let _ = send.send(ChannelMessage::Disconnected);

        let mut state = State {
            elapsed_time: Duration::ZERO,
            ipc_conn_thread_handle: None,
            tx_kill: None,
            rx_baton: None,
            latest_baton_send: None,
            recv: Some(recv),
            connection_status: None,
            active_baton_connection: false,
        };

        let _ = update(&mut state, Message::ConnectionMessage);

        match state.connection_status {
            Some(ChannelMessage::Disconnected) => assert!(true),
            _ => assert!(false),
        };
    }

    // Unable to test close_window() due to how the ICED gui closes the window. 
    // This is the same even in the control test and out of our hands :(
}