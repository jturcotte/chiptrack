// Copyright © 2022 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use midir::{MidiInput, MidiInputConnection};
use midly::{live::LiveEvent, MidiMessage};

pub struct Midi {
    _connections: Vec<MidiInputConnection<()>>,
}

impl Midi {
    pub fn new<F, G>(press_callback: F, release_callback: G) -> Midi
    where
        F: Fn(u8) + std::clone::Clone + std::marker::Send + 'static,
        G: Fn(u8) + std::clone::Clone + std::marker::Send + 'static,
    {
        let callback2 = move |_stamp: u64, message: &[u8], _: &mut ()| {
            let event = LiveEvent::parse(message).unwrap();
            println!("{:?} hex: {:x?}", event, message);
            if let LiveEvent::Midi { channel, message } = event {
                match message {
                    MidiMessage::NoteOn { key, vel } if vel == 0 => {
                        println!("release note {} on channel {} vel {}", key, channel, vel);
                        release_callback(key.as_int());
                    }
                    MidiMessage::NoteOn { key, vel } => {
                        println!("press note {} on channel {} vel {}", key, channel, vel);
                        press_callback(key.as_int());
                    }
                    MidiMessage::NoteOff { key, vel } => {
                        println!("release note {} on channel {} vel {}", key, channel, vel);
                        release_callback(key.as_int());
                    }
                    _ => {}
                }
            }
        };

        // Just connect to all available input MIDI devices for now.
        let port_count = MidiInput::new("").unwrap().port_count();
        log!("Found {} MIDI ports", port_count);
        let connections: Vec<_> = (0..port_count)
            .filter_map(|i| {
                let mut midi_in = MidiInput::new("Chiptrack").unwrap();
                midi_in.ignore(midir::Ignore::All);
                let port = &midi_in.ports()[i];
                log!("Connecting to port {}", midi_in.port_name(port).unwrap());
                midi_in
                    .connect(port, &format!("Chiptrack Input Port {}", i), callback2.clone(), ())
                    .map_err(|e| elog!("Couldn't connect to MIDI input: {}", e))
                    .ok()
            })
            .collect();

        Midi {
            _connections: connections,
        }
    }
}
