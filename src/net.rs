use std::{collections::VecDeque, time::{Duration, Instant}};

use macroquad::rand;

use crate::sim::Colour;

#[derive(Default, Debug)]
pub struct Message {
    pub sequence: i32,
    pub state: Option<Vec<State>>,
    pub input: Option<(bool, bool, bool, bool)>,
}


#[derive(Default, Debug, Clone, Copy)]
pub struct State {
    pub tick: i32,
    pub entity_id: i32,
    pub position: (f32, f32),
    pub colour: Colour,
}

pub struct ReliableOrderedNetwork {
    messages: VecDeque<(Duration, i32, Message)>,
    timer: Instant,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

impl ReliableOrderedNetwork {
    pub fn new() -> Self {
        ReliableOrderedNetwork {
            messages: VecDeque::new(),
            timer: Instant::now(),
            min_latency_ms: 0,
            max_latency_ms: 0,
        }
    }

    // Send a message along with who sent it
    pub fn send(&mut self, sender_id: i32, message: Message) {
        // Simulate latency between two random values
        let latency = rand::gen_range(self.min_latency_ms, self.max_latency_ms);
        let delay = self.timer.elapsed() + Duration::from_millis(latency);

        self.messages.push_back((delay, sender_id, message));

    }

    // Returns the next message along with sender_id who sent the message
    pub fn receive(&mut self) -> Option<(i32, Message)> {
        if let Some((delay, sender_id, message)) = self.messages.pop_front() {
            // If the delay has passed, we return the message
            if delay <= self.timer.elapsed() {
                return Some((sender_id, message));
            }

            // Otherwise we put it back
            self.messages.push_front((delay, sender_id, message));
        }
        None
    }
}

pub struct UnreliableNetwork {
    messages: VecDeque<(Duration, i32, Message)>,
    timer: Instant,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub drop_rate: f32,
}

impl UnreliableNetwork {
    pub fn new() -> Self {
        UnreliableNetwork {
            messages: VecDeque::new(),
            timer: Instant::now(),
            min_latency_ms: 0,
            max_latency_ms: 0,
            drop_rate: 0.0,
        }
    }

    // Send a message along with who sent it
    pub fn send(&mut self, sender_id: i32, message: Message) {
        // If the message is dropped, we don't send it
        if rand::gen_range(0.0, 1.0) < self.drop_rate {
            return;
        }

        // Simulate latency between two random values
        let latency = rand::gen_range(self.min_latency_ms, self.max_latency_ms);
        let delay = self.timer.elapsed() + Duration::from_millis(latency);

        self.messages.push_back((delay, sender_id, message));

        // Sort the messages by delay as messages can arrive out of order
      //  self.messages.make_contiguous().sort_by(|a, b| a.0.cmp(&b.0));
    }

    // Returns the next message along with sender_id who sent the message
    pub fn receive(&mut self) -> Option<(i32, Message)> {
        if let Some((delay, sender_id, message)) = self.messages.pop_front() {
            // If the delay has passed, we return the message
            if delay <= self.timer.elapsed() {
                return Some((sender_id, message));
            }

            // Otherwise we put it back
            self.messages.push_front((delay, sender_id, message));
        }
        None
    }
}