use ::player::Player;
use ::fighter::Fighter;
use ::stage::Stage;
use ::graphics::Graphics;
use ::rules::Rules;
use ::controller::Control;

enum GameState {
    CharacterSelect,
    StageSelect,
    InGame,
    Results,
    Paused,
    Quit,
}

pub struct Game {
    // package data
    rules:    Rules,
    fighters: Vec<Fighter>,
    stages:   Vec<Stage>,

    // variables
    current_stage: u64,
    state:         GameState,
    players:       Vec<Player>,
    graphics:      Graphics,
    timer:         u64,
}

impl Game {
    pub fn new(rules: Rules, fighters: Vec<Fighter>, stages: Vec<Stage>) -> Game {
        Game {
            state:    GameState::CharacterSelect,
            rules:    rules,
            fighters: fighters,
            stages:   stages,

            current_stage: 0,
            players:       Game::setup_players(),
            timer:         0,
            graphics:      Graphics::new(),
        }
    }

    pub fn setup_players() -> Vec<Player> {
        let mut players: Vec<Player> = Vec::new();
        players.push(Player::new());
        players.push(Player::new());
        players
    }

    pub fn step_game(&mut self) {
        let control: Control = Default::default();
        for player in &mut self.players {
            if control.start {
                self.state = GameState::Paused; //TODO: on press
            }
            player.step(&control);
        }
        self.timer += 1;
        if self.timer > self.rules.time_limit {
            self.state = GameState::Results;
        }
    }

    pub fn step_pause(&mut self) {
        self.state = GameState::InGame;
        //TODO: Handle character/stage edits here
    }

    pub fn step_select(&mut self) {
        //TODO: control cursor etc.
        match self.state {
            GameState::CharacterSelect => {
                if self.fighters.len() > 0 {
                    self.players[0].fighter = self.fighters[0].clone();
                    self.players[1].fighter = self.fighters[0].clone();
                }
                self.state = GameState::StageSelect;
            },
            GameState::StageSelect => {
                self.current_stage = 0;
                self.state = GameState::InGame;
            }
            _ => ()
        }
    }

    pub fn step_results(&mut self) {
    }

    pub fn run(&mut self) {
        loop {
            if self.graphics.check_close() {
                self.state = GameState::Quit
            }

            match self.state {
                GameState::CharacterSelect => { self.step_select();  },
                GameState::StageSelect     => { self.step_select();  },
                GameState::InGame          => { self.step_game();    },
                GameState::Results         => { self.step_results(); },
                GameState::Paused          => { self.step_pause();   },
                GameState::Quit            =>   return,
            }
            
            if self.stages.len() > 0 {
                self.graphics.render(&self.stages[self.current_stage as usize], &self.players);
            }
        }
    }
}
