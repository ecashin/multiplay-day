use js_sys::Date;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::HtmlAudioElement;
use yew::format::Json;
use yew::prelude::*;
use yew::services::storage::{Area, StorageService};
use yew::KeyboardEvent;

#[wasm_bindgen()]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = "log")]
    fn console_log(s: &str);
}

const N_CHOICES: usize = 4;
const MAX_FACTOR: usize = 12;
const SUFFICIENT: usize = 2;
const FAST_MILLISECONDS: f64 = 2000.0;
const SOUND_FILES: &'static [&'static str] = &[
    "broccoli1.wav",
    "carrot1.wav",
    "carrot2.wav",
    "corn.wav",
    "potato.wav",
    "squash1.wav",
    "zuccini1.wav",
    "zuccini2.wav",
];
const SOUNDS_DIR: &'static str = "sounds";
// https://dev.to/davidedelpapa/yew-tutorial-04-and-services-for-all-1non
const STORAGE_KEY_TALLY: &'static str = "net.noserose.multiplayday:tally";
const STORAGE_KEY_TIMINGS: &'static str = "net.noserose.multiplayday:timings";
const TIMINGS_HISTORY_LENGTH: usize = 5;

enum Msg {
    ChoiceMade(usize),
    ClearTimings,
    EnterGame(bool),
    KeyPressed(KeyboardEvent),
}

#[derive(Serialize, Deserialize)]
pub struct Timings {
    response_time_history: Vec<Vec<Vec<f64>>>,
}

impl Timings {
    fn new() -> Self {
        let response_time_history =
            vec![
                vec![vec![FAST_MILLISECONDS * 2.0; TIMINGS_HISTORY_LENGTH]; MAX_FACTOR + 1];
                MAX_FACTOR + 1
            ];
        Timings {
            response_time_history,
        }
    }

    fn record(&mut self, a: usize, b: usize, response_ms: Option<f64>) {
        if let Some(response_ms) = response_ms {
            self.response_time_history[a][b].insert(0, response_ms);
            self.response_time_history[a][b].pop();
        } else {
            console_log(&format!("No response time for {}x{}", a, b));
        }
    }

    fn median(&self, a: usize, b: usize) -> f64 {
        let mut times = self.response_time_history[a][b].clone();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let middle = times.len() / 2;
        if times.len() % 2 == 1 {
            times[middle]
        } else {
            (times[middle] + times[middle - 1]) / 2.0
        }
    }

