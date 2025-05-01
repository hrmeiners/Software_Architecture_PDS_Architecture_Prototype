use iced::{
    widget::{button, column, container, text},
    Element,
};

use crate::{Message, State};

// Render the full UI view
pub(crate) fn view(state: &State) -> Element<Message> {
    column![
        render_elapsed_time(state),
        baton_text(state),
        render_connection_text(state),
        render_status_button(),
    ]
    .into()
}


// Renders elapsed time text
fn render_elapsed_time(state: &State) -> Element<Message> {
    text(format!("Elapsed time: {:?}", state.elapsed_time)).into()
}

// Renders baton elevation or no-data text
fn baton_text(state: &State) -> Element<Message> {
    let baton_data = match &state.latest_baton_send {
        Some(num) => format!("[Baton] Pilot Elevation: {num:.3} ft"),
        None => "No data from baton.".into(),
    };
    text(baton_data).into()
}

// Renders connection status text (normal connection status + baton connection)
fn render_connection_text(state: &State) -> Element<Message> {
    let connection_status = match &state.connection_status {
        Some(channel_msg) => format!("{:?}", channel_msg),
        None => "No connection established".to_string(),
    };

    let baton_connection_status = if state.active_baton_connection {
        format!(":) Baton Connected!")
    } else {
        format!(":( No Baton Connection")
    };

    // Combine connection status + baton connection in a small column
    container(
        column![
            text(format!("Connection Status: {}", connection_status)),
            text(baton_connection_status)
        ]
    )
    .padding(10)
    .center(400)
    .style(container::rounded_box)
    .into()
}

// Renders the button to check connection status
fn render_status_button<'a>() -> Element<'a, Message> {
    button("Check Connection Status")
        .on_press(Message::ConnectionMessage)
        .into()
}

/*
We decided to not include unit tests from the UI, as our previous tests were just smoke tests 
    (i.e. doing nothing the Rust static code analyzer wasn't already doing).
If we have conditional UI elements being displayed, perhaps this could change, 
    but since we are already testing state manipulation in the `update.rs` file, they seemed redundant.
*/