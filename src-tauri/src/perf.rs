//! Debug-only command timing (docs/spec/polish-and-release.md §1). Release
//! builds compile this to the bare closure call.

pub(crate) fn timed<T>(name: &str, f: impl FnOnce() -> T) -> T {
    if !cfg!(debug_assertions) {
        return f();
    }
    let started = std::time::Instant::now();
    let out = f();
    eprintln!("perf: {name} {} ms", started.elapsed().as_millis());
    out
}

#[cfg(test)]
mod tests {
    use super::timed;

    #[test]
    fn returns_the_closure_result() {
        assert_eq!(timed("x", || 3 + 4), 7);
    }
}
