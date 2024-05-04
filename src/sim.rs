use std::collections::HashMap;

#[derive(Default, Debug, Clone, Copy)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Colour {
    #[default]
    Red,
    Green,
    Blue,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Entity {
    pub position: (f32, f32),
    pub speed: f32,
    pub colour: Colour,
}

impl Entity {
    pub fn new() -> Self {
        Entity {
            position: (0.0, 0.0),
            speed: 5.0,
            colour: Colour::Red,
        }
    }

    pub fn integrate_input(&mut self, input: &Input) {
        if input.left {
            self.position.0 -= self.speed;
        }
        if input.right {
            self.position.0 += self.speed;
        }
        if input.up {
            self.position.1 -= self.speed;
        }
        if input.down {
            self.position.1 += self.speed;
        }
    }
}

pub struct World {
    entities: HashMap<i32, Entity>,
    latest_entity_id: i32,
}

impl World {
    pub fn new() -> Self {
        World {
            entities: HashMap::new(),
            latest_entity_id: 0,
        }
    }

    pub fn add_entity(&mut self, entity: Entity) -> i32 {
        self.latest_entity_id += 1;
        self.entities.insert(self.latest_entity_id, entity);
        self.latest_entity_id
    }

    pub fn get_entity(&mut self, entity_id: i32) -> Option<&mut Entity> {
        self.entities.get_mut(&entity_id)
    }

    pub fn get_entities(&self) -> &HashMap<i32, Entity> {
        &self.entities
    }

    pub fn get_entities_mut(&mut self) -> &mut HashMap<i32, Entity> {
        &mut self.entities
    }

}