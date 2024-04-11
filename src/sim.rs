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