use rand::prelude::*;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::KeyboardEvent;

#[wasm_bindgen()]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = "log")]
    fn console_log(s: &str);
}

const N_CHOICES: usize = 4;
const MAX_FACTOR: usize = 5;
const SUFFICIENT: usize = 2;

enum Msg {
    ChoiceMade(usize),
    KeyPressed(KeyboardEvent),
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
        let n_correct = self
            .history
            .iter()
            .copied()
            .filter(|(_, correct)| *correct)
            .count();
        let check = "✅";
        let x_mar = "❌";
        let history_viz: Vec<&str> = self
            .history
            .iter()
            .copied()
            .map(|(_, correct)| if correct { check } else { x_mar })
            .collect();
        html! {
            <span class="progress-bar">
                <p>{ history_viz.join("") }</p>
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

    fn update_pairs(&mut self, correct: bool) {
        let (a, b) = self.problem;
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

    fn matrix_row(&self, i: usize) -> Html {
        self.pairs[i]
            .iter()
            .map(|pair| {
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
                html! {
                    <td class=html_class>{ html_content }</td>
                }
            })
            .collect()
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

    fn matrix_display(&self) -> Html {
        html! {
            <table class="pairs-progress">
                { self.matrix_rows() }
            </table>
        }
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
                let n = match status {
                    PairStatus::Unknown => SUFFICIENT,
                    PairStatus::InProgress(n) => std::cmp::max(*n, 0) as usize,
                    PairStatus::Finished => 0,
                };
                vec![i; n]
            })
            .flatten()
            .collect();
        console_log(&format!("weighted_choices:{:?}", weighed_choices));
        let mut chosen = vec![];
        for _ in 0..N_CHOICES {
            let i = rng.gen_range(0..weighed_choices.len());
            let pair_index = weighed_choices[i];
            chosen.push((pair_index / (MAX_FACTOR + 1), pair_index % (MAX_FACTOR + 1)));
        }
        console_log(&format!("chosen:{:?}", chosen));
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
        Self {
            link,
            history: vec![],
            problem,
            choices,
            pairs: create_pairs_matrix(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ChoiceMade(response) => {
                console_log(&format!("ChoiceMade:{}", response));
                let answer = self.problem.0 * self.problem.1;
                let correct = response == answer;
                self.history.push((self.problem, correct));
                self.update_pairs(correct);
                let (problem, choices) = new_problem(Some(self));
                self.problem = problem;
                self.choices = choices;
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
                    Ok(n) => {
                        let i = n - 1;
                        if i < self.choices.len() {
                            self.update(Msg::ChoiceMade(self.choices[i]))
                        } else {
                            false
                        }
                    }
                }
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div onkeypress=self.link.callback(Msg::KeyPressed)>
                <div>{ self.progress_bar() }</div>
                <div>{ self.problem_display() }</div>
                <div class="flex demo">{ self.choices_display() }</div>
                <div>{ self.matrix_display() }</div>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
