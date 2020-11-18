A library for simple profiling your code with HTML reports as result.

# Usage
At first the rprofiler must be initialized by the call `rprofiler::PROFILER.initialize()` method.
This method is returned an object of ProfilerData struct, where will be gathering all runtime information.
Then you can use special `profile_block` macro for profiling blocks of your code. It has some syntax variations:
```
profile_block!();
profile_block!(name "name of code block");
// Conditional profiling
profile_block!(if_feature "feature name of your crate");
profile_block!(if_feature "feature name of your crate", name "name of code block");
```
This macro generates special internal events, which will be pushed to internal events queue.
You should call the `rprofiler::PROFILER.process_events(...)` method periodically to process this events and clear the queue.
As example, this method can be called at end of each game frame.

At end of profiling you should call the `rprofiler::PROFILER.shutdown(...)` method.
It will process all gathered information and save result as HTML document into specified file.

You can disable all profiling at compile-time by enabling a feature *"disable_profiling"* in *Cargo.toml* of your project.
```toml
[package]
name = "game"
version = "0.1.0"
edition = "2018"

[dependencies.rprofiler]
version = "0.2"
features = ["disable_profiling"]
```

# Examples
```
fn factorial(value: i32) -> i32 {
    match value > 1 {
        true => value*factorial(value - 1),
        false => 1,
    }
}

fn test_func() -> i32 {
    profile_block!();
    (0..10).map(|i| factorial(i)).sum()
}

fn main() {
    let mut profiler_data = PROFILER.initialize();

    for _ in 0..1000 {
        for _ in 0..1_000_000 {
            test_func();
        }
        PROFILER.process_events(&mut profiler_data);
    }

    PROFILER.shutdown("./profiler_report.html", &mut profiler_data);
}
```
