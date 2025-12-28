use stats_tool::{Article, Stack, Summarize, Tweet, mean, median, mode};

fn main() {
    demo_stats();
    demo_stack();
    demo_summarize();
}

fn demo_stats() {
    let data = [1.0, 2.0, 2.0, 3.0, 4.0];
    println!("Data: {:?}", data);
    println!("Mean: {:?}", mean(&data));
    println!("Median: {:?}", median(&data));
    println!("Mode: {:?}", mode(&data));
}

fn demo_stack() {
    let mut stack = Stack::new();
    stack.push("first");
    stack.push("second");
    println!("Stack top: {:?}", stack.peek());
    println!("Pop -> {:?}", stack.pop());
    println!("Remaining: {:?}", stack.peek());
}

fn demo_summarize() {
    let items: Vec<Box<dyn Summarize>> = vec![
        Box::new(Article {
            title: "Generics in Rust".into(),
            author: "Ferris".into(),
            content: "Deep dive into trait bounds.".into(),
        }),
        Box::new(Tweet {
            username: "rustacean".into(),
            text: "Traits are powerful!".into(),
        }),
    ];

    for item in items {
        println!("Summary: {}", item.summarize());
    }
}
