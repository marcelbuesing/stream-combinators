Additional combinators for [Tokio](https://github.com/tokio-rs/).

# ZipLatest

Combines the latest output of two streams.
It will produce a new tuple everytime any of the streams produces a new value.
The stream will produce `Option::None` values for streams that have not yet produced a value.

```Rust
        let mut a_counter = 0;
        let mut b_counter = 0;

        let a = Timer::default()
            .interval(Duration::from_millis(1000))
            .map(|_| {
                a_counter += 1;
                a_counter
            })
            // We'll stop the stream after 10 loops
            .take_while(|x| Ok(x <= &10));
        let b = Timer::default().interval(Duration::from_millis(5000)).map(
            |_| {
                b_counter += 1;
                b_counter
            },
        );

        let zl = zip_latest(a, b).for_each(|tuple| {
            println!("Tuple {:?}", tuple);
            Ok(())
        });

        let mut core = Core::new().unwrap();
        core.run(zl).unwrap();
```

Output
```
    ///    Tuple (Some(1), None)
    ///    Tuple (Some(2), None)
    ///    Tuple (Some(3), None)
    ///    Tuple (Some(4), None)
    ///    Tuple (Some(5), Some(1))
    ///    Tuple (Some(6), Some(1))
    ///    Tuple (Some(7), Some(1))
    ///    Tuple (Some(8), Some(1))
    ///    Tuple (Some(9), Some(1))
    ///    Tuple (Some(10), Some(2))
```
