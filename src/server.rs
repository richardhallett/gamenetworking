use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{client::Client, net::{Message, ReliableOrderedNetwork, State, UnreliableNetwork}, sim::{Entity, Input}, ticktimer::TickTimer};

/// Represents networked server
pub struct Server {
    id: i32,

    // Timer for fixed tickrate
    tick_timer: TickTimer,

    // The tick rate in milliseconds
    pub tick_rate_ms: u64,

    // The local servers network interface
    network: Rc<RefCell<UnreliableNetwork>>,

    // Map of an id to a client network interface
    connected_clients: HashMap<i32, Rc<RefCell<UnreliableNetwork>>>,

    // Server simulation data
    entities: HashMap<i32, Entity>,

    // List of entities with their last tick rate that was integrated
    last_processed_input: HashMap<i32, i32>,
}

impl Server {
    pub fn new(tick_rate_ms: u64) -> Self {
        Server {
            id: 0,
            tick_timer: TickTimer::new(std::time::Duration::from_millis(tick_rate_ms)),
            tick_rate_ms,
            network: Rc::new(RefCell::new(UnreliableNetwork::new())),
            connected_clients: HashMap::new(),
            entities: HashMap::new(),
            last_processed_input: HashMap::new(),
        }
    }

    pub fn get_network(&self) -> Rc<RefCell<UnreliableNetwork>> {
        Rc::clone(&self.network)
    }

    pub fn get_entities(&self) -> Vec<&Entity> {
        self.entities.values().collect()
    }

    // This is a function to fake connections on our fake network
    // up the network connection.
    // In the real world this would happen via network messages.
    // The server version of stores the client that wants to connect
    // and creates the entity for mirroring.
    pub fn connect(&mut self, client: &mut Client) -> i32 {
        let client_network = client.get_network();
        self.connected_clients.insert(client.get_id(), client_network);

        // Create a new entity for the client
        let mut entity = Entity::new();
        entity.position = (0., 0.);
        entity.colour = client.colour;
        // We just give the entity same id as the client id
        let new_id = client.get_id();
        self.entities.insert(new_id, entity);

        // Return it for assignment
        // In real world this assignment would probably happen via a RPC
        new_id
    }

    pub fn update(&mut self) {

        // Fixed tickrate
        for tick in self.tick_timer.tick() {
            self.process_client_messages();
            self.broadcast_state(tick)
        }
    }

    fn process_client_messages(&mut self) {
        let mut network = self.network.borrow_mut();
        // Process all pending messages from clients
        while let Some((client_id, message)) = network.receive() {
            // Get the entity based on the one we're wanting to update
            let entity = self.entities.get_mut(&client_id).unwrap();

            // Integrate the client input from the message into the sim
            if let Some(input) = message.input {
                entity.integrate_input(&Input {
                    left: input.0,
                    right: input.1,
                    up: input.2,
                    down: input.3,
                });
            }

            // Store the last sequence(or tick in our case) we processed input for
            self.last_processed_input.insert(client_id, message.sequence);
        }
    }

    fn broadcast_state(&mut self, tick: i32) {

        let mut world_state: Vec<State> = Vec::new();

        // Collect the state of all entities
        // In this example we're working with just one entity per client
        for (client_id, _client_network) in self.connected_clients.iter() {
            // Use the client_id to get the entity
            let entity = self.entities.get(client_id).unwrap();

            // Get the last tick we processed input for this entity
            let last_processed_tick = self.last_processed_input.get(client_id).unwrap_or(&0);

            let state = State {
                tick: *last_processed_tick,
                entity_id: *client_id,
                position: entity.position,
                colour: entity.colour,
            };

            // Bundle up the state updates
            world_state.push(state);
        }

        // Broadcast the state to all connected clients
        // This might happen at a different rate than the tickrate
        for (_client_id, client_network) in self.connected_clients.iter() {
            let message = Message {
                state: Some(world_state.clone()),
                input: None, // Unused
                sequence: tick, // Send the server tick so we know what state we're at
            };

            let mut client_network = client_network.borrow_mut();
            client_network.send(self.id, message);
        }
    }
}
