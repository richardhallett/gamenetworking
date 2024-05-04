use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{client::Client, net::{Message, State, UnreliableNetwork}, sim::{Entity, Input, World}, ticktimer::TickTimer};

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
    pub world: World,

    npc_entities: Vec<i32>,

    // Map of the network id to the local sim entity id
    networked_players: HashMap<i32, i32>,

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
            world: World::new(),
            npc_entities: Vec::new(),
            networked_players: HashMap::new(),
            last_processed_input: HashMap::new(),
        }
    }

    pub fn get_network(&self) -> Rc<RefCell<UnreliableNetwork>> {
        Rc::clone(&self.network)
    }

    pub fn create_npc_entities(&mut self) {
        // Create non player entities
        let mut entity = Entity::new();
        entity.position = (100., 100.);
        entity.colour = crate::sim::Colour::Blue;
        let npc_id = self.world.add_entity(entity);

        self.npc_entities.push(npc_id);
    }

    pub fn update_npc_entities(&mut self, tick: i32) {
        for npc_id in self.npc_entities.iter() {
            let entity = self.world.get_entity(*npc_id).unwrap();

            // Move the npc entities in a circle
            let ticks_ms = tick as f32 * self.tick_rate_ms as f32;
            let angle = ticks_ms / 1000.0;
            let new_x = entity.position.0 + angle.cos() * 5.0;
            let new_y = entity.position.1 + angle.sin() * 5.0;

            entity.position = (new_x, new_y);
        }
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
        let entity_id = self.world.add_entity(entity);

        // Store the network id to the entity id
        self.networked_players.insert(client.get_id(), entity_id);

        // Return it for assignment
        // In real world this assignment would probably happen via a RPC
        entity_id
    }

    pub fn update(&mut self) {

        // Fixed tickrate
        for tick in self.tick_timer.tick() {
            //println!("Server tick: {}", tick);
            self.update_npc_entities(tick);

            self.process_client_messages();
            self.broadcast_state(tick)
        }
    }

    fn process_client_messages(&mut self) {
        let mut network = self.network.borrow_mut();
        // Process all pending messages from clients
        while let Some((client_id, message)) = network.receive() {
            // Get the entity based on the one we're wanting to update
            // Look up the entity id based on the network id
            let local_entity_id = self.networked_players.get(&client_id).unwrap();

            let entity = self.world.get_entity(*local_entity_id).unwrap();

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
        for (entity_id, entity) in self.world.get_entities() {
            let state = State {
                entity_id: *entity_id,
                position: entity.position,
                colour: entity.colour,
            };

            world_state.push(state);
        }

        // Broadcast the state to all connected clients
        // This might happen at a different rate than the tickrate
        for (client_id, client_network) in self.connected_clients.iter() {

            let last_processed_tick = self.last_processed_input.get(client_id).unwrap_or(&0);

            let message = Message {
                state: Some(world_state.clone()),
                input: None, // Unused
                sequence: *last_processed_tick, // Send the server tick so we know what state we're at
            };

            let mut client_network = client_network.borrow_mut();
            client_network.send(self.id, message);
        }
    }
}
