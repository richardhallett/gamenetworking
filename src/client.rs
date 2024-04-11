use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use macroquad::input::{is_key_down, KeyCode};

use crate::{
    net::{Message, ReliableOrderedNetwork, State, UnreliableNetwork},
    server::Server,
    sim::{Colour, Entity, Input},
    ticktimer::TickTimer,
};

/// Represents networked client
pub struct Client {
    id: i32,

    // Timer for fixed tickrate
    pub tick_timer: TickTimer,

    // The tick rate in milliseconds
    pub tick_rate_ms: u64,

    // Network interface for sending and receiving messages to this client
    // The RC/RefCell is for mutable borrowing to the client network
    pub network: Rc<RefCell<UnreliableNetwork>>,
    // The RC/RefCell is for mutable borrowing to the server network
    server_network: Option<Rc<RefCell<UnreliableNetwork>>>,

    // Client simulation data
    entities: HashMap<i32, Entity>,

    // The entity that the client controls
    controlled_entity: Option<i32>,

    // The current state of the input, to be used for sending to server and
    // processing locally
    input_state: Option<Input>,

    // To keep track of pending inputs for reconciliation
    // We store the processed sequence(tick) and the input
    pub input_history: VecDeque<(i32, Input)>,

    pub last_message_sequence: i32,

    pub client_prediction_enabled: bool,
    pub server_reconciliation_enabled: bool,

    pub extrapolation_enabled: bool,
    // Stores the state snapshots from the server for use with extrapolation
    pub state_snapshots: VecDeque<(i32, State)>,

    pub use_alternate_input: bool,
    pub colour: Colour,

    pub connected: bool,
}

