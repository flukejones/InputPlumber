use std::{error::Error, fmt::Debug};

use crate::{
    drivers::opineo::{
        driver::{self, Driver},
        event,
    },
    input::{
        capability::{Capability, Touch, Touchpad},
        event::{native::NativeEvent, value::InputValue},
        source::{InputError, SourceInputDevice, SourceOutputDevice},
    },
    udev::device::UdevDevice,
};

/// The OrangePi Neo has two touchpads; one on the left and one on the right
#[derive(Debug, Clone, Copy)]
enum TouchpadSide {
    Unknown,
    Left,
    Right,
}

/// OrangePi Neo Touchpad source device implementation
pub struct OrangePiNeoTouchpad {
    driver: Driver,
    side: TouchpadSide,
}

impl OrangePiNeoTouchpad {
    /// Create a new OrangePi Neo touchscreen source device with the given udev
    /// device information
    pub fn new(device_info: UdevDevice) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Query the udev module to determine if this is the left or right touchpad.
        let name = device_info.name();
        let touchpad_side = {
            if name == "OPI0001:00" {
                log::debug!("Detected left pad.");
                TouchpadSide::Left
            } else if name == "OPI0002:00" {
                log::debug!("Detected right pad.");
                TouchpadSide::Right
            } else {
                log::debug!("Unable to detect pad side.");
                TouchpadSide::Unknown
            }
        };
        let driver = Driver::new(device_info)?;

        Ok(Self {
            driver,
            side: touchpad_side,
        })
    }
}

impl SourceInputDevice for OrangePiNeoTouchpad {
    /// Poll the given input device for input events
    fn poll(&mut self) -> Result<Vec<NativeEvent>, InputError> {
        let events = self.driver.poll()?;
        let native_events = translate_events(events, self.side);
        Ok(native_events)
    }

    /// Returns the possible input events this device is capable of emitting
    fn get_capabilities(&self) -> Result<Vec<Capability>, InputError> {
        Ok(CAPABILITIES.into())
    }
}

impl SourceOutputDevice for OrangePiNeoTouchpad {}

impl Debug for OrangePiNeoTouchpad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrangePiNeoTouchpad")
            .field("side", &self.side)
            .finish()
    }
}

// Returns a value between 0.0 and 1.0 based on the given value with its
// maximum.
fn normalize_unsigned_value(raw_value: f64, max: f64) -> f64 {
    raw_value / max
}

/// Normalize the value to something between -1.0 and 1.0 based on the Deck's
/// minimum and maximum axis ranges.
fn normalize_axis_value(event: event::TouchAxisInput) -> InputValue {
    let max = driver::PAD_X_MAX;
    let x = normalize_unsigned_value(event.x as f64, max);

    let max = driver::PAD_Y_MAX;
    let y = normalize_unsigned_value(event.y as f64, max);

    // If this is an UP event, don't override the position of X/Y
    let (x, y) = if !event.is_touching {
        (None, None)
    } else {
        (Some(x), Some(y))
    };

    InputValue::Touch {
        index: event.index,
        is_touching: event.is_touching,
        pressure: Some(1.0),
        x,
        y,
    }
}

/// Translate the given OrangePi NEO events into native events
fn translate_events(events: Vec<event::Event>, touchpad_side: TouchpadSide) -> Vec<NativeEvent> {
    let mut translated = Vec::with_capacity(events.len());
    for event in events.into_iter() {
        translated.push(translate_event(event, touchpad_side));
    }
    translated
}

/// Translate the given OrangePi NEO event into a native event
fn translate_event(event: event::Event, touchpad_side: TouchpadSide) -> NativeEvent {
    match event {
        event::Event::TouchAxis(axis) => match touchpad_side {
            TouchpadSide::Unknown => {
                NativeEvent::new(Capability::NotImplemented, InputValue::Bool(false))
            }
            TouchpadSide::Left => NativeEvent::new(
                Capability::Touchpad(Touchpad::LeftPad(Touch::Motion)),
                normalize_axis_value(axis),
            ),
            TouchpadSide::Right => NativeEvent::new(
                Capability::Touchpad(Touchpad::RightPad(Touch::Motion)),
                normalize_axis_value(axis),
            ),
        },
        //_ => NativeEvent::new(Capability::NotImplemented, InputValue::Bool(false)),
    }
}

/// List of all capabilities that the OrangePi NEO driver implements
pub const CAPABILITIES: &[Capability] = &[
    Capability::Touchpad(Touchpad::LeftPad(Touch::Motion)),
    Capability::Touchpad(Touchpad::RightPad(Touch::Motion)),
];
