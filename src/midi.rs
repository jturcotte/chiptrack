// Copyright © 2022 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use midir::{MidiInput, MidiInputConnection};
use midly::{live::LiveEvent, MidiMessage};

pub struct Midi {
    _connections: Vec<MidiInputConnection<()>>,
}

impl Midi {
    pub fn new<F>(callback: F) -> Midi
        where
            F: Fn(u32) + std::clone::Clone + std::marker::Send + 'static,
        {
        let callback2 = move |_stamp: u64, message: &[u8], _: &mut ()| {
            let event = LiveEvent::parse(message).unwrap();
            println!("{:?} hex: {:x?}", event, message);
            match event {
                LiveEvent::Midi { channel, message } => match message {
                    MidiMessage::NoteOn { key, vel } if vel != 0 => {
                        println!("hit note {} on channel {} vel {}", key, channel, vel);
                        callback(key.as_int() as u32);
                    }
                    _ => {}
                },
                _ => {}
            }
        };

        // Just connect to all available input MIDI devices for now.
        let port_count = MidiInput::new("").unwrap().port_count();
        log!("Found {} midi ports", port_count);
        let connections: Vec<_> =
            (0 .. port_count).map(|i| {
                let mut midi_in = MidiInput::new("Chiptrack").unwrap();
                midi_in.ignore(midir::Ignore::All);
                let port = &midi_in.ports()[i];
                log!("Connecting to port {}", midi_in.port_name(&port).unwrap());
                midi_in.connect(&port, &format!("Chiptrack Input Port {}", i), callback2.clone(), ()).unwrap()
            }).collect();

        Midi { _connections: connections }
    }
}
