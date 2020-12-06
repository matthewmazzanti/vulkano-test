use winit::{
    event::{
        Event,
        WindowEvent,
        KeyboardInput,
        VirtualKeyCode as Key,
        ElementState as Keyvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

mod renderer;
use renderer::{Renderer, InstanceData};

enum Rot {
    Left,
    Right,
    No,
}

struct Asteroid {
    x: f32,
    y: f32,
    vel_x: f32,
    vel_y: f32,
    angle: f32,
}

struct State {
    x: f32,
    y: f32,
    vel_x: f32,
    vel_y: f32,
    accel: bool,
    angle: f32,
    rot: Rot,
    asteroids: Vec<Asteroid>,
}

fn render(st: &State) -> Vec<Vec<InstanceData>> {
    let ships = vec![
        InstanceData {
            pos_offset: [st.x, st.y],
            angle: st.angle,
            scale: 0.05,
        },
    ];

    let mut asteroids = Vec::new();
    for asteroid in st.asteroids.iter() {
        asteroids.push(InstanceData {
            pos_offset: [asteroid.x, asteroid.y],
            angle: asteroid.angle,
            scale: 0.1,
        });
    }

    vec![ships, asteroids]
}

fn update(st: &mut State) {
    st.angle = match st.rot {
        Rot::Left => st.angle + 5.0,
        Rot::Right => st.angle - 5.0,
        Rot::No => st.angle,
    };

    let angle = st.angle.to_radians();

    if st.accel {
        let delta_vel_x = angle.sin() * 0.0005;
        let delta_vel_y = angle.cos() * 0.0005;
        st.vel_x += delta_vel_x;
        st.vel_y += delta_vel_y;
    }

    // println!("angle: {}, vel_x: {}, vel_y: {}", angle, st.vel_x, st.vel_y);

    st.x -= st.vel_x;
    st.y -= st.vel_y;

    if st.x > 1.0 {
        st.x -= 2.0;
    } else if st.x < -1.0 {
        st.x += 2.0;
    }

    if st.y > 1.0 {
        st.y -= 2.0;
    } else if st.y < -1.0 {
        st.y += 2.0;
    }

    for asteroid in st.asteroids.iter_mut() {
        // asteroid.angle += 3.0;
        asteroid.x += asteroid.vel_x;
        asteroid.y += asteroid.vel_y;

        if asteroid.x > 1.0 {
            asteroid.x -= 2.0;
        } else if asteroid.x < -1.0 {
            asteroid.x += 2.0;
        }

        if asteroid.y > 1.0 {
            asteroid.y -= 2.0;
        } else if asteroid.y < -1.0 {
            asteroid.y += 2.0;
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop);

    let mut game_state = State {
        x: 0.5,
        y: 0.5,
        vel_x: 0.0,
        vel_y: 0.0,
        angle: 0.0,
        accel: false,
        rot: Rot::No,
        asteroids: vec![
            Asteroid { x: 0.0, y: 0.0, vel_x: 0.0, vel_y: 0.0, angle: 0.0 },
        ],
    };

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                    ..
                },
                ..
            } => {
                if state == Keyvent::Pressed {
                    match key {
                        Key::A => game_state.rot = Rot::Left,
                        Key::F => game_state.rot = Rot::Right,
                        Key::D => game_state.accel = true,
                        _ => (),
                    }
                } else {
                    match key {
                        Key::A => game_state.rot = Rot::No,
                        Key::F => game_state.rot = Rot::No,
                        Key::D => game_state.accel = false,
                        _ => (),
                    }
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, ..  } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::Resized(_), ..  } => {
                renderer.recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                update(&mut game_state);
                renderer.redraw(render(&game_state));
            }
            _ => (),
        }
    });
}
