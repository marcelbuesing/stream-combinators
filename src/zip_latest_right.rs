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

    use zip_latest_right::zip_latest_right;

    #[test]
    /// Combine two timer streams that produce values independent of each other
    /// Output
    /// ```
    ///  Tuple (Some(5), Some(1))
    ///  Tuple (Some(10), Some(2))
    ///  Tuple (Some(15), Some(3))
    ///  Tuple (Some(20), Some(4))
    ///  Tuple (Some(25), Some(5))
    /// ```
    ///
    fn combine_two_timer_streams() {
        let mut a_counter = 0;
        let mut b_counter = 0;

        let a = Timer::default()
            .interval(Duration::from_millis(1000))
            .map(|_| {
                a_counter += 1;
                //println!("Left {}", a_counter);
                a_counter
            })
            // We'll stop the stream after 10 loops
            .take_while(|x| Ok(x <= &30));
        let b = Timer::default().interval(Duration::from_millis(5000)).map(
            |_| {
                b_counter += 1;
                //println!("Right {}", b_counter);
                b_counter
            },
        );

        let zl = zip_latest_right(a, b).for_each(|tuple| {
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
pub struct ZipLatestRight<SLeft: Stream, SRight: Stream> {
    stream_left: Fuse<SLeft>,
    stream_right: Fuse<SRight>,
    /// Value that is currently published on the stream
    queued_left_live: Option<SLeft::Item>,
    /// Latest value on left stream this may not have been published yet on the stream
    queued_left_latest: Option<SLeft::Item>,
    queued_right: Option<SRight::Item>,
    flag_poll_right: bool,
}

///
/// Combines the latest output of two streams.
/// It will produce a new tuple everytime the right streams produces a new value,
/// containing the latest values of both streams.
///
pub fn zip_latest_right<SLeft, SRight, T1: Clone, T2: Clone>(
    stream_left: SLeft,
    stream_right: SRight,
) -> ZipLatestRight<SLeft, SRight>
where
    SLeft: Stream<Item = T1>,
    SRight: Stream<Item = T2, Error = SLeft::Error>,
{
    ZipLatestRight {
        stream_left: stream_left.fuse(),
        stream_right: stream_right.fuse(),
        queued_left_live: None,
        queued_left_latest: None,
        queued_right: None,
        flag_poll_right: true,
    }
}

impl<SLeft, SRight, T1: Clone, T2: Clone> Stream for ZipLatestRight<SLeft, SRight>
where
    SLeft: Stream<Item = T1>,
    SRight: Stream<
        Item = T2,
        Error = SLeft::Error,
    >,
{
    type Item = (Option<SLeft::Item>, Option<SRight::Item>);
    type Error = SLeft::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {

        match (self.stream_right.poll()?, self.stream_left.poll()?) {
            (Async::Ready(Some(item_right)), Async::Ready(Some(item_left))) => {
                self.queued_left_live = Some(item_left);
                self.queued_left_latest = self.queued_left_live.clone();
                self.queued_right = Some(item_right);
                Ok(Some((self.queued_left_latest.clone(), self.queued_right.clone())).into())
            },
            (Async::Ready(Some(item_right)), Async::NotReady) => {
                self.queued_right = Some(item_right);
                self.queued_left_live = self.queued_left_latest.clone();
                Ok(Some((self.queued_left_live.clone(), self.queued_right.clone())).into())
            },
            (Async::NotReady, Async::Ready(Some(item_left))) => {
                self.queued_left_latest = Some(item_left);
                Ok(Some((self.queued_left_live.clone(), self.queued_right.clone())).into())
            },
            (Async::NotReady, Async::NotReady) => Ok(Async::NotReady),
            (Async::Ready(None), _) => Ok(Async::Ready(None)),
            (_, Async::Ready(None)) => Ok(Async::Ready(None)),
        }
    }
}
