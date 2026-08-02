use std::collections::HashSet;

pub type Keycode = sdl2::keyboard::Keycode;
pub type MouseButton = sdl2::mouse::MouseButton;

pub struct InputContainer {
    pressed_keys_this_frame: HashSet<Keycode>,
    released_keys_this_frame: HashSet<Keycode>,

    pressed_keys: HashSet<Keycode>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    cursor_position: [f32; 2],
    mouse_delta: [f32; 2],
}

#[derive(Clone, Copy)]
pub struct Input<'a> {
    input_container: &'a InputContainer,
}

impl<'a> Input<'a> {
    pub fn get_key(&self, key: Keycode) -> bool {
        self.input_container.pressed_keys.contains(&key)
    }

    pub fn get_key_down(&self, key: Keycode) -> bool {
        self.input_container.pressed_keys_this_frame.contains(&key)
    }

    pub fn get_key_up(&self, key: Keycode) -> bool {
        self.input_container.released_keys_this_frame.contains(&key)
    }

    pub fn get_mouse_button(&self, button: MouseButton) -> bool {
        self.input_container.pressed_mouse_buttons.contains(&button)
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        (
            self.input_container.mouse_delta[0],
            self.input_container.mouse_delta[1],
        )
    }
}

impl InputContainer {
    pub fn new() -> Self {
        Self {
            released_keys_this_frame: HashSet::new(),
            pressed_keys_this_frame: HashSet::new(),
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            cursor_position: [0., 0.],
            mouse_delta: [0., 0.],
        }
    }

    pub fn new_frame(&mut self) {
        self.mouse_delta = [0.0, 0.0];
        self.pressed_keys_this_frame.clear();
        self.released_keys_this_frame.clear();
    }

    pub fn set_cursor_position(&mut self, x: f32, y: f32) {
        self.cursor_position = [x, y];
    }

    pub fn set_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta = [dx, dy];
    }

    pub fn add_pressed_mouse_button(&mut self, button: MouseButton) {
        self.pressed_mouse_buttons.insert(button);
    }

    pub fn remove_pressed_mouse_button(&mut self, button: MouseButton) {
        self.pressed_mouse_buttons.remove(&button);
    }

    pub fn add_pressed_key(&mut self, key: Keycode) {
        if self.pressed_keys.contains(&key) {
            return;
        }

        self.pressed_keys_this_frame.insert(key);
        self.pressed_keys.insert(key);
    }

    pub fn remove_pressed_key(&mut self, key: Keycode) {
        if self.pressed_keys.contains(&key) {
            self.pressed_keys.remove(&key);
            self.released_keys_this_frame.insert(key);
        }
    }

    pub fn as_input(&self) -> Input<'_> {
        Input {
            input_container: self,
        }
    }
}
