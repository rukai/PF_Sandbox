use game::Game;
use input::ControllerInput;

pub fn gen_inputs(game: &Game) -> Vec<ControllerInput> {
    game.selected_ais.iter().map(|_| {
        ControllerInput {
            plugged_in: true,

            up:    false,
            down:  false,
            right: false,
            left:  false,
            y:     false,
            x:     false,
            b:     false,
            a:     false,
            l:     false,
            r:     false,
            z:     false,
            start: false,

            stick_x:   0.0,
            stick_y:   0.0,
            c_stick_x: 0.0,
            c_stick_y: 0.0,
            l_trigger: 0.0,
            r_trigger: 0.0,
        }
    }).collect()
}