    fn valid_or_new(self) -> Self {
        let valid = self.response_time_history.len() == MAX_FACTOR + 1
            && self.response_time_history[0].len() == MAX_FACTOR + 1;
        if valid {
            self
        } else {
            Self::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Tally {
    correct_counts: Vec<Vec<i32>>,
}

impl Tally {
    fn new() -> Self {
        let correct_counts = vec![vec![0; MAX_FACTOR + 1]; MAX_FACTOR + 1];
        Tally { correct_counts }
    }

    fn valid_or_new(self) -> Self {
        let valid = self.correct_counts.len() == MAX_FACTOR + 1
            && self.correct_counts[0].len() == MAX_FACTOR + 1;
        if valid {
            self
        } else {
            console_log("Ignoring invalid tally from local storage");
            Self::new()
        }
    }
}

#[derive(Debug)]
enum PairStatus {
    Unknown,
    InProgress(i32),
    Finished,
}

type Problem = (usize, usize);

struct Model {
    link: ComponentLink<Self>,
    history: Vec<(Problem, bool)>,
    problem: Problem,
    choices: Vec<usize>,
    pairs: Vec<Vec<PairStatus>>,
    prompt_time: Option<f64>,
    feedback: Option<String>,
    storage: StorageService,
    tally: Tally,
    sounds: Vec<HtmlAudioElement>,
    timings: Timings,
    in_lobby: bool,
}

fn create_pairs_matrix() -> Vec<Vec<PairStatus>> {
    let mut rows = vec![];
    for _ in 0..=MAX_FACTOR {
        let mut row = vec![];
        for _ in 0..=MAX_FACTOR {
            row.push(PairStatus::Unknown);
        }
        rows.push(row);
    }
    rows
}

impl Model {
    fn progress_bar(&self) -> Html {
        let check = "✅";
        let x_mar = "❌";
        let history_viz: Vec<Html> = self
            .history
            .iter()
            .copied()
            .map(|((a, b), correct)| {
                html! {
                    <span title=format!("{}x{}", a, b)>
                        { if correct { check } else { x_mar } }
                    </span>
                }
            })
            .collect();
        html! {
            <span class="progress-bar">
                <p>{ history_viz }</p>
            </span>
        }
    }

    fn problem_display(&self) -> Html {
        html! {
            <p>{ format!("{} x {} = ...", self.problem.0, self.problem.1) }</p>
        }
    }

    fn choices_display(&self) -> Html {
        self.choices
            .iter()
            .copied()
            .map(|response| {
                html! {
                    <div>
                        <span>
                            <button class="flex"
                                onclick=self.link.callback(move |_| Msg::ChoiceMade(response))
                            >{ response }</button>
                        </span>
                    </div>
                }
            })
            .collect()
    }

    fn update_storage(&mut self, correct: bool) {
        let (a, b) = self.problem;
        self.tally.correct_counts[a][b] += if correct { 1 } else { -1 };
        self.storage.store(STORAGE_KEY_TALLY, Json(&self.tally));
    }

    fn response_time(&self) -> Option<f64> {
        let start = self.prompt_time?;
        Some(Date::now() - start)
    }

    fn update_pairs(&mut self, correct: bool) {
        let (a, b) = self.problem;
        if correct && self.response_time().unwrap_or(FAST_MILLISECONDS * 2.0) < FAST_MILLISECONDS {
            self.pairs[a as usize][b as usize] = PairStatus::Finished;
        } else {
            let status = &self.pairs[a as usize][b as usize];
            let adjust = if correct { 1 } else { -1 };
            let new_status = match status {
                PairStatus::Unknown => PairStatus::InProgress(adjust),
                PairStatus::InProgress(n) => {
                    if n + adjust >= SUFFICIENT as i32 {
                        PairStatus::Finished
                    } else {
                        PairStatus::InProgress(n + adjust)
                    }
                }
                PairStatus::Finished => PairStatus::InProgress(SUFFICIENT as i32 + adjust),
            };
            console_log(&format!("self.pairs[{}][{}] <- {:?}", a, b, new_status));
            self.pairs[a as usize][b as usize] = new_status;
        }
    }

    fn matrix_row(&self, i: usize) -> Vec<Html> {
        let mut cells = vec![html! { <td class="left-index">{ i }</td> }];
        for pair in &self.pairs[i] {
            let html_class = match pair {
                PairStatus::Unknown => "unknown",
                PairStatus::InProgress(_) => "in-progress",
                PairStatus::Finished => "finished",
            };
            let html_content = match pair {
                PairStatus::Unknown => 0,
                PairStatus::InProgress(n) => std::cmp::max(*n, 0) as usize,
                PairStatus::Finished => SUFFICIENT,
            };
            cells.push(html! {
                <td class=html_class>{ html_content }</td>
            });
        }
        cells
    }

    fn matrix_rows(&self) -> Html {
        self.pairs
            .iter()
            .enumerate()
            .map(|(i, _)| {
                html! {
                    <tr>{ self.matrix_row(i) }</tr>
                }
            })
            .collect()
    }

    fn matrix_header(&self) -> Vec<Html> {
        let mut cells = vec![html! {
            <th></th>
        }];
        for i in 0..=MAX_FACTOR {
            cells.push(html! {
                <th>{ i }</th>
            });
        }
        cells
    }

    fn matrix_display(&self) -> Html {
        html! {
            <table class="pairs-progress">
                <thead>
                    { self.matrix_header() }
                </thead>
                <tbody>
                    { self.matrix_rows() }
                </tbody>
            </table>
        }
    }

    fn play(&self) {
        if let Some(resp_time) = self.response_time() {
            let typical_resp_time = self.timings.median(self.problem.0, self.problem.1);
            console_log(&format!(
                "resp_time:{} vs typical:{}",
                resp_time, typical_resp_time
            ));
            if resp_time < typical_resp_time {
                let mut rng = thread_rng();
                let i = rng.gen_range(0..SOUND_FILES.len());
                let msg = match self.sounds[i].play() {
                    Ok(result) => format!("play {} success: {:?}", SOUND_FILES[i], result),
                    Err(e) => format!("play {} error: {:?}", SOUND_FILES[i], e),
                };
                console_log(&msg);
            }
        }
    }

    fn audio_elements(&self) -> Html {
        SOUND_FILES
            .iter()
            .copied()
            .map(|f| {
                let path = format!("{}/{}", SOUNDS_DIR, f);
                html! {
                    <audio src=path id=f preload="auto" crossorigin="anonymous"></audio>
                }
            })
            .collect()
    }
}

fn choose_choices(rng: &mut ThreadRng, model: Option<&Model>) -> Vec<Problem> {
    if let Some(model) = model {
        let weighed_choices: Vec<usize> = model
            .pairs
            .iter()
            .flatten()
            .enumerate()
            .map(|(i, status)| {
                let mut n = match status {
                    PairStatus::Unknown => SUFFICIENT,
                    PairStatus::InProgress(n) => std::cmp::max(*n, 0) as usize,
                    PairStatus::Finished => 0,
                };
                let m = model.tally.correct_counts[i / (MAX_FACTOR + 1)][i % (MAX_FACTOR + 1)];
                if m < 0 {
                    n += -m as usize;
                } else if m > 0 && n > 1 {
                    n -= 1;
                }
                vec![i; n + 1]
            })
            .flatten()
            .collect();
        // console_log(&format!("weighted_choices:{:?}", weighed_choices));
        let mut chosen = vec![];
        for _ in 0..N_CHOICES {
            let i = rng.gen_range(0..weighed_choices.len());
            let pair_index = weighed_choices[i];
            chosen.push((pair_index / (MAX_FACTOR + 1), pair_index % (MAX_FACTOR + 1)));
        }
        // console_log(&format!("chosen:{:?}", chosen));
        chosen
    } else {
        let mut choices = vec![];
        for _ in 0..N_CHOICES {
            let a = rng.gen_range(0..=MAX_FACTOR);
            let b = rng.gen_range(0..=MAX_FACTOR);
            choices.push((a, b));
        }
        choices
    }
}

fn new_problem(model: Option<&Model>) -> (Problem, Vec<usize>) {
    let mut rng = rand::thread_rng();
    let choices = choose_choices(&mut rng, model);
    let problem_i = rng.gen_range(0..N_CHOICES);
    let answers = choices.iter().map(|(a, b)| a * b).collect();

    (choices[problem_i], answers)
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let (problem, choices) = new_problem(None);
        let storage = StorageService::new(Area::Local).unwrap();
        let Json(tally) = storage.restore(STORAGE_KEY_TALLY);
        let tally = tally.unwrap_or(Tally::new()).valid_or_new();
        let Json(timings) = storage.restore(STORAGE_KEY_TIMINGS);
        let timings = timings.unwrap_or(Timings::new()).valid_or_new();
        let sounds = SOUND_FILES
            .iter()
            .map(|f| {
                let path = format!("{}/{}", SOUNDS_DIR, f);
                HtmlAudioElement::new_with_src(&path).expect("Creating HtmlAudioElement")
            })
            .collect();
        Self {
            link,
            history: vec![],
            problem,
            choices,
            pairs: create_pairs_matrix(),
            prompt_time: Some(Date::now()),
            feedback: None,
            storage,
            tally,
            sounds,
            timings,
            in_lobby: true,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ChoiceMade(response) => {
                console_log(&format!("ChoiceMade:{}", response));
                let answer = self.problem.0 * self.problem.1;
                let correct = response == answer;
                if correct {
                    self.play();
                }
                self.timings
                    .record(self.problem.0, self.problem.1, self.response_time());
                self.storage.store(STORAGE_KEY_TIMINGS, Json(&self.timings));
                console_log(&format!(
                    "times:{:?}",
                    self.timings.response_time_history[self.problem.0][self.problem.1]
                ));
                self.history.push((self.problem, correct));
                self.update_pairs(correct);
                self.update_storage(correct);
                self.prompt_time = Some(Date::now());
                let (problem, choices) = new_problem(Some(self));
                self.problem = problem;
                self.choices = choices;
                true
            }
            Msg::ClearTimings => {
                self.timings = Timings::new();
                self.storage.store(STORAGE_KEY_TIMINGS, Json(&self.timings));
                false
            }
            Msg::EnterGame(yesno) => {
                self.in_lobby = !yesno;
                true
            }
            Msg::KeyPressed(event) => {
                console_log(&format!(
                    "KeyboardEvent:{:?} with key:{:?}",
                    event,
                    event.key()
                ));
                let n = event.key().parse::<usize>();
                match n {
                    Err(_) => false,
                    Ok(n) if n > 0 => {
                        let i = n - 1;
                        if i < self.choices.len() {
                            self.update(Msg::ChoiceMade(self.choices[i]))
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        if self.in_lobby {
            html! {
                <div>
                    <p>{ "IN LOBBY!" }</p>
                    <button onclick=self.link.callback(|_| Msg::EnterGame(true))>{ "MKultilpcliashun" }</button>
                </div>
            }
        } else {
            html! {
                <div onkeypress=self.link.callback(Msg::KeyPressed)>
                    <div class="progress-bar"><p>{ self.progress_bar() }</p></div>
                    <div>{ self.feedback.as_ref().unwrap_or(&"".to_owned()) }</div>
                    <div>{ self.problem_display() }</div>
                    <div>
                        <button onclick=self.link.callback(|_| Msg::ClearTimings)>{ "Clear Timings" }</button>
                        <button onclick=self.link.callback(|_| Msg::EnterGame(false))>{ "Lobby" }</button>
                    </div>
                    <div class="flex demo">{ self.choices_display() }</div>
                    <div>{ self.matrix_display() }</div>
                    <div>{ self.audio_elements() }</div>
                </div>
            }
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
