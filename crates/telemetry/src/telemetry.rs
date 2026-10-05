#[macro_export]
macro_rules! event {
    ($name:expr) => {{}};
    ($name:expr, $($key:ident $(= $value:expr)?),+ $(,)?) => {{}};
}

#[cfg(test)]
mod tests {
    #[test]
    fn event_does_not_evaluate_arguments() {
        let calls = std::cell::Cell::new(0);
        event!(
            {
                calls.set(calls.get() + 1);
                "name"
            },
            property = {
                calls.set(calls.get() + 1);
                "value"
            },
            shorthand,
        );
        event!({
            calls.set(calls.get() + 1);
            "name"
        });
        assert_eq!(calls.get(), 0);
    }
}
