use leptos::{leptos_server::ServerFn, ServerFnError};

pub fn register() -> Result<(), ServerFnError> {
    use crate::app::{ListSerialPorts, ToggleLaser};

    ListSerialPorts::register()?;
    ToggleLaser::register()?;

    Ok(())
}
