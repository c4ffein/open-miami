use macroquad::prelude::*;
use std::f32::consts::PI;

mod player;
mod enemy;
mod weapon;
mod level;
mod camera;
mod collision;

use player::Player;
use enemy::{Enemy, EnemyState};
use weapon::{Weapon, WeaponType};
use level::Level;
use camera::Camera;

#[macroquad::main("Open Miami")]
async fn main() {
    let mut player = Player::new(vec2(400.0, 300.0));
    let mut enemies = vec![
        Enemy::new(vec2(600.0, 300.0)),
        Enemy::new(vec2(800.0, 400.0)),
        Enemy::new(vec2(300.0, 500.0)),
        Enemy::new(vec2(700.0, 200.0)),
    ];
    let level = Level::new();
    let mut camera = Camera::new();

    loop {
        clear_background(Color::from_rgba(20, 12, 28, 255));

        // Update
        let dt = get_frame_time();

        // Player input and movement
        player.update(dt);

        // Update camera to follow player
        camera.follow_player(player.pos);

        // Apply camera transform
        camera.apply();

        // Render level
        level.render();

        // Update and render enemies
        for enemy in &mut enemies {
            if enemy.alive {
                enemy.update(dt, player.pos);
                enemy.render();
            }
        }

        // Handle player shooting
        if is_mouse_button_pressed(MouseButton::Left) && player.alive {
            let mouse_world_pos = camera.screen_to_world(mouse_position().into());
            player.shoot(mouse_world_pos, &mut enemies);
        }

        // Handle player melee attack
        if is_mouse_button_pressed(MouseButton::Right) && player.alive {
            let mouse_world_pos = camera.screen_to_world(mouse_position().into());
            player.melee_attack(mouse_world_pos, &mut enemies);
        }

        // Check if enemies hit player
        for enemy in &mut enemies {
            if enemy.alive && player.alive {
                enemy.try_attack_player(&mut player);
            }
        }

        // Render player
        player.render(camera.screen_to_world(mouse_position().into()));

        // Reset camera
        camera.reset();

        // UI - Draw health
        if player.alive {
            draw_text(
                &format!("Health: {}", player.health),
                10.0,
                30.0,
                30.0,
                WHITE,
            );
            draw_text(
                &format!("Ammo: {}", player.weapon.ammo),
                10.0,
                60.0,
                30.0,
                WHITE,
            );
            draw_text(
                &format!("Enemies: {}", enemies.iter().filter(|e| e.alive).count()),
                10.0,
                90.0,
                30.0,
                WHITE,
            );
        } else {
            draw_text(
                "YOU DIED",
                screen_width() / 2.0 - 100.0,
                screen_height() / 2.0,
                60.0,
                RED,
            );
            draw_text(
                "Press R to restart",
                screen_width() / 2.0 - 120.0,
                screen_height() / 2.0 + 40.0,
                30.0,
                WHITE,
            );

            if is_key_pressed(KeyCode::R) {
                player = Player::new(vec2(400.0, 300.0));
                enemies = vec![
                    Enemy::new(vec2(600.0, 300.0)),
                    Enemy::new(vec2(800.0, 400.0)),
                    Enemy::new(vec2(300.0, 500.0)),
                    Enemy::new(vec2(700.0, 200.0)),
                ];
            }
        }

        // Controls info
        draw_text("WASD: Move | Mouse: Aim | Left Click: Shoot | Right Click: Melee", 10.0, screen_height() - 20.0, 20.0, GRAY);

        next_frame().await;
    }
}
