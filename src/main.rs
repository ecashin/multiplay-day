use probability::prelude::*;
use rand::prelude::*;
use yew::prelude::*;

const N_REQUIRED: usize = 3;
const N_CHOICES: usize = 4;
const MAX_FACTOR: usize = 12;
const SUFFICIENT: usize = 4;

enum Msg {
    ChoiceMade(i32),
}

enum PairStatus {
    Unknown,
    InProgress(i32),
    Finished,
}

type Problem = (i32, i32);

struct Model {
    link: ComponentLink<Self>,
    history: Vec<(Problem, bool)>,
    problem: Problem,
    choices: Vec<i32>,
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
            <span>
                <p>{ format!("{} / {} - {}", n_correct, N_REQUIRED, history_viz.join("")) }</p>
            </span>
        }
    }
    fn problem_display(&self) -> Html {
        html! {
            <p>{ format!("{} x {} = ...", self.problem.0, self.problem.1) }</p>
        }
    }
    fn choices_display(&self) -> Html {
        self.choices.iter().copied().map(|response| {
            html! {
                <div>
                    <button onclick=self.link.callback(move |_| Msg::ChoiceMade(response))>{ response }</button>
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
            PairStatus::Unknown => PairStatus::InProgress(0),
            PairStatus::InProgress(n) => {
                if n + adjust >= SUFFICIENT as i32 {
                    PairStatus::Finished
                } else {
                    PairStatus::InProgress(n + adjust)
                }
            }
            PairStatus::Finished => PairStatus::InProgress(SUFFICIENT as i32 + adjust),
        };
        self.pairs[a as usize][b as usize] = new_status;
    }
}

fn choose_choices(rng: &mut ThreadRng, model: Option<&Model>) -> Vec<Problem> {
    if let Some(model) = model {
        let weights: Vec<f64> = model
            .pairs
            .iter()
            .flatten()
            .map(|status| match status {
                PairStatus::Unknown => SUFFICIENT as f64,
                PairStatus::InProgress(n) => *n as f64,
                PairStatus::Finished => 0.0,
            })
            .collect();
        let mut source = source::default();
        let distribution = Categorical::new(&weights);
        let mut sampler = Independent(&distribution, &mut source);
        let samples = sampler.take(N_CHOICES).collect::<Vec<_>>();
        samples
            .iter()
            .map(|i| ((i / MAX_FACTOR) as i32, (i % MAX_FACTOR) as i32))
            .collect()
    } else {
        let mut choices = vec![];
        for _ in 0..N_CHOICES {
            let a = rng.gen_range(0..12);
            let b = rng.gen_range(0..12);
            choices.push((a, b));
        }
        choices
    }
}
fn new_problem(model: Option<&Model>) -> ((i32, i32), Vec<i32>) {
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
                let answer = self.problem.0 * self.problem.1;
                let correct = response == answer;
                self.history.push((self.problem, correct));
                self.update_pairs(correct);
                let (problem, choices) = new_problem(Some(self));
                self.problem = problem;
                self.choices = choices;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <div>{ self.progress_bar() }</div>
                <div>{ self.problem_display() }</div>
                <div>{ self.choices_display() }</div>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<Model>();
}
