use futures::{Async, Stream, Poll};
use futures::stream::Fuse;

#[cfg(test)]
mod tests {

    extern crate tokio_core;
    extern crate tokio_timer;
    use self::tokio_timer::*;
    use self::tokio_core::reactor::Core;

    use futures::Stream;
    use std::time::Duration;

    use zip_latest::zip_latest;

    #[test]
    /// Combine two timer streams that produce values independent of each other
    /// Output
    /// ```
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
    /// ```
    ///
    fn combine_two_timer_streams() {
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
    }
}

#[derive(Debug)]
/// get at most one error at a time.
#[must_use = "streams do nothing unless polled"]
pub struct ZipLatest<S1: Stream, S2: Stream> {
    stream1: Fuse<S1>,
    stream2: Fuse<S2>,
    queued1: Option<S1::Item>,
    queued2: Option<S2::Item>,
}

///
/// Combines the latest output of two streams.
/// It will produce a new tuple everytime any of the streams produces a new value.
/// The stream will produce `Option::None` values for streams that have not yet produced a value.
///
pub fn zip_latest<S1, S2, T1: Clone, T2: Clone>(stream1: S1, stream2: S2) -> ZipLatest<S1, S2>
where
    S1: Stream<Item = T1>,
    S2: Stream<Item = T2, Error = S1::Error>,
{
    ZipLatest {
        stream1: stream1.fuse(),
        stream2: stream2.fuse(),
        queued1: None,
        queued2: None,
    }
}

impl<S1, S2, T1: Clone, T2: Clone> Stream for ZipLatest<S1, S2>
where
    S1: Stream<Item = T1>,
    S2: Stream<Item = T2, Error = S1::Error>,
{
    type Item = (Option<S1::Item>, Option<S2::Item>);
    type Error = S1::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {

        match (self.stream1.poll()?, self.stream2.poll()?) {
            (Async::Ready(None), _) => Ok(Async::Ready(None)),
            (_, Async::Ready(None)) => Ok(Async::Ready(None)),
            (Async::Ready(Some(item1)), Async::Ready(Some(item2))) => {
                self.queued1 = Some(item1);
                self.queued2 = Some(item2);
                Ok(Some((self.queued1.clone(), self.queued2.clone())).into())
            },
            (Async::Ready(Some(item1)), Async::NotReady) => {
                self.queued1 = Some(item1);
                Ok(Some((self.queued1.clone(), self.queued2.clone())).into())
            },
            (Async::NotReady, Async::Ready(Some(item2))) => {
                self.queued2 = Some(item2);
                Ok(Some((self.queued1.clone(), self.queued2.clone())).into())
            },
            (Async::NotReady, Async::NotReady) => Ok(Async::NotReady)
        }
    }
}
