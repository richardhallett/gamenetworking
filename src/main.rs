use macroquad::input::{is_key_pressed, KeyCode};

use client::Client;
use macroquad::{prelude::*, ui::*};
use sim::Entity;

mod client;
mod net;
mod server;
mod sim;
mod ticktimer;

fn create_grid_camera(width: f32, height: f32) -> Camera2D {
    let rect = Rect::new(0., 0., width, height);
    let target = vec2(rect.x + rect.w / 2., rect.y + rect.h / 2.);
    Camera2D {
        target,
        zoom: vec2(1. / rect.w * 2., 1. / rect.h * 2.),
        ..Default::default()
    }
}

fn draw_client(client: &Client) {
    draw_text(
        format!("Client {}", client.get_id()).as_str(),
        20.,
        20.,
        32.,
        WHITE,
    );

    if !client.connected {
        draw_text(
            format!("Press {} to connect", client.get_id()).as_str(),
            20.,
            40.,
            16.,
            WHITE,
        );
    }

    // Draw tick rate
    draw_text(
        format!("Tick Rate: {}ms", client.tick_rate_ms).as_str(),
        20.,
        60.,
        16.,
        WHITE,
    );
    // Draw input history
    draw_text(
        format!("Input History: {}", client.input_history.len()).as_str(),
        20.,
        80.,
        16.,
        WHITE,
    );
    // Draw latency info
    draw_text(
        format!("Min Latency: {}ms", client.network.borrow().min_latency_ms).as_str(),
        20.,
        100.,
        16.,
        WHITE,
    );
    draw_text(
        format!("Max Latency: {}ms", client.network.borrow().max_latency_ms).as_str(),
        20.,
        120.,
        16.,
        WHITE,
    );

    draw_entities(client.world.get_entities().values().collect());
}

fn draw_server(server: &server::Server) {
    draw_text("Server", 20., 20., 32., WHITE);

    // Draw tick rate
    draw_text(
        format!("Tick Rate: {}ms", server.tick_rate_ms).as_str(),
        20.,
        60.,
        16.,
        WHITE,
    );

    draw_entities(server.world.get_entities().values().collect());
}

fn draw_entities(entities: Vec<&Entity>) {
    for entity in entities {
        let macroquad_colour = match entity.colour {
            sim::Colour::Red => RED,
            sim::Colour::Green => GREEN,
            sim::Colour::Blue => BLUE,
        };

        draw_rectangle(
            entity.position.0,
            entity.position.1,
            50.,
            50.,
            macroquad_colour,
        );
    }
}

fn draw_top_left(width: f32, height: f32) {
    let mut top_left_cam = create_grid_camera(width, height);
    top_left_cam.viewport = Some((0, height as i32, width as i32, height as i32));
    set_camera(&top_left_cam);
}

fn draw_top_right(width: f32, height: f32) {
    let mut top_right_cam = create_grid_camera(width, height);
    top_right_cam.viewport = Some((width as i32, height as i32, width as i32, height as i32));
    set_camera(&top_right_cam);
}

fn draw_bottom_left(width: f32, height: f32) {
    let mut bottom_left_cam = create_grid_camera(width, height);
    bottom_left_cam.viewport = Some((0, 0, width as i32, height as i32));
    set_camera(&bottom_left_cam);
}

fn draw_bottom_right(width: f32, height: f32) {
    let mut bottom_right_cam = create_grid_camera(width, height);
    bottom_right_cam.viewport = Some((width as i32, 0, width as i32, height as i32));
    set_camera(&bottom_right_cam);
}

fn draw_ui(ui_state: &mut UIState) {
    //set_default_camera();

    if root_ui().button(vec2(screen_width() - 100., 5.), "Settings") {
        ui_state.open_settings = !ui_state.open_settings;
    }

    if ui_state.open_settings {
        widgets::Window::new(hash!(), vec2(screen_width() / 2., 50.), vec2(300., 300.))
            .label("Settings")
            .ui(&mut *root_ui(), |ui| {
                ui.label(None, "Client 1");
                widgets::Checkbox::new(hash!())
                    .label("Prediction")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_1_prediction);
                widgets::Checkbox::new(hash!())
                    .label("Reconciliation")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_1_reconciliation);
                widgets::Checkbox::new(hash!())
                    .label("Extrapolation")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_1_extrapolation);
                ui.separator();
                ui.label(None, "Client 2");
                widgets::Checkbox::new(hash!())
                    .label("Prediction")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_2_prediction);
                widgets::Checkbox::new(hash!())
                    .label("Reconciliation")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_2_reconciliation);
                widgets::Checkbox::new(hash!())
                    .label("Extrapolation")
                    .ratio(0.2)
                    .ui(ui, &mut ui_state.client_2_extrapolation);
            });
    }
}

struct UIState {
    open_settings: bool,
    client_1_prediction: bool,
    client_1_reconciliation: bool,
    client_1_extrapolation: bool,
    client_2_prediction: bool,
    client_2_reconciliation: bool,
    client_2_extrapolation: bool,
}

#[macroquad::main("Fast GameNetworking Example")]
async fn main() {
    let mut server = server::Server::new(50);
    let mut client1 = Client::new(1, 16);
    let mut client2 = Client::new(2, 16);

    // This is just for helping us identify which is which
    client1.colour = sim::Colour::Red;
    client2.colour = sim::Colour::Green;

    // Seperate control scheme for player 2
    client2.use_alternate_input = true;

    let mut ui_state = UIState {
        open_settings: false,
        client_1_prediction: true,
        client_1_reconciliation: true,
        client_1_extrapolation: true,
        client_2_prediction: true,
        client_2_reconciliation: true,
        client_2_extrapolation: true,
    };

    let mut pause_client_1 = false;

    client1.connect(&mut server, 250, 250, 0.);
    client2.connect(&mut server, 100, 100, 0.);

    server.create_npc_entities();

    loop {
        let grid_section_width = screen_width() / 2.;
        let grid_section_height = screen_height() / 2.;

        // On press 1, connect client 1
        if is_key_pressed(KeyCode::Key1) {
            client1.connect(&mut server, 250, 250, 0.);
        }
        // On press 2, connect client 2
        if is_key_pressed(KeyCode::Key2) {
            client2.connect(&mut server, 100, 100, 0.);
        }

        if is_key_pressed(KeyCode::P) {
            pause_client_1 = !pause_client_1;
        }

        if !pause_client_1 {
            client1.update();
            client1.client_prediction_enabled = ui_state.client_1_prediction;
            client1.server_reconciliation_enabled = ui_state.client_1_reconciliation;
            client1.extrapolation_enabled = ui_state.client_1_extrapolation;
        }

        client2.update();
        client2.client_prediction_enabled = ui_state.client_2_prediction;
        client2.server_reconciliation_enabled = ui_state.client_2_reconciliation;
        client2.extrapolation_enabled = ui_state.client_2_extrapolation;

        server.update();

        clear_background(LIGHTGRAY);

        draw_top_left(grid_section_width, grid_section_height);
        draw_client(&client1);

        draw_top_right(grid_section_width, grid_section_height);
        draw_client(&client2);

        draw_bottom_left(grid_section_width, grid_section_height);
        draw_server(&server);

        draw_bottom_right(grid_section_width, grid_section_height);

        draw_ui(&mut ui_state);

        next_frame().await
    }
}