impl Client {
    pub fn new(id: i32, tick_rate_ms: u64) -> Self {
        Client {
            id,
            tick_timer: TickTimer::new(std::time::Duration::from_millis(tick_rate_ms)),
            tick_rate_ms,
            network: Rc::new(RefCell::new(UnreliableNetwork::new())),
            server_network: None,
            entities: HashMap::new(),
            controlled_entity: None,
            input_state: None,
            input_history: VecDeque::new(),
            last_message_sequence: 0,
            client_prediction_enabled: true,
            server_reconciliation_enabled: true,
            extrapolation_enabled: true,
            state_snapshots: VecDeque::new(),
            use_alternate_input: false,
            colour: Colour::Red,
            connected: false,
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn get_entities(&self) -> Vec<&Entity> {
        self.entities.values().collect()
    }

    pub fn get_network(&self) -> Rc<RefCell<UnreliableNetwork>> {
        Rc::clone(&self.network)
    }

    // This is a function to fake connections on our fake network
    // up the network connection.
    // In the real world this would happen via network messages.
    // The client version sets its own controlled entity
    pub fn connect(&mut self, server: &mut Server, min_latency_ms: u64, max_latency_ms: u64, drop_rate: f32) {
        let client_entity_id = server.connect(self);
        let server_network = server.get_network();

        // Set the same latency for both client and server
        server_network.borrow_mut().min_latency_ms = min_latency_ms;
        server_network.borrow_mut().max_latency_ms = max_latency_ms;
        server_network.borrow_mut().drop_rate = drop_rate;

        self.network.borrow_mut().min_latency_ms = min_latency_ms;
        self.network.borrow_mut().max_latency_ms = max_latency_ms;
        self.network.borrow_mut().drop_rate = drop_rate;

        // Store the server network for sending messages to the server
        self.server_network = Some(server_network);

        // Set controlled entity to the entity we got from the server
        // As in server this probably would have happened over RPC assignment
        self.controlled_entity = Some(client_entity_id);

        self.connected = true;
    }

    pub fn update(&mut self) {
        if !self.connected {
            return;
        }

        self.get_input();

        // Fixed tickrate
        for tick in self.tick_timer.tick() {
            // Listen to the server and process server messages
            self.process_server_messages(tick);

            // If we don't have a controlled entity we're not connected so don't do anything
            if self.controlled_entity.is_none() {
                continue;
            }

            // Interpolate entities
            if self.extrapolation_enabled {
                self.interpolate_entities(tick);
            }

            // Process input and send it to the server
            self.process_input();
        }
    }

    fn process_server_messages(&mut self, tick: i32) {
        let mut network = self.network.borrow_mut();
        while let Some((_sender_id, message)) = network.receive() {
            // If message sequence is less than the last processed message
            // we ignore it as it's out of sequence and therefore old
            if message.sequence < self.last_message_sequence {
                continue;
            } else {
                self.last_message_sequence = message.sequence;
            }

            // In this example entities represent the world state
            if let Some(world_state) = message.state {
                for state in world_state {
                    // If the entity in state update is not created locally then create
                    if !self.entities.contains_key(&state.entity_id) {
                        self.entities.insert(
                            state.entity_id,
                            Entity {
                                position: state.position,
                                speed: 5.0,
                                colour: state.colour,
                            },
                        );
                    }

                    // Get the entity from the message
                    let entity = self.entities.get_mut(&state.entity_id).unwrap();

                    if self
                        .controlled_entity
                        .is_some_and(|id| id == state.entity_id)
                    {
                        // Set authoriative position to whatever server says
                        entity.position = state.position;

                        if self.server_reconciliation_enabled {
                            // Reconciliation
                            // We re-apply all inputs that the server hasn't processed yet
                            // This is based on the last processed input tick
                            // We need to reapply up to the latest current tick
                            let last_sync_tick = state.tick + 1;

                            // We only keep inputs that are newer than the last processed tick from server
                            // So we're only removing stuff the server has already said it's processed
                            self.input_history
                                .retain(|(input_tick, _)| *input_tick >= last_sync_tick);

                            for (_input_tick, input) in &self.input_history {
                                let entity = self.entities.get_mut(&state.entity_id).unwrap();
                                entity.integrate_input(&input);
                            }
                        } else {
                            // Disabled so drop all input history
                            self.input_history.clear();
                        }
                    } else {
                        if self.extrapolation_enabled {
                            // Store the state for use with extrapolation
                            self.state_snapshots.push_back((tick, state));
                        } else {
                            // Extrapolation disabled so just set the position
                            entity.position = state.position;
                        }
                    }
                }
            }
        }
    }

    fn interpolate_entities(&mut self, tick: i32) {
        for (entity_id, entity) in &mut self.entities {
            // Smoothing value
            let smoothing_rate = 10;

            // This tick should probably match the server?
            let render_tick = tick - smoothing_rate;

            // Ignore the controlled entity
            if self.controlled_entity.is_some_and(|id| id == *entity_id) {
                continue;
            }

            // Interpolate between the two latest snapshots
            if self.state_snapshots.len() >= 2 {
                // Drop the older snapshots
                while let Some((snapshot_tick, _)) = self.state_snapshots.get(1) {
                    if self.state_snapshots.len() >= 2 && *snapshot_tick <= render_tick {
                        self.state_snapshots.pop_front();
                    } else {
                        break;
                    }
                }

                if let Some((snapshot1_tick, snapshot1_state)) = self.state_snapshots.get(0) {
                    if let Some((snapshot2_tick, snapshot2_state)) = self.state_snapshots.get(1) {

                        if snapshot1_tick <= &render_tick && snapshot2_tick >= &render_tick {
                            let x0 = snapshot1_state.position.0;
                            let x1 = snapshot2_state.position.0;
                            let y0 = snapshot1_state.position.1;
                            let y1 = snapshot2_state.position.1;

                            let t0 = snapshot1_tick;
                            let t1 = snapshot2_tick;

                            // Difference between the two snapshots
                            let delta = t1 - t0;
                            let time_since_snapshot = render_tick - t0;
                            let lerp_fac = time_since_snapshot as f32 / delta as f32;

                            let position = (
                                x0 + (x1 - x0) * lerp_fac,
                                y0 + (y1 - y0) * lerp_fac,
                            );

                            entity.position = position;
                        }
                    }
                }
            }
        }
    }

    /// Gets the current input state
    fn get_input(&mut self) {
        let left: bool;
        let right: bool;
        let up: bool;
        let down: bool;

        if !self.use_alternate_input {
            left = is_key_down(KeyCode::A);
            right = is_key_down(KeyCode::D);
            up = is_key_down(KeyCode::W);
            down = is_key_down(KeyCode::S);
        } else {
            left = is_key_down(KeyCode::Left);
            right = is_key_down(KeyCode::Right);
            up = is_key_down(KeyCode::Up);
            down = is_key_down(KeyCode::Down);
        }

        if left || right || up || down {
            self.input_state = Some(Input {
                left,
                right,
                up,
                down,
            });
        }
    }

    fn process_input(&mut self) {
        if let Some(server_network) = &self.server_network {
            let mut server_network = server_network.borrow_mut();

            if let Some(input_state) = self.input_state.take() {
                // Send an update to server with the latest input
                // We also send the local tick this can then
                // be sent back and later used for reconciliation the
                // differences between client and server.
                server_network.send(
                    self.id,
                    Message {
                        state: None,
                        // We can use the current tick as the input sequence number
                        sequence: self.tick_timer.current_tick,
                        input: Some((
                            input_state.left,
                            input_state.right,
                            input_state.up,
                            input_state.down,
                        )),
                    },
                );

                // Client side prediction
                // We let the client carry out it's local simulation changes
                if self.client_prediction_enabled {
                    if let Some(entity) = self.entities.get_mut(&self.controlled_entity.unwrap()) {
                        entity.integrate_input(&input_state);
                    }
                }

                // Store the input for reconciliation
                self.input_history
                    .push_back((self.tick_timer.current_tick, input_state));
            }
        }
    }
}
