#![allow(unused, reason = "wip")]
use bytes::Buf;
use http_body::{Body, Frame, SizeHint};
use pin_project_lite::pin_project;
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    #[project = BufBodyProj]
    struct BufBody<B: Body> {
        #[pin]
        inner: B,
        capacity: usize,
        buffer: VecDeque<B::Data>,
    }
}

struct Buffered<D>(VecDeque<D>);

// === impl BufBody ===

impl<B: Body> BufBody<B> {
    fn with_capacity(capacity: usize, body: B) -> Self {
        Self {
            inner: body,
            capacity,
            buffer: VecDeque::default(),
        }
    }
}

impl<B> Body for BufBody<B>
where
    B: Body,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let BufBodyProj {
            inner,
            capacity,
            buffer,
        } = self.project();

        inner.poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        todo!()
    }

    fn size_hint(&self) -> SizeHint {
        todo!()
    }
}

// === impl Buffered ===

impl<D: Buf> Buf for Buffered<D> {
    fn remaining(&self) -> usize {
        todo!()
    }

    fn chunk(&self) -> &[u8] {
        todo!()
    }

    fn advance(&mut self, cnt: usize) {
        todo!()
    }
}

#[cfg(test)]
mod buf_body_tests {
    use super::BufBody;
    use crate::Full;
    use bytes::Bytes;
    use http_body::Body;
    use std::{
        ops::Not,
        pin::Pin,
        task::{Context, Poll},
    };

    const HELLO_WORLD: &str = "hello world!";

    #[test]
    fn simple_body_above_capacity_passes_through() {
        let mut body = {
            let inner = Full::<Bytes>::from(HELLO_WORLD);
            BufBody::with_capacity(4, inner)
        };

        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        // TODO(kate): implement hints
        /*assert!(
            body.is_end_stream().not(),
            "body is not finished until polled"
        );
        assert_eq!(body.size_hint().lower(), HELLO_WORLD.len() as u64);
        assert_eq!(
            body.size_hint().upper(),
            Some(0),
            "empty bodies size hint is 0"
        );*/

        // The body yields the whole "hello world!" chunk because it exceeds the buffer capacity.
        match Pin::new(&mut body).poll_frame(&mut cx) {
            Poll::Ready(Some(Ok(frame))) => {
                let frame = frame.into_data().expect("should yield data");
                assert_eq!(frame, HELLO_WORLD);
            }
            other => panic!("unexpected poll outcome: {:?}", other),
        }

        // TODO(kate): implement hints
        // assert!(body.is_end_stream(), "body is finished after being polled");
    }
}

#[cfg(test)]
mod buffered_tests {
    use super::Buffered;
    use bytes::Buf;
    use std::{collections::VecDeque, io::Read};

    #[test]
    fn hello_world() {
        let chunks: Vec<&[u8]> = vec![b"hello ", b"world!"];
        let buffered = Buffered(VecDeque::from(chunks));

        let mut dst = vec![];
        let mut reader = buffered.reader();
        reader.read_to_end(&mut dst);

        assert_eq!(dst, b"hello world!");
    }
}
