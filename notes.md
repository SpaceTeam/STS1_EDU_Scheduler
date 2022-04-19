### Structure
```rust
enum payload {
    // ...
}

let mut process_watchdog: Option<thread::JoinHandle<>> = None;

let p = preprocess(command, payload_path);
match command {
    StoreArchive => {
        store_archive(p);
    },
    ExecuteProgram => {
        execute_program(&mut process_watchdog, p);
    },
    // ...
}
```