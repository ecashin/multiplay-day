use yew::prelude::*;

const N_REQUIRED: usize = 3;
const N_CHOICES: usize = 4;

enum Msg {
    ChoiceMade(i32),
}

struct Model {
    link: ComponentLink<Self>,
    history: Vec<bool>,
    problem: (i32, i32),
    choices: Vec<i32>,
}

impl Model {
    fn progress_bar(&self) -> Html {
        let n_correct = self.history.iter().copied().filter(|p| *p).count();
        let check = "✅";
        let x_mar = "❌";
        let history_viz: Vec<&str> = self
            .history
            .iter()
            .copied()
            .map(|correct| if correct { check } else { x_mar })
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
}

fn new_problem(_model: Option<&Model>) -> ((i32, i32), Vec<i32>) {
    ((6, 4), vec![24, 2, 3, 4])
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
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::ChoiceMade(response) => {
                let answer = self.problem.0 * self.problem.1;
                let correct = response == answer;
                self.history.push(correct);
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